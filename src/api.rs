//! HTTP 接口：鉴权、解析请求，所有响应（含错误）统一为 flow::Reply。

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Instant,
};

use axum::{
    Extension, Json, Router,
    extract::{
        DefaultBodyLimit, Multipart, Request, State, multipart::MultipartRejection,
        rejection::JsonRejection,
    },
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::post,
};
use serde::Deserialize;

use crate::{
    catalog::Catalog,
    db::{self, Db},
    flow::{self, Draft, Outcome, Reply},
    recognize::{self, Recognizer},
};

/// Mac Retina 全屏截图可达十几 MB。
const MAX_BODY_BYTES: usize = 32 * 1024 * 1024;

pub struct AppState {
    pub token: String,
    pub recognizer: Recognizer,
    pub db: Db,
    /// 识别图片留存目录。
    pub images_dir: PathBuf,
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/recognize", post(recognize_handler))
        .route("/step", post(step_handler))
        .fallback(not_found)
        .method_not_allowed_fallback(not_found)
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(middleware::from_fn_with_state(state.clone(), auth))
        .layer(middleware::from_fn_with_state(state.clone(), access_log))
        .with_state(state)
}

/// 处理器补充的请求信息，由访问日志写进 `requests` 表。
#[derive(Default)]
pub struct Trace {
    pub draft_id: Option<String>,
    pub field: Option<String>,
    pub value: Option<String>,
}

type TraceSlot = Arc<Mutex<Trace>>;

/// 失败原因，由 `ApiError` 挂在响应上。
#[derive(Clone)]
struct Failure(String);

/// 每个请求：终端一行（方法、路径、状态码、耗时）；通过鉴权的再在库里记一行。
async fn access_log(State(state): State<Arc<AppState>>, mut req: Request, next: Next) -> Response {
    let method = req.method().to_string();
    let path = req.uri().path().to_owned();
    let at = flow::now().to_rfc3339();
    let started = Instant::now();
    let slot = TraceSlot::default();
    req.extensions_mut().insert(slot.clone());
    let resp = next.run(req).await;
    let elapsed_ms = started.elapsed().as_millis() as u64;
    let status = resp.status();
    tracing::info!(method, path, status = status.as_u16(), elapsed_ms, "请求");
    // 401 只来自鉴权：token 不对的不进库。
    if status == StatusCode::UNAUTHORIZED {
        return resp;
    }
    let error = resp.extensions().get::<Failure>().map(|f| f.0.clone());
    let trace = std::mem::take(&mut *slot.lock().expect("trace 锁未中毒"));
    let row = db::RequestRow {
        at: &at,
        method: &method,
        path: &path,
        status: status.as_u16(),
        elapsed_ms,
        trace: &trace,
        error: error.as_deref(),
    };
    if let Err(e) = state.db.record_request(&row) {
        tracing::error!(error = %e, "保存请求记录失败");
    }
    resp
}

async fn not_found() -> ApiError {
    ApiError(StatusCode::NOT_FOUND, "接口不存在".to_owned())
}

struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if self.0.is_server_error() {
            tracing::error!(status = self.0.as_u16(), reason = %self.1, "请求失败");
        } else {
            tracing::warn!(status = self.0.as_u16(), reason = %self.1, "请求失败");
        }
        let mut resp = (self.0, Json(Reply::done(self.1.clone()))).into_response();
        resp.extensions_mut().insert(Failure(self.1));
        resp
    }
}

fn bad_request(reason: impl std::fmt::Display) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, format!("请求无效：{reason}"))
}

fn load_catalog(state: &AppState) -> Result<Catalog, ApiError> {
    state.db.catalog().map_err(|e| {
        ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("读取分类失败：{e:#}"),
        )
    })
}

async fn auth(State(state): State<Arc<AppState>>, req: Request, next: Next) -> Response {
    let expected = format!("Bearer {}", state.token);
    let ok = req
        .headers()
        .get(header::AUTHORIZATION)
        .is_some_and(|v| v.as_bytes() == expected.as_bytes());
    if !ok {
        return ApiError(StatusCode::UNAUTHORIZED, "鉴权失败".to_owned()).into_response();
    }
    next.run(req).await
}

async fn recognize_handler(
    State(state): State<Arc<AppState>>,
    Extension(trace): Extension<TraceSlot>,
    multipart: Result<Multipart, MultipartRejection>,
) -> Result<Json<Reply>, ApiError> {
    let started = Instant::now();
    let mut multipart = multipart.map_err(|e| bad_request(e.body_text()))?;
    let (mut image, mut names) = (None, Vec::new());
    while let Some(field) = multipart.next_field().await.map_err(bad_request)? {
        let name = field.name().unwrap_or("").to_owned();
        if name == "image" {
            image = Some(field.bytes().await.map_err(bad_request)?);
        }
        names.push(name);
    }
    let image = image.ok_or_else(|| bad_request(format!("缺少 image，收到字段 {names:?}")))?;
    let mime =
        recognize::mime(&image).ok_or_else(|| bad_request("图片格式不支持，仅支持 PNG / JPEG"))?;
    let upload_ms = started.elapsed().as_millis() as u64;

    // 留存不挡识别：后台写盘（PNG 顺便无损压缩），失败只记日志。
    let sha = sha256_hex(&image);
    let ext = if mime == "image/png" { "png" } else { "jpg" };
    let path = state.images_dir.join(format!("{sha}.{ext}"));
    tokio::task::spawn_blocking({
        let (image, path) = (image.clone(), path.clone());
        move || {
            if let Err(e) = store_image(&path, &image) {
                tracing::error!(path = %path.display(), error = %format!("{e:#}"), "图片留存失败");
            }
        }
    });

    let cat = load_catalog(&state)?;
    let now = flow::now();
    let r = state.recognizer.recognize(&image, mime, now, &cat).await;
    trace.lock().expect("trace 锁未中毒").draft_id = Some(r.draft.id.clone());
    let image_meta = db::Image {
        sha256: &sha,
        bytes: image.len(),
        path: &path.to_string_lossy(),
    };
    state
        .db
        .record_recognition(&r, &image_meta, upload_ms)
        .map_err(|e| {
            tracing::error!(error = ?e, "保存识别记录失败");
            ApiError(StatusCode::INTERNAL_SERVER_ERROR, "保存识别记录失败".into())
        })?;
    tracing::info!(
        id = %r.draft.id,
        outcome = r.outcome.as_str(),
        bytes = image.len(),
        upload_ms,
        model_ms = r.model_ms,
        total_ms = started.elapsed().as_millis() as u64,
        "识别"
    );
    Ok(Json(flow::menu(&r.draft, now)))
}

/// 已存在（同一张图）就跳过；PNG 无损重压，更小才用。
fn store_image(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let smaller = if path.extension().is_some_and(|e| e == "png") {
        recompress_png(bytes).unwrap_or_else(|e| {
            tracing::warn!(error = %format!("{e:#}"), "PNG 重压失败，存原图");
            None
        })
    } else {
        None
    };
    std::fs::write(path, smaller.as_deref().unwrap_or(bytes))?;
    Ok(())
}

fn recompress_png(bytes: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
    use image::{
        ImageEncoder,
        codecs::png::{CompressionType, FilterType, PngEncoder},
    };
    let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)?;
    let mut out = Vec::new();
    PngEncoder::new_with_quality(&mut out, CompressionType::Best, FilterType::Adaptive)
        .write_image(
            img.as_bytes(),
            img.width(),
            img.height(),
            img.color().into(),
        )?;
    Ok((out.len() < bytes.len()).then_some(out))
}

/// 图片内容指纹，也是留存文件名。
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Deserialize)]
struct StepRequest {
    draft: String,
    field: String,
    value: String,
}

async fn step_handler(
    State(state): State<Arc<AppState>>,
    Extension(trace): Extension<TraceSlot>,
    body: Result<Json<StepRequest>, JsonRejection>,
) -> Result<Json<Reply>, ApiError> {
    let Json(req) = body.map_err(|e| bad_request(e.body_text()))?;
    {
        let mut t = trace.lock().expect("trace 锁未中毒");
        t.field = Some(req.field.clone());
        t.value = Some(req.value.clone());
    }
    let cat = load_catalog(&state)?;
    let before = Draft::decode(&req.draft, &cat).map_err(bad_request)?;
    trace.lock().expect("trace 锁未中毒").draft_id = Some(before.id.clone());
    let known = state.db.has_recognition(&before.id).map_err(|e| {
        ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("读取识别记录失败：{e}"),
        )
    })?;
    if !known {
        return Err(bad_request(format!("草稿不存在：{}", before.id)));
    }
    tracing::info!(id = %before.id, field = %req.field, "步骤");
    match flow::step(before, &req.field, &req.value, flow::now(), &cat).map_err(bad_request)? {
        Outcome::Reply(reply) => {
            tracing::info!(next = ?reply.step, field = %reply.field, "下一步");
            Ok(Json(reply))
        }
        Outcome::Confirm {
            draft,
            amount_cents,
        } => match state.db.save(&draft, amount_cents) {
            Ok(inserted) => {
                tracing::info!(id = %draft.id, inserted, "入账");
                Ok(Json(Reply::done(flow::saved_message(&draft, amount_cents))))
            }
            Err(e) => {
                tracing::error!(id = %draft.id, error = %format!("{e:#}"), "入账失败");
                Err(ApiError(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("入账失败，未记账：{e:#}"),
                ))
            }
        },
    }
}

#[cfg(test)]
mod tests;
