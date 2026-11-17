//! SQLite：`entries`/`entry_tags` 是最终账目，引用 `categories`/`accounts`/`tags`；
//! `recognitions` 留存每次识别的输入输出；`settings` 是单行的用户设置。
//! 只认当前表结构（新库直接建）；旧库由 `./build.sh` 调 `deploy/db/convert.py` 转换。

use std::{
    path::Path,
    sync::{Mutex, MutexGuard},
};

use anyhow::Context;
use rusqlite::{Connection, OptionalExtension, params};

use crate::{
    catalog::{self, Catalog},
    flow::{self, Draft},
    recognize::{Outcome, Recognized},
};

pub mod entries;

pub const SCHEMA: &str = "
CREATE TABLE web_admin (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  password_hash TEXT NOT NULL,
  session_key BLOB NOT NULL
) STRICT;

-- parent_id 为空是一级；每个分类都有模型 key。外键保证有账目或二级的分类删不掉。
-- position 是同级内的优先级：菜单顺序、模型分类模糊时的先后。
CREATE TABLE categories (
  id        INTEGER PRIMARY KEY,
  position  INTEGER NOT NULL,
  kind      TEXT NOT NULL CHECK (kind IN ('expense', 'income')),
  parent_id INTEGER REFERENCES categories(id),
  name      TEXT NOT NULL,
  key       TEXT NOT NULL UNIQUE
) STRICT;
CREATE UNIQUE INDEX idx_categories_name
  ON categories(kind, coalesce(parent_id, 0), name);

-- position 越小越优先，识别不出账户时取第一个；卡号完整存，只把尾号给模型。
CREATE TABLE accounts (
  id          INTEGER PRIMARY KEY,
  position    INTEGER NOT NULL,
  name        TEXT NOT NULL UNIQUE,
  brand       TEXT,
  card_type   TEXT CHECK (card_type IN ('debit', 'credit')),
  card_number TEXT,
  CHECK ((card_type IS NULL) = (card_number IS NULL))
) STRICT;

CREATE TABLE tags (
  id   INTEGER PRIMARY KEY,
  name TEXT NOT NULL UNIQUE
) STRICT;

-- 一次识别一行，主键是草稿 ID。识别时刻看 /recognize 那条 requests。
CREATE TABLE recognitions (
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

-- id 是账目自己的 ID；截图来的 recognition_id 指向那次识别，也是确认的幂等键。
-- 支出、收入有分类（一级或二级，有二级也可只记一级）；转账从 account_id 到 to_account_id；平账的金额是差额，可正可负。
-- 时间一律 UTC（flow::stamp）。
CREATE TABLE entries (
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
CREATE INDEX idx_entries_occurred ON entries(occurred_at);

CREATE TABLE entry_tags (
  entry_id TEXT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
  tag_id   INTEGER NOT NULL REFERENCES tags(id),
  PRIMARY KEY (entry_id, tag_id)
) STRICT;

-- 访问日志：通过鉴权的快捷指令请求与网页写请求。draft_id 原样记收到的，不设外键。
CREATE TABLE requests (
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
CREATE INDEX idx_requests_draft ON requests(draft_id);

-- 用户设置，恒为一行，每个设置一列。周期从 cycle_day 号起；预算为空是不设。
CREATE TABLE settings (
  id           INTEGER PRIMARY KEY CHECK (id = 1),
  -- 正数：本月 N 号起算本月；负数：上月 N 号起算本月。
  cycle_day    INTEGER NOT NULL DEFAULT 1
               CHECK (cycle_day BETWEEN -28 AND 28 AND cycle_day NOT IN (-1, 0)),
  budget_cents INTEGER CHECK (budget_cents > 0)
) STRICT;
INSERT INTO settings (id) VALUES (1);
";

/// 当前表结构版本，存在库的 `PRAGMA user_version`；`deploy/db/convert.py` 也读这个文件。
pub fn schema_version() -> u32 {
    include_str!("schema_version")
        .trim()
        .parse()
        .expect("schema_version 是数字")
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Missing(String),
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
}
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub struct Db(Mutex<Connection>);

/// 识别时收到的图片。
pub struct Image<'a> {
    pub sha256: &'a str,
    pub bytes: usize,
    /// 留存路径；文件由后台任务写入。
    pub path: &'a str,
}

/// `requests` 表的一行：快捷指令请求或网页写请求。
pub struct RequestRow<'a> {
    pub at: &'a str,
    pub method: &'a str,
    pub path: &'a str,
    pub status: u16,
    pub elapsed_ms: u64,
    pub trace: &'a crate::api::Trace,
    pub error: Option<&'a str>,
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
    pub fn catalog(&self) -> anyhow::Result<Catalog> {
        catalog::load(&self.conn())
    }

    /// 草稿对应的识别结果与原因；不是本服务识别发出的草稿为 `None`。
    pub fn recognition(
        &self,
        draft_id: &str,
    ) -> rusqlite::Result<Option<(Outcome, Option<String>)>> {
        self.conn()
            .query_row(
                "SELECT outcome, reason FROM recognitions WHERE draft_id = ?1",
                [draft_id],
                |r| {
                    let outcome: String = r.get(0)?;
                    let outcome = Outcome::parse(&outcome).ok_or_else(|| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            format!("未知识别结果：{outcome}").into(),
                        )
                    })?;
                    Ok((outcome, r.get(1)?))
                },
            )
            .optional()
    }

    /// 确认截图草稿：账目新生成 id，`recognition_id` 是草稿 ID；账目和标签同一事务。
    /// 同一草稿重复确认只记一笔；草稿期间被删的分类、账户、标签由外键拦下，入账失败。
    /// 返回是否新插入。
    pub fn save(&self, d: &Draft, amount_cents: i64) -> anyhow::Result<bool> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        if tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM entries WHERE recognition_id = ?1)",
            [&d.id],
            |r| r.get::<_, bool>(0),
        )? {
            return Ok(false);
        }
        let input = entries::EntryInput {
            kind: match d.kind {
                flow::Kind::Expense => entries::EntryKind::Expense,
                flow::Kind::Income => entries::EntryKind::Income,
            },
            amount_cents,
            category_id: Some(d.category_id),
            account_id: d.account_id,
            to_account_id: None,
            merchant: d.merchant.clone(),
            occurred_at: flow::stamp(d.occurred_at),
            note: d.note.clone(),
            tag_ids: d.tag_ids.clone(),
        };
        entries::insert(&tx, entries::new_id(), input.parse()?, Some(&d.id))?;
        tx.commit()?;
        Ok(true)
    }

    pub fn record_request(&self, r: &RequestRow<'_>) -> rusqlite::Result<()> {
        self.conn().execute(
            "INSERT INTO requests (at, method, path, status, elapsed_ms, draft_id, field, value,
                                   error)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                r.at,
                r.method,
                r.path,
                r.status,
                r.elapsed_ms as i64,
                r.trace.draft_id,
                r.trace.field,
                r.trace.value,
                r.error,
            ],
        )?;
        Ok(())
    }

    /// 保存一次识别；`origin` 是用户第一眼看到的草稿。
    pub fn record_recognition(
        &self,
        r: &Recognized,
        image: &Image<'_>,
        upload_ms: u64,
    ) -> rusqlite::Result<()> {
        self.conn().execute(
            "INSERT INTO recognitions (draft_id, outcome, model, upload_ms, model_ms, image_sha256,
                                       image_bytes, image_path, prompt, parameters, raw, reason,
                                       origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                r.draft.id,
                r.outcome.as_str(),
                r.model,
                upload_ms as i64,
                r.model_ms as i64,
                image.sha256,
                image.bytes as i64,
                image.path,
                r.prompt,
                r.parameters,
                r.raw,
                r.reason,
                serde_json::to_string(&r.draft).expect("草稿可序列化"),
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
