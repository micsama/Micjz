//! 分类、账户、标签的唯一来源：工作目录下的 `catalog.toml`，启动时加载并校验。
//! 文件里的顺序就是菜单顺序，也是模型分类模糊时的优先级。

use std::{collections::HashSet, sync::LazyLock};

use anyhow::{Context, ensure};
use serde::Deserialize;

use crate::flow::Kind;

pub const OTHER: &str = "其他";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    expense: Vec<Category>,
    income: Vec<Category>,
    pub accounts: Vec<String>,
    pub tags: Vec<String>,
}

/// 叶子才有模型 key：有二级时 key 在二级上，没有二级时 key 在一级上。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Category {
    pub name: String,
    #[serde(default)]
    pub key: Option<String>,
    /// `[key, 名称]`
    #[serde(default)]
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

static CATALOG: LazyLock<anyhow::Result<Catalog>> = LazyLock::new(|| load("catalog.toml"));

/// 启动时调用：文件有问题就带着原因退出。
pub fn check() -> anyhow::Result<()> {
    CATALOG
        .as_ref()
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!("加载 catalog.toml：{e:#}"))
}

pub fn get() -> &'static Catalog {
    CATALOG.as_ref().expect("启动时已校验 catalog.toml")
}

fn load(path: &str) -> anyhow::Result<Catalog> {
    let text = std::fs::read_to_string(path).with_context(|| format!("读取 {path}"))?;
    let catalog: Catalog = toml::from_str(&text)?;
    let mut keys = HashSet::new();
    for list in [&catalog.expense, &catalog.income] {
        ensure!(
            list.iter().any(|c| c.name == OTHER),
            "收支分类都须含「{OTHER}」"
        );
        let mut names = HashSet::new();
        for c in list {
            ensure!(
                !c.name.trim().is_empty() && names.insert(&c.name),
                "一级分类重复或为空：{}",
                c.name
            );
            ensure!(
                c.key.is_some() == c.subs.is_empty(),
                "{}：无二级时写 key，有二级时只在二级上写 key",
                c.name
            );
            let mut subs = HashSet::new();
            for (key, name) in &c.subs {
                ensure!(
                    !name.trim().is_empty() && subs.insert(name),
                    "{} 的二级分类重复或为空",
                    c.name
                );
                ensure!(
                    valid_key(key) && keys.insert(key),
                    "分类 key 须唯一且为小写英文：{key}"
                );
            }
            if let Some(key) = &c.key {
                ensure!(
                    valid_key(key) && keys.insert(key),
                    "分类 key 须唯一且为小写英文：{key}"
                );
            }
        }
    }
    for list in [&catalog.accounts, &catalog.tags] {
        let mut seen = HashSet::new();
        ensure!(
            list.iter().all(|s| !s.trim().is_empty() && seen.insert(s)),
            "账户或标签重复或为空"
        );
    }
    Ok(catalog)
}

fn valid_key(key: &str) -> bool {
    !key.is_empty() && key.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
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

/// 模型给的 key → (一级, 二级)。
pub fn by_key(kind: Kind, key: &str) -> Option<(&'static str, Option<&'static str>)> {
    categories(kind).iter().find_map(|c| {
        if c.key.as_deref() == Some(key) {
            return Some((c.name.as_str(), None));
        }
        c.subs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, sub)| (c.name.as_str(), Some(sub.as_str())))
    })
}

/// 给 prompt 用：每行 `key: 一级/二级`。
pub fn key_lines(kind: Kind) -> String {
    categories(kind)
        .iter()
        .flat_map(|c| {
            let leaf = c.key.iter().map(move |k| format!("{k}: {}", c.name));
            let subs = c
                .subs
                .iter()
                .map(move |(k, s)| format!("{k}: {}/{s}", c.name));
            leaf.chain(subs)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn accounts() -> &'static [String] {
    &get().accounts
}

pub fn tags() -> &'static [String] {
    &get().tags
}
