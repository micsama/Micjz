//! 分类、账户、标签的唯一定义。v2 迁入 SQLite。

use crate::flow::Kind;

pub struct Category {
    pub name: &'static str,
    pub subs: &'static [&'static str],
}

const fn c(name: &'static str, subs: &'static [&'static str]) -> Category {
    Category { name, subs }
}

pub const OTHER: &str = "其他";

const EXPENSE: &[Category] = &[
    c("餐饮", &["三餐", "咖啡饮品", "零食水果", "聚餐"]),
    c(
        "交通",
        &["公交地铁", "打车", "加油充电", "停车", "火车飞机"],
    ),
    c("购物", &["日用", "服饰", "数码", "家居"]),
    c("居住", &["房租", "水电燃气", "话费网费", "物业"]),
    c("娱乐", &["会员订阅", "游戏", "电影演出", "旅行"]),
    c("医疗", &["药品", "门诊"]),
    c("人情", &["红包", "礼物", "请客"]),
    c(OTHER, &[]),
];

const INCOME: &[Category] = &[
    c("工资", &[]),
    c("奖金", &[]),
    c("红包", &[]),
    c("退款", &[]),
    c("理财", &[]),
    c(OTHER, &[]),
];

pub const ACCOUNTS: &[&str] = &["微信", "银行卡"];

pub const TAGS: &[&str] = &["恋爱"];

pub fn categories(kind: Kind) -> &'static [Category] {
    match kind {
        Kind::Expense => EXPENSE,
        Kind::Income => INCOME,
    }
}

pub fn category(kind: Kind, name: &str) -> Option<&'static Category> {
    categories(kind).iter().find(|c| c.name == name)
}
