//! 记账草稿与交互步骤：快捷指令每提交一步，这里算出下一步。不做 IO。

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Datelike, FixedOffset, Utc};
use serde::{Deserialize, Serialize};

use crate::catalog::{self, ACCOUNTS, OTHER, TAGS};

pub fn tz() -> FixedOffset {
    FixedOffset::east_opt(8 * 3600).expect("固定偏移合法")
}

pub fn now() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&tz())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Expense,
    Income,
}

impl Kind {
    fn label(self) -> &'static str {
        match self {
            Kind::Expense => "支出",
            Kind::Income => "收入",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Draft {
    pub id: String,
    pub kind: Kind,
    pub amount_cents: i64,
    pub category: String,
    pub subcategory: Option<String>,
    pub account: Option<String>,
    pub merchant: Option<String>,
    pub occurred_at: DateTime<FixedOffset>,
    pub note: Option<String>,
    pub tags: Vec<String>,
    pub warnings: Vec<String>,
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
    pub fn encode(&self) -> String {
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(self).expect("Draft 可序列化"))
    }

    pub fn decode(s: &str) -> Result<Draft, FlowError> {
        let invalid = |e: &dyn std::fmt::Display| FlowError::InvalidDraft(e.to_string());
        let bytes = URL_SAFE_NO_PAD.decode(s).map_err(|e| invalid(&e))?;
        let draft: Draft = serde_json::from_slice(&bytes).map_err(|e| invalid(&e))?;
        draft.validate().map_err(|e| invalid(&e))?;
        Ok(draft)
    }

    fn validate(&self) -> Result<(), &'static str> {
        if self.amount_cents <= 0 {
            return Err("金额须大于 0");
        }
        let category = catalog::category(self.kind, &self.category).ok_or("分类不存在")?;
        if let Some(sub) = &self.subcategory
            && !category.subs.contains(&sub.as_str())
        {
            return Err("二级分类不存在");
        }
        if let Some(account) = &self.account
            && !ACCOUNTS.contains(&account.as_str())
        {
            return Err("账户不存在");
        }
        if self.tags.iter().any(|t| !TAGS.contains(&t.as_str())) {
            return Err("标签不存在");
        }
        Ok(())
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

    fn ask(draft: &Draft, now: DateTime<FixedOffset>, step: StepType, field: &str) -> Reply {
        Reply {
            step,
            summary: summary(draft, now),
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
}

pub enum Outcome {
    Reply(Reply),
    /// 用户点了确认，由调用方入账后回复。
    Confirm(Draft),
}

const CONFIRM: &str = "确认";
const ACTIONS: &[&str] = &[
    "改金额",
    "收支类型",
    "改分类",
    "改账户",
    "商家",
    "时间",
    "加标签",
    "备注",
    CONFIRM,
];

fn menu_options(draft: &Draft) -> Vec<String> {
    let value = |v: &Option<String>| v.as_deref().unwrap_or("无").to_owned();
    vec![
        format!("💰 金额 · {}", yuan(draft.amount_cents)),
        format!("↔️ 类型 · {}", draft.kind.label()),
        format!("🗂️ 分类 · {}", category_label(draft).replace('/', " / ")),
        format!("💳 账户 · {}", value(&draft.account)),
        format!("🏪 商家 · {}", value(&draft.merchant)),
        format!("🕒 时间 · {}", draft.occurred_at.format("%m-%d %H:%M")),
        format!(
            "🏷️ 标签 · {}",
            if draft.tags.is_empty() {
                "无".to_owned()
            } else {
                draft.tags.join(" / ")
            }
        ),
        format!("📝 备注 · {}", value(&draft.note)),
        "✅ 确认记账".to_owned(),
    ]
}

pub fn menu(draft: &Draft, now: DateTime<FixedOffset>) -> Reply {
    let mut reply = Reply::ask(draft, now, StepType::Choose, "action").options(menu_options(draft));
    reply.summary.clear();
    reply
}

fn decorated<'a>(values: impl IntoIterator<Item = &'a str>, emoji: &str) -> Vec<String> {
    values.into_iter().map(|v| format!("{emoji} {v}")).collect()
}

// 展示值只在交互边界映射成干净的数据值。
fn choice<'a>(value: &'a str, values: &[&'a str], emoji: &str) -> Option<&'a str> {
    values
        .iter()
        .copied()
        .find(|v| value == format!("{emoji} {v}"))
}

pub fn saved_message(draft: &Draft) -> String {
    format!(
        "已记账 ¥{} · {}",
        yuan(draft.amount_cents),
        category_label(draft)
    )
}

pub fn step(
    mut draft: Draft,
    field: &str,
    value: &str,
    now: DateTime<FixedOffset>,
) -> Result<Outcome, FlowError> {
    let invalid = || FlowError::InvalidValue {
        field: field.to_owned(),
        value: value.to_owned(),
    };
    let ask = |d: &Draft, step, field| Reply::ask(d, now, step, field);

    let action_options = menu_options(&draft);
    let value = if field == "action" {
        action_options
            .iter()
            .position(|option| option == value)
            .map(|index| ACTIONS[index])
            .ok_or_else(invalid)?
    } else {
        value
    };
    let reply = match field {
        "action" => match value {
            CONFIRM => return Ok(Outcome::Confirm(draft)),
            "改分类" => ask_category(&draft, now),
            "改金额" => {
                ask(&draft, StepType::Number, "amount").default(yuan(draft.amount_cents))
            }
            "改账户" => ask(&draft, StepType::Choose, "account")
                .options(decorated(ACCOUNTS.iter().copied(), "💳")),
            "加标签" => ask(&draft, StepType::ChooseMany, "tags")
                .options(decorated(TAGS.iter().copied(), "🏷️")),
            "商家" => ask(&draft, StepType::Text, "merchant")
                .default(draft.merchant.clone().unwrap_or_default()),
            "时间" => {
                ask(&draft, StepType::Date, "occurred_at").default(draft.occurred_at.to_rfc3339())
            }
            "备注" => {
                ask(&draft, StepType::Text, "note").default(draft.note.clone().unwrap_or_default())
            }
            "收支类型" => ask(&draft, StepType::Choose, "kind").options(["↘️ 支出", "↗️ 收入"]),
            _ => return Err(invalid()),
        },
        "category" => {
            let categories: Vec<_> = catalog::categories(draft.kind)
                .iter()
                .map(|c| c.name)
                .collect();
            let value = choice(value, &categories, "🗂️").ok_or_else(invalid)?;
            let category = catalog::category(draft.kind, value).ok_or_else(invalid)?;
            draft.category = category.name.to_owned();
            draft.subcategory = None;
            if category.subs.is_empty() {
                menu(&draft, now)
            } else {
                ask(&draft, StepType::Choose, "subcategory")
                    .options(decorated(category.subs.iter().copied(), "📂"))
            }
        }
        "subcategory" => {
            let category = catalog::category(draft.kind, &draft.category).ok_or_else(invalid)?;
            let value = choice(value, category.subs, "📂").ok_or_else(invalid)?;
            draft.subcategory = Some(value.to_owned());
            menu(&draft, now)
        }
        "amount" => match parse_cents(value) {
            Some(cents) => {
                draft.amount_cents = cents;
                menu(&draft, now)
            }
            None => {
                let mut reply =
                    ask(&draft, StepType::Number, "amount").default(yuan(draft.amount_cents));
                reply.summary = format!("⚠ 金额无效：{value}\n{}", reply.summary);
                reply
            }
        },
        "account" => {
            let value = choice(value, &ACCOUNTS, "💳").ok_or_else(invalid)?;
            draft.account = Some(value.to_owned());
            menu(&draft, now)
        }
        "tags" => {
            let tags: Vec<String> = value
                .lines()
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(|t| {
                    choice(t, &TAGS, "🏷️")
                        .map(str::to_owned)
                        .ok_or_else(invalid)
                })
                .collect::<Result<_, _>>()?;
            draft.tags = tags;
            menu(&draft, now)
        }
        "merchant" => {
            draft.merchant = non_empty(value);
            menu(&draft, now)
        }
        "note" => {
            draft.note = non_empty(value);
            menu(&draft, now)
        }
        "occurred_at" => {
            let at = DateTime::parse_from_rfc3339(value.trim()).map_err(|_| invalid())?;
            draft.occurred_at = at.with_timezone(&tz());
            menu(&draft, now)
        }
        "kind" => {
            let kind = match value {
                "↘️ 支出" => Kind::Expense,
                "↗️ 收入" => Kind::Income,
                _ => return Err(invalid()),
            };
            if kind == draft.kind {
                menu(&draft, now)
            } else {
                draft.kind = kind;
                draft.category = OTHER.to_owned();
                draft.subcategory = None;
                ask_category(&draft, now)
            }
        }
        _ => return Err(FlowError::UnknownField(field.to_owned())),
    };
    Ok(Outcome::Reply(reply))
}

fn ask_category(draft: &Draft, now: DateTime<FixedOffset>) -> Reply {
    Reply::ask(draft, now, StepType::Choose, "category").options(decorated(
        catalog::categories(draft.kind).iter().map(|c| c.name),
        "🗂️",
    ))
}

fn summary(draft: &Draft, now: DateTime<FixedOffset>) -> String {
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
            yuan(draft.amount_cents),
            dash(&draft.merchant)
        ),
        format!("{} · {}", category_label(draft), dash(&draft.account)),
    ];
    let mut time_line = draft.occurred_at.format(time_fmt).to_string();
    if !draft.tags.is_empty() {
        time_line.push_str("  ");
        let tags: Vec<String> = draft.tags.iter().map(|t| format!("#{t}")).collect();
        time_line.push_str(&tags.join(" "));
    }
    lines.push(time_line);
    if let Some(note) = &draft.note {
        lines.push(format!("备注：{note}"));
    }
    lines.join("\n")
}

fn category_label(draft: &Draft) -> String {
    match &draft.subcategory {
        Some(sub) => format!("{}/{sub}", draft.category),
        None => draft.category.clone(),
    }
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
