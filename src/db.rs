//! SQLite：`entries`/`entry_tags` 是最终账目，引用 `categories`/`accounts`/`tags`；
//! `recognitions` 留存每次识别的输入输出。
//! 只认当前表结构（新库直接建）；旧库由 `./build.sh` 调 `deploy/db/convert.py` 转换。

use std::{
    path::Path,
    sync::{Mutex, MutexGuard},
};

use anyhow::Context;
use rusqlite::{Connection, OptionalExtension, Transaction, params};

use crate::{
    catalog::{self, Catalog},
    flow::{self, Draft},
    recognize::Recognized,
};

pub const SCHEMA: &str = "
-- parent_id 为空是一级；每个分类都有模型 key。外键保证有账目或二级的分类删不掉。
CREATE TABLE categories (
  id        INTEGER PRIMARY KEY,
  kind      TEXT NOT NULL CHECK (kind IN ('expense', 'income')),
  parent_id INTEGER REFERENCES categories(id),
  name      TEXT NOT NULL,
  key       TEXT NOT NULL UNIQUE
);
CREATE UNIQUE INDEX idx_categories_name
  ON categories(kind, coalesce(parent_id, 0), name);

CREATE TABLE accounts (
  id   INTEGER PRIMARY KEY,
  name TEXT NOT NULL UNIQUE
);

CREATE TABLE tags (
  id   INTEGER PRIMARY KEY,
  name TEXT NOT NULL UNIQUE
);

-- category_id 指向最细一级：选了二级就是二级。
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

-- outcome：ok / rejected（模型说不是支付页）/ error（调用或解析失败）
CREATE TABLE recognitions (
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

/// 当前表结构版本，存在库的 `PRAGMA user_version`；`deploy/db/convert.py` 也读这个文件。
pub fn schema_version() -> u32 {
    include_str!("schema_version")
        .trim()
        .parse()
        .expect("schema_version 是数字")
}

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
        let fresh: bool = conn.query_row(
            "SELECT NOT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table')",
            [],
            |r| r.get(0),
        )?;
        if fresh {
            conn.execute_batch(&format!(
                "BEGIN;\n{SCHEMA}\n{}\nPRAGMA user_version = {};\nCOMMIT;",
                catalog::SEED,
                schema_version()
            ))
            .context("建库")?;
        }
        let version: u32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        anyhow::ensure!(
            version == schema_version(),
            "数据库结构 v{version}，程序需要 v{}：先运行 ./build.sh 转换",
            schema_version()
        );
        Ok(Db(Mutex::new(conn)))
    }

    pub fn conn(&self) -> MutexGuard<'_, Connection> {
        self.0.lock().expect("数据库锁未中毒")
    }

    /// 当前分类、账户、标签。
    pub fn catalog(&self) -> rusqlite::Result<Catalog> {
        catalog::load(&self.conn())
    }

    /// 以草稿 ID 入账并标记识别已确认，同一事务；已存在则不变（重复确认是同一笔）。
    /// 名称在这里换成 id；草稿期间被删改的分类、账户、标签会让入账失败。返回是否新插入。
    pub fn save(&self, d: &Draft, amount_cents: i64) -> anyhow::Result<bool> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let now = flow::now().to_rfc3339();
        let category_id = category_id(&tx, d)?;
        let account_id = match &d.account {
            Some(name) => Some(catalog::name_id(&tx, "accounts", name)?),
            None => None,
        };
        let inserted = tx.execute(
            "INSERT INTO entries (id, kind, amount_cents, category_id, account_id,
                                  merchant, occurred_at, note, created_at, draft_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO NOTHING",
            params![
                d.id,
                d.kind.as_str(),
                amount_cents,
                category_id,
                account_id,
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
                    "INSERT INTO entry_tags (entry_id, tag_id) VALUES (?1, ?2)",
                    params![d.id, catalog::name_id(&tx, "tags", tag)?],
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
        self.conn().execute(
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

fn category_id(tx: &Transaction<'_>, d: &Draft) -> anyhow::Result<i64> {
    tx.query_row(
        "SELECT coalesce(s.id, c.id) FROM categories c
         LEFT JOIN categories s ON s.parent_id = c.id AND s.name = ?3
         WHERE c.kind = ?1 AND c.parent_id IS NULL AND c.name = ?2
           AND (?3 IS NULL OR s.id IS NOT NULL)",
        params![d.kind.as_str(), d.category, d.subcategory],
        |r| r.get(0),
    )
    .optional()?
    .with_context(|| format!("分类已不存在：{}", flow::category_label(d)))
}

#[cfg(test)]
mod tests;
