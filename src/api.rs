//! HTTP 接口：鉴权、解析请求，所有响应（含错误）统一为 flow::Reply。

use std::{sync::Arc, time::Instant};

use axum::{
    Json, Router,
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
    db::{Db, Recognition},
    flow::{self, Draft, Outcome, Reply},
    recognize::Recognizer,
};

/// Mac Retina 全屏截图可达十几 MB。
const MAX_BODY_BYTES: usize = 32 * 1024 * 1024;

pub struct AppState {
    pub token: String,
    pub recognizer: Recognizer,
    pub db: Db,
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/recognize", post(recognize_handler))
        .route("/step", post(step_handler))
        .fallback(not_found)
        .method_not_allowed_fallback(not_found)
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(middleware::from_fn_with_state(state.clone(), auth))
        .layer(middleware::from_fn(access_log))
        .with_state(state)
}

/// 每个请求一行：方法、路径、状态码、耗时。失败原因由各处理点另记。
async fn access_log(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_owned();
    let started = Instant::now();
    let resp = next.run(req).await;
    tracing::info!(
        %method,
        path,
        status = resp.status().as_u16(),
        elapsed_ms = started.elapsed().as_millis() as u64,
        "请求"
    );
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
        (self.0, Json(Reply::done(self.1))).into_response()
    }
}

fn bad_request(reason: impl std::fmt::Display) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, format!("请求无效：{reason}"))
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
    let upload_ms = started.elapsed().as_millis() as u64;
    let saving = Instant::now();
    let decoded =
        image::load_from_memory(&image).map_err(|e| bad_request(format!("图片无法解码：{e}")))?;
    let image_path = save_image(&state.db.1, &image, &decoded).map_err(|e| {
        tracing::error!(error = %e, "图片解码或保存失败");
        ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            "图片无法解码或保存".into(),
        )
    })?;
    let image_ms = saving.elapsed().as_millis() as u64;
    let now = flow::now();
    let sha = sha256_hex(&image);
    let recognized = state.recognizer.recognize(&image, now).await;
    let meta = &recognized.meta;
    let draft_id = match &recognized.result {
        Ok(draft) => draft.id.clone(),
        Err(_) => uuid::Uuid::now_v7().to_string(),
    };
    let (outcome, reason, origin) = match &recognized.result {
        Ok(draft) => ("ok", None, Some(draft)),
        Err(e) => (e.outcome(), Some(e.to_string()), None),
    };
    let db_started = Instant::now();
    state.db.record_recognition(&Recognition {
        draft_id: &draft_id,
        outcome,
        model: &meta.model,
        elapsed_ms: meta.elapsed_ms,
        image_sha256: &sha,
        image_bytes: image.len(),
        raw: &meta.raw,
        reason: reason.as_deref(),
        origin,
        image_path: &image_path,
        prompt: &meta.prompt,
        parameters: &meta.parameters,
        timings: &serde_json::json!({"upload_ms":upload_ms,"image_ms":image_ms,"model_ms":meta.elapsed_ms}).to_string(),
    }).map_err(|e| {
        tracing::error!(error = %e, "保存识别记录失败");
        ApiError(StatusCode::INTERNAL_SERVER_ERROR, "保存识别记录失败".into())
    })?;
    tracing::info!(
        upload_ms,
        image_ms,
        model_ms = meta.elapsed_ms,
        db_ms = db_started.elapsed().as_millis() as u64,
        total_ms = started.elapsed().as_millis() as u64,
        "识别阶段耗时"
    );
    match recognized.result {
        Ok(draft) => {
            tracing::info!(id = %draft.id, bytes = image.len(), "识别完成");
            Ok(Json(flow::menu(&draft, now)))
        }
        Err(e) => {
            if outcome == "error" {
                tracing::warn!(reason = %e, "识别失败");
            } else {
                tracing::info!(reason = %e, "识别无结果");
            }
            Ok(Json(Reply::done(e.to_string())))
        }
    }
}

/// 图片内容指纹，同一张截图重复识别可查。
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[derive(Deserialize)]
struct StepRequest {
    draft: String,
    field: String,
    value: String,
}

async fn step_handler(
    State(state): State<Arc<AppState>>,
    body: Result<Json<StepRequest>, JsonRejection>,
) -> Result<Json<Reply>, ApiError> {
    let Json(req) = body.map_err(|e| bad_request(e.body_text()))?;
    let before = Draft::decode(&req.draft).map_err(bad_request)?;
    tracing::info!(id = %before.id, field = %req.field, "步骤");
    match flow::step(before, &req.field, &req.value, flow::now()).map_err(bad_request)? {
        Outcome::Reply(reply) => {
            tracing::info!(next = ?reply.step, field = %reply.field, "下一步");
            Ok(Json(reply))
        }
        Outcome::Confirm(draft) => match state.db.save(&draft) {
            Ok(inserted) => {
                tracing::info!(id = %draft.id, inserted, "入账");
                Ok(Json(Reply::done(flow::saved_message(&draft))))
            }
            Err(e) => {
                tracing::error!(id = %draft.id, error = ?e, "入账失败");
                Err(ApiError(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("入账失败，未记账：{e}"),
                ))
            }
        },
    }
}

#[cfg(test)]
mod tests;

fn save_image(
    dir: &std::path::Path,
    bytes: &[u8],
    image: &image::DynamicImage,
) -> anyhow::Result<String> {
    use image::ImageEncoder;
    let mut encoded = Vec::new();
    image::codecs::png::PngEncoder::new_with_quality(
        &mut encoded,
        image::codecs::png::CompressionType::Best,
        image::codecs::png::FilterType::Adaptive,
    )
    .write_image(
        image.as_bytes(),
        image.width(),
        image.height(),
        image.color().into(),
    )?;
    let (data, ext) = if encoded.len() < bytes.len() {
        (encoded.as_slice(), "png")
    } else if bytes.starts_with(b"\x89PNG") {
        (bytes, "png")
    } else {
        (bytes, "jpg")
    };
    std::fs::create_dir_all(dir)?;
    let path = dir.join(format!("{}.{}", sha256_hex(bytes), ext));
    if !path.exists() {
        std::fs::write(&path, data)?;
    }
    Ok(path.to_string_lossy().into_owned())
}
