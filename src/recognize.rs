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
    pub prompt: String,
    pub parameters: String,
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
        if mime(image).is_none() {
            return Recognized {
                result: Err(RecognizeError::UnsupportedImage),
                meta: Meta {
                    model: "unsupported".into(),
                    elapsed_ms: 0,
                    raw: String::new(),
                    prompt: prompt(now),
                    parameters: parameters(),
                },
            };
        }
        let deepseek = match self {
            Recognizer::DeepSeek(d) => d,
            Recognizer::Mock => {
                return Recognized {
                    result: Ok(mock_draft(now)),
                    meta: Meta {
                        model: "mock".to_owned(),
                        elapsed_ms: 0,
                        raw: String::new(),
                        prompt: prompt(now),
                        parameters: parameters(),
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
                Ok(raw) => {
                    let result = parse_response(&raw).and_then(|output| build_draft(output, now));
                    (result, raw)
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
                prompt: prompt(now),
                parameters: parameters(),
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
    ) -> anyhow::Result<String> {
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

        Ok(text)
    }
}

fn parse_response(text: &str) -> Result<Output, RecognizeError> {
    (|| -> anyhow::Result<Output> {
        let resp: Value = serde_json::from_str(text).context("解析 DeepSeek 响应")?;
        let content = resp["choices"][0]["message"]["content"]
            .as_str()
            .context("DeepSeek 响应缺少 content")?;
        serde_json::from_str(content).context("模型输出不是约定的 JSON")
    })()
    .map_err(RecognizeError::Upstream)
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

    let (category, subcategory) = match out.category.as_deref() {
        Some(key) => catalog::model_category(kind, key)
            .ok_or_else(|| RecognizeError::Upstream(anyhow::anyhow!("模型分类 key 无效")))?,
        None => (OTHER, None),
    };
    let occurred_at = match out.occurred_at.as_deref() {
        Some(s) => {
            let at = parse_local_time(s)
                .ok_or_else(|| RecognizeError::Upstream(anyhow::anyhow!("模型时间无效")))?;
            if (at - now).num_seconds().abs() > 1800 {
                at
            } else {
                now
            }
        }
        None => now,
    };

    Ok(Draft {
        id: uuid::Uuid::now_v7().to_string(),
        kind,
        amount_cents,
        category: category.to_owned(),
        subcategory: subcategory.map(str::to_owned),
        account: out.account.filter(|a| ACCOUNTS.contains(&a.as_str())),
        merchant: out.merchant.as_deref().and_then(flow::non_empty),
        occurred_at,
        note: None,
        tags: Vec::new(),
        warnings: Vec::new(),
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
        warnings: Vec::new(),
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

static PROMPT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
pub fn init_prompt() -> anyhow::Result<()> {
    let text = std::fs::read_to_string("prompt.txt")?;
    anyhow::ensure!(
        text.contains("{now}") && text.contains("{categories}") && text.contains("{accounts}"),
        "prompt 须包含 now/categories/accounts 占位符"
    );
    let _ = PROMPT.set(text);
    Ok(())
}
fn parameters() -> String {
    json!({"response_format":{"type":"json_object"},"reasoning_effort":"none","temperature":0,"max_tokens":500,"stream":false}).to_string()
}
fn prompt(now: DateTime<FixedOffset>) -> String {
    let list = [Kind::Expense, Kind::Income]
        .into_iter()
        .map(|kind| {
            let rows = catalog::categories(kind)
                .iter()
                .flat_map(|c| {
                    c.keys.iter().enumerate().map(move |(i, key)| {
                        format!(
                            "{key}: {}{}",
                            c.name,
                            c.subs.get(i).map(|s| format!("/{s}")).unwrap_or_default()
                        )
                    })
                })
                .collect::<Vec<_>>()
                .join("\n");
            format!("{kind:?}:\n{rows}")
        })
        .collect::<Vec<_>>()
        .join("\n");
    if PROMPT.get().is_none() {
        init_prompt().expect("prompt 配置有效");
    }
    PROMPT
        .get()
        .expect("已加载 prompt")
        .replace("{now}", &now.format("%Y-%m-%d %H:%M:%S").to_string())
        .replace("{categories}", &list)
        .replace("{accounts}", &ACCOUNTS.join(" / "))
}

#[cfg(test)]
mod tests;
