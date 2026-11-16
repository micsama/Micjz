//! SQLite：`entries`/`entry_tags` 是最终账目，`recognitions` 留存每次识别的输入输出。
//! 只认当前表结构；结构变化写 `deploy/` 下的 SQL 脚本手动执行。

use std::{path::Path, sync::Mutex};

use anyhow::Context;
use rusqlite::{Connection, params};

use crate::{
    flow::{self, Draft},
    recognize::Recognized,
};

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

-- outcome：ok / rejected（模型说不是支付页）/ error（调用或解析失败）
CREATE TABLE IF NOT EXISTS recognitions (
  draft_id     TEXT PRIMARY KEY,
  at           TEXT NOT NULL,
  outcome      TEXT NOT NULL,
  model        TEXT NOT NULL,
  elapsed_ms   INTEGER,
  image_sha256 TEXT,
  image_bytes  INTEGER,
  image_path   TEXT,
  prompt       TEXT NOT NULL,
  parameters   TEXT NOT NULL,
  raw          TEXT,
  reason       TEXT,
  origin       TEXT,
  timings      TEXT,
  confirmed_at TEXT
);
";

pub struct Db(Mutex<Connection>);

/// 识别时收到的图片。
pub struct Image<'a> {
    pub sha256: &'a str,
    pub bytes: usize,
    /// 留存路径；文件由后台任务写入。
    pub path: &'a str,
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
        Ok(Db(Mutex::new(conn)))
    }

    /// 以草稿 ID 入账并标记识别已确认，同一事务；已存在则不变（重复确认是同一笔）。
    /// 返回是否新插入。
    pub fn save(&self, d: &Draft, amount_cents: i64) -> rusqlite::Result<bool> {
        let mut conn = self.0.lock().expect("数据库锁未中毒");
        let tx = conn.transaction()?;
        let now = flow::now().to_rfc3339();
        let inserted = tx.execute(
            "INSERT INTO entries (id, kind, amount_cents, category, subcategory, account,
                                  merchant, occurred_at, note, created_at,
                                  draft_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(id) DO NOTHING",
            params![
                d.id,
                d.kind.as_str(),
                amount_cents,
                d.category,
                d.subcategory,
                d.account,
                d.merchant,
                d.occurred_at.to_rfc3339(),
                d.note,
                now,
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
        tx.execute(
            "UPDATE recognitions SET confirmed_at = ?2 WHERE draft_id = ?1 AND confirmed_at IS NULL",
            params![d.id, now],
        )?;
        tx.commit()?;
        Ok(inserted)
    }

    /// 保存一次识别；`origin` 是用户第一眼看到的草稿。
    pub fn record_recognition(
        &self,
        r: &Recognized,
        image: &Image<'_>,
        timings: &str,
    ) -> rusqlite::Result<()> {
        self.0.lock().expect("数据库锁未中毒").execute(
            "INSERT INTO recognitions (draft_id, at, outcome, model, elapsed_ms, image_sha256,
                                       image_bytes, image_path, prompt, parameters, raw, reason,
                                       origin, timings)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                r.draft.id,
                flow::now().to_rfc3339(),
                r.outcome.as_str(),
                r.model,
                r.model_ms as i64,
                image.sha256,
                image.bytes as i64,
                image.path,
                r.prompt,
                r.parameters,
                r.raw,
                r.reason,
                serde_json::to_string(&r.draft).expect("草稿可序列化"),
                timings,
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
