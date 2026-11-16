//! 截图识别：发给 DeepSeek 视觉模型，输出逐字段校验后组装成草稿。
//!
//! 识别永不失败：调用出错、不是支付页、字段不合规，都落到空字段由用户补。
//! 被拒或出错的原因写进识别记录，之后每一屏顶部都提示一行（见 [`notice`]）；
//! 被拒时模型填了的字段照留，确认与否由用户决定。

use std::{
    sync::LazyLock,
    time::{Duration, Instant},
};

use anyhow::Context;
use base64::{Engine, engine::general_purpose::STANDARD};
use chrono::{DateTime, FixedOffset, NaiveDateTime};
use serde_json::{Value, json};

use crate::{
    catalog::Catalog,
    flow::{self, Draft, Kind},
};

const URL: &str = "https://api.deepseek.com/chat/completions";

/// `DeepSeek` 走真实识别；`Mock` 不调模型、固定返回假账单，用于联调快捷指令。
pub enum Recognizer {
    DeepSeek(DeepSeek),
    Mock,
}

pub struct DeepSeek {
    http: reqwest::Client,
    api_key: String,
    model: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Ok,
    /// 模型说不是支付页。
    Rejected,
    /// 调用失败或返回无法解析。
    Error,
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Ok => "ok",
            Outcome::Rejected => "rejected",
            Outcome::Error => "error",
        }
    }

    pub fn parse(s: &str) -> Option<Outcome> {
        [Outcome::Ok, Outcome::Rejected, Outcome::Error]
            .into_iter()
            .find(|o| o.as_str() == s)
    }
}

/// 给用户看的提醒：识别没成功时说明原因。出错的原因是技术细节，只进记录。
pub fn notice(outcome: Outcome, reason: Option<&str>) -> Option<String> {
    match outcome {
        Outcome::Ok => None,
        Outcome::Rejected => Some(format!("不像支付页：{}", reason.unwrap_or("未说明"))),
        Outcome::Error => Some("识别失败，请手动填写".to_owned()),
    }
}

pub struct Recognized {
    pub draft: Draft,
    pub outcome: Outcome,
    /// 被拒或出错的完整原因，落库用。
    pub reason: Option<String>,
    pub model: String,
    pub model_ms: u64,
    /// DeepSeek 原始 HTTP 响应体；调用失败时是错误链。
    pub raw: String,
    pub prompt: String,
    pub parameters: String,
}

impl Recognizer {
    pub fn deepseek(
        api_key: String,
        model: String,
        timeout: Duration,
    ) -> anyhow::Result<Recognizer> {
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .context("创建 HTTP 客户端")?;
        Ok(Recognizer::DeepSeek(DeepSeek {
            http,
            api_key,
            model,
        }))
    }

    /// `mime` 由入口校验过，只会是 PNG / JPEG。
    pub async fn recognize(
        &self,
        image: &[u8],
        mime: &str,
        now: DateTime<FixedOffset>,
        cat: &Catalog,
    ) -> Recognized {
        let prompt = prompt(now, cat);
        let parameters = parameters().to_string();
        let ds = match self {
            Recognizer::Mock => {
                return Recognized {
                    draft: mock_draft(now, cat),
                    outcome: Outcome::Ok,
                    reason: None,
                    model: "mock".to_owned(),
                    model_ms: 0,
                    raw: String::new(),
                    prompt,
                    parameters,
                };
            }
            Recognizer::DeepSeek(ds) => ds,
        };
        let started = Instant::now();
        let called = ds.call(image, mime, &prompt).await;
        let model_ms = started.elapsed().as_millis() as u64;
        let (raw, parsed) = match called {
            Ok(raw) => {
                let parsed = parse_output(&raw);
                (raw, parsed)
            }
            Err(e) => (format!("{e:#}"), Err(e)),
        };
        let (draft, outcome, reason) = match parsed {
            Ok(out) if out.ok => (build_draft(out, now, cat), Outcome::Ok, None),
            Ok(out) => {
                let reason = out.reason.clone().unwrap_or_else(|| "未说明".to_owned());
                (build_draft(out, now, cat), Outcome::Rejected, Some(reason))
            }
            Err(e) => {
                tracing::warn!(error = %format!("{e:#}"), "识别失败，返回空白草稿");
                (
                    Draft::blank(now, cat),
                    Outcome::Error,
                    Some(format!("{e:#}")),
                )
            }
        };
        Recognized {
            draft,
            outcome,
            reason,
            model: ds.model.clone(),
            model_ms,
            raw,
            prompt,
            parameters,
        }
    }
}

/// 请求体里除 model、messages 外的固定参数；落库的也是它，两边不会不一致。
fn parameters() -> Value {
    json!({
        "response_format": { "type": "json_object" },
        "reasoning_effort": "none",
        "temperature": 0,
        "max_tokens": 500,
        "stream": false,
    })
}

impl DeepSeek {
    async fn call(&self, image: &[u8], mime: &str, prompt: &str) -> anyhow::Result<String> {
        let mut body = parameters();
        body["model"] = json!(self.model);
        body["messages"] = json!([
            { "role": "system", "content": prompt },
            { "role": "user", "content": [
                { "type": "image_url", "image_url": {
                    "url": format!("data:{mime};base64,{}", STANDARD.encode(image))
                } },
                { "type": "text", "text": "识别这张截图。" },
            ] },
        ]);
        let resp = self
            .http
            .post(URL)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .context("请求 DeepSeek")?;
        let status = resp.status();
        let text = resp.text().await.context("读取 DeepSeek 响应")?;
        anyhow::ensure!(status.is_success(), "DeepSeek 返回 {status}：{text}");
        Ok(text)
    }
}

/// 模型输出的固定结构：每个字段独立取值，类型不对或为空都当作没有。
#[derive(Debug, Default)]
struct Output {
    ok: bool,
    reason: Option<String>,
    kind: Option<String>,
    amount: Option<String>,
    merchant: Option<String>,
    /// 商品名，作为备注默认值。
    item: Option<String>,
    account: Option<String>,
    occurred_at: Option<String>,
    category: Option<String>,
}

impl Output {
    fn from_json(v: &Value) -> Output {
        let text = |key: &str| match &v[key] {
            Value::String(s) => flow::non_empty(s),
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        };
        Output {
            ok: v["ok"].as_bool() != Some(false),
            reason: text("reason"),
            kind: text("kind"),
            amount: text("amount"),
            merchant: text("merchant"),
            item: text("item"),
            account: text("account"),
            occurred_at: text("occurred_at"),
            category: text("category"),
        }
    }
}

/// DeepSeek 响应体 → 模型输出；只有整体不是约定的 JSON 才算失败。
fn parse_output(raw: &str) -> anyhow::Result<Output> {
    let resp: Value = serde_json::from_str(raw).context("DeepSeek 响应不是 JSON")?;
    let content = resp["choices"][0]["message"]["content"]
        .as_str()
        .context("DeepSeek 响应缺少 content")?;
    let v: Value = serde_json::from_str(content).context("模型输出不是 JSON")?;
    anyhow::ensure!(v.is_object(), "模型输出不是 JSON 对象");
    Ok(Output::from_json(&v))
}

/// 逐字段校验；不合规的字段当作没给，再补默认值（分类「其他」、账户取第一个、时间取当前），并记日志。
fn build_draft(out: Output, now: DateTime<FixedOffset>, cat: &Catalog) -> Draft {
    let mut dropped = Vec::new();
    let mut check = |name: &'static str, raw: &Option<String>, ok: bool| {
        if raw.is_some() && !ok {
            dropped.push(name);
        }
    };

    let kind = match out.kind.as_deref() {
        Some("income") => Kind::Income,
        _ => Kind::Expense,
    };
    let amount_cents = out.amount.as_deref().and_then(flow::parse_cents);
    check("amount", &out.amount, amount_cents.is_some());
    let category = out.category.as_deref().and_then(|k| cat.by_key(kind, k));
    check("category", &out.category, category.is_some());
    let (category, subcategory) = category.unwrap_or((cat.fallback(kind), None));
    let account = out.account.clone().filter(|a| cat.has_account(a));
    check("account", &out.account, account.is_some());
    // 时间与识别时刻相差不到 30 分钟就用识别时刻：截图只到分钟，当前时间更准。
    let parsed_at = out.occurred_at.as_deref().and_then(parse_local_time);
    check("occurred_at", &out.occurred_at, parsed_at.is_some());
    let occurred_at = parsed_at
        .filter(|at| (*at - now).num_seconds().abs() > 1800)
        .unwrap_or(now);
    if !dropped.is_empty() {
        tracing::warn!(fields = ?dropped, "模型输出字段不合规，已置空");
    }

    Draft {
        kind,
        amount_cents,
        category: category.to_owned(),
        subcategory: subcategory.map(str::to_owned),
        account: account.unwrap_or_else(|| cat.default_account().to_owned()),
        merchant: out.merchant,
        occurred_at,
        note: out.item,
        ..Draft::blank(now, cat)
    }
}

fn mock_draft(now: DateTime<FixedOffset>, cat: &Catalog) -> Draft {
    // 按 key 取，分类改名不影响；库里删了就落到「其他」。
    let (category, subcategory) = cat
        .by_key(Kind::Expense, "coffee")
        .unwrap_or((cat.fallback(Kind::Expense), None));
    Draft {
        amount_cents: Some(3250),
        category: category.to_owned(),
        subcategory: subcategory.map(str::to_owned),
        merchant: Some("瑞幸咖啡".to_owned()),
        note: Some("生椰拿铁".to_owned()),
        ..Draft::blank(now, cat)
    }
}

fn parse_local_time(s: &str) -> Option<DateTime<FixedOffset>> {
    ["%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M"]
        .iter()
        .find_map(|f| NaiveDateTime::parse_from_str(s.trim(), f).ok())
        .and_then(|t| t.and_local_timezone(flow::tz()).single())
}

/// 按文件头判断图片类型；只收 PNG / JPEG。
pub fn mime(image: &[u8]) -> Option<&'static str> {
    if image.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if image.starts_with(b"\xff\xd8\xff") {
        Some("image/jpeg")
    } else {
        None
    }
}

/// 工作目录下的 `prompt.txt`，占位符 `{now}` `{categories}` `{accounts}`。
static TEMPLATE: LazyLock<anyhow::Result<String>> = LazyLock::new(|| {
    let text = std::fs::read_to_string("prompt.txt").context("读取 prompt.txt")?;
    for key in ["{now}", "{categories}", "{accounts}"] {
        anyhow::ensure!(text.contains(key), "prompt.txt 缺少占位符 {key}");
    }
    Ok(text)
});

/// 启动时调用：模板有问题就带着原因退出。
pub fn check_prompt() -> anyhow::Result<()> {
    TEMPLATE
        .as_ref()
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!("{e:#}"))
}

fn prompt(now: DateTime<FixedOffset>, cat: &Catalog) -> String {
    let categories = [Kind::Expense, Kind::Income]
        .map(|kind| format!("{}:\n{}", kind.as_str(), cat.key_lines(kind)))
        .join("\n");
    TEMPLATE
        .as_ref()
        .expect("启动时已校验 prompt.txt")
        .replace("{now}", &now.format("%Y-%m-%d %H:%M:%S").to_string())
        .replace("{categories}", &categories)
        .replace("{accounts}", &cat.account_lines())
}

#[cfg(test)]
mod tests;
