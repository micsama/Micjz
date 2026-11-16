ALTER TABLE accounts ADD COLUMN brand TEXT;
CREATE TABLE web_admin (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  password_hash TEXT NOT NULL,
  session_key BLOB NOT NULL
) STRICT;
