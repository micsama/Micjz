//! 分类、账户、标签：存 SQLite，命令行增删改；每个请求读一份快照（几十行，不缓存，改了立即生效）。
//! 新库用 `src/seed.sql` 初始化。顺序按添加先后，也是模型分类模糊时的优先级。

use std::collections::HashMap;

use anyhow::{Context, bail, ensure};
use rusqlite::{Connection, OptionalExtension, Transaction, params};

use crate::flow::Kind;

pub const OTHER: &str = "其他";

pub struct Catalog {
    expense: Vec<Category>,
    income: Vec<Category>,
    pub accounts: Vec<String>,
    pub tags: Vec<String>,
}

/// 每个分类都有模型 key；只有叶子（没有二级的一级、二级）的 key 会给模型。
pub struct Category {
    pub name: String,
    pub key: String,
    /// `(key, 名称)`
    pub subs: Vec<(String, String)>,
}

impl Category {
    pub fn sub_names(&self) -> impl Iterator<Item = &str> {
        self.subs.iter().map(|(_, name)| name.as_str())
    }

    pub fn has_sub(&self, name: &str) -> bool {
        self.sub_names().any(|s| s == name)
    }
}

impl Catalog {
    pub fn categories(&self, kind: Kind) -> &[Category] {
        match kind {
            Kind::Expense => &self.expense,
            Kind::Income => &self.income,
        }
    }

    pub fn category(&self, kind: Kind, name: &str) -> Option<&Category> {
        self.categories(kind).iter().find(|c| c.name == name)
    }

    /// 模型给的叶子 key → (一级, 二级)。
    pub fn by_key(&self, kind: Kind, key: &str) -> Option<(&str, Option<&str>)> {
        self.categories(kind).iter().find_map(|c| {
            if c.subs.is_empty() && c.key == key {
                return Some((c.name.as_str(), None));
            }
            c.subs
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, sub)| (c.name.as_str(), Some(sub.as_str())))
        })
    }

    /// 给 prompt 用：每个叶子一行 `key: 一级/二级`。
    pub fn key_lines(&self, kind: Kind) -> String {
        let mut lines = Vec::new();
        for c in self.categories(kind) {
            if c.subs.is_empty() {
                lines.push(format!("{}: {}", c.key, c.name));
            }
            for (k, s) in &c.subs {
                lines.push(format!("{k}: {}/{s}", c.name));
            }
        }
        lines.join("\n")
    }
}

/// 新库（还没有分类）写入初始数据。
pub fn seed(conn: &Connection) -> anyhow::Result<()> {
    let empty: bool = conn.query_row("SELECT NOT EXISTS (SELECT 1 FROM categories)", [], |r| {
        r.get(0)
    })?;
    if empty {
        conn.execute_batch(&format!("BEGIN;\n{}\nCOMMIT;", include_str!("seed.sql")))
            .context("写入初始分类")?;
    }
    Ok(())
}

/// 测试用：初始数据的快照。
#[cfg(test)]
pub fn defaults() -> Catalog {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(crate::db::SCHEMA).unwrap();
    seed(&conn).unwrap();
    load(&conn).unwrap()
}

pub fn load(conn: &Connection) -> rusqlite::Result<Catalog> {
    let mut catalog = Catalog {
        expense: Vec::new(),
        income: Vec::new(),
        accounts: names(conn, "accounts")?,
        tags: names(conn, "tags")?,
    };
    // 父分类的 id 总比子分类小，按 id 顺序读即可先见到父。
    let mut stmt =
        conn.prepare("SELECT id, kind, parent_id, name, key FROM categories ORDER BY id")?;
    let mut rows = stmt.query([])?;
    let mut parents: HashMap<i64, usize> = HashMap::new();
    while let Some(r) = rows.next()? {
        let (id, kind, parent, name, key): (i64, String, Option<i64>, String, String) =
            (r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?);
        let list = match kind.as_str() {
            "income" => &mut catalog.income,
            _ => &mut catalog.expense,
        };
        match parent.and_then(|p| parents.get(&p)) {
            Some(&i) => list[i].subs.push((key, name)),
            None => {
                parents.insert(id, list.len());
                list.push(Category {
                    name,
                    key,
                    subs: Vec::new(),
                });
            }
        }
    }
    Ok(catalog)
}

fn names(conn: &Connection, table: &str) -> rusqlite::Result<Vec<String>> {
    conn.prepare(&format!("SELECT name FROM {table} ORDER BY id"))?
        .query_map([], |r| r.get(0))?
        .collect()
}

fn insert_category(
    tx: &Transaction<'_>,
    kind: Kind,
    parent: Option<i64>,
    name: &str,
    key: &str,
) -> anyhow::Result<i64> {
    ensure!(
        !key.is_empty()
            && key
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
        "key 须为小写英文、数字或下划线：{key}"
    );
    if let Some(owner) = tx
        .query_row("SELECT name FROM categories WHERE key = ?1", [key], |r| {
            r.get::<_, String>(0)
        })
        .optional()?
    {
        bail!("key {key} 已被「{owner}」使用");
    }
    tx.execute(
        "INSERT INTO categories (kind, parent_id, name, key) VALUES (?1, ?2, ?3, ?4)",
        params![kind.as_str(), parent, check_name(name)?, key],
    )?;
    Ok(tx.last_insert_rowid())
}

/// 名称会出现在菜单和多选结果（按换行合并）里，分类名还用 `/` 分隔路径。
fn check_name(name: &str) -> anyhow::Result<&str> {
    let name = name.trim();
    ensure!(
        !name.is_empty() && !name.contains(['/', '\n']),
        "名称不能为空，不能含 / 或换行：{name:?}"
    );
    Ok(name)
}

const USAGE: &str = "\
用法：micjz [config.toml] <命令>
  category list
  category add    <支出|收入> <一级>[/<二级>] <key>
  category rename <支出|收入> <一级>[/<二级>] <新名称>
  category rm     <支出|收入> <一级>[/<二级>]
  account  list | add <名称> | rename <名称> <新名称> | rm <名称>
  tag      list | add <名称> | rename <名称> <新名称> | rm <名称>";

/// 命令行管理；返回要打印的结果。有账目引用的不能删。
pub fn cli(conn: &mut Connection, args: &[String]) -> anyhow::Result<String> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let tx = conn.transaction()?;
    let out = match args.as_slice() {
        ["category", "list"] => list_categories(&tx)?,
        ["category", "add", kind, path, key] => {
            let kind = parse_kind(kind)?;
            match path.split_once('/') {
                None => insert_category(&tx, kind, None, path, key)?,
                Some((parent, name)) => {
                    let parent = category_id(&tx, kind, parent)?;
                    insert_category(&tx, kind, Some(parent), name, key)?
                }
            };
            format!("已添加分类 {path}（{key}）")
        }
        ["category", "rename", kind, path, new] => {
            let id = category_id(&tx, parse_kind(kind)?, path)?;
            ensure!(*path != OTHER, "「{OTHER}」是兜底分类，不能改名");
            tx.execute(
                "UPDATE categories SET name = ?2 WHERE id = ?1",
                params![id, check_name(new)?],
            )?;
            format!("已改名 {path} → {new}")
        }
        ["category", "rm", kind, path] => {
            let id = category_id(&tx, parse_kind(kind)?, path)?;
            ensure!(*path != OTHER, "「{OTHER}」是兜底分类，不能删");
            let subs = count(&tx, "categories WHERE parent_id = ?1", id)?;
            ensure!(subs == 0, "{path} 下还有 {subs} 个二级分类，先删它们");
            let used = count(&tx, "entries WHERE category_id = ?1", id)?;
            ensure!(used == 0, "{path} 下有 {used} 笔账，不能删");
            tx.execute("DELETE FROM categories WHERE id = ?1", [id])?;
            format!("已删除分类 {path}")
        }
        [table @ ("account" | "tag"), rest @ ..] => {
            let (table, used_sql) = match *table {
                "account" => ("accounts", "entries WHERE account_id = ?1"),
                _ => ("tags", "entry_tags WHERE tag_id = ?1"),
            };
            match rest {
                ["list"] => names(&tx, table)?.join("\n"),
                ["add", name] => {
                    tx.execute(
                        &format!("INSERT INTO {table} (name) VALUES (?1)"),
                        [check_name(name)?],
                    )
                    .with_context(|| format!("{name} 已存在？"))?;
                    format!("已添加 {name}")
                }
                ["rename", name, new] => {
                    let id = name_id(&tx, table, name)?;
                    tx.execute(
                        &format!("UPDATE {table} SET name = ?2 WHERE id = ?1"),
                        params![id, check_name(new)?],
                    )
                    .with_context(|| format!("{new} 已存在？"))?;
                    format!("已改名 {name} → {new}")
                }
                ["rm", name] => {
                    let id = name_id(&tx, table, name)?;
                    let used = count(&tx, used_sql, id)?;
                    ensure!(used == 0, "{name} 被 {used} 笔账使用，不能删");
                    tx.execute(&format!("DELETE FROM {table} WHERE id = ?1"), [id])?;
                    format!("已删除 {name}")
                }
                _ => bail!("{USAGE}"),
            }
        }
        _ => bail!("{USAGE}"),
    };
    tx.commit()?;
    Ok(out)
}

fn parse_kind(s: &str) -> anyhow::Result<Kind> {
    match s {
        "支出" => Ok(Kind::Expense),
        "收入" => Ok(Kind::Income),
        _ => bail!("类型须为 支出 或 收入：{s}"),
    }
}

/// `一级` 或 `一级/二级` → id。
fn category_id(tx: &Transaction<'_>, kind: Kind, path: &str) -> anyhow::Result<i64> {
    let (top, sub) = match path.split_once('/') {
        Some((top, sub)) => (top, Some(sub)),
        None => (path, None),
    };
    let top_id: Option<i64> = tx
        .query_row(
            "SELECT id FROM categories WHERE kind = ?1 AND parent_id IS NULL AND name = ?2",
            params![kind.as_str(), top],
            |r| r.get(0),
        )
        .optional()?;
    let top_id = top_id.with_context(|| format!("没有分类 {top}"))?;
    let Some(sub) = sub else { return Ok(top_id) };
    tx.query_row(
        "SELECT id FROM categories WHERE parent_id = ?1 AND name = ?2",
        params![top_id, sub],
        |r| r.get(0),
    )
    .optional()?
    .with_context(|| format!("没有分类 {path}"))
}

pub fn name_id(tx: &Transaction<'_>, table: &str, name: &str) -> anyhow::Result<i64> {
    tx.query_row(
        &format!("SELECT id FROM {table} WHERE name = ?1"),
        [name],
        |r| r.get(0),
    )
    .optional()?
    .with_context(|| format!("没有 {name}"))
}

fn count(tx: &Transaction<'_>, from_where: &str, id: i64) -> rusqlite::Result<i64> {
    tx.query_row(&format!("SELECT count(*) FROM {from_where}"), [id], |r| {
        r.get(0)
    })
}

fn list_categories(tx: &Transaction<'_>) -> anyhow::Result<String> {
    let mut stmt = tx.prepare(
        "SELECT c.kind, c.name, c.key, p.name,
                (SELECT count(*) FROM entries e WHERE e.category_id = c.id)
         FROM categories c LEFT JOIN categories p ON c.parent_id = p.id
         ORDER BY c.kind = 'income', coalesce(c.parent_id, c.id), c.parent_id IS NOT NULL, c.id",
    )?;
    let mut rows = stmt.query([])?;
    let mut out = Vec::new();
    while let Some(r) = rows.next()? {
        let (kind, name, key, parent, used): (String, String, String, Option<String>, i64) =
            (r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?);
        let kind = if kind == "income" { "收入" } else { "支出" };
        let indent = if parent.is_some() { "    " } else { "" };
        out.push(format!("{kind} {indent}{name}  {key}  {used} 笔"));
    }
    Ok(out.join("\n"))
}

#[cfg(test)]
mod tests;
