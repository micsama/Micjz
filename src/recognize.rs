//! 截图识别：发给 DeepSeek 视觉模型，输出按 catalog 校验后组装成草稿。

use std::time::{Duration, Instant};

use anyhow::Context;
use base64::{Engine, engine::general_purpose::STANDARD};
use chrono::{DateTime, FixedOffset, NaiveDateTime};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    catalog::{self, ACCOUNTS, OTHER},
    flow::{self, Draft, Kind},
};

const URL: &str = "https://api.deepseek.com/chat/completions";

#[derive(Debug, thiserror::Error)]
pub enum RecognizeError {
    #[error("图片格式不支持，仅支持 PNG / JPEG")]
    UnsupportedImage,
    #[error("未识别到支付信息：{0}")]
    NotPayment(String),
    #[error("未识别到金额")]
    NoAmount,
    #[error("识别服务出错：{0:#}")]
    Upstream(anyhow::Error),
}

impl RecognizeError {
    /// 落库时给这次识别分类：模型看完说不是账单算 `rejected`，其余算 `error`。
    pub fn outcome(&self) -> &'static str {
        match self {
            RecognizeError::NotPayment(_) | RecognizeError::NoAmount => "rejected",
            _ => "error",
        }
    }
}

/// 一次识别的元信息，无论成败都留：谁算的、花了多久、原始返回是什么。
pub struct Meta {
    pub model: String,
    pub elapsed_ms: u64,
    /// 模型的原始 content；调用失败时是错误正文。
    pub raw: String,
}

pub struct Recognized {
    pub result: Result<Draft, RecognizeError>,
    pub meta: Meta,
}

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

    pub async fn recognize(&self, image: &[u8], now: DateTime<FixedOffset>) -> Recognized {
        let started = Instant::now();
        let deepseek = match self {
            Recognizer::DeepSeek(d) => d,
            Recognizer::Mock => {
                return Recognized {
                    result: Ok(mock_draft(now)),
                    meta: Meta {
                        model: "mock".to_owned(),
                        elapsed_ms: 0,
                        raw: String::new(),
                    },
                };
            }
        };
        // raw 两条路都要留下：成功时是模型 content，失败时是错误正文（含响应体）。
        let (result, raw) = match mime(image) {
            None => (Err(RecognizeError::UnsupportedImage), String::new()),
            Some(mime) => match deepseek.call(image, mime, now).await {
                Err(e) => {
                    let err = RecognizeError::Upstream(e);
                    let raw = err.to_string();
                    (Err(err), raw)
                }
                Ok((output, raw)) => {
                    tracing::info!(raw = %raw, "模型返回");
                    (build_draft(output, now), raw)
                }
            },
        };
        let elapsed_ms = started.elapsed().as_millis() as u64;
        tracing::info!(elapsed_ms, "识别耗时");
        Recognized {
            result,
            meta: Meta {
                model: deepseek.model.clone(),
                elapsed_ms,
                raw,
            },
        }
    }
}

impl DeepSeek {
    async fn call(
        &self,
        image: &[u8],
        mime: &str,
        now: DateTime<FixedOffset>,
    ) -> anyhow::Result<(Output, String)> {
        let body = json!({
            "model": self.model,
            "messages": [
                { "role": "system", "content": prompt(now) },
                { "role": "user", "content": [
                    { "type": "image_url", "image_url": {
                        "url": format!("data:{mime};base64,{}", STANDARD.encode(image))
                    } },
                    { "type": "text", "text": "识别这张截图。" },
                ] },
            ],
            "response_format": { "type": "json_object" },
            "reasoning_effort": "none",
            "temperature": 0,
            "max_tokens": 500,
            "stream": false,
        });
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

        let resp: Value = serde_json::from_str(&text).context("解析 DeepSeek 响应")?;
        if let Some(usage) = resp.get("usage") {
            tracing::info!(
                prompt_tokens = usage["prompt_tokens"].as_u64(),
                completion_tokens = usage["completion_tokens"].as_u64(),
                "DeepSeek 用量"
            );
        }
        let content = resp["choices"][0]["message"]["content"]
            .as_str()
            .context("DeepSeek 响应缺少 content")?;
        let parsed = serde_json::from_str(content)
            .with_context(|| format!("模型输出不是约定的 JSON：{content}"))?;
        Ok((parsed, content.to_owned()))
    }
}

/// 模型输出；字段一律可空，由 `build_draft` 决定取舍。
#[derive(Debug, Deserialize)]
struct Output {
    ok: bool,
    reason: Option<String>,
    kind: Option<String>,
    amount: Option<Value>,
    merchant: Option<String>,
    account: Option<String>,
    occurred_at: Option<String>,
    category: Option<String>,
    subcategory: Option<String>,
}

fn build_draft(out: Output, now: DateTime<FixedOffset>) -> Result<Draft, RecognizeError> {
    if !out.ok {
        let reason = out.reason.unwrap_or_else(|| "不是支付页面".to_owned());
        return Err(RecognizeError::NotPayment(reason));
    }
    let amount = match &out.amount {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => return Err(RecognizeError::NoAmount),
    };
    let amount_cents = flow::parse_cents(&amount).ok_or(RecognizeError::NoAmount)?;
    let kind = match out.kind.as_deref() {
        Some("income") => Kind::Income,
        _ => Kind::Expense,
    };

    let mut warnings = Vec::new();
    let category = out
        .category
        .as_deref()
        .and_then(|name| catalog::category(kind, name));
    let (category, subcategory) = match category {
        Some(c) => (
            c.name.to_owned(),
            out.subcategory.filter(|s| c.subs.contains(&s.as_str())),
        ),
        None => {
            warnings.push("分类未识别，已设为其他".to_owned());
            (OTHER.to_owned(), None)
        }
    };
    let occurred_at = match out.occurred_at.as_deref().and_then(parse_local_time) {
        Some(at) => at,
        None => {
            warnings.push("未看到交易时间，已用当前时间".to_owned());
            now
        }
    };

    Ok(Draft {
        id: uuid::Uuid::now_v7().to_string(),
        kind,
        amount_cents,
        category,
        subcategory,
        account: out.account.filter(|a| ACCOUNTS.contains(&a.as_str())),
        merchant: out.merchant.as_deref().and_then(flow::non_empty),
        occurred_at,
        note: None,
        tags: Vec::new(),
        warnings,
    })
}

fn mock_draft(now: DateTime<FixedOffset>) -> Draft {
    Draft {
        id: uuid::Uuid::now_v7().to_string(),
        kind: Kind::Expense,
        amount_cents: 3250,
        category: "餐饮".to_owned(),
        subcategory: Some("咖啡饮品".to_owned()),
        account: Some("微信".to_owned()),
        merchant: Some("瑞幸咖啡".to_owned()),
        occurred_at: now,
        note: None,
        tags: Vec::new(),
        warnings: vec!["mock 识别：假账单".to_owned()],
    }
}

fn parse_local_time(s: &str) -> Option<DateTime<FixedOffset>> {
    ["%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M"]
        .iter()
        .find_map(|f| NaiveDateTime::parse_from_str(s.trim(), f).ok())
        .and_then(|t| t.and_local_timezone(flow::tz()).single())
}

fn mime(image: &[u8]) -> Option<&'static str> {
    if image.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if image.starts_with(b"\xff\xd8\xff") {
        Some("image/jpeg")
    } else {
        None
    }
}

fn prompt(now: DateTime<FixedOffset>) -> String {
    let list = |kind| {
        catalog::categories(kind)
            .iter()
            .map(|c| {
                if c.subs.is_empty() {
                    format!("- {}", c.name)
                } else {
                    format!("- {}：{}", c.name, c.subs.join("、"))
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    format!(
        r#"你是记账助手，从手机支付截图中提取一笔交易，只输出 JSON。

当前时间：{now}（北京时间）

输出格式：
{{"ok": true, "reason": null, "kind": "expense", "amount": "32.50", "merchant": "瑞幸咖啡", "account": "微信", "occurred_at": "2026-11-16 13:42", "category": "餐饮", "subcategory": "咖啡饮品"}}

规则：
1. 截图里看不到的字段填 null，不许猜测或编造。
2. ok：是支付成功页、账单详情页或收款到账页填 true；否则填 false，并在 reason 用一句中文说明。
3. kind：付钱是 "expense"，收钱（收款、退款到账、红包收入）是 "income"。
4. amount：实际付款或到账金额，字符串，单位元。有多个金额时取实付金额，不取原价、优惠、余额。
5. merchant：商家或对方名称。
6. account：只能是 {accounts} 之一；用微信付的填 "微信"，银行卡付的填 "银行卡"，看不出填 null。
7. occurred_at：交易时间，格式 "YYYY-MM-DD HH:MM"；截图没写年份按当前年份。
8. category / subcategory：只能从下面列表选；没有合适的二级分类，subcategory 填 null。

支出分类：
{expense}

收入分类（无二级）：
{income}"#,
        now = now.format("%Y-%m-%d %H:%M"),
        accounts = ACCOUNTS.join(" / "),
        expense = list(Kind::Expense),
        income = list(Kind::Income),
    )
}

#[cfg(test)]
mod tests;
