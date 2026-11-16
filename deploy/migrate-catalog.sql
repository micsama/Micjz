-- 适用：已执行过 migrate-retention.sql、entries 仍以文本存分类 / 账户 / 标签的库。只执行一次。
-- 分类、账户、标签挪进各自的表，账目改为按 id 引用。在仓库根目录执行：
--   sqlite3 data/micjz.db < deploy/migrate-catalog.sql
-- 旧账目里有初始数据之外的分类会因 category_id 为空而整体回滚，先补进 categories 再执行。
PRAGMA foreign_keys = OFF;
BEGIN IMMEDIATE;

CREATE TABLE categories (
  id        INTEGER PRIMARY KEY,
  kind      TEXT NOT NULL CHECK (kind IN ('expense', 'income')),
  parent_id INTEGER REFERENCES categories(id),
  name      TEXT NOT NULL,
  key       TEXT NOT NULL UNIQUE
);
CREATE UNIQUE INDEX idx_categories_name ON categories(kind, coalesce(parent_id, 0), name);
CREATE TABLE accounts (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE);
CREATE TABLE tags (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE);

.read src/seed.sql
-- 旧账目里用过、初始数据里没有的账户和标签照样保留。
INSERT OR IGNORE INTO accounts (name) SELECT DISTINCT account FROM entries WHERE account IS NOT NULL ORDER BY account;
INSERT OR IGNORE INTO tags (name) SELECT DISTINCT tag FROM entry_tags ORDER BY tag;

ALTER TABLE entry_tags RENAME TO entry_tags_old;
ALTER TABLE entries RENAME TO entries_old;
DROP INDEX idx_entries_occurred;

CREATE TABLE entries (
  id           TEXT PRIMARY KEY,
  kind         TEXT NOT NULL CHECK (kind IN ('expense', 'income')),
  amount_cents INTEGER NOT NULL CHECK (amount_cents > 0),
  category_id  INTEGER NOT NULL REFERENCES categories(id),
  account_id   INTEGER REFERENCES accounts(id),
  merchant     TEXT,
  occurred_at  TEXT NOT NULL,
  note         TEXT,
  created_at   TEXT NOT NULL,
  draft_id     TEXT
);
CREATE INDEX idx_entries_occurred ON entries(occurred_at);

CREATE TABLE entry_tags (
  entry_id TEXT NOT NULL REFERENCES entries(id),
  tag_id   INTEGER NOT NULL REFERENCES tags(id),
  PRIMARY KEY (entry_id, tag_id)
);

INSERT INTO entries (id, kind, amount_cents, category_id, account_id, merchant, occurred_at,
                     note, created_at, draft_id)
SELECT o.id, o.kind, o.amount_cents,
       (SELECT coalesce(s.id, c.id) FROM categories c
        LEFT JOIN categories s ON s.parent_id = c.id AND s.name = o.subcategory
        WHERE c.kind = o.kind AND c.parent_id IS NULL AND c.name = o.category
          AND (o.subcategory IS NULL OR s.id IS NOT NULL)),
       (SELECT id FROM accounts WHERE name = o.account),
       o.merchant, o.occurred_at, o.note, o.created_at, o.draft_id
FROM entries_old o;
INSERT INTO entry_tags (entry_id, tag_id)
SELECT t.entry_id, (SELECT id FROM tags WHERE name = t.tag) FROM entry_tags_old t;

DROP TABLE entry_tags_old;
DROP TABLE entries_old;

COMMIT;
PRAGMA foreign_keys = ON;
PRAGMA foreign_key_check;
