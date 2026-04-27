CREATE TABLE IF NOT EXISTS engine_reports (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    profile TEXT NOT NULL,
    report_json TEXT NOT NULL,
    candidate_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
