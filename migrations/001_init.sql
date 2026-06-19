CREATE TABLE IF NOT EXISTS reports (
    id TEXT PRIMARY KEY,
    filename TEXT NOT NULL,
    uploaded_at TEXT NOT NULL,
    raw_text TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS indicators (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    report_id TEXT NOT NULL REFERENCES reports(id),
    category TEXT NOT NULL,
    name TEXT NOT NULL,
    value TEXT NOT NULL,
    unit TEXT NOT NULL DEFAULT '',
    reference_range TEXT NOT NULL DEFAULT '',
    is_abnormal INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_indicators_report_id ON indicators(report_id);
CREATE INDEX IF NOT EXISTS idx_indicators_category ON indicators(category);
CREATE INDEX IF NOT EXISTS idx_indicators_is_abnormal ON indicators(is_abnormal);
