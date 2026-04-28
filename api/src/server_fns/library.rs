use dioxus::prelude::*;
use shared::library::AlbumEntry;

#[cfg(feature = "server")]
use super::server_error;
#[cfg(feature = "server")]
use crate::AuthSession;
#[cfg(feature = "server")]
use crate::services::navidrome_client_for_user;
#[cfg(feature = "server")]
use std::collections::HashMap;
#[cfg(feature = "server")]
use std::path::Path;
#[cfg(feature = "server")]
use tokio::process::Command;
#[cfg(feature = "server")]
use rusqlite;

#[get("/api/library", auth: AuthSession)]
pub async fn get_library() -> Result<Vec<AlbumEntry>, ServerFnError> {
    let _claims = auth.0;

    let library_dir = std::env::var("BEETS_LIBRARY_DIR")
        .unwrap_or_else(|_| "/app/library".to_string());

    let beets_db = std::env::var("BEETS_DB")
        .unwrap_or_else(|_| "/data/musiclibrary.db".to_string());

    let library_dir_clone = library_dir.clone();
    let rows: Vec<(String, String, String, f64)> = tokio::task::spawn_blocking(move || {
        let db = rusqlite::Connection::open(&beets_db)?;
        let mut stmt = db.prepare(
            "SELECT CAST(path AS TEXT), albumartist, album, added FROM items WHERE albumartist != '' AND album != ''"
        )?;
        let rows: Vec<(String, String, String, f64)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, f64>(3).unwrap_or(0.0),
                ))
            })?
            .filter_map(|r| r.ok())
            .map(|(rel_path, artist, album, added)| {
                let full_path = if rel_path.starts_with('/') {
                    rel_path
                } else {
                    format!("{}/{}", library_dir_clone, rel_path)
                };
                (full_path, artist, album, added)
            })
            .collect();
        Ok::<_, rusqlite::Error>(rows)
    })
    .await
    .map_err(|e| server_error(format!("Task error: {}", e)))?
    .map_err(|e| server_error(format!("Failed to query beets DB: {}", e)))?;

    let mut album_map: HashMap<(String, String), (usize, String, f64)> = HashMap::new();

    for (path, artist, album, added) in rows {
        let album_dir = Path::new(&path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| format!("{}/{}/{}", library_dir, artist, album));

        let entry = album_map
            .entry((artist.clone(), album.clone()))
            .or_insert((0, album_dir, added));
        entry.0 += 1;
        if added > entry.2 {
            entry.2 = added;
        }
    }

    let albums: Vec<AlbumEntry> = album_map
        .into_iter()
        .map(|((artist, album), (track_count, album_path, added))| AlbumEntry {
            artist,
            album,
            track_count,
            album_path,
            added,
        })
        .collect();

    Ok(albums)
}

#[delete("/api/library/album", auth: AuthSession)]
pub async fn delete_library_album(
    album_path: String,
    artist: String,
    album: String,
) -> Result<(), ServerFnError> {
    let claims = auth.0;

    // 1. Delete files from disk
    tokio::fs::remove_dir_all(&album_path)
        .await
        .map_err(|e| server_error(format!("Failed to delete album directory: {}", e)))?;

    // 2. Remove from beets DB directly via SQLite
    let beets_db = std::env::var("BEETS_DB")
        .unwrap_or_else(|_| "/data/musiclibrary.db".to_string());
    let library_dir = std::env::var("BEETS_LIBRARY_DIR")
        .unwrap_or_else(|_| "/app/library".to_string());
    let album_path_clone = album_path.clone();
    let artist_clone = artist.clone();
    let album_clone = album.clone();
    tokio::task::spawn_blocking(move || {
        let db = rusqlite::Connection::open(&beets_db)?;
        // Strip library dir prefix to get relative path prefix stored in DB
        let rel_prefix = album_path_clone
            .strip_prefix(&format!("{}/", library_dir))
            .unwrap_or(&album_path_clone)
            .to_string();
        // Delete by relative path prefix first, fall back to artist+album
        let deleted = db.execute(
            "DELETE FROM items WHERE CAST(path AS TEXT) LIKE ?1 OR (albumartist = ?2 AND album = ?3)",
            rusqlite::params![format!("{}%", rel_prefix), artist_clone, album_clone],
        )?;
        tracing::info!("Removed {} items from beets DB", deleted);
        Ok::<_, rusqlite::Error>(())
    })
    .await
    .ok();

    // 3. Clear beets incremental import history for this album
    //    state.pickle taghistory stores original download paths — match on album folder name
    let album_folder = Path::new(&album_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let _ = Command::new("python3")
        .arg("-c")
        .arg(format!(
            r#"
import pickle, os
state_path = "/root/.config/beets/state.pickle"
if not os.path.exists(state_path):
    exit(0)
with open(state_path, "rb") as f:
    state = pickle.load(f)
history = state.get("taghistory", set())
before = len(history)
history = {{e for e in history if not any(part.decode("utf-8", errors="replace").split("/")[-1] == "{}" for part in e if isinstance(part, bytes))}}
state["taghistory"] = history
with open(state_path, "wb") as f:
    pickle.dump(state, f)
print(f"Cleared {{before - len(history)}} entries from taghistory")
"#,
            album_folder
        ))
        .output()
        .await;

    // 4. Trigger a Navidrome library scan so it removes the now-missing files from its DB
    if let Ok(client) = navidrome_client_for_user(&claims.sub).await {
        if let Err(e) = client.start_scan().await {
            tracing::warn!("Failed to trigger Navidrome scan after delete: {}", e);
        }
    }

    Ok(())
}

/// Consolidate a split album: find all directories in the beets DB that contain tracks
/// for the given (albumartist, album), pick the one with the most tracks as canonical,
/// move all other tracks into it, update beets DB paths, then trigger a Navidrome scan.
/// This fixes the case where Navidrome shows the same album as multiple entries because
/// beets placed tracks in more than one directory during import.
#[post("/api/library/album/consolidate", auth: AuthSession)]
pub async fn consolidate_album(
    artist: String,
    album: String,
) -> Result<(), ServerFnError> {
    let claims = auth.0;

    let beets_db = std::env::var("BEETS_DB")
        .unwrap_or_else(|_| "/data/musiclibrary.db".to_string());
    let library_dir = std::env::var("BEETS_LIBRARY_DIR")
        .unwrap_or_else(|_| "/app/library".to_string());

    let artist_c = artist.clone();
    let album_c = album.clone();
    let lib_c = library_dir.clone();

    // 1. Find all unique directories containing tracks for this album, with their track counts.
    //    Returns (abs_path, rel_path, count) sorted descending so [0] is the canonical dir.
    let dirs: Vec<(String, String, usize)> = tokio::task::spawn_blocking(move || {
        let db = rusqlite::Connection::open(&beets_db)?;
        let mut stmt = db.prepare(
            "SELECT CAST(path AS TEXT) FROM items \
             WHERE albumartist = ?1 AND album = ?2",
        )?;
        let paths: Vec<String> = stmt
            .query_map(rusqlite::params![artist_c, album_c], |row| {
                row.get::<_, String>(0)
            })?
            .filter_map(|r| r.ok())
            .collect();

        // Build a map of dir → count, storing both abs and rel versions
        let mut dir_map: std::collections::HashMap<String, (String, usize)> =
            std::collections::HashMap::new();
        let lib_prefix = format!("{}/", lib_c);
        for p in &paths {
            let abs = if p.starts_with('/') {
                p.clone()
            } else {
                format!("{}/{}", lib_c, p)
            };
            let abs_dir = std::path::Path::new(&abs)
                .parent()
                .map(|d| d.to_string_lossy().to_string())
                .unwrap_or_default();
            let rel_dir = abs_dir
                .strip_prefix(&lib_prefix)
                .unwrap_or(&abs_dir)
                .to_string();
            let e = dir_map.entry(abs_dir.clone()).or_insert((rel_dir, 0));
            e.1 += 1;
        }

        let mut dirs: Vec<(String, String, usize)> = dir_map
            .into_iter()
            .map(|(abs, (rel, cnt))| (abs, rel, cnt))
            .collect();
        // Canonical = most tracks first
        dirs.sort_by(|a, b| b.2.cmp(&a.2));
        Ok::<_, rusqlite::Error>(dirs)
    })
    .await
    .map_err(|e| server_error(format!("Task error: {}", e)))?
    .map_err(|e| server_error(format!("DB error: {}", e)))?;

    if dirs.len() <= 1 {
        // Already consolidated — still trigger a scan in case Navidrome is stale
        if let Ok(client) = navidrome_client_for_user(&claims.sub).await {
            let _ = client.start_scan().await;
        }
        return Ok(());
    }

    let (canonical_abs, canonical_rel, _) = &dirs[0];

    // 2. Move files from every minority directory into the canonical one, update DB paths.
    for (src_abs, src_rel, _) in dirs.iter().skip(1) {
        // Move files
        if let Ok(mut rd) = tokio::fs::read_dir(src_abs).await {
            while let Ok(Some(entry)) = rd.next_entry().await {
                let ft = entry.file_type().await;
                if !matches!(ft, Ok(ft) if ft.is_file()) {
                    continue;
                }
                let src_file = entry.path();
                let name = src_file
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let mut dest = Path::new(canonical_abs).join(&name);
                if dest.exists() {
                    let stem = Path::new(&name)
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let ext = Path::new(&name)
                        .extension()
                        .map(|e| format!(".{}", e.to_string_lossy()))
                        .unwrap_or_default();
                    dest = Path::new(canonical_abs).join(format!("{}_2{}", stem, ext));
                }
                let _ = tokio::fs::rename(&src_file, &dest).await;
            }
        }

        // Update beets DB: rewrite path prefix for tracks that were in src_abs
        let beets_db2 = std::env::var("BEETS_DB")
            .unwrap_or_else(|_| "/data/musiclibrary.db".to_string());
        let src_rel_c = src_rel.clone();
        let canonical_rel_c = canonical_rel.clone();
        tokio::task::spawn_blocking(move || {
            let db = rusqlite::Connection::open(&beets_db2)?;
            db.execute(
                "UPDATE items SET \
                    path = CAST(replace(CAST(path AS TEXT), ?1, ?2) AS BLOB) \
                 WHERE CAST(path AS TEXT) LIKE ?3",
                rusqlite::params![
                    src_rel_c,
                    canonical_rel_c,
                    format!("{}%", src_rel_c),
                ],
            )?;
            Ok::<_, rusqlite::Error>(())
        })
        .await
        .ok();

        // Remove the now-empty source directory
        let _ = tokio::fs::remove_dir(src_abs).await;
    }

    // 3. Trigger Navidrome scan so it picks up the consolidated directory
    if let Ok(client) = navidrome_client_for_user(&claims.sub).await {
        if let Err(e) = client.start_scan().await {
            tracing::warn!("Failed to trigger Navidrome scan after consolidate: {}", e);
        }
    }

    Ok(())
}
