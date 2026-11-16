//! 网页认证的 HTTP 边界；密码运算不占异步线程，密钥每次从库里读。
use super::web::{WebError, body, internal, invalid};
use super::*;
use crate::auth::{self, Admin};
use axum::http::{HeaderMap, HeaderValue};

const COOKIE: &str = "micjz_session";
const LOGIN_ERROR: &str = "用户名或密码不正确";

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
fn denied() -> WebError {
    WebError(StatusCode::UNAUTHORIZED, "未登录或登录已过期".into())
}
fn limited() -> WebError {
    WebError(
        StatusCode::TOO_MANY_REQUESTS,
        "登录尝试过多，请稍后再试".into(),
    )
}
fn cookie(admin: &Admin, headers: &HeaderMap) -> HeaderValue {
    let secure = if headers
        .get("x-forwarded-proto")
        .is_some_and(|v| v == "https")
    {
        "; Secure"
    } else {
        ""
    };
    HeaderValue::from_str(&format!(
        "{COOKIE}={}; HttpOnly; SameSite=Strict; Path=/; Max-Age={}{secure}",
        admin.session(now_ms()),
        auth::SESSION_SECONDS
    ))
    .expect("签名 Cookie 仅含合法 ASCII")
}
fn with_cookie(admin: &Admin, headers: &HeaderMap) -> Response {
    (
        StatusCode::NO_CONTENT,
        [(header::SET_COOKIE, cookie(admin, headers))],
    )
        .into_response()
}
fn cookie_value(headers: &HeaderMap) -> Option<&str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|s| s.split(';'))
        .filter_map(|s| s.trim().split_once('='))
        .find(|(key, _)| *key == COOKIE)
        .map(|(_, v)| v)
}
/// 浏览器携带请求元数据时校验确切来源；非浏览器客户端仍可用 Cookie 调试。
fn same_origin(headers: &HeaderMap) -> bool {
    if let Some(site) = headers.get("sec-fetch-site") {
        return site == "same-origin";
    }
    let Some(origin) = headers.get(header::ORIGIN) else {
        return true;
    };
    let Some(host) = headers.get(header::HOST).and_then(|v| v.to_str().ok()) else {
        return false;
    };
    let Ok(origin) = origin.to_str() else {
        return false;
    };
    let Ok(uri) = origin.parse::<axum::http::Uri>() else {
        return false;
    };
    matches!(uri.scheme_str(), Some("http" | "https"))
        && uri
            .authority()
            .is_some_and(|a| a.as_str().eq_ignore_ascii_case(host))
}

pub(super) async fn middleware(
    State(s): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Response {
    if !matches!(
        *req.method(),
        axum::http::Method::GET | axum::http::Method::HEAD
    ) && !same_origin(req.headers())
    {
        return WebError(StatusCode::FORBIDDEN, "只允许从本站页面发起操作".into()).into_response();
    }
    if req.uri().path() == "/api/login" && req.method() == axum::http::Method::POST {
        return next.run(req).await;
    }
    let admin = match auth::load(&s.db.conn()) {
        Ok(Some(a)) => a,
        Ok(None) => return denied().into_response(),
        Err(e) => return internal(e).into_response(),
    };
    let Some(renew) = cookie_value(req.headers()).and_then(|v| admin.verify_session(v, now_ms()))
    else {
        return denied().into_response();
    };
    let renewed = renew.then(|| cookie(&admin, req.headers()));
    req.extensions_mut().insert(admin);
    let mut response = next.run(req).await;
    // 改密和退出的响应优先，不能被旧密钥的续期 Cookie 覆盖。
    if !response.headers().contains_key(header::SET_COOKIE)
        && let Some(cookie) = renewed
    {
        response.headers_mut().insert(header::SET_COOKIE, cookie);
    }
    response
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Login {
    username: String,
    password: String,
}
pub(super) async fn login(
    State(s): State<Arc<AppState>>,
    headers: HeaderMap,
    v: Result<Json<Login>, JsonRejection>,
) -> Result<Response, WebError> {
    let v = body(v)?;
    if s.login_limit
        .lock()
        .expect("登录限速锁未中毒")
        .blocked(Instant::now())
    {
        return Err(limited());
    }
    let admin = auth::load(&s.db.conn())?.ok_or_else(|| {
        WebError(
            StatusCode::CONFLICT,
            "尚未设置密码：在服务器上运行 micjz password".into(),
        )
    })?;
    let (admin, valid) = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
        let valid = admin.verify_password(&v.password)? & (v.username == auth::USERNAME);
        Ok((admin, valid))
    })
    .await
    .map_err(internal)?
    .map_err(internal)?;
    let current = auth::load(&s.db.conn())?.is_some_and(|a| a.session_key == admin.session_key);
    let mut limit = s.login_limit.lock().expect("登录限速锁未中毒");
    if limit.blocked(Instant::now()) {
        return Err(limited());
    }
    if !valid || !current {
        limit.failed(Instant::now());
        return Err(WebError(StatusCode::UNAUTHORIZED, LOGIN_ERROR.into()));
    }
    Ok(with_cookie(&admin, &headers))
}
pub(super) async fn logout() -> Response {
    (
        StatusCode::NO_CONTENT,
        [(header::SET_COOKIE, "micjz_session=; Max-Age=0; Path=/")],
    )
        .into_response()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Password {
    current: String,
    new: String,
}
pub(super) async fn password(
    State(s): State<Arc<AppState>>,
    Extension(old): Extension<Admin>,
    headers: HeaderMap,
    v: Result<Json<Password>, JsonRejection>,
) -> Result<Response, WebError> {
    let v = body(v)?;
    auth::check_new_password(&v.new).map_err(invalid)?;
    let previous_key = old.session_key.clone();
    let admin = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
        if !old.verify_password(&v.current)? {
            return Ok(None);
        }
        Ok(Some(Admin::for_password(&v.new)?))
    })
    .await
    .map_err(internal)?
    .map_err(internal)?
    .ok_or_else(|| invalid("当前密码不正确"))?;
    let mut conn = s.db.conn();
    let tx = conn.transaction()?;
    if auth::load(&tx)?.is_none_or(|a| a.session_key != previous_key) {
        return Err(denied());
    }
    auth::store(&tx, &admin)?;
    tx.commit()?;
    Ok(with_cookie(&admin, &headers))
}
