//! 分类、账户、标签：存 SQLite，命令行增删改；每个请求读一份快照（几十行，不缓存，改了立即生效）。
//! 新库用 `src/seed.sql` 初始化。`position` 是优先级：菜单顺序、模型分类模糊时的先后，
//! 识别不出账户时取第一个账户。

pub mod accounts;
pub mod categories;
pub mod tags;

use std::collections::HashMap;

use anyhow::{Context, bail, ensure};
use rusqlite::{Connection, OptionalExtension, Transaction, params};

use crate::flow::Kind;
use accounts::Account;

/// 兜底分类的 key：模型给不出分类、切换类型时用。必须是一级，不能删，可以改名。
pub fn fallback_key(kind: Kind) -> &'static str {
    match kind {
        Kind::Expense => "other",
        Kind::Income => "other_in",
    }
}

pub struct Catalog {
    expense: Vec<Category>,
    income: Vec<Category>,
    /// 按优先级排序，至少一个。
    pub accounts: Vec<Account>,
    pub tags: Vec<Tag>,
}

#[derive(serde::Serialize)]
pub struct Tag {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardType {
    Debit,
    Credit,
}

impl CardType {
    fn as_str(self) -> &'static str {
        match self {
            CardType::Debit => "debit",
            CardType::Credit => "credit",
        }
    }

    fn label(self) -> &'static str {
        match self {
            CardType::Debit => "储蓄卡",
            CardType::Credit => "信用卡",
        }
    }

    fn parse(s: &str) -> anyhow::Result<CardType> {
        match s {
            "储蓄卡" | "debit" => Ok(CardType::Debit),
            "信用卡" | "credit" => Ok(CardType::Credit),
            _ => bail!("卡类型须为 储蓄卡 或 信用卡：{s}"),
        }
    }
}

/// 每个分类都有模型 key，一级、二级的 key 都会给模型。
pub struct Category {
    pub id: i64,
    pub name: String,
    pub key: String,
    pub subs: Vec<Sub>,
}

pub struct Sub {
    pub id: i64,
    pub name: String,
    pub key: String,
}

impl Catalog {
    pub fn categories(&self, kind: Kind) -> &[Category] {
        match kind {
            Kind::Expense => &self.expense,
            Kind::Income => &self.income,
        }
    }

    /// 分类 id → (一级, 二级)；`kind` 不符也算没有。
    pub fn locate(&self, kind: Kind, id: i64) -> Option<(&Category, Option<&Sub>)> {
        self.categories(kind).iter().find_map(|c| {
            if c.id == id {
                return Some((c, None));
            }
            c.subs.iter().find(|s| s.id == id).map(|s| (c, Some(s)))
        })
    }

    /// 展示用的分类路径「一级/二级」。
    pub fn category_path(&self, kind: Kind, id: i64) -> Option<String> {
        self.locate(kind, id).map(|(c, sub)| match sub {
            Some(s) => format!("{}/{}", c.name, s.name),
            None => c.name.clone(),
        })
    }

    /// 模型给的 key → 分类 id；一级、二级都可以。
    pub fn by_key(&self, kind: Kind, key: &str) -> Option<i64> {
        self.categories(kind).iter().find_map(|c| {
            if c.key == key {
                return Some(c.id);
            }
            c.subs.iter().find(|s| s.key == key).map(|s| s.id)
        })
    }

    pub fn account(&self, id: i64) -> Option<&Account> {
        self.accounts.iter().find(|a| a.id == id)
    }

    /// 模型和快捷指令给的是账户名，在入口换成 id。
    pub fn account_named(&self, name: &str) -> Option<i64> {
        self.accounts.iter().find(|a| a.name == name).map(|a| a.id)
    }

    pub fn tag(&self, id: i64) -> Option<&Tag> {
        self.tags.iter().find(|t| t.id == id)
    }

    /// 兜底分类的 id；`load` 已保证存在。
    pub fn fallback(&self, kind: Kind) -> i64 {
        let key = fallback_key(kind);
        self.categories(kind)
            .iter()
            .find(|c| c.key == key)
            .expect("load 已保证兜底分类存在")
            .id
    }

    /// 识别不出账户时的默认值：优先级第一。
    pub fn default_account(&self) -> i64 {
        self.accounts[0].id
    }

    /// 给 prompt 用：每个账户一行，银行卡只带尾号，完整卡号不发给模型。
    pub fn account_lines(&self) -> String {
        self.accounts
            .iter()
            .map(|a| match &a.card {
                Some(card) => {
                    let number = &card.number;
                    let tail = &number[number.len().saturating_sub(4)..];
                    format!(
                        "{}（{}{} 尾号 {tail}）",
                        a.name,
                        a.brand
                            .as_ref()
                            .map(|b| format!("{b} "))
                            .unwrap_or_default(),
                        card.r#type.label()
                    )
                }
                None => match &a.brand {
                    Some(b) if b != &a.name => format!("{}（{b}）", a.name),
                    _ => a.name.clone(),
                },
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// 给 prompt 用：每个分类一行，一级 `key: 一级`，二级 `key: 一级/二级`。
    pub fn key_lines(&self, kind: Kind) -> String {
        let mut lines = Vec::new();
        for c in self.categories(kind) {
            lines.push(format!("{}: {}", c.key, c.name));
            for s in &c.subs {
                lines.push(format!("{}: {}/{}", s.key, c.name, s.name));
            }
        }
        lines.join("\n")
    }
}

/// 新库的初始分类、账户、标签，建库时执行。
pub const SEED: &str = include_str!("seed.sql");

/// 测试用：初始数据的快照。
#[cfg(test)]
pub fn defaults() -> Catalog {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(crate::db::SCHEMA).unwrap();
    conn.execute_batch(SEED).unwrap();
    load(&conn).unwrap()
}

/// 读快照；没有账户、缺兜底分类时报错（命令行不允许删掉它们）。
pub fn load(conn: &Connection) -> anyhow::Result<Catalog> {
    let accounts = accounts::list(conn)?;
    ensure!(!accounts.is_empty(), "没有任何账户，先 account add");
    let mut catalog = Catalog {
        expense: Vec::new(),
        income: Vec::new(),
        accounts,
        tags: tags::list(conn)?,
    };
    // 先读全部一级，再读二级，各自按优先级。
    let mut stmt = conn.prepare(
        "SELECT id, kind, parent_id, name, key FROM categories
         ORDER BY parent_id IS NOT NULL, position, id",
    )?;
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
            Some(&i) => list[i].subs.push(Sub { id, name, key }),
            None => {
                parents.insert(id, list.len());
                list.push(Category {
                    id,
                    name,
                    key,
                    subs: Vec::new(),
                });
            }
        }
    }
    for kind in [Kind::Expense, Kind::Income] {
        let key = fallback_key(kind);
        ensure!(
            catalog.categories(kind).iter().any(|c| c.key == key),
            "缺少兜底的一级分类（key {key}）"
        );
    }
    Ok(catalog)
}

fn names(conn: &Connection, table: &str) -> rusqlite::Result<Vec<String>> {
    conn.prepare(&format!("SELECT name FROM {table} ORDER BY id"))?
        .query_map([], |r| r.get(0))?
        .collect()
}

pub fn account_ids(tx: &Transaction<'_>) -> rusqlite::Result<Vec<i64>> {
    tx.prepare("SELECT id FROM accounts ORDER BY position, id")?
        .query_map([], |r| r.get(0))?
        .collect()
}

/// 把 `id` 挪到下标 `to`，夹到两端，其余顺延，整组重写 position。
pub fn reorder(
    tx: &Transaction<'_>,
    table: &str,
    mut ids: Vec<i64>,
    id: i64,
    to: usize,
) -> rusqlite::Result<()> {
    ids.retain(|&i| i != id);
    ids.insert(to.min(ids.len()), id);
    for (i, id) in ids.iter().enumerate() {
        tx.execute(
            &format!("UPDATE {table} SET position = ?2 WHERE id = ?1"),
            params![id, i as i64 + 1],
        )?;
    }
    Ok(())
}

fn position(s: &str) -> anyhow::Result<usize> {
    s.parse::<usize>()
        .ok()
        .filter(|n| *n > 0)
        .map(|n| n - 1)
        .with_context(|| format!("位置须为正整数：{s}"))
}

/// 卡号只收数字，可带空格分组。
pub fn check_card_number(s: &str) -> anyhow::Result<String> {
    let digits: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    ensure!(
        digits.len() >= 4 && digits.bytes().all(|b| b.is_ascii_digit()),
        "卡号须为至少 4 位数字：{s}"
    );
    Ok(digits)
}

/// 名称会出现在菜单和多选结果（按换行合并）里，分类名还用 `/` 分隔路径。
pub fn check_name(name: &str) -> anyhow::Result<&str> {
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
  category move   <支出|收入> <一级>[/<二级>] <位置>    同级内从 1 数
  account  list
  account  add    <名称> [储蓄卡|信用卡 <卡号>]
  account  move   <名称> <位置>
  account  rename <名称> <新名称> | rm <名称>
  tag      list | add <名称> | rename <名称> <新名称> | rm <名称>";

/// 命令行管理；返回要打印的结果。有账目引用的不能删。
pub fn cli(conn: &mut Connection, args: &[String]) -> anyhow::Result<String> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let tx = conn.transaction()?;
    let out = match args.as_slice() {
        ["category", "list"] => list_categories(&tx)?,
        ["category", "add", kind, path, key] => {
            let kind = parse_kind(kind)?;
            let (parent_id, name) = match path.split_once('/') {
                None => (None, *path),
                Some((parent, name)) => (Some(category_id(&tx, kind, parent)?), name),
            };
            categories::create(
                &tx,
                categories::Input {
                    kind: Some(kind),
                    parent_id,
                    name: name.into(),
                    key: Some(key.to_string()),
                },
            )?;
            format!("已添加分类 {path}（{key}）")
        }
        ["category", "rename", kind, path, new] => {
            let kind = parse_kind(kind)?;
            let id = category_id(&tx, kind, path)?;
            let parent_id = tx.query_row(
                "SELECT parent_id FROM categories WHERE id = ?1",
                [id],
                |r| r.get(0),
            )?;
            categories::update(
                &tx,
                id,
                categories::Input {
                    kind: None,
                    parent_id,
                    name: new.to_string(),
                    key: None,
                },
            )?;
            format!("已改名 {path} → {new}")
        }
        ["category", "rm", kind, path] => {
            let id = category_id(&tx, parse_kind(kind)?, path)?;
            categories::delete(&tx, id)?;
            format!("已删除分类 {path}")
        }
        ["category", "move", kind, path, to] => {
            let id = category_id(&tx, parse_kind(kind)?, path)?;
            categories::move_to(&tx, id, position(to)?)?;
            format!("已移动 {path} 到第 {to} 位")
        }
        ["account", "list"] => list_accounts(&tx)?,
        ["account", "add", name, card @ ..] => {
            let card = match card {
                [] => None,
                [t, number] => Some((CardType::parse(t)?, check_card_number(number)?)),
                _ => bail!("{USAGE}"),
            };
            let position = account_ids(&tx)?.len() + 1;
            tx.execute(
                "INSERT INTO accounts (name, card_type, card_number, position) VALUES (?1, ?2, ?3, ?4)",
                params![
                    check_name(name)?,
                    card.as_ref().map(|(t, _)| t.as_str()),
                    card.as_ref().map(|(_, n)| n),
                    position as i64
                ],
            )
            .with_context(|| format!("{name} 已存在？"))?;
            format!("已添加 {name}")
        }
        ["account", "move", name, to] => {
            let id = name_id(&tx, "accounts", name)?;
            reorder(&tx, "accounts", account_ids(&tx)?, id, position(to)?)?;
            format!("已移动 {name} 到第 {to} 位")
        }
        ["account", "rename", name, new] => {
            let id = name_id(&tx, "accounts", name)?;
            tx.execute(
                "UPDATE accounts SET name = ?2 WHERE id = ?1",
                params![id, check_name(new)?],
            )
            .with_context(|| format!("{new} 已存在？"))?;
            format!("已改名 {name} → {new}")
        }
        ["account", "rm", name] => {
            accounts::delete(&tx, name_id(&tx, "accounts", name)?)?;
            format!("已删除 {name}")
        }
        ["tag", "list"] => names(&tx, "tags")?.join("\n"),
        ["tag", "add", name] => {
            tags::save(&tx, None, name)?;
            format!("已添加 {name}")
        }
        ["tag", "rename", name, new] => {
            tags::save(&tx, Some(name_id(&tx, "tags", name)?), new)?;
            format!("已改名 {name} → {new}")
        }
        ["tag", "rm", name] => {
            tags::delete(&tx, name_id(&tx, "tags", name)?)?;
            format!("已删除 {name}")
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
    find_name(tx, table, name)?.with_context(|| format!("没有 {name}"))
}

pub fn find_name(conn: &Connection, table: &str, name: &str) -> rusqlite::Result<Option<i64>> {
    conn.query_row(
        &format!("SELECT id FROM {table} WHERE name = ?1"),
        [name],
        |r| r.get(0),
    )
    .optional()
}

fn list_accounts(tx: &Transaction<'_>) -> anyhow::Result<String> {
    let mut stmt = tx.prepare(
        "SELECT a.name, a.card_type, a.card_number,
                (SELECT count(*) FROM entries e WHERE a.id IN (e.account_id, e.to_account_id))
         FROM accounts a ORDER BY a.position, a.id",
    )?;
    let mut rows = stmt.query([])?;
    let mut out = Vec::new();
    while let Some(r) = rows.next()? {
        let (name, card_type, number, used): (String, Option<String>, Option<String>, i64) =
            (r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?);
        let card = match card_type.zip(number) {
            Some((t, n)) => format!("  {} {n}", CardType::parse(&t)?.label()),
            None => String::new(),
        };
        out.push(format!("{name}{card}  {used} 笔"));
    }
    Ok(out.join("\n"))
}

fn list_categories(tx: &Transaction<'_>) -> anyhow::Result<String> {
    let mut stmt = tx.prepare(
        "SELECT c.kind, c.name, c.key, p.name,
                (SELECT count(*) FROM entries e WHERE e.category_id = c.id)
         FROM categories c LEFT JOIN categories p ON c.parent_id = p.id
         ORDER BY c.kind = 'income', coalesce(p.position, c.position), coalesce(p.id, c.id),
                  c.parent_id IS NOT NULL, c.position",
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
