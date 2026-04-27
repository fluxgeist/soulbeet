-- Cached user music profiles
CREATE TABLE IF NOT EXISTS user_profiles (
    user_id TEXT PRIMARY KEY NOT NULL,
    profile_json TEXT NOT NULL DEFAULT '{}',
    top_artists_hash TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Cached recommendation candidates
CREATE TABLE IF NOT EXISTS discovery_candidates (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    artist TEXT NOT NULL,
    track TEXT NOT NULL,
    album TEXT,
    score REAL NOT NULL,
    signals TEXT NOT NULL DEFAULT '[]',
    source TEXT NOT NULL,
    used INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(user_id, artist, track)
);

-- Add discovery profile to discovery_playlists
ALTER TABLE discovery_playlists ADD COLUMN profile TEXT NOT NULL DEFAULT 'Balanced';

-- Add ListenBrainz credentials to user_settings
ALTER TABLE user_settings ADD COLUMN listenbrainz_username TEXT;
ALTER TABLE user_settings ADD COLUMN listenbrainz_token TEXT;

-- Drop old recommendations table (replaced by discovery_candidates)
DROP TABLE IF EXISTS recommendations;
