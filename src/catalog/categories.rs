//! 分类目录的读写规则，供 HTTP 和命令行共用。契约：docs/blueprints/categories-web.md。
use super::{check_name, fallback_key, reorder};
use crate::{db::Error, flow::Kind};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct Row {
    pub id: i64,
    pub kind: Kind,
    pub parent_id: Option<i64>,
    pub name: String,
    pub key: String,
}

fn kind(s: String) -> Kind {
    match s.as_str() {
        "expense" => Kind::Expense,
        "income" => Kind::Income,
        _ => unreachable!("CHECK 约束了取值"),
    }
}

/// 一级在前，各级按优先级。
pub fn list(conn: &Connection) -> Result<Vec<Row>, Error> {
    Ok(conn
        .prepare(
            "SELECT id, kind, parent_id, name, key FROM categories
             ORDER BY parent_id IS NOT NULL, parent_id, position, id",
        )?
        .query_map([], |r| {
            Ok(Row {
                id: r.get(0)?,
                kind: kind(r.get(1)?),
                parent_id: r.get(2)?,
                name: r.get(3)?,
                key: r.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?)
}

/// 新建、修改共用的字段；`kind` 只在新建时给。`key` 为空：新建时生成 `c<id>`，修改时保持原值。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    #[serde(default)]
    pub kind: Option<Kind>,
    #[serde(deserialize_with = "crate::db::entries::required_nullable")]
    pub parent_id: Option<i64>,
    pub name: String,
    #[serde(default)]
    pub key: Option<String>,
}

fn invalid(e: anyhow::Error) -> Error {
    Error::Invalid(e.to_string())
}

fn check_key(key: Option<String>) -> Result<Option<String>, Error> {
    let key = key.map(|k| k.trim().to_owned()).filter(|k| !k.is_empty());
    if let Some(k) = &key
        && !k
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    {
        return Err(Error::Invalid(format!(
            "key 须为小写英文、数字或下划线：{k}"
        )));
    }
    Ok(key)
}

struct Current {
    kind: Kind,
    parent: Option<i64>,
    key: String,
}

fn current(tx: &Transaction<'_>, id: i64) -> Result<Current, Error> {
    tx.query_row(
        "SELECT kind, parent_id, key FROM categories WHERE id = ?1",
        [id],
        |r| {
            Ok(Current {
                kind: kind(r.get(0)?),
                parent: r.get(1)?,
                key: r.get(2)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| Error::Missing("分类不存在".into()))
}

fn count(tx: &Transaction<'_>, from_where: &str, id: i64) -> Result<i64, Error> {
    Ok(
        tx.query_row(&format!("SELECT count(*) FROM {from_where}"), [id], |r| {
            r.get(0)
        })?,
    )
}

/// 同类型、同父级的分类 id，按优先级。
fn siblings(tx: &Transaction<'_>, kind: Kind, parent: Option<i64>) -> Result<Vec<i64>, Error> {
    Ok(tx
        .prepare(
            "SELECT id FROM categories WHERE kind = ?1 AND parent_id IS ?2 ORDER BY position, id",
        )?
        .query_map(params![kind.as_str(), parent], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?)
}

/// 上级须是同类型的一级；`id` 是正在改的分类，带着二级的不能挪到别的一级下面。
fn check_parent(
    tx: &Transaction<'_>,
    kind: Kind,
    parent: Option<i64>,
    id: Option<i64>,
) -> Result<(), Error> {
    let Some(p) = parent else { return Ok(()) };
    let ok = Some(p) != id
        && tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM categories
                 WHERE id = ?1 AND kind = ?2 AND parent_id IS NULL)",
            params![p, kind.as_str()],
            |r| r.get::<_, bool>(0),
        )?;
    if !ok {
        return Err(Error::Conflict("上级须是同类型的一级分类".into()));
    }
    if let Some(id) = id {
        let subs = count(tx, "categories WHERE parent_id = ?1", id)?;
        if subs > 0 {
            return Err(Error::Conflict(format!(
                "下面还有 {subs} 个二级，不能挪到别的一级下面"
            )));
        }
    }
    Ok(())
}

/// 同级重名、key 被占用；`id` 是正在改的分类，不和自己比。
fn check_unique(
    tx: &Transaction<'_>,
    kind: Kind,
    parent: Option<i64>,
    name: &str,
    key: Option<&str>,
    id: Option<i64>,
) -> Result<(), Error> {
    if tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM categories
         WHERE kind = ?1 AND parent_id IS ?2 AND name = ?3 AND id IS NOT ?4)",
        params![kind.as_str(), parent, name, id],
        |r| r.get::<_, bool>(0),
    )? {
        return Err(Error::Conflict(format!("同级已有分类「{name}」")));
    }
    if let Some(key) = key
        && let Some(owner) = tx
            .query_row(
                "SELECT name FROM categories WHERE key = ?1 AND id IS NOT ?2",
                params![key, id],
                |r| r.get::<_, String>(0),
            )
            .optional()?
    {
        return Err(Error::Conflict(format!("key {key} 已被「{owner}」使用")));
    }
    Ok(())
}

/// 新建，排在同级最后；返回 id。
pub fn create(tx: &Transaction<'_>, v: Input) -> Result<i64, Error> {
    let kind = v
        .kind
        .ok_or_else(|| Error::Invalid("新建分类须给 kind".into()))?;
    let name = check_name(&v.name).map_err(invalid)?;
    let key = check_key(v.key)?;
    check_parent(tx, kind, v.parent_id, None)?;
    check_unique(tx, kind, v.parent_id, name, key.as_deref(), None)?;
    let position = siblings(tx, kind, v.parent_id)?.len() + 1;
    // 没给 key 时先占一个不会冲突的值，拿到 id 再换成 `c<id>`。
    let placeholder = key
        .clone()
        .unwrap_or_else(|| format!("_new_{}", uuid::Uuid::now_v7().simple()));
    tx.execute(
        "INSERT INTO categories (kind, parent_id, name, key, position) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            kind.as_str(),
            v.parent_id,
            name,
            placeholder,
            position as i64
        ],
    )?;
    let id = tx.last_insert_rowid();
    if key.is_none() {
        let generated = format!("c{id}");
        check_unique(tx, kind, v.parent_id, name, Some(&generated), Some(id))?;
        tx.execute(
            "UPDATE categories SET key = ?2 WHERE id = ?1",
            params![id, generated],
        )?;
    }
    Ok(id)
}

/// 整体替换名称、key、上级；换了上级就排到新同级最后。类型不能改。
pub fn update(tx: &Transaction<'_>, id: i64, v: Input) -> Result<(), Error> {
    let cur = current(tx, id)?;
    if v.kind.is_some_and(|k| k != cur.kind) {
        return Err(Error::Invalid("分类的类型不能改".into()));
    }
    let name = check_name(&v.name).map_err(invalid)?;
    let key = check_key(v.key)?.unwrap_or_else(|| cur.key.clone());
    if cur.key == fallback_key(cur.kind) && (key != cur.key || v.parent_id.is_some()) {
        return Err(Error::Conflict("兜底分类必须保持一级，key 不能改".into()));
    }
    if v.parent_id != cur.parent {
        check_parent(tx, cur.kind, v.parent_id, Some(id))?;
    }
    check_unique(tx, cur.kind, v.parent_id, name, Some(&key), Some(id))?;
    let position = if v.parent_id == cur.parent {
        None
    } else {
        Some(siblings(tx, cur.kind, v.parent_id)?.len() as i64 + 1)
    };
    tx.execute(
        "UPDATE categories SET name = ?2, key = ?3, parent_id = ?4,
                               position = coalesce(?5, position) WHERE id = ?1",
        params![id, name, key, v.parent_id, position],
    )?;
    Ok(())
}

/// 有账目、有二级、兜底分类都不能删。
pub fn delete(tx: &Transaction<'_>, id: i64) -> Result<(), Error> {
    let cur = current(tx, id)?;
    if cur.key == fallback_key(cur.kind) {
        return Err(Error::Conflict("兜底分类不能删".into()));
    }
    let subs = count(tx, "categories WHERE parent_id = ?1", id)?;
    if subs > 0 {
        return Err(Error::Conflict(format!("下面还有 {subs} 个二级，先删它们")));
    }
    let used = count(tx, "entries WHERE category_id = ?1", id)?;
    if used > 0 {
        return Err(Error::Conflict(format!("还有 {used} 笔账目在用，不能删除")));
    }
    tx.execute("DELETE FROM categories WHERE id = ?1", [id])?;
    Ok(())
}

/// 同级内挪到第 `to` 位（从 0 数，超出夹到末尾）。
pub fn move_to(tx: &Transaction<'_>, id: i64, to: usize) -> Result<(), Error> {
    let cur = current(tx, id)?;
    reorder(
        tx,
        "categories",
        siblings(tx, cur.kind, cur.parent)?,
        id,
        to,
    )?;
    Ok(())
}
