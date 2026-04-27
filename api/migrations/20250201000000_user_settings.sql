CREATE TABLE IF NOT EXISTS user_settings (
    user_id TEXT PRIMARY KEY NOT NULL,
    default_metadata_provider TEXT DEFAULT 'musicbrainz',
    last_search_type TEXT DEFAULT 'track',
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

INSERT OR IGNORE INTO user_settings (user_id)
SELECT id FROM users;
