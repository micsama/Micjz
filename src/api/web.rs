//! 网页协议边界：JSON、Cookie、静态资源；业务写入复用数据库事务。
use super::*;
use crate::catalog::{
    accounts::{self, Account, AccountInput},
    categories, tags,
};
use crate::{
    catalog,
    db::{
        Error,
        entries::{self, EntryInput, EntryKind, required_nullable},
    },
};
use axum::{
    extract::{Path as Param, Query},
    routing::{get, put},
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

#[derive(rust_embed::RustEmbed)]
#[folder = "web/dist/"]
struct Assets;

pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/login", post(super::session::login))
        .route(
            "/api/web-performance",
            post(web_performance).layer(axum::extract::DefaultBodyLimit::max(8 * 1024)),
        )
        .route("/api/logout", post(super::session::logout))
        .route("/api/password", put(super::session::password))
        .route("/api/catalog", get(catalog_handler))
        .route("/api/entries", get(list_entries).post(create_entry))
        .route(
            "/api/entries/{id}",
            axum::routing::patch(patch_entry).delete(delete_entry),
        )
        .route("/api/balances", get(balances))
        .route("/api/settings", get(settings).put(save_settings))
        .route("/api/accounts", post(create_account))
        .route(
            "/api/accounts/{id}",
            put(replace_account).delete(delete_account),
        )
        .route("/api/accounts/{id}/move", post(move_account))
        .route("/api/accounts/{id}/adjust", post(adjust))
        .route("/api/categories", post(create_category))
        .route(
            "/api/categories/{id}",
            put(replace_category).delete(delete_category),
        )
        .route("/api/categories/{id}/move", post(move_category))
        .route("/api/tags", post(create_tag))
        .route("/api/tags/{id}", put(rename_tag).delete(delete_tag))
        .route("/api/recognitions/{id}", get(recognition))
        .route("/api/recognitions/{id}/image", get(image))
        .route("/api/export.csv", get(export))
        .route("/api/update", get(update_status).post(start_update))
}

#[derive(Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum PerformanceKind {
    Startup,
    Login,
    Retry,
    Resume,
}

#[derive(Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum PerformanceOutcome {
    Ready,
    Login,
    Failed,
}

#[derive(Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum ResourceKind {
    Js,
    Css,
    Catalog,
    Settings,
    Balances,
    Entries,
}

#[derive(Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum NavigationType {
    Navigate,
    Reload,
    BackForward,
    Prerender,
}

#[derive(Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum PeriodGuess {
    None,
    Hit,
    Miss,
}

#[derive(Deserialize, serde::Serialize)]
struct ResourceTiming {
    start_ms: u32,
    duration_ms: u32,
    ttfb_ms: Option<u32>,
    download_ms: Option<u32>,
    transfer_bytes: u32,
    encoded_bytes: u32,
    decoded_bytes: u32,
}

#[derive(Deserialize, serde::Serialize)]
struct ResourcePerformance {
    label: ResourceKind,
    #[serde(flatten)]
    timing: ResourceTiming,
}

#[derive(Deserialize, serde::Serialize)]
struct NavigationPerformance {
    dns_ms: u32,
    connect_ms: u32,
    server_wait_ms: Option<u32>,
    navigation_type: NavigationType,
    /// 导航起点到浏览器发出 HTML 请求；连接复用时这段是浏览器内部排队。
    request_at_ms: Option<u32>,
    #[serde(flatten)]
    timing: ResourceTiming,
}

#[derive(Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct WebPerformance {
    kind: PerformanceKind,
    outcome: PerformanceOutcome,
    standalone: bool,
    total_ms: u32,
    paint_ms: Option<u32>,
    before_boot_ms: Option<u32>,
    first_contentful_paint_ms: Option<u32>,
    catalog_ms: Option<u32>,
    settings_ms: Option<u32>,
    entries_ms: Option<u32>,
    balances_ms: Option<u32>,
    navigation: Option<NavigationPerformance>,
    resource_count: u32,
    resources: Vec<ResourcePerformance>,
    period_guess: Option<PeriodGuess>,
}

async fn web_performance(
    payload: Result<Json<WebPerformance>, JsonRejection>,
) -> Result<StatusCode, WebError> {
    let p = body(payload)?;
    if p.resources.len() > 24 {
        return Err(invalid("性能记录含太多资源"));
    }
    let metrics = serde_json::to_string(&p).map_err(internal)?;
    tracing::info!(
        version = option_env!("MICJZ_VERSION").unwrap_or("dev"),
        %metrics,
        "web_performance"
    );
    Ok(StatusCode::NO_CONTENT)
}

pub(super) struct WebError(pub(super) StatusCode, pub(super) String);
impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        let mut resp = (self.0, Json(json!({"error": self.1}))).into_response();
        resp.extensions_mut().insert(Failure(self.1));
        resp
    }
}
impl From<Error> for WebError {
    fn from(e: Error) -> Self {
        match e {
            Error::Invalid(s) => Self(StatusCode::BAD_REQUEST, s),
            Error::Conflict(s) => Self(StatusCode::CONFLICT, s),
            Error::Missing(s) => Self(StatusCode::NOT_FOUND, s),
            Error::Sql(e) => internal(e),
        }
    }
}
impl From<rusqlite::Error> for WebError {
    fn from(e: rusqlite::Error) -> Self {
        internal(e)
    }
}
pub(super) fn internal(e: impl std::fmt::Display) -> WebError {
    tracing::error!(error = %format!("{e:#}"), "网页请求失败");
    WebError(
        StatusCode::INTERNAL_SERVER_ERROR,
        "服务器处理失败，请稍后再试".into(),
    )
}
pub(super) fn invalid(e: impl std::fmt::Display) -> WebError {
    WebError(StatusCode::BAD_REQUEST, format!("请求无效：{e}"))
}
fn missing(label: &str) -> WebError {
    WebError(StatusCode::NOT_FOUND, format!("{label}不存在"))
}
pub(super) fn body<T: DeserializeOwned>(v: Result<Json<T>, JsonRejection>) -> Result<T, WebError> {
    v.map(|Json(v)| v)
        .map_err(|_| invalid("JSON 格式或字段不正确"))
}
fn path_id<T>(v: Result<Param<T>, axum::extract::rejection::PathRejection>) -> Result<T, WebError> {
    v.map(|Param(v)| v).map_err(|_| invalid("ID 格式不正确"))
}

pub(super) async fn static_file(req: Request) -> Response {
    let path = req.uri().path();
    if path.starts_with("/api/") {
        return WebError(StatusCode::NOT_FOUND, "接口不存在".into()).into_response();
    }
    if req.method() != axum::http::Method::GET && req.method() != axum::http::Method::HEAD {
        return not_found(req).await;
    }
    let path = if path == "/" {
        "index.html"
    } else {
        path.trim_start_matches('/')
    };
    let Some(file) = Assets::get(path) else {
        return not_found(req).await;
    };
    let mime = match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "svg" => "image/svg+xml",
        "webmanifest" => "application/manifest+json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    };
    let cache = if path.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    (
        [(header::CONTENT_TYPE, mime), (header::CACHE_CONTROL, cache)],
        file.data.into_owned(),
    )
        .into_response()
}

async fn catalog_handler(State(s): State<Arc<AppState>>) -> Result<Json<Value>, WebError> {
    let conn = s.db.conn();
    Ok(Json(json!({
        "categories": categories::list(&conn)?,
        "accounts": accounts::list(&conn)?,
        "tags": tags::list(&conn)?,
    })))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Range {
    from: String,
    to: String,
}
async fn list_entries(
    State(s): State<Arc<AppState>>,
    q: Result<Query<Range>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<Value>, WebError> {
    let Query(q) = q.map_err(|_| invalid("必须提供 from 和 to"))?;
    let (from, to) = (entries::timestamp(&q.from)?, entries::timestamp(&q.to)?);
    if from >= to {
        return Err(invalid("from 必须早于 to"));
    }
    let conn = s.db.conn();
    let earliest: Option<String> =
        conn.query_row("SELECT min(occurred_at) FROM entries", [], |r| r.get(0))?;
    Ok(Json(
        json!({"entries":entries::list(&conn,&from,&to,false)?,"earliest":earliest}),
    ))
}
async fn balances(State(s): State<Arc<AppState>>) -> Result<Json<Value>, WebError> {
    let conn = s.db.conn();
    let balances = accounts::list(&conn)?
        .iter()
        .map(|a| Ok(json!({"account_id":a.id,"balance_cents":entries::balance(&conn,a.id)?})))
        .collect::<Result<Vec<_>, WebError>>()?;
    Ok(Json(json!({"balances":balances})))
}
fn read_settings(conn: &Connection) -> Result<Value, WebError> {
    let (cycle, budget): (i64, Option<i64>) = conn.query_row(
        "SELECT cycle_day,budget_cents FROM settings WHERE id=1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let count: i64 = conn.query_row("SELECT count(*) FROM entries", [], |r| r.get(0))?;
    let earliest: Option<String> =
        conn.query_row("SELECT min(occurred_at) FROM entries", [], |r| r.get(0))?;
    Ok(
        json!({"cycle_day":cycle,"budget_cents":budget,"version":option_env!("MICJZ_VERSION").unwrap_or("dev"),"entry_count":count,"earliest":earliest}),
    )
}
async fn settings(State(s): State<Arc<AppState>>) -> Result<Json<Value>, WebError> {
    Ok(Json(read_settings(&s.db.conn())?))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Settings {
    cycle_day: i64,
    #[serde(deserialize_with = "required_nullable")]
    budget_cents: Option<i64>,
}
async fn save_settings(
    State(s): State<Arc<AppState>>,
    v: Result<Json<Settings>, JsonRejection>,
) -> Result<Json<Value>, WebError> {
    let v = body(v)?;
    if !(1..=28).contains(&v.cycle_day.abs())
        || v.cycle_day == -1
        || v.budget_cents
            .is_some_and(|n| !(1..=entries::MAX_AMOUNT).contains(&n))
    {
        return Err(invalid("周期或预算超出范围"));
    }
    let conn = s.db.conn();
    conn.execute(
        "UPDATE settings SET cycle_day=?1,budget_cents=?2 WHERE id=1",
        params![v.cycle_day, v.budget_cents],
    )?;
    Ok(Json(read_settings(&conn)?))
}
/// 网页手填；`id` 由网页生成，重试沿用，用来防重复提交。
#[derive(Deserialize)]
struct NewEntry {
    id: String,
    #[serde(flatten)]
    input: EntryInput,
}
/// 同一 `id` 已入账就原样返回那一笔（200），不比较内容；新建 201。
async fn create_entry(
    State(s): State<Arc<AppState>>,
    v: Result<Json<NewEntry>, JsonRejection>,
) -> Result<Response, WebError> {
    let NewEntry { id, input } = body(v)?;
    let id = uuid::Uuid::parse_str(&id)
        .ok()
        .filter(|u| u.get_version_num() == 7)
        .ok_or_else(|| invalid("id 须为 UUIDv7"))?
        .to_string();
    if input.kind == EntryKind::Adjust {
        return Err(invalid("平账请用账户平账接口"));
    }
    let mut conn = s.db.conn();
    let tx = conn.transaction()?;
    if let Some(existing) = entries::find(&tx, &id)? {
        return Ok(Json(existing).into_response());
    }
    let entry = entries::insert(&tx, id, input.parse()?, None)?;
    tx.commit()?;
    Ok((StatusCode::CREATED, Json(entry)).into_response())
}
async fn patch_entry(
    State(s): State<Arc<AppState>>,
    p: Result<Param<String>, axum::extract::rejection::PathRejection>,
    v: Result<Json<serde_json::Map<String, Value>>, JsonRejection>,
) -> Result<Json<entries::Entry>, WebError> {
    let id = path_id(p)?;
    let patch = body(v)?;
    if patch.is_empty() {
        return Err(invalid("没有要修改的字段"));
    }
    let mut conn = s.db.conn();
    let tx = conn.transaction()?;
    let current = entries::get(&tx, &id)?;
    let mut merged = serde_json::to_value(&current.input).map_err(internal)?;
    for (k, v) in &patch {
        if merged.get(k).is_none() {
            return Err(invalid("未知字段"));
        }
        if v.is_null() && k != "merchant" && k != "note" {
            return Err(invalid("该字段不能清空"));
        }
        merged[k] = v.clone();
    }
    let input: EntryInput =
        serde_json::from_value(merged).map_err(|_| invalid("字段类型不正确"))?;
    if input.kind != current.input.kind
        && (!matches!(
            (current.input.kind, input.kind),
            (EntryKind::Expense, EntryKind::Income) | (EntryKind::Income, EntryKind::Expense)
        ) || !patch.contains_key("category_id"))
    {
        return Err(invalid("只允许支出收入互换，且必须同时指定分类"));
    }
    let updated = entries::update(&tx, &id, input.parse()?)?;
    tx.commit()?;
    Ok(Json(updated))
}
async fn delete_entry(
    State(s): State<Arc<AppState>>,
    p: Result<Param<String>, axum::extract::rejection::PathRejection>,
) -> Result<StatusCode, WebError> {
    let id = path_id(p)?;
    if s.db
        .conn()
        .execute("DELETE FROM entries WHERE id=?1", [id])?
        == 0
    {
        return Err(missing("账目"));
    }
    Ok(StatusCode::NO_CONTENT)
}
fn save_account(s: &AppState, id: Option<i64>, v: AccountInput) -> Result<Account, WebError> {
    let mut conn = s.db.conn();
    let tx = conn.transaction()?;
    let account = accounts::save(&tx, id, v)?;
    tx.commit()?;
    Ok(account)
}
async fn create_account(
    State(s): State<Arc<AppState>>,
    v: Result<Json<AccountInput>, JsonRejection>,
) -> Result<Response, WebError> {
    Ok((StatusCode::CREATED, Json(save_account(&s, None, body(v)?)?)).into_response())
}
async fn replace_account(
    State(s): State<Arc<AppState>>,
    p: Result<Param<i64>, axum::extract::rejection::PathRejection>,
    v: Result<Json<AccountInput>, JsonRejection>,
) -> Result<Json<Account>, WebError> {
    Ok(Json(save_account(&s, Some(path_id(p)?), body(v)?)?))
}
async fn delete_account(
    State(s): State<Arc<AppState>>,
    p: Result<Param<i64>, axum::extract::rejection::PathRejection>,
) -> Result<StatusCode, WebError> {
    let id = path_id(p)?;
    let mut conn = s.db.conn();
    let tx = conn.transaction()?;
    accounts::delete(&tx, id)?;
    tx.commit()?;
    Ok(StatusCode::NO_CONTENT)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Move {
    to: i64,
}
async fn move_account(
    State(s): State<Arc<AppState>>,
    p: Result<Param<i64>, axum::extract::rejection::PathRejection>,
    v: Result<Json<Move>, JsonRejection>,
) -> Result<Json<Value>, WebError> {
    let id = path_id(p)?;
    let v = body(v)?;
    let mut conn = s.db.conn();
    let tx = conn.transaction()?;
    accounts::require(&tx, id)?;
    let ids = catalog::account_ids(&tx)?;
    catalog::reorder(&tx, "accounts", ids, id, v.to.max(0) as usize)?;
    let result = accounts::list(&tx)?;
    tx.commit()?;
    Ok(Json(json!({"accounts":result})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Adjust {
    balance_cents: i64,
    #[serde(default)]
    occurred_at: Option<String>,
}
async fn adjust(
    State(s): State<Arc<AppState>>,
    p: Result<Param<i64>, axum::extract::rejection::PathRejection>,
    v: Result<Json<Adjust>, JsonRejection>,
) -> Result<Json<Value>, WebError> {
    let id = path_id(p)?;
    let v = body(v)?;
    if v.balance_cents.unsigned_abs() > entries::MAX_AMOUNT as u64 {
        return Err(invalid("余额超出范围"));
    }
    let at = v
        .occurred_at
        .map(|s| entries::timestamp(&s))
        .transpose()?
        .unwrap_or_else(|| flow::stamp(flow::now()));
    let mut conn = s.db.conn();
    let tx = conn.transaction()?;
    accounts::require(&tx, id)?;
    let difference = v
        .balance_cents
        .checked_sub(entries::balance(&tx, id)?)
        .ok_or_else(|| invalid("差额超出范围"))?;
    let entry = if difference == 0 {
        None
    } else {
        Some(entries::insert(
            &tx,
            entries::new_id(),
            EntryInput {
                kind: EntryKind::Adjust,
                amount_cents: difference,
                category_id: None,
                account_id: id,
                to_account_id: None,
                merchant: None,
                occurred_at: at,
                note: None,
                tag_ids: vec![],
            }
            .parse()?,
            None,
        )?)
    };
    tx.commit()?;
    Ok(Json(json!({"entry":entry,"balance_cents":v.balance_cents})))
}
/// 目录写操作：一个事务里改完，返回改后的整张列表。
fn edit_catalog<T>(
    s: &AppState,
    f: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<(), Error>,
    list: impl FnOnce(&Connection) -> Result<T, Error>,
) -> Result<T, WebError> {
    let mut conn = s.db.conn();
    let tx = conn.transaction()?;
    f(&tx)?;
    let result = list(&tx)?;
    tx.commit()?;
    Ok(result)
}
fn category_list(conn: &Connection) -> Result<Value, Error> {
    Ok(json!({ "categories": categories::list(conn)? }))
}
fn tag_list(conn: &Connection) -> Result<Value, Error> {
    Ok(json!({ "tags": tags::list(conn)? }))
}
async fn create_category(
    State(s): State<Arc<AppState>>,
    v: Result<Json<categories::Input>, JsonRejection>,
) -> Result<Response, WebError> {
    let v = body(v)?;
    let list = edit_catalog(&s, |tx| categories::create(tx, v).map(drop), category_list)?;
    Ok((StatusCode::CREATED, Json(list)).into_response())
}
async fn replace_category(
    State(s): State<Arc<AppState>>,
    p: Result<Param<i64>, axum::extract::rejection::PathRejection>,
    v: Result<Json<categories::Input>, JsonRejection>,
) -> Result<Json<Value>, WebError> {
    let (id, v) = (path_id(p)?, body(v)?);
    Ok(Json(edit_catalog(
        &s,
        |tx| categories::update(tx, id, v),
        category_list,
    )?))
}
async fn delete_category(
    State(s): State<Arc<AppState>>,
    p: Result<Param<i64>, axum::extract::rejection::PathRejection>,
) -> Result<StatusCode, WebError> {
    let id = path_id(p)?;
    edit_catalog(&s, |tx| categories::delete(tx, id), |_| Ok(()))?;
    Ok(StatusCode::NO_CONTENT)
}
async fn move_category(
    State(s): State<Arc<AppState>>,
    p: Result<Param<i64>, axum::extract::rejection::PathRejection>,
    v: Result<Json<Move>, JsonRejection>,
) -> Result<Json<Value>, WebError> {
    let (id, v) = (path_id(p)?, body(v)?);
    Ok(Json(edit_catalog(
        &s,
        |tx| categories::move_to(tx, id, v.to.max(0) as usize),
        category_list,
    )?))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TagInput {
    name: String,
}
async fn create_tag(
    State(s): State<Arc<AppState>>,
    v: Result<Json<TagInput>, JsonRejection>,
) -> Result<Response, WebError> {
    let v = body(v)?;
    let list = edit_catalog(&s, |tx| tags::save(tx, None, &v.name).map(drop), tag_list)?;
    Ok((StatusCode::CREATED, Json(list)).into_response())
}
async fn rename_tag(
    State(s): State<Arc<AppState>>,
    p: Result<Param<i64>, axum::extract::rejection::PathRejection>,
    v: Result<Json<TagInput>, JsonRejection>,
) -> Result<Json<Value>, WebError> {
    let (id, v) = (path_id(p)?, body(v)?);
    Ok(Json(edit_catalog(
        &s,
        |tx| tags::save(tx, Some(id), &v.name).map(drop),
        tag_list,
    )?))
}
async fn delete_tag(
    State(s): State<Arc<AppState>>,
    p: Result<Param<i64>, axum::extract::rejection::PathRejection>,
) -> Result<StatusCode, WebError> {
    let id = path_id(p)?;
    edit_catalog(&s, |tx| tags::delete(tx, id), |_| Ok(()))?;
    Ok(StatusCode::NO_CONTENT)
}
async fn recognition(
    State(s): State<Arc<AppState>>,
    p: Result<Param<String>, axum::extract::rejection::PathRejection>,
) -> Result<Json<Value>, WebError> {
    let id = path_id(p)?;
    let conn = s.db.conn();
    let (outcome, reason, origin): (String, Option<String>, String) = conn
        .query_row(
            "SELECT outcome,reason,origin FROM recognitions WHERE draft_id=?1",
            [&id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?
        .ok_or_else(|| missing("识别记录"))?;
    let d: Origin = serde_json::from_str(&origin).map_err(internal)?;
    Ok(Json(json!({
        "id":id, "outcome":outcome, "reason":reason,
        "first":{
            "kind":d.kind, "amount_cents":d.amount_cents, "category_id":d.category_id,
            "account_id":d.account_id, "to_account_id":null, "merchant":d.merchant,
            "occurred_at":flow::stamp(d.occurred_at), "note":d.note, "tag_ids":d.tag_ids,
        }
    })))
}

/// 识别留存的第一眼草稿，只读。旧库转换时对不上的分类、账户为空；
/// 不走 `Draft`，因为当时的 id 现在可能已删。
#[derive(Deserialize)]
struct Origin {
    kind: flow::Kind,
    amount_cents: Option<i64>,
    category_id: Option<i64>,
    account_id: Option<i64>,
    merchant: Option<String>,
    occurred_at: chrono::DateTime<chrono::FixedOffset>,
    note: Option<String>,
    tag_ids: Vec<i64>,
}
async fn image(
    State(s): State<Arc<AppState>>,
    p: Result<Param<String>, axum::extract::rejection::PathRejection>,
) -> Result<Response, WebError> {
    let id = path_id(p)?;
    let path: String =
        s.db.conn()
            .query_row(
                "SELECT image_path FROM recognitions WHERE draft_id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| missing("识别记录"))?;
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(missing("截图")),
        Err(e) => return Err(internal(e)),
    };
    let mime = if path.ends_with(".png") {
        "image/png"
    } else {
        "image/jpeg"
    };
    Ok((
        [
            (header::CONTENT_TYPE, mime),
            (
                header::CACHE_CONTROL,
                "private, max-age=31536000, immutable",
            ),
        ],
        bytes,
    )
        .into_response())
}
async fn export(State(s): State<Arc<AppState>>) -> Result<Response, WebError> {
    let conn = s.db.conn();
    let rows = entries::list(&conn, "", "~", true)?;
    let mut csv = String::from(
        "\u{feff}时间,类型,金额,一级分类,二级分类,账户,转入账户,商家,备注,标签,来源,ID\r\n",
    );
    let names = |table: &str, id: Option<i64>| -> Result<String, WebError> {
        Ok(match id {
            Some(id) => conn.query_row(
                &format!("SELECT name FROM {table} WHERE id=?1"),
                [id],
                |r| r.get(0),
            )?,
            None => String::new(),
        })
    };
    for e in rows {
        let v = e.input;
        let (top, sub) = if let Some(id) = v.category_id {
            conn.query_row(
                "SELECT coalesce(p.name, c.name),
                        CASE WHEN p.id IS NULL THEN '' ELSE c.name END
                 FROM categories c LEFT JOIN categories p ON p.id=c.parent_id WHERE c.id=?1",
                [id],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )?
        } else {
            (String::new(), String::new())
        };
        let at = chrono::DateTime::parse_from_rfc3339(&v.occurred_at)
            .map_err(internal)?
            .with_timezone(&flow::tz())
            .format("%Y-%m-%d %H:%M")
            .to_string();
        let kind = match v.kind {
            EntryKind::Expense => "支出",
            EntryKind::Income => "收入",
            EntryKind::Transfer => "转账",
            EntryKind::Adjust => "平账",
        };
        let amount = format!(
            "{}{}.{:02}",
            if v.amount_cents < 0 {
                "-"
            } else if v.kind == EntryKind::Adjust {
                "+"
            } else {
                ""
            },
            v.amount_cents.unsigned_abs() / 100,
            v.amount_cents.unsigned_abs() % 100
        );
        let tags = v
            .tag_ids
            .iter()
            .map(|id| names("tags", Some(*id)))
            .collect::<Result<Vec<_>, _>>()?
            .join("、");
        let fields = [
            at,
            kind.into(),
            amount,
            top,
            sub,
            names("accounts", Some(v.account_id))?,
            names("accounts", v.to_account_id)?,
            v.merchant.unwrap_or_default(),
            v.note.unwrap_or_default(),
            tags,
            if e.recognition_id.is_some() {
                "截图"
            } else {
                "手记"
            }
            .into(),
            e.id,
        ];
        csv.push_str(
            &fields
                .iter()
                .map(|s| format!("\"{}\"", s.replace('"', "\"\"")))
                .collect::<Vec<_>>()
                .join(","),
        );
        csv.push_str("\r\n");
    }
    let filename = format!(
        "attachment; filename=\"micjz-{}.csv\"",
        flow::now().format("%Y%m%d")
    );
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (header::CONTENT_DISPOSITION, filename.as_str()),
        ],
        csv,
    )
        .into_response())
}

async fn update_status(State(s): State<Arc<AppState>>) -> Response {
    let no_store = [(header::CACHE_CONTROL, "no-store")];
    match &s.updater {
        None => (no_store, Json(json!({"state": "unavailable"}))).into_response(),
        Some(u) => u.view(|t| (no_store, Json(t)).into_response()),
    }
}
async fn start_update(State(s): State<Arc<AppState>>) -> Result<StatusCode, WebError> {
    let updater = s.updater.as_ref().ok_or_else(|| {
        WebError(
            StatusCode::SERVICE_UNAVAILABLE,
            "请用 ./build.sh start 启动服务".into(),
        )
    })?;
    if !updater.start() {
        return Err(WebError(StatusCode::CONFLICT, "已有更新在进行".into()));
    }
    Ok(StatusCode::ACCEPTED)
}
