-- Add profile column to discovery_candidates
ALTER TABLE discovery_candidates ADD COLUMN profile TEXT NOT NULL DEFAULT 'Balanced';

-- Add profile column to discovery_tracks
ALTER TABLE discovery_tracks ADD COLUMN profile TEXT NOT NULL DEFAULT 'Balanced';

-- Update unique constraint on discovery_candidates to include profile
-- SQLite can't alter constraints, so recreate the table
CREATE TABLE discovery_candidates_new (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    artist TEXT NOT NULL,
    track TEXT NOT NULL,
    album TEXT,
    score REAL NOT NULL,
    signals TEXT NOT NULL DEFAULT '[]',
    source TEXT NOT NULL,
    profile TEXT NOT NULL DEFAULT 'Balanced',
    used INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(user_id, profile, artist, track)
);

INSERT INTO discovery_candidates_new (id, user_id, artist, track, album, score, signals, source, profile, used, created_at)
SELECT id, user_id, artist, track, album, score, signals, source, 'Balanced', used, created_at FROM discovery_candidates;

DROP TABLE discovery_candidates;
ALTER TABLE discovery_candidates_new RENAME TO discovery_candidates;
