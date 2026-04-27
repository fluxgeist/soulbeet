-- Update existing single-profile values to all-profiles default
UPDATE discovery_playlists SET profile = 'Conservative,Balanced,Adventurous' WHERE profile = 'Balanced';
