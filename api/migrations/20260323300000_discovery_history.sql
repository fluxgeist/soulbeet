CREATE TABLE IF NOT EXISTS discovery_history (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    artist TEXT NOT NULL,
    track TEXT NOT NULL,
    profile TEXT NOT NULL,
    outcome TEXT NOT NULL DEFAULT 'suggested',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_discovery_history_user_artist_track
ON discovery_history(user_id, lower(artist), lower(track));
