//! 标签目录的读写规则，供 HTTP 和命令行共用。
use super::{Tag, check_name};
use crate::db::Error;
use rusqlite::{Connection, Transaction, params};

pub fn list(conn: &Connection) -> Result<Vec<Tag>, Error> {
    Ok(conn
        .prepare("SELECT id, name FROM tags ORDER BY id")?
        .query_map([], |r| {
            Ok(Tag {
                id: r.get(0)?,
                name: r.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?)
}

fn require(tx: &Transaction<'_>, id: i64) -> Result<(), Error> {
    if !tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM tags WHERE id = ?1)",
        [id],
        |r| r.get::<_, bool>(0),
    )? {
        return Err(Error::Missing("标签不存在".into()));
    }
    Ok(())
}

/// 新建（`id` 为空）或改名；返回 id。
pub fn save(tx: &Transaction<'_>, id: Option<i64>, name: &str) -> Result<i64, Error> {
    let name = check_name(name).map_err(|e| Error::Invalid(e.to_string()))?;
    if let Some(id) = id {
        require(tx, id)?;
    }
    if tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM tags WHERE name = ?1 AND id IS NOT ?2)",
        params![name, id],
        |r| r.get::<_, bool>(0),
    )? {
        return Err(Error::Conflict(format!("已经有标签「{name}」")));
    }
    Ok(match id {
        Some(id) => {
            tx.execute("UPDATE tags SET name = ?2 WHERE id = ?1", params![id, name])?;
            id
        }
        None => {
            tx.execute("INSERT INTO tags (name) VALUES (?1)", [name])?;
            tx.last_insert_rowid()
        }
    })
}

pub fn delete(tx: &Transaction<'_>, id: i64) -> Result<(), Error> {
    require(tx, id)?;
    let used: i64 = tx.query_row(
        "SELECT count(*) FROM entry_tags WHERE tag_id = ?1",
        [id],
        |r| r.get(0),
    )?;
    if used > 0 {
        return Err(Error::Conflict(format!("还有 {used} 笔账目在用，不能删除")));
    }
    tx.execute("DELETE FROM tags WHERE id = ?1", [id])?;
    Ok(())
}
