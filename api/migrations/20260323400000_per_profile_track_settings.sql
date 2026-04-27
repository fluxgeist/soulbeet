-- Convert global discovery_track_count (integer) to per-profile JSON map.
-- Splits the total evenly across all three profiles.
UPDATE user_settings
SET discovery_track_count = json_object(
    'Conservative', discovery_track_count / 3,
    'Balanced', discovery_track_count / 3,
    'Adventurous', discovery_track_count - (discovery_track_count / 3) * 2
)
WHERE typeof(discovery_track_count) = 'integer';

-- Convert global discovery_lifetime_days (integer) to per-profile JSON map.
-- Each profile gets the same lifetime as the original global value.
UPDATE user_settings
SET discovery_lifetime_days = json_object(
    'Conservative', discovery_lifetime_days,
    'Balanced', discovery_lifetime_days,
    'Adventurous', discovery_lifetime_days
)
WHERE typeof(discovery_lifetime_days) = 'integer';
