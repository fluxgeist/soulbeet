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

    let config_path =
        std::env::var("BEETS_CONFIG").unwrap_or_else(|_| "beets_config.yaml".to_string());

    let output = Command::new("beet")
        .arg("-c")
        .arg(&config_path)
        .arg("ls")
        .arg("-f")
        .arg("$path|||$albumartist|||$album")
        .output()
        .await
        .map_err(|e| server_error(format!("Failed to query beets library: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut album_map: HashMap<(String, String), (usize, String)> = HashMap::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split("|||").collect();
        if parts.len() < 3 {
            continue;
        }
        let path = parts[0].trim().to_string();
        let artist = parts[1].trim().to_string();
        let album = parts[2].trim().to_string();

        let album_dir = Path::new(&path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

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

    // 2. Remove from beets DB
    let config_path =
        std::env::var("BEETS_CONFIG").unwrap_or_else(|_| "beets_config.yaml".to_string());

    let query = format!("albumartist:{} album:{}", artist, album);

    let mut child = Command::new("beet")
        .arg("-c")
        .arg(&config_path)
        .arg("remove")
        .arg(&query)
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| server_error(format!("Failed to spawn beet remove: {}", e)))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(b"y\n")
            .await
            .map_err(|e| server_error(format!("Failed to write to beet stdin: {}", e)))?;
    }

    child
        .wait()
        .await
        .map_err(|e| server_error(format!("beet remove failed: {}", e)))?;

    // 3. Remove from Navidrome DB directly via SQLite
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
