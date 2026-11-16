//! 本地可编辑目录，启动时加载并校验；配置顺序即模糊分类优先级。
use crate::flow::Kind;
use serde::Deserialize;
use std::sync::{LazyLock, OnceLock};

pub const OTHER: &str = "其他";
pub struct Category {
    pub name: &'static str,
    pub subs: &'static [&'static str],
    pub keys: &'static [&'static str],
}
struct Catalog {
    expense: Vec<Category>,
    income: Vec<Category>,
    accounts: &'static [&'static str],
    tags: &'static [&'static str],
}
static CATALOG: OnceLock<Catalog> = OnceLock::new();
pub static ACCOUNTS: LazyLock<&'static [&'static str]> = LazyLock::new(|| get().accounts);
pub static TAGS: LazyLock<&'static [&'static str]> = LazyLock::new(|| get().tags);
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    expense: Vec<Row>,
    income: Vec<Row>,
    accounts: Vec<String>,
    tags: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    name: String,
    subs: Vec<String>,
    keys: Vec<String>,
}
fn strings(values: Vec<String>) -> &'static [&'static str] {
    Box::leak(
        values
            .into_iter()
            .map(|s| &*Box::leak(s.into_boxed_str()))
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    )
}
pub fn init() -> anyhow::Result<()> {
    if CATALOG.get().is_some() {
        return Ok(());
    }
    let text = std::fs::read_to_string("catalog.toml")?;
    let file: File = toml::from_str(&text)?;
    let mut keys = std::collections::HashSet::new();
    for rows in [&file.expense, &file.income] {
        let mut names = std::collections::HashSet::new();
        anyhow::ensure!(rows.iter().any(|r| r.name == OTHER), "分类须含其他");
        for row in rows {
            anyhow::ensure!(
                !row.name.trim().is_empty() && names.insert(&row.name),
                "一级分类重复或为空"
            );
            anyhow::ensure!(
                row.keys.len() == row.subs.len().max(1),
                "分类 key 数量不匹配"
            );
            let mut subs = std::collections::HashSet::new();
            anyhow::ensure!(
                row.subs
                    .iter()
                    .all(|s| !s.trim().is_empty() && subs.insert(s)),
                "二级分类重复或为空"
            );
            for key in &row.keys {
                anyhow::ensure!(
                    !key.is_empty()
                        && key.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
                        && keys.insert(key),
                    "分类 key 须唯一且为小写英文"
                );
            }
        }
    }
    for values in [&file.accounts, &file.tags] {
        let mut seen = std::collections::HashSet::new();
        anyhow::ensure!(
            values
                .iter()
                .all(|s| !s.trim().is_empty() && seen.insert(s)),
            "账户/标签重复或为空"
        );
    }
    let rows = |rows: Vec<Row>| {
        rows.into_iter()
            .map(|r| Category {
                name: Box::leak(r.name.into_boxed_str()),
                subs: strings(r.subs),
                keys: strings(r.keys),
            })
            .collect()
    };
    let _ = CATALOG.set(Catalog {
        expense: rows(file.expense),
        income: rows(file.income),
        accounts: strings(file.accounts),
        tags: strings(file.tags),
    });
    Ok(())
}
fn get() -> &'static Catalog {
    if CATALOG.get().is_none() {
        init().expect("目录配置有效");
    }
    CATALOG.get().expect("目录已加载")
}
pub fn categories(kind: Kind) -> &'static [Category] {
    match kind {
        Kind::Expense => &get().expense,
        Kind::Income => &get().income,
    }
}
pub fn category(kind: Kind, name: &str) -> Option<&'static Category> {
    categories(kind).iter().find(|c| c.name == name)
}
pub fn model_category(kind: Kind, key: &str) -> Option<(&'static str, Option<&'static str>)> {
    categories(kind).iter().find_map(|c| {
        c.keys
            .iter()
            .position(|k| *k == key)
            .map(|i| (c.name, c.subs.get(i).copied()))
    })
}
