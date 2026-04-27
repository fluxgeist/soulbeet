-- Move discovery config from per-folder to per-user
-- Add discovery columns to user_settings
ALTER TABLE user_settings ADD COLUMN discovery_enabled INTEGER NOT NULL DEFAULT 0;
ALTER TABLE user_settings ADD COLUMN discovery_folder_id TEXT;
ALTER TABLE user_settings ADD COLUMN discovery_track_count INTEGER NOT NULL DEFAULT 20;
ALTER TABLE user_settings ADD COLUMN discovery_lifetime_days INTEGER NOT NULL DEFAULT 7;
ALTER TABLE user_settings ADD COLUMN discovery_profiles TEXT NOT NULL DEFAULT 'Conservative,Balanced,Adventurous';
ALTER TABLE user_settings ADD COLUMN discovery_playlist_name TEXT NOT NULL DEFAULT 'Soulful Finds';
ALTER TABLE user_settings ADD COLUMN discovery_navidrome_playlist_id TEXT;
ALTER TABLE user_settings ADD COLUMN discovery_last_generated_at TEXT;

-- Migrate existing discovery_playlists data to user_settings
-- For each user that had a discovery playlist, enable discovery and copy settings
UPDATE user_settings SET
    discovery_enabled = 1,
    discovery_folder_id = (SELECT dp.folder_id FROM discovery_playlists dp JOIN folders f ON dp.folder_id = f.id WHERE f.user_id = user_settings.user_id LIMIT 1),
    discovery_track_count = COALESCE((SELECT dp.track_count FROM discovery_playlists dp JOIN folders f ON dp.folder_id = f.id WHERE f.user_id = user_settings.user_id LIMIT 1), 20),
    discovery_lifetime_days = COALESCE((SELECT dp.lifetime_days FROM discovery_playlists dp JOIN folders f ON dp.folder_id = f.id WHERE f.user_id = user_settings.user_id LIMIT 1), 7),
    discovery_profiles = COALESCE((SELECT dp.profile FROM discovery_playlists dp JOIN folders f ON dp.folder_id = f.id WHERE f.user_id = user_settings.user_id LIMIT 1), 'Conservative,Balanced,Adventurous'),
    discovery_navidrome_playlist_id = (SELECT dp.navidrome_playlist_id FROM discovery_playlists dp JOIN folders f ON dp.folder_id = f.id WHERE f.user_id = user_settings.user_id LIMIT 1),
    discovery_last_generated_at = (SELECT dp.last_generated_at FROM discovery_playlists dp JOIN folders f ON dp.folder_id = f.id WHERE f.user_id = user_settings.user_id LIMIT 1)
WHERE EXISTS (SELECT 1 FROM discovery_playlists dp JOIN folders f ON dp.folder_id = f.id WHERE f.user_id = user_settings.user_id);

-- Drop old per-folder discovery playlists table
DROP TABLE IF EXISTS discovery_playlists;
