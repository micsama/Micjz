//! 完整账目的唯一校验、写入路径；调用者持有事务。
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};

pub const MAX_AMOUNT: i64 = 99_999_999_999;

use super::{Error, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    Expense,
    Income,
    Transfer,
    Adjust,
}
impl EntryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Expense => "expense",
            Self::Income => "income",
            Self::Transfer => "transfer",
            Self::Adjust => "adjust",
        }
    }
}

// 显式反序列化 Option，缺字段必须报错；PATCH 在 HTTP 边界另行合并。
pub fn required_nullable<'de, D, T>(d: D) -> std::result::Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(d)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EntryInput {
    pub kind: EntryKind,
    pub amount_cents: i64,
    #[serde(deserialize_with = "required_nullable")]
    pub category_id: Option<i64>,
    pub account_id: i64,
    #[serde(deserialize_with = "required_nullable")]
    pub to_account_id: Option<i64>,
    #[serde(deserialize_with = "required_nullable")]
    pub merchant: Option<String>,
    pub occurred_at: String,
    #[serde(deserialize_with = "required_nullable")]
    pub note: Option<String>,
    pub tag_ids: Vec<i64>,
}
#[derive(Debug, Serialize)]
pub struct Entry {
    pub id: String,
    pub recognition_id: Option<String>,
    #[serde(flatten)]
    pub input: EntryInput,
    pub created_at: String,
}

pub fn timestamp(s: &str) -> Result<String> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(crate::flow::stamp)
        .map_err(|_| Error::Invalid("时间须为 RFC 3339".into()))
}
pub fn text(value: Option<String>, max: usize) -> Result<Option<String>> {
    let value = value.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty());
    if value.as_ref().is_some_and(|s| s.chars().count() > max) {
        return Err(Error::Invalid(format!("文字不能超过 {max} 字")));
    }
    Ok(value)
}
fn exists(tx: &Connection, table: &str, id: i64, label: &str) -> Result<()> {
    if !tx.query_row(
        &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id = ?1)"),
        [id],
        |r| r.get::<_, bool>(0),
    )? {
        return Err(Error::Conflict(format!("{label}已不存在，请刷新后重选")));
    }
    Ok(())
}
/// 入口校验后的完整账目；只有 parse 能构造，写入层只检查会变化的引用。
pub struct ValidatedEntry(EntryInput);
impl EntryInput {
    pub fn parse(mut self) -> Result<ValidatedEntry> {
        let v = &mut self;
        if v.amount_cents == 0
            || v.amount_cents.unsigned_abs() > MAX_AMOUNT as u64
            || (v.kind != EntryKind::Adjust && v.amount_cents < 0)
        {
            return Err(Error::Invalid("金额超出范围或为零".into()));
        }
        match v.kind {
            EntryKind::Expense | EntryKind::Income => {
                if v.category_id.is_none() {
                    return Err(Error::Invalid("支出收入必须有分类".into()));
                }
            }
            _ if v.category_id.is_some() => {
                return Err(Error::Invalid("转账平账不能有分类".into()));
            }
            _ => (),
        }
        match (v.kind, v.to_account_id) {
            (EntryKind::Transfer, Some(to)) if to != v.account_id => (),
            (EntryKind::Transfer, _) => {
                return Err(Error::Invalid("转账必须选择不同的转入账户".into()));
            }
            (_, Some(_)) => return Err(Error::Invalid("只有转账能有转入账户".into())),
            _ => (),
        }
        v.tag_ids.sort_unstable();
        v.tag_ids.dedup();
        v.merchant = text(v.merchant.take(), 200)?;
        v.note = text(v.note.take(), 200)?;
        v.occurred_at = timestamp(&v.occurred_at)?;
        Ok(ValidatedEntry(self))
    }
}
fn validate_references(tx: &Transaction<'_>, v: &EntryInput) -> Result<()> {
    exists(tx, "accounts", v.account_id, "账户")?;
    if let Some(to) = v.to_account_id {
        exists(tx, "accounts", to, "转入账户")?;
    }
    if let Some(id) = v.category_id {
        let kind: Option<String> = tx
            .query_row("SELECT kind FROM categories WHERE id = ?1", [id], |r| {
                r.get(0)
            })
            .optional()?;
        let kind = kind.ok_or_else(|| Error::Conflict("分类已不存在，请刷新后重选".into()))?;
        if kind != v.kind.as_str() {
            return Err(Error::Invalid("类型与分类不匹配".into()));
        }
    }
    for id in &v.tag_ids {
        exists(tx, "tags", *id, "标签")?;
    }
    Ok(())
}
fn tags(tx: &Transaction<'_>, id: &str, ids: &[i64]) -> Result<()> {
    tx.execute("DELETE FROM entry_tags WHERE entry_id = ?1", [id])?;
    for tag in ids {
        tx.execute(
            "INSERT INTO entry_tags(entry_id, tag_id) VALUES (?1, ?2)",
            params![id, tag],
        )?;
    }
    Ok(())
}
pub fn new_id() -> String {
    uuid::Uuid::now_v7().to_string()
}
/// `id` 由调用者给：新生成，或网页请求带来的幂等键（已存在由调用者先查）。
pub fn insert(
    tx: &Transaction<'_>,
    id: String,
    input: ValidatedEntry,
    recognition: Option<&str>,
) -> Result<Entry> {
    let ValidatedEntry(v) = input;
    validate_references(tx, &v)?;
    let created_at = crate::flow::stamp(crate::flow::now());
    tx.execute(
        "INSERT INTO entries (id, recognition_id, kind, amount_cents, category_id, account_id,
                              to_account_id, merchant, occurred_at, note, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            id,
            recognition,
            v.kind.as_str(),
            v.amount_cents,
            v.category_id,
            v.account_id,
            v.to_account_id,
            v.merchant,
            v.occurred_at,
            v.note,
            created_at
        ],
    )?;
    tags(tx, &id, &v.tag_ids)?;
    Ok(Entry {
        id,
        recognition_id: recognition.map(str::to_owned),
        input: v,
        created_at,
    })
}
pub fn update(tx: &Transaction<'_>, id: &str, input: ValidatedEntry) -> Result<Entry> {
    let ValidatedEntry(v) = input;
    validate_references(tx, &v)?;
    tx.execute(
        "UPDATE entries SET kind=?2, amount_cents=?3, category_id=?4, account_id=?5,
                            to_account_id=?6, merchant=?7, occurred_at=?8, note=?9 WHERE id=?1",
        params![
            id,
            v.kind.as_str(),
            v.amount_cents,
            v.category_id,
            v.account_id,
            v.to_account_id,
            v.merchant,
            v.occurred_at,
            v.note
        ],
    )?;
    tags(tx, id, &v.tag_ids)?;
    get(tx, id)
}
pub fn get(conn: &Connection, id: &str) -> Result<Entry> {
    find(conn, id)?.ok_or_else(|| Error::Missing("账目不存在".into()))
}
pub fn find(conn: &Connection, id: &str) -> Result<Option<Entry>> {
    let Some(mut e) = conn
        .query_row(
            "SELECT id, recognition_id, kind, amount_cents, category_id, account_id,
                to_account_id, merchant, occurred_at, note, created_at FROM entries WHERE id=?1",
            [id],
            |r| {
                let k: String = r.get(2)?;
                let kind = match k.as_str() {
                    "expense" => EntryKind::Expense,
                    "income" => EntryKind::Income,
                    "transfer" => EntryKind::Transfer,
                    "adjust" => EntryKind::Adjust,
                    _ => return Err(rusqlite::Error::InvalidQuery),
                };
                Ok(Entry {
                    id: r.get(0)?,
                    recognition_id: r.get(1)?,
                    input: EntryInput {
                        kind,
                        amount_cents: r.get(3)?,
                        category_id: r.get(4)?,
                        account_id: r.get(5)?,
                        to_account_id: r.get(6)?,
                        merchant: r.get(7)?,
                        occurred_at: r.get(8)?,
                        note: r.get(9)?,
                        tag_ids: vec![],
                    },
                    created_at: r.get(10)?,
                })
            },
        )
        .optional()?
    else {
        return Ok(None);
    };
    e.input.tag_ids = conn
        .prepare("SELECT tag_id FROM entry_tags WHERE entry_id=?1 ORDER BY tag_id")?
        .query_map([id], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(Some(e))
}
pub fn list(conn: &Connection, from: &str, to: &str, ascending: bool) -> Result<Vec<Entry>> {
    let order = if ascending { "ASC" } else { "DESC" };
    let ids = conn
        .prepare(&format!(
            "SELECT id FROM entries WHERE occurred_at>=?1 AND occurred_at<?2
         ORDER BY occurred_at {order}, id {order}"
        ))?
        .query_map(params![from, to], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    ids.iter().map(|id| get(conn, id)).collect()
}
pub fn balance(conn: &Connection, id: i64) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT coalesce(sum(CASE
           WHEN kind='expense' THEN -amount_cents
           WHEN kind='transfer' AND account_id=?1 THEN -amount_cents
           ELSE amount_cents END), 0)
         FROM entries WHERE account_id=?1 OR to_account_id=?1",
        [id],
        |r| r.get(0),
    )?)
}
