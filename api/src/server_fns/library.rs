use dioxus::prelude::*;
use shared::library::AlbumEntry;

#[cfg(feature = "server")]
use super::server_error;
#[cfg(feature = "server")]
use crate::AuthSession;
#[cfg(feature = "server")]
use std::collections::HashMap;
#[cfg(feature = "server")]
use std::path::Path;
#[cfg(feature = "server")]
use tokio::io::AsyncWriteExt;
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
    let rows: Vec<(String, String, String)> = tokio::task::spawn_blocking(move || {
        let db = rusqlite::Connection::open(&beets_db)?;
        let mut stmt = db.prepare(
            "SELECT CAST(path AS TEXT), albumartist, album FROM items WHERE albumartist != '' AND album != ''"
        )?;
        let rows: Vec<(String, String, String)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .map(|(rel_path, artist, album)| {
                let full_path = if rel_path.starts_with('/') {
                    rel_path
                } else {
                    format!("{}/{}", library_dir_clone, rel_path)
                };
                (full_path, artist, album)
            })
            .collect();
        Ok::<_, rusqlite::Error>(rows)
    })
    .await
    .map_err(|e| server_error(format!("Task error: {}", e)))?
    .map_err(|e| server_error(format!("Failed to query beets DB: {}", e)))?;

    let mut album_map: HashMap<(String, String), (usize, String)> = HashMap::new();

    for (path, artist, album) in rows {
        let album_dir = Path::new(&path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| format!("{}/{}/{}", library_dir, artist, album));

        let entry = album_map
            .entry((artist.clone(), album.clone()))
            .or_insert((0, album_dir));
        entry.0 += 1;
    }

    let mut albums: Vec<AlbumEntry> = album_map
        .into_iter()
        .map(|((artist, album), (track_count, album_path))| AlbumEntry {
            artist,
            album,
            track_count,
            album_path,
        })
        .collect();

    albums.sort_by(|a, b| a.artist.cmp(&b.artist).then(a.album.cmp(&b.album)));

    Ok(albums)
}

#[delete("/api/library/album", auth: AuthSession)]
pub async fn delete_library_album(
    album_path: String,
    artist: String,
    album: String,
) -> Result<(), ServerFnError> {
    let _claims = auth.0;

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

    // 4. Remove from Navidrome DB directly via SQLite
    if let Ok(navidrome_db) = std::env::var("NAVIDROME_DB_PATH") {
        let album_name = album.clone();
        let artist_name = artist.clone();
        tokio::task::spawn_blocking(move || {
            let db = rusqlite::Connection::open(&navidrome_db)?;
            // Find album ID by name + artist
            let album_id: Option<String> = db
                .query_row(
                    "SELECT id FROM album WHERE name = ?1 AND album_artist = ?2",
                    rusqlite::params![album_name, artist_name],
                    |row| row.get(0),
                )
                .ok();
            if let Some(id) = album_id {
                db.execute("DELETE FROM media_file WHERE album_id = ?1", rusqlite::params![id])?;
                db.execute("DELETE FROM album WHERE id = ?1", rusqlite::params![id])?;
            }
            Ok::<_, rusqlite::Error>(())
        })
        .await
        .ok();
    }

    Ok(())
}
