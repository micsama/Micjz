//! 接口功能测试：真实监听端口，mock 识别 + 临时数据库，按快捷指令的方式发请求。

use serde_json::{Value, json};

use super::*;
use crate::recognize::Recognizer;

const GET: reqwest::Method = reqwest::Method::GET;
const POST: reqwest::Method = reqwest::Method::POST;
const PUT: reqwest::Method = reqwest::Method::PUT;
const PATCH: reqwest::Method = reqwest::Method::PATCH;
const DELETE: reqwest::Method = reqwest::Method::DELETE;

const TOKEN: &str = "test-token";
const PNG: &[u8] = include_bytes!("test.png");

struct Server {
    base: String,
    db_path: std::path::PathBuf,
    images_dir: std::path::PathBuf,
    http: reqwest::Client,
    cookie: String,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.db_path);
        let _ = std::fs::remove_dir_all(&self.images_dir);
    }
}

async fn server() -> Server {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let dir = std::env::temp_dir().join(format!("micjz-test-{}", uuid::Uuid::now_v7()));
    let (db_path, images_dir) = (dir.with_extension("db"), dir.join("images"));
    let db = Db::open(&db_path).unwrap();
    let admin = crate::auth::Admin::for_password("test-password").unwrap();
    crate::auth::store(&db.conn(), &admin).unwrap();
    let cookie = format!(
        "micjz_session={}",
        admin.session(chrono::Utc::now().timestamp_millis())
    );
    let state = AppState {
        token: TOKEN.to_owned(),
        login_limit: Default::default(),
        recognizer: Recognizer::Mock,
        db,
        images_dir: images_dir.clone(),
        updater: None,
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, router(Arc::new(state))).await });
    Server {
        base,
        db_path,
        images_dir,
        http: reqwest::Client::new(),
        cookie,
    }
}

/// 手写 multipart（reqwest 未开 multipart feature）。
fn multipart(name: &str, data: &[u8]) -> (String, Vec<u8>) {
    let boundary = "micjzboundary";
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"; filename=\"a.png\"\r\n\
         Content-Type: image/png\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(data);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

impl Server {
    async fn send(&self, req: reqwest::RequestBuilder) -> (u16, Value) {
        let resp = req.send().await.unwrap();
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap();
        let body = serde_json::from_str(&text)
            .unwrap_or_else(|_| panic!("响应不是 JSON（{status}）：{text}"));
        (status, body)
    }

    async fn recognize(&self, field: &str, data: &[u8]) -> (u16, Value) {
        let (ct, body) = multipart(field, data);
        self.send(
            self.http
                .post(format!("{}/recognize", self.base))
                .bearer_auth(TOKEN)
                .header("content-type", ct)
                .body(body),
        )
        .await
    }

    async fn step(&self, reply: &Value, value: &str) -> (u16, Value) {
        self.send(
            self.http
                .post(format!("{}/step", self.base))
                .bearer_auth(TOKEN)
                .json(&json!({ "draft": reply["draft"], "field": reply["field"], "value": value })),
        )
        .await
    }

    fn count(&self, sql: &str) -> i64 {
        rusqlite::Connection::open(&self.db_path)
            .unwrap()
            .query_row(sql, [], |r| r.get(0))
            .unwrap()
    }
}

/// 快捷指令依赖：每个响应都含全部字段。
fn assert_shape(v: &Value) {
    for key in [
        "type", "summary", "draft", "field", "options", "default", "message",
    ] {
        assert!(v.get(key).is_some(), "缺少字段 {key}：{v}");
    }
}

fn assert_done(v: &Value, contains: &str) {
    assert_shape(v);
    assert_eq!(v["type"], "done", "{v}");
    let msg = v["message"].as_str().unwrap();
    assert!(msg.contains(contains), "message「{msg}」不含「{contains}」");
}

#[tokio::test]
async fn recognize_then_confirm_saves_once() {
    let s = server().await;
    let (status, menu) = s.recognize("image", PNG).await;
    assert_eq!(status, 200);
    assert_shape(&menu);
    assert_eq!(menu["type"], "choose");
    assert_eq!(menu["field"], "action");
    assert_eq!(
        menu["options"].as_array().unwrap().last().unwrap(),
        "✅ 确认记账"
    );
    assert_eq!(menu["summary"], "");

    let (status, done) = s.step(&menu, "✅ 确认记账").await;
    assert_eq!(status, 200);
    assert_done(&done, "已记账");
    // 重复确认同一草稿：仍是「已记账」，库里只有一条。
    let (_, again) = s.step(&menu, "✅ 确认记账").await;
    assert_done(&again, "已记账");
    assert_eq!(s.count("SELECT COUNT(*) FROM entries"), 1);
    assert_eq!(
        s.count(
            "SELECT count(*) FROM requests r JOIN entries e ON e.recognition_id = r.draft_id
             WHERE r.status = 200"
        ),
        3,
        "识别 + 两次确认，都按 draft_id 记下"
    );
    assert_eq!(
        s.count("SELECT count(*) FROM requests WHERE field = 'action' AND value = '✅ 确认记账'"),
        2
    );
    assert_eq!(s.count("SELECT count(*) FROM entries e JOIN recognitions r ON e.recognition_id = r.draft_id WHERE r.prompt != '' AND r.parameters LIKE '%none%'"), 1);
    let conn = rusqlite::Connection::open(&s.db_path).unwrap();
    let path: String = conn
        .query_row("SELECT image_path FROM recognitions LIMIT 1", [], |r| {
            r.get(0)
        })
        .unwrap();
    // 图片由后台任务写入。
    for _ in 0..50 {
        if std::path::Path::new(&path).exists() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(std::path::Path::new(&path).exists(), "图片已留存：{path}");
}

/// 模型说不是支付页：每一屏顶部提醒原因，字段照留，仍可确认。
#[tokio::test]
async fn rejected_recognition_reminds_but_still_confirms() {
    let s = server().await;
    let (_, menu) = s.recognize("image", PNG).await;
    rusqlite::Connection::open(&s.db_path)
        .unwrap()
        .execute(
            "UPDATE recognitions SET outcome = 'rejected', reason = '这是聊天记录'",
            [],
        )
        .unwrap();
    let warning = "⚠ 不像支付页：这是聊天记录";
    let (_, accounts) = s.step(&menu, menu["options"][3].as_str().unwrap()).await;
    assert!(accounts["summary"].as_str().unwrap().starts_with(warning));
    let (_, menu) = s.step(&accounts, "微信").await;
    assert_eq!(menu["field"], "action");
    assert_eq!(menu["summary"], warning);
    let (_, done) = s.step(&menu, "✅ 确认记账").await;
    assert_done(&done, "已记账");
}

#[tokio::test]
async fn edit_category_and_tags_before_confirm() {
    let s = server().await;
    let (_, menu) = s.recognize("image", PNG).await;
    let (_, cats) = s.step(&menu, menu["options"][2].as_str().unwrap()).await;
    assert_eq!(cats["field"], "category");
    let (_, subs) = s.step(&cats, "🚇 交通").await;
    assert_eq!(subs["field"], "subcategory");
    let (_, menu) = s.step(&subs, "🚕 打车").await;
    assert!(
        menu["options"][2]
            .as_str()
            .unwrap()
            .contains("🚇 交通 / 🚕 打车")
    );
    let (_, tags) = s.step(&menu, menu["options"][6].as_str().unwrap()).await;
    assert_eq!(tags["type"], "choose_many");
    let (_, menu) = s.step(&tags, "恋爱").await;
    let (_, done) = s.step(&menu, "✅ 确认记账").await;
    assert_done(&done, "🚇 交通/🚕 打车");
    assert_eq!(
        s.count(
            "SELECT COUNT(*) FROM entries e
             JOIN categories c ON c.id = e.category_id AND c.key = 'taxi'
             JOIN entry_tags et ON et.entry_id = e.id
             JOIN tags t ON t.id = et.tag_id AND t.name = '恋爱'"
        ),
        1,
        "账目按 id 引用二级分类和标签"
    );
}

#[tokio::test]
async fn errors_are_json_done() {
    let s = server().await;
    let url = |p: &str| format!("{}{p}", s.base);

    let (status, v) = s.send(s.http.post(url("/recognize"))).await;
    assert_eq!(status, 401);
    assert_done(&v, "鉴权失败");

    let (status, v) = s
        .send(s.http.post(url("/recognize")).bearer_auth("wrong"))
        .await;
    assert_eq!(status, 401);
    assert_done(&v, "鉴权失败");

    let (status, v) = s
        .send(s.http.post(url("/recognize")).bearer_auth(TOKEN).body("x"))
        .await;
    assert_eq!(status, 400);
    assert_done(&v, "请求无效");

    let (status, v) = s.recognize("file", PNG).await;
    assert_eq!(status, 400);
    assert_done(&v, "缺少 image");

    let (status, v) = s.recognize("image", b"not an image").await;
    assert_eq!(status, 400);
    assert_done(&v, "图片格式不支持");

    let (status, v) = s
        .send(s.http.post(url("/step")).bearer_auth(TOKEN).body("{"))
        .await;
    assert_eq!(status, 400);
    assert_done(&v, "请求无效");

    let bad = json!({ "draft": "garbage", "field": "action" });
    let (status, v) = s.step(&bad, "确认").await;
    assert_eq!(status, 400);
    assert_done(&v, "请求无效");

    // 格式正确但不是本服务发出的草稿。
    let stranger = serde_json::to_value(crate::flow::menu(
        &crate::flow::Draft::blank(crate::flow::now(), &crate::catalog::defaults()),
        crate::flow::now(),
        &crate::catalog::defaults(),
    ))
    .unwrap();
    let (status, v) = s.step(&stranger, "✅ 确认记账").await;
    assert_eq!(status, 400);
    assert_done(&v, "草稿不存在");

    let (_, menu) = s.recognize("image", PNG).await;
    let (status, v) = s.step(&menu, "不存在的选项").await;
    assert_eq!(status, 400);
    assert_done(&v, "请求无效");

    let (status, v) = s.send(s.http.get(url("/nope")).bearer_auth(TOKEN)).await;
    assert_eq!(status, 404);
    assert_done(&v, "接口不存在");

    let (status, v) = s.send(s.http.get(url("/step")).bearer_auth(TOKEN)).await;
    assert_eq!(status, 404);
    assert_done(&v, "接口不存在");

    assert_eq!(s.count("SELECT COUNT(*) FROM entries"), 0);
    assert_eq!(
        s.count("SELECT count(*) FROM requests WHERE status = 401"),
        0,
        "鉴权失败不进库"
    );
    assert_eq!(
        s.count("SELECT count(*) FROM requests WHERE status = 400 AND error LIKE '请求无效%'"),
        7
    );
    assert_eq!(
        s.count(
            "SELECT count(*) FROM requests WHERE method = 'GET' AND path = '/step' AND status = 404"
        ),
        1
    );
}

impl Server {
    fn api(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        self.http
            .request(method, format!("{}/api/{path}", self.base))
            .header("Cookie", &self.cookie)
    }
    async fn web(&self, method: reqwest::Method, path: &str, value: Value) -> (u16, Value) {
        self.send(self.api(method, path).json(&value)).await
    }
    async fn catalog(&self) -> Value {
        let (status, v) = self.send(self.api(reqwest::Method::GET, "catalog")).await;
        assert_eq!(status, 200);
        v
    }
}
fn expense(catalog: &Value) -> Value {
    json!({"id":uuid::Uuid::now_v7().to_string(),"kind":"expense","amount_cents":3250,"category_id":catalog["categories"].as_array().unwrap().iter().find(|c|c["kind"]=="expense").unwrap()["id"],"account_id":catalog["accounts"][0]["id"],"to_account_id":null,"merchant":"  商家  ","occurred_at":"2026-11-17T12:30:00.123+08:00","note":" 午饭 ","tag_ids":[catalog["tags"][0]["id"],catalog["tags"][0]["id"]]})
}

#[tokio::test]
async fn web_login_password_and_separate_shortcut_auth() {
    let s = server().await;
    let login = |username: &str, password: &str| {
        s.http
            .post(format!("{}/api/login", s.base))
            .json(&json!({"username":username,"password":password}))
    };
    let (status, v) = s
        .send(
            s.http
                .get(format!("{}/api/catalog", s.base))
                .bearer_auth(TOKEN),
        )
        .await;
    assert_eq!(status, 401);
    assert_eq!(v, json!({"error":"未登录或登录已过期"}));
    assert_eq!(s.send(login("dzmfg", "wrong")).await.0, 401);
    let resp = login("dzmfg", "test-password")
        .header("X-Forwarded-Proto", "https")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);
    let cookie = resp.headers()["set-cookie"].to_str().unwrap();
    for flag in [
        "micjz_session=",
        "HttpOnly",
        "SameSite=Strict",
        "Max-Age=2592000",
        "Secure",
    ] {
        assert!(cookie.contains(flag));
    }
    assert_eq!(
        s.api(POST, "logout")
            .header("Sec-Fetch-Site", "same-site")
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    assert_eq!(
        s.api(POST, "logout")
            .header("Origin", "https://other.example")
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    assert_eq!(
        s.web(
            PUT,
            "password",
            json!({"current":"wrong","new":"new-password"})
        )
        .await
        .0,
        400
    );
    let changed = s
        .api(PUT, "password")
        .json(&json!({"current":"test-password","new":"new-password"}))
        .send()
        .await
        .unwrap();
    assert_eq!(changed.status(), 204);
    let fresh = changed.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap();
    assert_eq!(s.send(s.api(GET, "catalog")).await.0, 401);
    let resp = s
        .http
        .get(format!("{}/api/catalog", s.base))
        .header("Cookie", fresh)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(s.send(login("dzmfg", "test-password")).await.0, 401);
    assert_eq!(
        login("dzmfg", "new-password")
            .send()
            .await
            .unwrap()
            .status(),
        204
    );
    // 密码写操作只记录请求元数据。
    assert_eq!(s.count("SELECT count(*) FROM requests WHERE path IN ('/api/login','/api/password') AND (value IS NOT NULL OR field IS NOT NULL)"),0);
    // 密码找回、续期、限速分别验证模块和真实路由的连接。
    let db = Db::open(&s.db_path).unwrap();
    let admin = crate::auth::load(&db.conn()).unwrap().unwrap();
    let old = admin.session(chrono::Utc::now().timestamp_millis() - 86_400_001);
    let resp = s
        .http
        .get(format!("{}/api/settings", s.base))
        .header("Cookie", format!("micjz_session={old}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert!(resp.headers().contains_key("set-cookie"));
    let new = crate::auth::Admin::for_password("reset-password").unwrap();
    crate::auth::store(&db.conn(), &new).unwrap();
    assert_eq!(
        s.http
            .get(format!("{}/api/catalog", s.base))
            .header("Cookie", fresh)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    // 已有两次登录失败，再失败八次后，第十一次尝试被限速。
    for _ in 0..8 {
        let status = login("dzmfg", "wrong").send().await.unwrap().status();
        assert_eq!(status, 401);
    }
    assert_eq!(s.send(login("dzmfg", "reset-password")).await.0, 429);
    db.conn().execute("DELETE FROM web_admin", []).unwrap();
    let s2 = server().await;
    rusqlite::Connection::open(&s2.db_path)
        .unwrap()
        .execute("DELETE FROM web_admin", [])
        .unwrap();
    assert_eq!(
        s2.web(
            POST,
            "login",
            json!({"username":"dzmfg","password":"test-password"})
        )
        .await
        .0,
        409
    );
    for path in ["/", "/icon.svg"] {
        let resp = s
            .http
            .get(format!("{}{path}", s.base))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        assert_eq!(resp.headers()["cache-control"], "no-cache");
    }
}

#[tokio::test]
async fn web_entries_validation_patch_and_balances() {
    let s = server().await;
    let c = s.catalog().await;
    let input = expense(&c);
    let (status, e) = s.web(POST, "entries", input.clone()).await;
    assert_eq!(status, 201, "{e}");
    assert_eq!(e["id"], input["id"], "id 用网页给的");
    let mut retry = input.clone();
    retry["amount_cents"] = json!(1);
    let (status, again) = s.web(POST, "entries", retry).await;
    assert_eq!(
        (status, &again),
        (200, &e),
        "同一 id 重试返回原来那笔，不比较内容"
    );
    for bad in [
        json!("nope"),
        json!("550e8400-e29b-41d4-a716-446655440000"),
        Value::Null,
    ] {
        let mut v = input.clone();
        v["id"] = bad;
        assert_eq!(s.web(POST, "entries", v).await.0, 400);
    }
    assert_eq!(e["merchant"], "商家");
    assert_eq!(e["note"], "午饭");
    assert_eq!(e["occurred_at"], "2026-11-17T04:30:00Z");
    assert_eq!(e["tag_ids"].as_array().unwrap().len(), 1);
    assert!(e["recognition_id"].is_null());
    assert_eq!(
        uuid::Uuid::parse_str(e["id"].as_str().unwrap())
            .unwrap()
            .get_version_num(),
        7
    );
    let path = format!("entries/{}", e["id"].as_str().unwrap());
    let (status, p) = s.web(PATCH, &path, json!({"note":null})).await;
    assert_eq!(status, 200);
    assert!(p["note"].is_null());
    assert_eq!(p["merchant"], e["merchant"]);
    let (_, p) = s
        .web(PATCH, &path, json!({"merchant":" ","tag_ids":[]}))
        .await;
    assert!(p["merchant"].is_null());
    assert_eq!(p["tag_ids"], json!([]));
    assert_eq!(
        s.web(PATCH, &path, json!({"amount_cents":null})).await.0,
        400
    );
    assert_eq!(s.web(PATCH, &path, json!({"kind":"income"})).await.0, 400);
    let income = c["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["kind"] == "income")
        .unwrap()["id"]
        .clone();
    assert_eq!(
        s.web(PATCH, &path, json!({"category_id":income})).await.0,
        400
    );
    assert_eq!(
        s.web(PATCH, &path, json!({"kind":"income","category_id":income}))
            .await
            .0,
        200
    );
    let mut transfer = input.clone();
    transfer["id"] = json!(uuid::Uuid::now_v7().to_string());
    transfer["kind"] = json!("transfer");
    transfer["category_id"] = Value::Null;
    transfer["to_account_id"] = c["accounts"][0]["id"].clone();
    assert_eq!(s.web(POST, "entries", transfer.clone()).await.0, 400);
    transfer["to_account_id"] = c["accounts"][1]["id"].clone();
    let (status, t) = s.web(POST, "entries", transfer).await;
    assert_eq!(status, 201, "{t}");
    assert_eq!(
        s.web(
            PATCH,
            &format!("entries/{}", t["id"].as_str().unwrap()),
            json!({"kind":"expense","category_id":input["category_id"]})
        )
        .await
        .0,
        400
    );
    let (_, b) = s.send(s.api(GET, "balances")).await;
    assert_eq!(b["balances"][0]["balance_cents"], 0);
    assert_eq!(b["balances"][1]["balance_cents"], 3250);
    let (_, list) = s
        .send(s.api(
            GET,
            "entries?from=2026-01-01T00:00:00Z&to=2027-01-01T00:00:00Z",
        ))
        .await;
    assert_eq!(list["entries"].as_array().unwrap().len(), 2);
    assert_eq!(list["entries"][0]["id"], t["id"]);
    assert_eq!(list["earliest"], e["occurred_at"]);
    let (_, empty) = s
        .send(s.api(
            GET,
            "entries?from=2025-01-01T00:00:00Z&to=2026-01-01T00:00:00Z",
        ))
        .await;
    assert_eq!(empty["entries"], json!([]));
    assert_eq!(empty["earliest"], e["occurred_at"]);
    for path in [
        "entries",
        "entries?from=x&to=y",
        "entries?from=2026-01-01T00:00:00Z&to=2025-01-01T00:00:00Z",
    ] {
        assert_eq!(s.send(s.api(GET, path)).await.0, 400);
    }
    assert_eq!(s.api(DELETE, &path).send().await.unwrap().status(), 204);
    assert_eq!(s.send(s.api(DELETE, &path)).await.0, 404);
    assert_eq!(s.web(PATCH, &path, json!({"note":"x"})).await.0, 404);
}

#[tokio::test]
async fn web_accounts_adjust_settings() {
    let s = server().await;
    let c = s.catalog().await;
    let account =
        json!({"name":" 新卡 ","brand":" 招商银行 ","card":{"type":"debit","number":"1234 5678"}});
    let (status, a) = s.web(POST, "accounts", account.clone()).await;
    assert_eq!(status, 201, "{a}");
    assert_eq!(a["name"], "新卡");
    assert_eq!(a["brand"], "招商银行");
    assert_eq!(a["card"]["number"], "12345678");
    assert_eq!(s.web(POST, "accounts", account.clone()).await.0, 409);
    let path = format!("accounts/{}", a["id"].as_i64().unwrap());
    let (status, m) = s
        .web(POST, &format!("{path}/move"), json!({"to":-99}))
        .await;
    assert_eq!(status, 200);
    assert_eq!(m["accounts"][0]["id"], a["id"]);
    let (_, m) = s
        .web(POST, &format!("{path}/move"), json!({"to":999}))
        .await;
    assert_eq!(
        m["accounts"].as_array().unwrap().last().unwrap()["id"],
        a["id"]
    );
    let mut replacement = account.clone();
    replacement["name"] = json!("工资卡");
    assert_eq!(s.web(PUT, &path, replacement).await.0, 200);
    let cat = Db::open(&s.db_path).unwrap().catalog().unwrap();
    assert!(
        cat.account_lines()
            .contains("工资卡（招商银行 储蓄卡 尾号 5678）")
    );
    assert!(!cat.account_lines().contains("12345678"));
    let (status, adj) = s
        .web(
            POST,
            &format!("{path}/adjust"),
            json!({"balance_cents":-1200}),
        )
        .await;
    assert_eq!(status, 200, "{adj}");
    assert_eq!(adj["entry"]["amount_cents"], -1200);
    let (_, zero) = s
        .web(
            POST,
            &format!("{path}/adjust"),
            json!({"balance_cents":-1200}),
        )
        .await;
    assert!(zero["entry"].is_null());
    assert_eq!(s.send(s.api(DELETE, &path)).await.0, 409);
    let apath = format!("entries/{}", adj["entry"]["id"].as_str().unwrap());
    assert_eq!(
        s.web(PATCH, &apath, json!({"amount_cents":-1300,"note":"改差额"}))
            .await
            .0,
        200
    );
    assert_eq!(s.web(PATCH, &apath, json!({"amount_cents":0})).await.0, 400);
    let conn = rusqlite::Connection::open(&s.db_path).unwrap();
    conn.execute("DELETE FROM entries", []).unwrap();
    assert_eq!(s.api(DELETE, &path).send().await.unwrap().status(), 204);
    let first = c["accounts"][0]["id"].as_i64().unwrap();
    let second = c["accounts"][1]["id"].as_i64().unwrap();
    assert_eq!(
        s.api(DELETE, &format!("accounts/{second}"))
            .send()
            .await
            .unwrap()
            .status(),
        204
    );
    assert_eq!(
        s.send(s.api(DELETE, &format!("accounts/{first}"))).await.0,
        409
    );
    assert_eq!(s.send(s.api(DELETE, "accounts/99999")).await.0, 404);
    assert_eq!(
        s.web(POST, "accounts/99999/adjust", json!({"balance_cents":0}))
            .await
            .0,
        404
    );
    assert_eq!(s.send(s.api(DELETE, "accounts/nope")).await.0, 400);
    let (status, settings) = s
        .web(PUT, "settings", json!({"cycle_day":25,"budget_cents":null}))
        .await;
    assert_eq!(status, 200);
    assert_eq!(settings["cycle_day"], 25);
    for (day, status) in [
        (-25, 200),
        (-2, 200),
        (-1, 400),
        (0, 400),
        (29, 400),
        (-29, 400),
    ] {
        let v = json!({"cycle_day":day,"budget_cents":null});
        assert_eq!(s.web(PUT, "settings", v).await.0, status, "{day}");
    }
    assert!(settings["budget_cents"].is_null());
    assert_eq!(settings["entry_count"], 0);

    // 没经 build.sh 启动：更新不可用。
    assert_eq!(
        s.web(GET, "update", Value::Null).await,
        (200, json!({"state":"unavailable"}))
    );
    assert_eq!(s.web(POST, "update", Value::Null).await.0, 503);
}

#[tokio::test]
async fn web_recognition_export_and_shortcut_share_entries() {
    let s = server().await;
    let (_, menu) = s.recognize("image", PNG).await;
    let (_, done) = s.step(&menu, "✅ 确认记账").await;
    assert_done(&done, "已记账");
    let (_, list) = s
        .send(s.api(
            GET,
            "entries?from=2000-01-01T00:00:00Z&to=2100-01-01T00:00:00Z",
        ))
        .await;
    let entry = &list["entries"][0];
    let rid = entry["recognition_id"].as_str().unwrap();
    let (status, r) = s.send(s.api(GET, &format!("recognitions/{rid}"))).await;
    assert_eq!(status, 200);
    assert_eq!(r["first"]["account_id"], entry["account_id"]);
    assert_eq!(r["first"]["amount_cents"], entry["amount_cents"]);
    for _ in 0..50 {
        if s.api(GET, &format!("recognitions/{rid}/image"))
            .send()
            .await
            .unwrap()
            .status()
            == 200
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let img = s
        .api(GET, &format!("recognitions/{rid}/image"))
        .send()
        .await
        .unwrap();
    assert_eq!(img.status(), 200);
    assert_eq!(img.headers()["content-type"], "image/png");
    assert_eq!(
        img.headers()["cache-control"],
        "private, max-age=31536000, immutable"
    );
    let path = format!("entries/{}", entry["id"].as_str().unwrap());
    s.web(PATCH, &path, json!({"note":"逗号,引号\"和\n换行"}))
        .await;
    let csv = s.api(GET, "export.csv").send().await.unwrap();
    assert_eq!(csv.headers()["content-type"], "text/csv; charset=utf-8");
    assert!(
        csv.headers()["content-disposition"]
            .to_str()
            .unwrap()
            .contains("micjz-")
    );
    let csv = csv.text().await.unwrap();
    assert!(csv.starts_with('\u{feff}'));
    assert!(csv.contains("\"逗号,引号\"\"和\n换行\""));
    assert!(csv.contains("截图"));
    s.api(DELETE, &path).send().await.unwrap();
    assert_eq!(
        s.send(s.api(GET, &format!("recognitions/{rid}"))).await.0,
        200
    );
    let conn = rusqlite::Connection::open(&s.db_path).unwrap();
    conn.execute(
        "UPDATE accounts SET name='已改名' WHERE id=?1",
        [entry["account_id"].as_i64().unwrap()],
    )
    .unwrap();
    let (_, r) = s.send(s.api(GET, &format!("recognitions/{rid}"))).await;
    assert_eq!(
        r["first"]["account_id"], entry["account_id"],
        "留存认 id，账户改名不影响"
    );
    assert_eq!(s.send(s.api(GET, "recognitions/missing")).await.0, 404);
    assert_eq!(
        s.send(s.api(GET, "recognitions/missing/image")).await.0,
        404
    );
    conn.execute("UPDATE recognitions SET image_path='/nonexistent'", [])
        .unwrap();
    assert_eq!(
        s.send(s.api(GET, &format!("recognitions/{rid}/image")))
            .await
            .0,
        404
    );
}

#[tokio::test]
async fn web_categories_and_tags() {
    let s = server().await;
    let find = |list: &Value, id: &Value| {
        list["categories"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == *id)
            .cloned()
            .unwrap()
    };
    let (status, list) = s
        .web(
            POST,
            "categories",
            json!({"kind":"expense","parent_id":null,"name":" 📚 学习 ","key":"study"}),
        )
        .await;
    assert_eq!(status, 201, "{list}");
    let top = list["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["key"] == "study")
        .unwrap()["id"]
        .clone();
    assert_eq!(find(&list, &top)["name"], "📚 学习");
    let (status, list) = s
        .web(
            POST,
            "categories",
            json!({"kind":"expense","parent_id":top,"name":"书","key":""}),
        )
        .await;
    assert_eq!(status, 201);
    let book = list["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "书")
        .unwrap()
        .clone();
    assert_eq!(book["key"], format!("c{}", book["id"]), "没给 key 就生成");
    for (body, status) in [
        (json!({"kind":"expense","parent_id":top,"name":"书"}), 409),
        (
            json!({"kind":"expense","parent_id":null,"name":"x","key":"study"}),
            409,
        ),
        (
            json!({"kind":"expense","parent_id":null,"name":"x","key":"Bad"}),
            400,
        ),
        (json!({"kind":"expense","parent_id":null,"name":"a/b"}), 400),
        (json!({"kind":"income","parent_id":top,"name":"x"}), 409),
        (
            json!({"kind":"expense","parent_id":book["id"],"name":"x"}),
            409,
        ),
        (json!({"parent_id":null,"name":"x"}), 400),
    ] {
        assert_eq!(
            s.web(POST, "categories", body.clone()).await.0,
            status,
            "{body}"
        );
    }

    let path = format!("categories/{}", book["id"]);
    let (status, list) = s
        .web(
            PUT,
            &path,
            json!({"parent_id":book["parent_id"],"name":"📖 书","key":"books"}),
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(find(&list, &book["id"])["key"], "books");
    let (status, list) = s
        .web(PUT, &path, json!({"parent_id":null,"name":"📖 书"}))
        .await;
    assert_eq!(status, 200);
    assert!(find(&list, &book["id"])["parent_id"].is_null(), "挪成一级");
    assert_eq!(find(&list, &book["id"])["key"], "books", "key 为空保持原值");
    let (_, list) = s.web(POST, &format!("{path}/move"), json!({"to":0})).await;
    assert_eq!(
        list["categories"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["kind"] == "expense")
            .unwrap()["id"],
        book["id"]
    );
    s.web(PUT, &path, json!({"parent_id":top,"name":"📖 书"}))
        .await;
    let other = list["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["key"] == "other")
        .unwrap()["id"]
        .clone();
    assert_eq!(
        s.web(
            PUT,
            &format!("categories/{top}"),
            json!({"parent_id":other,"name":"📚 学习"})
        )
        .await
        .0,
        409,
        "带着二级不能挪"
    );
    let opath = format!("categories/{other}");
    assert_eq!(
        s.web(
            PUT,
            &opath,
            json!({"parent_id":null,"name":"杂项","key":"misc"})
        )
        .await
        .0,
        409
    );
    assert_eq!(
        s.web(PUT, &opath, json!({"parent_id":null,"name":"杂项"}))
            .await
            .0,
        200
    );
    assert_eq!(s.send(s.api(DELETE, &opath)).await.0, 409);
    assert_eq!(
        s.web(
            PUT,
            "categories/99999",
            json!({"parent_id":null,"name":"x"})
        )
        .await
        .0,
        404
    );

    // 有账目在用不能删；有二级先删二级。
    let c = s.catalog().await;
    let mut e = expense(&c);
    e["category_id"] = book["id"].clone();
    e["tag_ids"] = json!([]);
    assert_eq!(s.web(POST, "entries", e.clone()).await.0, 201);
    assert_eq!(s.send(s.api(DELETE, &path)).await.0, 409);
    assert_eq!(
        s.send(s.api(DELETE, &format!("categories/{top}"))).await.0,
        409
    );
    s.api(DELETE, &format!("entries/{}", e["id"].as_str().unwrap()))
        .send()
        .await
        .unwrap();
    assert_eq!(s.api(DELETE, &path).send().await.unwrap().status(), 204);
    assert_eq!(
        s.api(DELETE, &format!("categories/{top}"))
            .send()
            .await
            .unwrap()
            .status(),
        204
    );

    let (status, tags) = s.web(POST, "tags", json!({"name":" 出差 "})).await;
    assert_eq!(status, 201);
    let tag = tags["tags"].as_array().unwrap().last().unwrap().clone();
    assert_eq!(tag["name"], "出差");
    assert_eq!(s.web(POST, "tags", json!({"name":"出差"})).await.0, 409);
    let tpath = format!("tags/{}", tag["id"]);
    let (status, tags) = s.web(PUT, &tpath, json!({"name":"公干"})).await;
    assert_eq!(status, 200);
    assert!(
        tags["tags"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["name"] == "公干")
    );
    let mut e = expense(&c);
    e["tag_ids"] = json!([tag["id"]]);
    assert_eq!(s.web(POST, "entries", e).await.0, 201);
    assert_eq!(s.send(s.api(DELETE, &tpath)).await.0, 409);
    assert_eq!(s.web(PUT, "tags/99999", json!({"name":"x"})).await.0, 404);
}

#[tokio::test]
async fn web_performance_accepts_detailed_timing() {
    let s = server().await;
    let timing = json!({"start_ms":0,"duration_ms":180,"ttfb_ms":170,
        "transfer_bytes":900,"encoded_bytes":600,"decoded_bytes":986});
    let mut navigation = timing.clone();
    navigation.as_object_mut().unwrap().extend(
        json!({"dns_ms":0,"connect_ms":0,"server_wait_ms":90,"request_at_ms":80})
            .as_object()
            .unwrap()
            .clone(),
    );
    let mut resource = timing.clone();
    resource["label"] = json!("entries");
    let report = json!({"kind":"startup","outcome":"ready","standalone":true,"total_ms":300,"paint_ms":350,
        "before_boot_ms":200,"navigation":navigation,"resource_count":1,"resources":[resource]});
    let resp = s
        .api(POST, "web-performance")
        .json(&report)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204, "{}", resp.text().await.unwrap());
    let mut bad = report;
    bad["kind"] = json!("maybe");
    assert_eq!(s.web(POST, "web-performance", bad).await.0, 400);
}

#[tokio::test]
async fn responses_compress_when_client_accepts_gzip() {
    let s = server().await;
    let catalog = s
        .api(reqwest::Method::GET, "catalog")
        .header("Accept-Encoding", "gzip")
        .send()
        .await
        .unwrap();
    assert_eq!(catalog.headers()["content-encoding"], "gzip");
}
