//! SQLite 账本：启动时建表与补列；入账、识别留痕、纠正留痕。
//!
//! 三张表分工：
//! - `entries`：已确认的账目宽表，一列一个字段，直接给统计用。
//! - `recognitions`：每次识别一行，存模型原始返回 `raw` 与归一化后的原始字段 `origin_*`。
//! - `edits`：用户改过的每个字段一行，`before`/`after` 都留。
//!
//! `entries.draft_id` 把三者串起来；`v_bills` 是拼好的宽视图。

use std::{collections::HashSet, path::Path, sync::Mutex};

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
  draft_id     TEXT,
  source       TEXT NOT NULL DEFAULT 'micjz',
  external_id  TEXT
);
CREATE INDEX IF NOT EXISTS idx_entries_occurred ON entries(occurred_at);
CREATE UNIQUE INDEX IF NOT EXISTS idx_entries_external
  ON entries(source, external_id) WHERE external_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS entry_tags (
  entry_id TEXT NOT NULL REFERENCES entries(id),
  tag      TEXT NOT NULL,
  PRIMARY KEY (entry_id, tag)
);

-- 一次识别一行。outcome：ok 识别成功 / rejected 模型说不是支付页 / error 调模型失败。
CREATE TABLE IF NOT EXISTS recognitions (
  draft_id            TEXT PRIMARY KEY,
  at                  TEXT NOT NULL,
  outcome             TEXT NOT NULL,
  model               TEXT NOT NULL,
  elapsed_ms          INTEGER,
  image_sha256        TEXT,
  image_bytes         INTEGER,
  raw                 TEXT,
  reason              TEXT,
  origin_kind         TEXT,
  origin_amount_cents INTEGER,
  origin_category     TEXT,
  origin_subcategory  TEXT,
  origin_account      TEXT,
  origin_merchant     TEXT,
  origin_occurred_at  TEXT,
  origin_note         TEXT,
  confirmed_at        TEXT
);
CREATE INDEX IF NOT EXISTS idx_recognitions_image ON recognitions(image_sha256);
CREATE INDEX IF NOT EXISTS idx_recognitions_at ON recognitions(at);

-- 用户每改一个字段一行。seq 按草稿内顺序递增。
CREATE TABLE IF NOT EXISTS edits (
  id       INTEGER PRIMARY KEY,
  draft_id TEXT NOT NULL,
  seq      INTEGER NOT NULL,
  field    TEXT NOT NULL,
  before   TEXT,
  after    TEXT,
  at       TEXT NOT NULL,
  UNIQUE (draft_id, seq)
);
CREATE INDEX IF NOT EXISTS idx_edits_draft ON edits(draft_id);

-- 统计与训练数据用的宽视图：一笔账一行，带原始识别结果和改动次数。
CREATE VIEW IF NOT EXISTS v_bills AS
SELECT e.*,
       (SELECT count(*) FROM edits d WHERE d.draft_id = e.draft_id) AS edit_count,
       r.model,
       r.elapsed_ms,
       r.raw,
       r.origin_kind,
       r.origin_amount_cents,
       r.origin_category,
       r.origin_subcategory,
       r.origin_account,
       r.origin_merchant,
       r.origin_occurred_at
FROM entries e
LEFT JOIN recognitions r ON r.draft_id = e.draft_id;
";

/// 老库补列：`CREATE TABLE IF NOT EXISTS` 不会给已有表加字段。
const ENTRY_ADDED_COLUMNS: &[(&str, &str)] = &[
    ("draft_id", "TEXT"),
    ("source", "TEXT NOT NULL DEFAULT 'micjz'"),
    ("external_id", "TEXT"),
];

pub struct Db(Mutex<Connection>);

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
}

/// 一个字段的改动。
pub struct Edit {
    pub field: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

impl Db {
    pub fn open(path: &Path) -> anyhow::Result<Db> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("创建目录 {}", dir.display()))?;
        }
        let conn =
            Connection::open(path).with_context(|| format!("打开数据库 {}", path.display()))?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;
        add_missing_columns(&conn).context("补列")?;
        conn.execute_batch(SCHEMA).context("建表")?;
        Ok(Db(Mutex::new(conn)))
    }

    /// 以草稿 ID 入账；已存在则不变（重复确认是同一笔）。返回是否新插入。
    pub fn save(&self, d: &Draft) -> rusqlite::Result<bool> {
        self.save_as(d, "micjz", None)
    }

    /// 指定来源入账。`external_id` 是别的 App 里的单号，用来去重。
    pub fn save_as(
        &self,
        d: &Draft,
        source: &str,
        external_id: Option<&str>,
    ) -> rusqlite::Result<bool> {
        let mut conn = self.0.lock().expect("数据库锁未中毒");
        let tx = conn.transaction()?;
        let kind = match d.kind {
            Kind::Expense => "expense",
            Kind::Income => "income",
        };
        let inserted = tx.execute(
            "INSERT INTO entries (id, kind, amount_cents, category, subcategory, account,
                                  merchant, occurred_at, note, created_at,
                                  draft_id, source, external_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
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
                source,
                external_id,
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
        tx.commit()?;
        Ok(inserted)
    }

    /// 记一次识别。同一 draft_id 重复记只留第一次。
    pub fn record_recognition(&self, r: &Recognition<'_>) -> rusqlite::Result<()> {
        let (kind, amount, category, subcategory, account, merchant, occurred_at, note) =
            match r.origin {
                Some(d) => (
                    Some(match d.kind {
                        Kind::Expense => "expense",
                        Kind::Income => "income",
                    }),
                    Some(d.amount_cents),
                    Some(d.category.as_str()),
                    d.subcategory.as_deref(),
                    d.account.as_deref(),
                    d.merchant.as_deref(),
                    Some(d.occurred_at.to_rfc3339()),
                    d.note.as_deref(),
                ),
                None => (None, None, None, None, None, None, None, None),
            };
        self.0.lock().expect("数据库锁未中毒").execute(
            "INSERT INTO recognitions (draft_id, at, outcome, model, elapsed_ms, image_sha256,
                                       image_bytes, raw, reason, origin_kind, origin_amount_cents,
                                       origin_category, origin_subcategory, origin_account,
                                       origin_merchant, origin_occurred_at, origin_note)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
             ON CONFLICT(draft_id) DO NOTHING",
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
                kind,
                amount,
                category,
                subcategory,
                account,
                merchant,
                occurred_at,
                note,
            ],
        )?;
        Ok(())
    }

    /// 记一批字段改动，`seq` 自动接着草稿已有的最大序号往下排。
    pub fn record_edits(&self, draft_id: &str, edits: &[Edit]) -> rusqlite::Result<()> {
        if edits.is_empty() {
            return Ok(());
        }
        let mut conn = self.0.lock().expect("数据库锁未中毒");
        let tx = conn.transaction()?;
        let mut seq: i64 = tx.query_row(
            "SELECT coalesce(max(seq), 0) FROM edits WHERE draft_id = ?1",
            params![draft_id],
            |r| r.get(0),
        )?;
        let at = flow::now().to_rfc3339();
        for e in edits {
            seq += 1;
            tx.execute(
                "INSERT INTO edits (draft_id, seq, field, before, after, at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![draft_id, seq, e.field, e.before, e.after, at],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// 标记这次识别已被确认入账。
    pub fn mark_confirmed(&self, draft_id: &str) -> rusqlite::Result<()> {
        self.0.lock().expect("数据库锁未中毒").execute(
            "UPDATE recognitions SET confirmed_at = ?2 WHERE draft_id = ?1 AND confirmed_at IS NULL",
            params![draft_id, flow::now().to_rfc3339()],
        )?;
        Ok(())
    }
}

fn add_missing_columns(conn: &Connection) -> rusqlite::Result<()> {
    let table_exists: bool = conn.query_row(
        "SELECT count(*) > 0 FROM sqlite_master WHERE type = 'table' AND name = 'entries'",
        [],
        |r| r.get(0),
    )?;
    if !table_exists {
        return Ok(());
    }
    let mut have = HashSet::new();
    let mut stmt = conn.prepare("PRAGMA table_info(entries)")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        have.insert(row.get::<_, String>(1)?);
    }
    drop(rows);
    drop(stmt);
    for (name, decl) in ENTRY_ADDED_COLUMNS {
        if !have.contains(*name) {
            conn.execute_batch(&format!("ALTER TABLE entries ADD COLUMN {name} {decl}"))?;
        }
    }
    Ok(())
}

/// 两个草稿之间被改过的字段；只报真正变了的，按固定顺序。
pub fn diff(before: &Draft, after: &Draft) -> Vec<Edit> {
    let yuan = |c: i64| format!("{}.{:02}", c / 100, c % 100);
    let mut out = Vec::new();
    let mut push = |field: &str, a: Option<String>, b: Option<String>| {
        if a != b {
            out.push(Edit {
                field: field.to_owned(),
                before: a,
                after: b,
            });
        }
    };
    push(
        "kind",
        Some(
            match before.kind {
                Kind::Expense => "expense",
                Kind::Income => "income",
            }
            .to_owned(),
        ),
        Some(
            match after.kind {
                Kind::Expense => "expense",
                Kind::Income => "income",
            }
            .to_owned(),
        ),
    );
    push(
        "amount",
        Some(yuan(before.amount_cents)),
        Some(yuan(after.amount_cents)),
    );
    push(
        "category",
        Some(before.category.clone()),
        Some(after.category.clone()),
    );
    push(
        "subcategory",
        before.subcategory.clone(),
        after.subcategory.clone(),
    );
    push("account", before.account.clone(), after.account.clone());
    push("merchant", before.merchant.clone(), after.merchant.clone());
    push(
        "occurred_at",
        Some(before.occurred_at.to_rfc3339()),
        Some(after.occurred_at.to_rfc3339()),
    );
    push("note", before.note.clone(), after.note.clone());
    push(
        "tags",
        Some(before.tags.join(",")),
        Some(after.tags.join(",")),
    );
    out
}

#[cfg(test)]
mod tests;
