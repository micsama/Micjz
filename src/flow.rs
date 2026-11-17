//! 记账草稿与交互步骤：快捷指令每提交一步，这里算出下一步。不做 IO。

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Datelike, FixedOffset, Utc};
use serde::{Deserialize, Serialize};

use crate::catalog::Catalog;

/// 显示和理解时间用的时区；以后加时区设置只改这里。
pub fn tz() -> FixedOffset {
    FixedOffset::east_opt(8 * 3600).expect("固定偏移合法")
}

pub fn now() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&tz())
}

/// 落库的时间：UTC，到秒。按时间段查询直接比较字符串，所以只有这一种写法。
pub fn stamp(at: DateTime<FixedOffset>) -> String {
    at.with_timezone(&Utc)
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Expense,
    Income,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Expense => "expense",
            Kind::Income => "income",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Kind::Expense => "支出",
            Kind::Income => "收入",
        }
    }
}

/// 识别结果与用户修改都落在这里。模型看不出的字段由服务器补默认值，
/// 补不了的（金额）为空，由用户补。分类、账户、标签只存 id，名字只在展示时取。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    pub id: String,
    pub kind: Kind,
    /// 未识别出金额时为空，确认前必须填。
    pub amount_cents: Option<i64>,
    /// 叶子，或还没选二级的一级。
    pub category_id: i64,
    /// 识别不出时是优先级第一的账户。
    pub account_id: i64,
    pub merchant: Option<String>,
    pub occurred_at: DateTime<FixedOffset>,
    pub note: Option<String>,
    pub tag_ids: Vec<i64>,
}

#[derive(Debug, thiserror::Error)]
pub enum FlowError {
    #[error("草稿无效：{0}")]
    InvalidDraft(String),
    #[error("未知字段：{0}")]
    UnknownField(String),
    #[error("字段 {field} 的值无效：{value}")]
    InvalidValue { field: String, value: String },
}

impl Draft {
    /// 空白草稿：识别不出任何东西时让用户手填；能定默认值的字段取默认值。
    pub fn blank(now: DateTime<FixedOffset>, cat: &Catalog) -> Draft {
        Draft {
            id: uuid::Uuid::now_v7().to_string(),
            kind: Kind::Expense,
            amount_cents: None,
            category_id: cat.fallback(Kind::Expense),
            account_id: cat.default_account(),
            merchant: None,
            occurred_at: now,
            note: None,
            tag_ids: Vec::new(),
        }
    }

    pub fn encode(&self) -> String {
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(self).expect("Draft 可序列化"))
    }

    /// 解码并按当前分类、账户、标签校验。
    pub fn decode(s: &str, cat: &Catalog) -> Result<Draft, FlowError> {
        let invalid = |e: &dyn std::fmt::Display| FlowError::InvalidDraft(e.to_string());
        let bytes = URL_SAFE_NO_PAD.decode(s).map_err(|e| invalid(&e))?;
        let draft: Draft = serde_json::from_slice(&bytes).map_err(|e| invalid(&e))?;
        draft.validate(cat).map_err(|e| invalid(&e))?;
        Ok(draft)
    }

    fn validate(&self, cat: &Catalog) -> Result<(), &'static str> {
        if self.amount_cents.is_some_and(|c| c <= 0) {
            return Err("金额须大于 0");
        }
        if cat.locate(self.kind, self.category_id).is_none() {
            return Err("分类不存在");
        }
        if cat.account(self.account_id).is_none() {
            return Err("账户不存在");
        }
        if self.tag_ids.iter().any(|&t| cat.tag(t).is_none()) {
            return Err("标签不存在");
        }
        Ok(())
    }

    /// 展示用的名字。草稿已按目录校验过，id 都在。
    fn category_path(&self, cat: &Catalog) -> String {
        cat.category_path(self.kind, self.category_id)
            .expect("草稿已校验分类")
    }

    fn account_name<'a>(&self, cat: &'a Catalog) -> &'a str {
        &cat.account(self.account_id).expect("草稿已校验账户").name
    }

    fn tag_names<'a>(&self, cat: &'a Catalog) -> Vec<&'a str> {
        self.tag_ids
            .iter()
            .map(|&t| cat.tag(t).expect("草稿已校验标签").name.as_str())
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StepType {
    Choose,
    ChooseMany,
    Number,
    Text,
    Date,
    Done,
}

/// 返回给快捷指令的扁平结构；字段全部输出，不省略。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Reply {
    #[serde(rename = "type")]
    pub step: StepType,
    pub summary: String,
    pub draft: String,
    pub field: String,
    pub options: Vec<String>,
    pub default: String,
    pub message: String,
}

impl Reply {
    pub fn done(message: impl Into<String>) -> Reply {
        Reply {
            step: StepType::Done,
            summary: String::new(),
            draft: String::new(),
            field: String::new(),
            options: Vec::new(),
            default: String::new(),
            message: message.into(),
        }
    }

    fn ask(
        draft: &Draft,
        now: DateTime<FixedOffset>,
        cat: &Catalog,
        step: StepType,
        field: &str,
    ) -> Reply {
        Reply {
            step,
            summary: summary(draft, now, cat),
            draft: draft.encode(),
            field: field.to_owned(),
            options: Vec::new(),
            default: String::new(),
            message: String::new(),
        }
    }

    fn options<S: ToString>(mut self, options: impl IntoIterator<Item = S>) -> Reply {
        self.options = options.into_iter().map(|o| o.to_string()).collect();
        self
    }

    fn default(mut self, default: impl Into<String>) -> Reply {
        self.default = default.into();
        self
    }

    /// 在提示语顶部加一行警告。
    pub fn warn(mut self, line: &str) -> Reply {
        self.summary = if self.summary.is_empty() {
            format!("⚠ {line}")
        } else {
            format!("⚠ {line}\n{}", self.summary)
        };
        self
    }
}

pub enum Outcome {
    Reply(Reply),
    /// 用户点了确认且金额已填，由调用方入账后回复。
    Confirm {
        draft: Draft,
        amount_cents: i64,
    },
}

#[derive(Clone, Copy)]
enum Action {
    Amount,
    Kind,
    Category,
    Account,
    Merchant,
    Time,
    Tags,
    Note,
    Confirm,
}

/// 主菜单：每项显示当前值，快捷指令原样回传展示文本。
fn menu_items(d: &Draft, cat: &Catalog) -> [(Action, String); 9] {
    let or_none = |v: &Option<String>| v.clone().unwrap_or_else(|| "无".to_owned());
    let tags = if d.tag_ids.is_empty() {
        "无".to_owned()
    } else {
        d.tag_names(cat).join(" / ")
    };
    [
        (Action::Amount, format!("💰 金额 · {}", amount_label(d))),
        (Action::Kind, format!("↔️ 类型 · {}", d.kind.label())),
        // 分类名自带 emoji，这一行只显示分类本身。
        (Action::Category, d.category_path(cat).replace('/', " / ")),
        (
            Action::Account,
            format!("💳 账户 · {}", d.account_name(cat)),
        ),
        (
            Action::Merchant,
            format!("🏪 商家 · {}", or_none(&d.merchant)),
        ),
        (
            Action::Time,
            format!("🕒 时间 · {}", d.occurred_at.format("%m-%d %H:%M")),
        ),
        (Action::Tags, format!("🏷️ 标签 · {tags}")),
        (Action::Note, format!("📝 备注 · {}", or_none(&d.note))),
        (Action::Confirm, "✅ 确认记账".to_owned()),
    ]
}

/// 主菜单不带摘要：选项里已有当前值，缺的显示「请补充」。
pub fn menu(draft: &Draft, now: DateTime<FixedOffset>, cat: &Catalog) -> Reply {
    let mut reply = Reply::ask(draft, now, cat, StepType::Choose, "action")
        .options(menu_items(draft, cat).map(|(_, label)| label));
    reply.summary.clear();
    reply
}

pub fn saved_message(draft: &Draft, amount_cents: i64, cat: &Catalog) -> String {
    format!(
        "已记账 ¥{} · {}",
        yuan(amount_cents),
        draft.category_path(cat)
    )
}

pub fn step(
    mut draft: Draft,
    field: &str,
    value: &str,
    now: DateTime<FixedOffset>,
    cat: &Catalog,
) -> Result<Outcome, FlowError> {
    let invalid = || FlowError::InvalidValue {
        field: field.to_owned(),
        value: value.to_owned(),
    };
    let ask = |d: &Draft, step, field| Reply::ask(d, now, cat, step, field);
    let menu = |d: &Draft| menu(d, now, cat);
    // 数字输入框只收数字，没金额时预填 0。
    let ask_amount = |d: &Draft| {
        ask(d, StepType::Number, "amount")
            .default(d.amount_cents.map(yuan).unwrap_or_else(|| "0".to_owned()))
    };

    let reply = match field {
        "action" => {
            let action = menu_items(&draft, cat)
                .into_iter()
                .find(|(_, label)| label == value)
                .map(|(action, _)| action)
                .ok_or_else(invalid)?;
            match action {
                Action::Confirm => match draft.amount_cents {
                    Some(amount_cents) => {
                        return Ok(Outcome::Confirm {
                            draft,
                            amount_cents,
                        });
                    }
                    None => ask_amount(&draft),
                },
                Action::Amount => ask_amount(&draft),
                Action::Kind => {
                    ask(&draft, StepType::Choose, "kind").options(["↘️ 支出", "↗️ 收入"])
                }
                Action::Category => ask_category(&draft, now, cat),
                Action::Account => ask(&draft, StepType::Choose, "account")
                    .options(cat.accounts.iter().map(|a| &a.name)),
                Action::Merchant => ask(&draft, StepType::Text, "merchant")
                    .default(draft.merchant.clone().unwrap_or_default()),
                Action::Time => ask(&draft, StepType::Date, "occurred_at")
                    .default(draft.occurred_at.to_rfc3339()),
                Action::Tags => ask(&draft, StepType::ChooseMany, "tags")
                    .options(cat.tags.iter().map(|t| &t.name)),
                Action::Note => ask(&draft, StepType::Text, "note")
                    .default(draft.note.clone().unwrap_or_default()),
            }
        }
        "category" => {
            let category = cat
                .categories(draft.kind)
                .iter()
                .find(|c| c.name == value)
                .ok_or_else(invalid)?;
            draft.category_id = category.id;
            if category.subs.is_empty() {
                menu(&draft)
            } else {
                ask(&draft, StepType::Choose, "subcategory")
                    .options(category.subs.iter().map(|s| &s.name))
            }
        }
        "subcategory" => {
            let (category, _) = cat
                .locate(draft.kind, draft.category_id)
                .expect("草稿已校验分类");
            let sub = category
                .subs
                .iter()
                .find(|s| s.name == value)
                .ok_or_else(invalid)?;
            draft.category_id = sub.id;
            menu(&draft)
        }
        "amount" => match parse_cents(value) {
            Some(cents) => {
                draft.amount_cents = Some(cents);
                menu(&draft)
            }
            None => ask_amount(&draft).warn(&format!("金额无效：{value}")),
        },
        "account" => {
            draft.account_id = cat.account_named(value).ok_or_else(invalid)?;
            menu(&draft)
        }
        "tags" => {
            draft.tag_ids = value
                .lines()
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(|t| {
                    cat.tags
                        .iter()
                        .find(|n| n.name == t)
                        .map(|n| n.id)
                        .ok_or_else(invalid)
                })
                .collect::<Result<_, _>>()?;
            menu(&draft)
        }
        "merchant" => {
            draft.merchant = non_empty(value);
            menu(&draft)
        }
        "note" => {
            draft.note = non_empty(value);
            menu(&draft)
        }
        "occurred_at" => {
            let at = DateTime::parse_from_rfc3339(value.trim()).map_err(|_| invalid())?;
            draft.occurred_at = at.with_timezone(&tz());
            menu(&draft)
        }
        "kind" => {
            let kind = match value {
                "↘️ 支出" => Kind::Expense,
                "↗️ 收入" => Kind::Income,
                _ => return Err(invalid()),
            };
            if kind == draft.kind {
                menu(&draft)
            } else {
                draft.kind = kind;
                draft.category_id = cat.fallback(kind);
                ask_category(&draft, now, cat)
            }
        }
        _ => return Err(FlowError::UnknownField(field.to_owned())),
    };
    Ok(Outcome::Reply(reply))
}

fn ask_category(draft: &Draft, now: DateTime<FixedOffset>, cat: &Catalog) -> Reply {
    let names = cat.categories(draft.kind).iter().map(|c| &c.name);
    // 分类名自带 emoji，原样作为选项。
    Reply::ask(draft, now, cat, StepType::Choose, "category").options(names)
}

/// 子步骤的提示语：当前账单的多行摘要。
fn summary(draft: &Draft, now: DateTime<FixedOffset>, cat: &Catalog) -> String {
    let dash = |v: &Option<String>| v.clone().unwrap_or_else(|| "—".to_owned());
    let time_fmt = if draft.occurred_at.year() == now.year() {
        "%m-%d %H:%M"
    } else {
        "%Y-%m-%d %H:%M"
    };
    let mut lines = vec![
        format!(
            "{} ¥{} · {}",
            draft.kind.label(),
            amount_label(draft),
            dash(&draft.merchant)
        ),
        format!("{} · {}", draft.category_path(cat), draft.account_name(cat)),
    ];
    let mut time_line = draft.occurred_at.format(time_fmt).to_string();
    for tag in draft.tag_names(cat) {
        time_line.push_str(&format!("  #{tag}"));
    }
    lines.push(time_line);
    if let Some(note) = &draft.note {
        lines.push(format!("备注：{note}"));
    }
    lines.join("\n")
}

fn amount_label(draft: &Draft) -> String {
    draft
        .amount_cents
        .map(yuan)
        .unwrap_or_else(|| "请补充".to_owned())
}

fn yuan(cents: i64) -> String {
    format!("{}.{:02}", cents / 100, cents % 100)
}

/// 「32」「32.5」「32.50」→ 分；接受逗号小数点，最多两位小数，须大于 0。
pub fn parse_cents(value: &str) -> Option<i64> {
    let value = value.trim().replace(',', ".");
    let (int, frac) = value.split_once('.').unwrap_or((&value, ""));
    let digits = |s: &str| s.chars().all(|c| c.is_ascii_digit());
    if int.is_empty() || !digits(int) || frac.len() > 2 || !digits(frac) {
        return None;
    }
    let frac: i64 = format!("{frac:0<2}").parse().ok()?;
    let cents = int
        .parse::<i64>()
        .ok()?
        .checked_mul(100)?
        .checked_add(frac)?;
    (cents > 0).then_some(cents)
}

pub fn non_empty(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

#[cfg(test)]
mod tests;
