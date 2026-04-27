-- Deletion history (auto-delete logs)
CREATE TABLE IF NOT EXISTS deletion_reviews (
    id TEXT PRIMARY KEY NOT NULL,
    song_id TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    artist TEXT NOT NULL,
    album TEXT NOT NULL,
    path TEXT,
    rating INTEGER,
    action TEXT NOT NULL DEFAULT 'Deleted',
    user_id TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Artist recommendations (per-user)
CREATE TABLE IF NOT EXISTS recommendations (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL DEFAULT '',
    artist TEXT NOT NULL,
    similar_to TEXT NOT NULL,
    similarity_score REAL NOT NULL,
    top_album TEXT,
    cover_url TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    status TEXT NOT NULL DEFAULT 'New',
    UNIQUE(user_id, artist)
);

-- Discovery playlists (per-folder, inherits user scope via folder)
CREATE TABLE IF NOT EXISTS discovery_playlists (
    id TEXT PRIMARY KEY NOT NULL,
    folder_id TEXT NOT NULL,
    navidrome_playlist_id TEXT,
    track_count INTEGER NOT NULL DEFAULT 20,
    lifetime_days INTEGER NOT NULL DEFAULT 7,
    last_generated_at TEXT,
    FOREIGN KEY (folder_id) REFERENCES folders(id) ON DELETE CASCADE,
    UNIQUE(folder_id)
);

-- Discovery tracks
CREATE TABLE IF NOT EXISTS discovery_tracks (
    id TEXT PRIMARY KEY NOT NULL,
    song_id TEXT,
    title TEXT NOT NULL,
    artist TEXT NOT NULL,
    album TEXT NOT NULL,
    path TEXT NOT NULL,
    folder_id TEXT NOT NULL,
    rating INTEGER,
    status TEXT NOT NULL DEFAULT 'Pending',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (folder_id) REFERENCES folders(id) ON DELETE CASCADE
);

-- Per-user Navidrome credentials
ALTER TABLE users ADD COLUMN navidrome_token TEXT;
ALTER TABLE users ADD COLUMN navidrome_status TEXT NOT NULL DEFAULT 'unknown';

-- Per-user settings (auto-delete, lastfm, promote threshold, banner state)
ALTER TABLE user_settings ADD COLUMN auto_delete_enabled INTEGER NOT NULL DEFAULT 0;
ALTER TABLE user_settings ADD COLUMN lastfm_api_key TEXT;
ALTER TABLE user_settings ADD COLUMN discovery_promote_threshold INTEGER NOT NULL DEFAULT 3;
ALTER TABLE user_settings ADD COLUMN navidrome_banner_dismissed INTEGER NOT NULL DEFAULT 0;

-- Clean up global keys that moved to per-user
DELETE FROM app_config WHERE key IN (
    'navidrome_url', 'navidrome_username', 'navidrome_password',
    'auto_delete_enabled', 'discovery_promote_threshold', 'lastfm_api_key'
);
