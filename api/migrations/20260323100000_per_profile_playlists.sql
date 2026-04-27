-- Migrate single playlist name/id to per-profile JSON
-- Old: "Soulful Finds" -> New: {"Conservative":"Comfort Zone","Balanced":"Fresh Picks","Adventurous":"Deep Cuts"}
UPDATE user_settings SET discovery_playlist_name = '{"Conservative":"Comfort Zone","Balanced":"Fresh Picks","Adventurous":"Deep Cuts"}' WHERE discovery_playlist_name = 'Soulful Finds';
UPDATE user_settings SET discovery_navidrome_playlist_id = '{}' WHERE discovery_navidrome_playlist_id IS NOT NULL;
