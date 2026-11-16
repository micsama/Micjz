-- 适用：仅有 entries / entry_tags 的当前生产旧库。只执行一次。
BEGIN IMMEDIATE;
ALTER TABLE entries ADD COLUMN draft_id TEXT;
UPDATE entries SET draft_id = id;

CREATE TABLE IF NOT EXISTS entries (
  id           TEXT PRIMARY KEY,
  kind         TEXT NOT NULL CHECK (kind IN ('expense', 'income')),
  amount_cents INTEGER NOT NULL CHECK (amount_cents > 0),
  category     TEXT NOT NULL,
  subcategory  TEXT,
  account      TEXT,
  merchant     TEXT,
  occurred_at  TEXT NOT NULL,
  note         TEXT,
  created_at   TEXT NOT NULL,
  draft_id     TEXT
);
CREATE INDEX IF NOT EXISTS idx_entries_occurred ON entries(occurred_at);

CREATE TABLE IF NOT EXISTS entry_tags (
  entry_id TEXT NOT NULL REFERENCES entries(id),
  tag      TEXT NOT NULL,
  PRIMARY KEY (entry_id, tag)
);

CREATE TABLE IF NOT EXISTS recognitions (
 draft_id TEXT PRIMARY KEY, at TEXT NOT NULL, outcome TEXT NOT NULL,
 model TEXT NOT NULL, elapsed_ms INTEGER, image_sha256 TEXT, image_bytes INTEGER,
 image_path TEXT, prompt TEXT NOT NULL, parameters TEXT NOT NULL,
 raw TEXT, reason TEXT, origin TEXT, timings TEXT, confirmed_at TEXT
);


COMMIT;
