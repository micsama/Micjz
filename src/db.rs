use std::{path::Path, sync::Mutex};

use anyhow::Context;
use rusqlite::{Connection, params};

use crate::flow::{self, Draft, Kind};

const SCHEMA: &str = "
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

";

pub struct Db(Mutex<Connection>, pub std::path::PathBuf);

/// 一次识别的留痕。
pub struct Recognition<'a> {
    pub draft_id: &'a str,
    /// ok / rejected / error
    pub outcome: &'a str,
    pub model: &'a str,
    pub elapsed_ms: u64,
    pub image_sha256: &'a str,
    pub image_bytes: usize,
    /// 模型原始返回（失败时是错误正文）。
    pub raw: &'a str,
    pub reason: Option<&'a str>,
    /// 归一化后的原始草稿，即用户第一次看到的那些值。
    pub origin: Option<&'a Draft>,
    pub image_path: &'a str,
    pub prompt: &'a str,
    pub parameters: &'a str,
    pub timings: &'a str,
}

impl Db {
    pub fn open(path: &Path) -> anyhow::Result<Db> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("创建目录 {}", dir.display()))?;
        }
        let conn =
            Connection::open(path).with_context(|| format!("打开数据库 {}", path.display()))?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;
        conn.execute_batch(SCHEMA).context("建表")?;
        Ok(Db(
            Mutex::new(conn),
            path.parent().unwrap_or(Path::new(".")).join("images"),
        ))
    }

    /// 以草稿 ID 入账；已存在则不变（重复确认是同一笔）。返回是否新插入。
    pub fn save(&self, d: &Draft) -> rusqlite::Result<bool> {
        let mut conn = self.0.lock().expect("数据库锁未中毒");
        let tx = conn.transaction()?;
        let kind = match d.kind {
            Kind::Expense => "expense",
            Kind::Income => "income",
        };
        let inserted = tx.execute(
            "INSERT INTO entries (id, kind, amount_cents, category, subcategory, account,
                                  merchant, occurred_at, note, created_at,
                                  draft_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(id) DO NOTHING",
            params![
                d.id,
                kind,
                d.amount_cents,
                d.category,
                d.subcategory,
                d.account,
                d.merchant,
                d.occurred_at.to_rfc3339(),
                d.note,
                flow::now().to_rfc3339(),
                d.id,
            ],
        )? == 1;
        if inserted {
            for tag in &d.tags {
                tx.execute(
                    "INSERT INTO entry_tags (entry_id, tag) VALUES (?1, ?2)",
                    params![d.id, tag],
                )?;
            }
        }
        tx.execute("UPDATE recognitions SET confirmed_at = ?2 WHERE draft_id = ?1 AND confirmed_at IS NULL", params![d.id, flow::now().to_rfc3339()])?;
        tx.commit()?;
        Ok(inserted)
    }

    /// 保存识别记录；失败明确返回。
    pub fn record_recognition(&self, r: &Recognition<'_>) -> rusqlite::Result<()> {
        self.0.lock().expect("数据库锁未中毒").execute(
            "INSERT INTO recognitions (draft_id, at, outcome, model, elapsed_ms, image_sha256,
             image_bytes, raw, reason, origin, image_path, prompt, parameters, timings)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![
                r.draft_id,
                flow::now().to_rfc3339(),
                r.outcome,
                r.model,
                r.elapsed_ms as i64,
                r.image_sha256,
                r.image_bytes as i64,
                r.raw,
                r.reason,
                r.origin
                    .map(|d| serde_json::to_string(d).expect("草稿可序列化")),
                r.image_path,
                r.prompt,
                r.parameters,
                r.timings
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
