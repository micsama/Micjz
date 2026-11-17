mod api;
mod auth;
mod catalog;
mod db;
mod flow;
mod recognize;
mod update;

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

    // micjz [config.toml] [管理命令...]：不带命令即启动服务。
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let path = if args.first().is_some_and(|a| a.ends_with(".toml")) {
        args.remove(0)
    } else {
        "config.toml".to_owned()
    };
    let text = std::fs::read_to_string(&path).with_context(|| format!("读取配置 {path}"))?;
    let config: Config = toml::from_str(&text).with_context(|| format!("解析配置 {path}"))?;
    anyhow::ensure!(!config.token.is_empty(), "token 不能为空");
    let db = db::Db::open(&config.db_path)?;
    if args == ["password"] {
        auth::password_cli(&db)?;
        return Ok(());
    }
    if !args.is_empty() {
        println!("{}", catalog::cli(&mut db.conn(), &args)?);
        return Ok(());
    }
    recognize::check_prompt()?;
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

    // 只有经 build.sh run|start 启动才有仓库路径，才能一键更新。
    let updater = std::env::var_os("MICJZ_REPO").map(|repo| {
        let config = std::path::absolute(&path).expect("配置路径已读取成功");
        Arc::new(update::Updater::new(repo.into(), config))
    });
    let state = api::AppState {
        updater: updater.clone(),
        token: config.token,
        login_limit: Default::default(),
        recognizer,
        db,
        images_dir: config
            .db_path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("images"),
    };
    let listener = tokio::net::TcpListener::bind(&config.listen)
        .await
        .with_context(|| format!("监听 {}", config.listen))?;
    tracing::info!(listen = %config.listen, db = %config.db_path.display(), "micjz 已启动");
    let watch = updater.clone();
    let restart = async move {
        match &watch {
            Some(u) => u.restart.notified().await,
            None => std::future::pending().await,
        }
    };
    axum::serve(listener, api::router(Arc::new(state)))
        .with_graceful_shutdown(restart)
        .await
        .context("HTTP 服务退出")?;
    // 只有更新成功才会走到这里：和手动更新同一条路，备份、转换后启动新版本。
    let mut start = updater.expect("只有更新会停服").start_command();
    tracing::info!(command = ?start, "正在重启");
    let error = std::os::unix::process::CommandExt::exec(&mut start);
    Err(anyhow::anyhow!("重启失败：{error}"))
}
