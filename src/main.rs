mod api;
mod catalog;
mod db;
mod flow;
mod recognize;

use std::{path::PathBuf, sync::Arc, time::Duration};

use anyhow::Context;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    listen: String,
    token: String,
    db_path: PathBuf,
    /// 缺省则用 mock 识别，不调 DeepSeek。
    deepseek: Option<DeepSeekConfig>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeepSeekConfig {
    model: String,
    timeout_secs: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "micjz=info".into()),
        )
        .init();
    // reqwest 只开 `rustls-no-provider`，建客户端前须装好进程级加密实现。
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("安装 rustls 加密实现失败"))?;

    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config.toml".to_owned());
    let text = std::fs::read_to_string(&path).with_context(|| format!("读取配置 {path}"))?;
    let config: Config = toml::from_str(&text).with_context(|| format!("解析配置 {path}"))?;
    anyhow::ensure!(!config.token.is_empty(), "token 不能为空");
    let recognizer = match config.deepseek {
        Some(ds) => {
            let api_key = std::env::var("DEEPSEEK_API_KEY")
                .ok()
                .filter(|k| !k.is_empty())
                .context("配置了 [deepseek] 但缺少环境变量 DEEPSEEK_API_KEY")?;
            tracing::info!(model = %ds.model, "识别：DeepSeek");
            recognize::Recognizer::deepseek(
                api_key,
                ds.model,
                Duration::from_secs(ds.timeout_secs),
            )?
        }
        None => {
            tracing::warn!("识别：mock（config.toml 未配置 [deepseek]），返回假账单");
            recognize::Recognizer::Mock
        }
    };

    let state = api::AppState {
        token: config.token,
        recognizer,
        db: db::Db::open(&config.db_path)?,
    };
    let listener = tokio::net::TcpListener::bind(&config.listen)
        .await
        .with_context(|| format!("监听 {}", config.listen))?;
    tracing::info!(listen = %config.listen, db = %config.db_path.display(), "micjz 已启动");
    axum::serve(listener, api::router(Arc::new(state)))
        .await
        .context("HTTP 服务退出")
}
