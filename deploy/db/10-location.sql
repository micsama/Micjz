-- v9 → v10：识别记录加入账后补采的位置（WGS-84）。SQLite 的 ADD COLUMN 不能带表级 CHECK，重建表。

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
  origin       TEXT NOT NULL,
  latitude     REAL CHECK (latitude BETWEEN -90 AND 90),
  longitude    REAL CHECK (longitude BETWEEN -180 AND 180),
  CHECK ((latitude IS NULL) = (longitude IS NULL))
) STRICT;
INSERT INTO recognitions_new SELECT draft_id, outcome, model, upload_ms, model_ms, image_sha256,
  image_bytes, image_path, prompt, parameters, raw, reason, origin, NULL, NULL FROM recognitions;
DROP TABLE recognitions;
ALTER TABLE recognitions_new RENAME TO recognitions;
