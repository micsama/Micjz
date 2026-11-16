//! 记账草稿与交互步骤：快捷指令每提交一步，这里算出下一步。不做 IO。

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Datelike, FixedOffset, Utc};
use serde::{Deserialize, Serialize};

use crate::catalog::{Catalog, OTHER};

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
/// 补不了的（金额）为空，由用户补。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Draft {
    pub id: String,
    pub kind: Kind,
    /// 未识别出金额时为空，确认前必须填。
    pub amount_cents: Option<i64>,
    pub category: String,
    pub subcategory: Option<String>,
    /// 识别不出时是优先级第一的账户。
    pub account: String,
    pub merchant: Option<String>,
    pub occurred_at: DateTime<FixedOffset>,
    pub note: Option<String>,
    pub tags: Vec<String>,
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
            category: OTHER.to_owned(),
            subcategory: None,
            account: cat.default_account().to_owned(),
            merchant: None,
            occurred_at: now,
            note: None,
            tags: Vec::new(),
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
        let category = cat
            .category(self.kind, &self.category)
            .ok_or("分类不存在")?;
        if self
            .subcategory
            .as_deref()
            .is_some_and(|s| !category.has_sub(s))
        {
            return Err("二级分类不存在");
        }
        if !cat.has_account(&self.account) {
            return Err("账户不存在");
        }
        if self.tags.iter().any(|t| !cat.tags.contains(t)) {
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

    /// 在提示语顶部加一行警告。
    fn warn(mut self, line: &str) -> Reply {
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
fn menu_items(d: &Draft) -> [(Action, String); 9] {
    let or_none = |v: &Option<String>| v.clone().unwrap_or_else(|| "无".to_owned());
    let tags = if d.tags.is_empty() {
        "无".to_owned()
    } else {
        d.tags.join(" / ")
    };
    [
        (Action::Amount, format!("💰 金额 · {}", amount_label(d))),
        (Action::Kind, format!("↔️ 类型 · {}", d.kind.label())),
        (
            Action::Category,
            format!("🗂️ 分类 · {}", category_label(d).replace('/', " / ")),
        ),
        (Action::Account, format!("💳 账户 · {}", d.account)),
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
pub fn menu(draft: &Draft, now: DateTime<FixedOffset>) -> Reply {
    let mut reply = Reply::ask(draft, now, StepType::Choose, "action")
        .options(menu_items(draft).map(|(_, label)| label));
    reply.summary.clear();
    reply
}

fn decorated<'a>(values: impl IntoIterator<Item = &'a str>, emoji: &str) -> Vec<String> {
    values.into_iter().map(|v| format!("{emoji} {v}")).collect()
}

/// 带 emoji 的展示文本 → 纯数据值。
fn choice<'a>(
    value: &str,
    values: impl IntoIterator<Item = &'a str>,
    emoji: &str,
) -> Option<&'a str> {
    values.into_iter().find(|v| value == format!("{emoji} {v}"))
}

fn strs(values: &[String]) -> impl Iterator<Item = &str> {
    values.iter().map(String::as_str)
}

pub fn saved_message(draft: &Draft, amount_cents: i64) -> String {
    format!("已记账 ¥{} · {}", yuan(amount_cents), category_label(draft))
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
    let ask = |d: &Draft, step, field| Reply::ask(d, now, step, field);
    // 数字输入框只收数字，没金额时预填 0。
    let ask_amount = |d: &Draft| {
        ask(d, StepType::Number, "amount")
            .default(d.amount_cents.map(yuan).unwrap_or_else(|| "0".to_owned()))
    };

    let reply = match field {
        "action" => {
            let action = menu_items(&draft)
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
                    .options(decorated(cat.account_names(), "💳")),
                Action::Merchant => ask(&draft, StepType::Text, "merchant")
                    .default(draft.merchant.clone().unwrap_or_default()),
                Action::Time => ask(&draft, StepType::Date, "occurred_at")
                    .default(draft.occurred_at.to_rfc3339()),
                Action::Tags => ask(&draft, StepType::ChooseMany, "tags")
                    .options(decorated(strs(&cat.tags), "🏷️")),
                Action::Note => ask(&draft, StepType::Text, "note")
                    .default(draft.note.clone().unwrap_or_default()),
            }
        }
        "category" => {
            let names = cat.categories(draft.kind).iter().map(|c| c.name.as_str());
            let name = choice(value, names, "🗂️").ok_or_else(invalid)?;
            let category = cat.category(draft.kind, name).ok_or_else(invalid)?;
            draft.category = category.name.clone();
            draft.subcategory = None;
            if category.subs.is_empty() {
                menu(&draft, now)
            } else {
                ask(&draft, StepType::Choose, "subcategory")
                    .options(decorated(category.sub_names(), "📂"))
            }
        }
        "subcategory" => {
            let category = cat
                .category(draft.kind, &draft.category)
                .ok_or_else(invalid)?;
            let sub = choice(value, category.sub_names(), "📂").ok_or_else(invalid)?;
            draft.subcategory = Some(sub.to_owned());
            menu(&draft, now)
        }
        "amount" => match parse_cents(value) {
            Some(cents) => {
                draft.amount_cents = Some(cents);
                menu(&draft, now)
            }
            None => ask_amount(&draft).warn(&format!("金额无效：{value}")),
        },
        "account" => {
            let account = choice(value, cat.account_names(), "💳").ok_or_else(invalid)?;
            draft.account = account.to_owned();
            menu(&draft, now)
        }
        "tags" => {
            draft.tags = value
                .lines()
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(|t| {
                    choice(t, strs(&cat.tags), "🏷️")
                        .map(str::to_owned)
                        .ok_or_else(invalid)
                })
                .collect::<Result<_, _>>()?;
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
                ask_category(&draft, now, cat)
            }
        }
        _ => return Err(FlowError::UnknownField(field.to_owned())),
    };
    Ok(Outcome::Reply(reply))
}

fn ask_category(draft: &Draft, now: DateTime<FixedOffset>, cat: &Catalog) -> Reply {
    let names = cat.categories(draft.kind).iter().map(|c| c.name.as_str());
    Reply::ask(draft, now, StepType::Choose, "category").options(decorated(names, "🗂️"))
}

/// 子步骤的提示语：当前账单的多行摘要。
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
            amount_label(draft),
            dash(&draft.merchant)
        ),
        format!("{} · {}", category_label(draft), draft.account),
    ];
    let mut time_line = draft.occurred_at.format(time_fmt).to_string();
    for tag in &draft.tags {
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

pub fn category_label(draft: &Draft) -> String {
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
