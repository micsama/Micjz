-- v8 → v9：支出可为负（退款）；账户加 external（别人的钱，不进净资产、不能平账）。

ALTER TABLE accounts ADD COLUMN external INTEGER NOT NULL DEFAULT 0 CHECK (external IN (0, 1));

CREATE TABLE entries_new (
  id             TEXT PRIMARY KEY,
  recognition_id TEXT UNIQUE REFERENCES recognitions(draft_id),
  kind           TEXT NOT NULL CHECK (kind IN ('expense', 'income', 'transfer', 'adjust')),
  amount_cents   INTEGER NOT NULL
                 CHECK (amount_cents > 0 OR (kind IN ('expense', 'adjust') AND amount_cents <> 0)),
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
INSERT INTO entries_new SELECT id, recognition_id, kind, amount_cents, category_id, account_id,
  to_account_id, merchant, occurred_at, note, created_at FROM entries;
DROP TABLE entries;
ALTER TABLE entries_new RENAME TO entries;
CREATE INDEX idx_entries_occurred ON entries(occurred_at);
