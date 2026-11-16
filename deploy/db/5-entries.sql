-- v4 → v5：账目有自己的 id（截图来的用 recognition_id 指向识别），加转账、平账；所有表改 STRICT；
-- entry_tags 随账目级联删除；时间列换成 UTC。
-- 每张表按新定义建 <表>_new、复制、删旧表、改名。外键写最终表名，改名后自然对上。
-- 旧库里有小数或非法值时复制报错，convert.py 整体回滚。

CREATE TABLE categories_new (
  id        INTEGER PRIMARY KEY,
  position  INTEGER NOT NULL,
  kind      TEXT NOT NULL CHECK (kind IN ('expense', 'income')),
  parent_id INTEGER REFERENCES categories(id),
  name      TEXT NOT NULL,
  key       TEXT NOT NULL UNIQUE
) STRICT;
INSERT INTO categories_new SELECT id, position, kind, parent_id, name, key FROM categories;

CREATE TABLE accounts_new (
  id          INTEGER PRIMARY KEY,
  position    INTEGER NOT NULL,
  name        TEXT NOT NULL UNIQUE,
  card_type   TEXT CHECK (card_type IN ('debit', 'credit')),
  card_number TEXT,
  CHECK ((card_type IS NULL) = (card_number IS NULL))
) STRICT;
INSERT INTO accounts_new SELECT id, position, name, card_type, card_number FROM accounts;

CREATE TABLE tags_new (
  id   INTEGER PRIMARY KEY,
  name TEXT NOT NULL UNIQUE
) STRICT;
INSERT INTO tags_new SELECT id, name FROM tags;

CREATE TABLE recognitions_new (
  draft_id     TEXT PRIMARY KEY,
  outcome      TEXT NOT NULL CHECK (outcome IN ('ok', 'rejected', 'error')),
  model        TEXT NOT NULL,
  upload_ms    INTEGER NOT NULL,
  model_ms     INTEGER NOT NULL,
  image_sha256 TEXT NOT NULL,
  image_bytes  INTEGER NOT NULL,
  image_path   TEXT NOT NULL,
  prompt       TEXT NOT NULL,
  parameters   TEXT NOT NULL,
  raw          TEXT NOT NULL,
  reason       TEXT,
  origin       TEXT NOT NULL
) STRICT;
INSERT INTO recognitions_new
SELECT draft_id, outcome, model, upload_ms, model_ms, image_sha256, image_bytes, image_path,
       prompt, parameters, raw, reason, origin
FROM recognitions;

CREATE TABLE entries_new (
  id             TEXT PRIMARY KEY,
  recognition_id TEXT UNIQUE REFERENCES recognitions(draft_id),
  kind           TEXT NOT NULL CHECK (kind IN ('expense', 'income', 'transfer', 'adjust')),
  amount_cents   INTEGER NOT NULL CHECK (amount_cents > 0 OR (kind = 'adjust' AND amount_cents <> 0)),
  category_id    INTEGER REFERENCES categories(id),
  account_id     INTEGER NOT NULL REFERENCES accounts(id),
  to_account_id  INTEGER REFERENCES accounts(id),
  merchant       TEXT,
  occurred_at    TEXT NOT NULL,
  note           TEXT,
  created_at     TEXT NOT NULL,
  CHECK ((kind IN ('expense', 'income')) = (category_id IS NOT NULL)),
  CHECK ((kind = 'transfer') = (to_account_id IS NOT NULL)),
  CHECK (to_account_id <> account_id)
) STRICT;
INSERT INTO entries_new
SELECT id, id, kind, amount_cents, category_id, account_id, NULL, merchant,
       strftime('%Y-%m-%dT%H:%M:%SZ', occurred_at), note, strftime('%Y-%m-%dT%H:%M:%SZ', created_at)
FROM entries;

CREATE TABLE entry_tags_new (
  entry_id TEXT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
  tag_id   INTEGER NOT NULL REFERENCES tags(id),
  PRIMARY KEY (entry_id, tag_id)
) STRICT;
INSERT INTO entry_tags_new SELECT entry_id, tag_id FROM entry_tags;

CREATE TABLE requests_new (
  id         INTEGER PRIMARY KEY,
  at         TEXT NOT NULL,
  method     TEXT NOT NULL,
  path       TEXT NOT NULL,
  status     INTEGER NOT NULL,
  elapsed_ms INTEGER NOT NULL,
  draft_id   TEXT,
  field      TEXT,
  value      TEXT,
  error      TEXT
) STRICT;
INSERT INTO requests_new
SELECT id, strftime('%Y-%m-%dT%H:%M:%SZ', at), method, path, status, elapsed_ms, draft_id, field,
       value, error
FROM requests;

CREATE TABLE settings_new (
  id           INTEGER PRIMARY KEY CHECK (id = 1),
  cycle_day    INTEGER NOT NULL DEFAULT 1 CHECK (cycle_day BETWEEN 1 AND 28),
  budget_cents INTEGER CHECK (budget_cents > 0)
) STRICT;
INSERT INTO settings_new SELECT id, cycle_day, budget_cents FROM settings;

DROP TABLE settings;
DROP TABLE requests;
DROP TABLE entry_tags;
DROP TABLE entries;
DROP TABLE recognitions;
DROP TABLE tags;
DROP TABLE accounts;
DROP TABLE categories;

ALTER TABLE categories_new RENAME TO categories;
ALTER TABLE accounts_new RENAME TO accounts;
ALTER TABLE tags_new RENAME TO tags;
ALTER TABLE recognitions_new RENAME TO recognitions;
ALTER TABLE entries_new RENAME TO entries;
ALTER TABLE entry_tags_new RENAME TO entry_tags;
ALTER TABLE requests_new RENAME TO requests;
ALTER TABLE settings_new RENAME TO settings;

CREATE UNIQUE INDEX idx_categories_name
  ON categories(kind, coalesce(parent_id, 0), name);
CREATE INDEX idx_entries_occurred ON entries(occurred_at);
CREATE INDEX idx_requests_draft ON requests(draft_id);
