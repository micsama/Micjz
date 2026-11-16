//! 接口功能测试：真实监听端口，mock 识别 + 临时数据库，按快捷指令的方式发请求。

use serde_json::{Value, json};

use super::*;
use crate::recognize::Recognizer;

const TOKEN: &str = "test-token";
const PNG: &[u8] = include_bytes!("test.png");

struct Server {
    base: String,
    db_path: std::path::PathBuf,
    http: reqwest::Client,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.db_path);
    }
}

async fn server() -> Server {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let db_path = std::env::temp_dir().join(format!("micjz-test-{}.db", uuid::Uuid::now_v7()));
    let state = AppState {
        token: TOKEN.to_owned(),
        recognizer: Recognizer::Mock,
        db: Db::open(&db_path).unwrap(),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, router(Arc::new(state))).await });
    Server {
        base,
        db_path,
        http: reqwest::Client::new(),
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
    assert_eq!(s.count("SELECT count(*) FROM entries e JOIN recognitions r ON e.draft_id=r.draft_id WHERE r.prompt != '' AND r.parameters LIKE '%none%' AND r.origin IS NOT NULL AND r.confirmed_at IS NOT NULL"), 1);
    let conn = rusqlite::Connection::open(&s.db_path).unwrap();
    let path: String = conn
        .query_row("SELECT image_path FROM recognitions LIMIT 1", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert!(std::path::Path::new(&path).exists());
}

#[tokio::test]
async fn edit_category_and_tags_before_confirm() {
    let s = server().await;
    let (_, menu) = s.recognize("image", PNG).await;
    let (_, cats) = s.step(&menu, menu["options"][2].as_str().unwrap()).await;
    assert_eq!(cats["field"], "category");
    let (_, subs) = s.step(&cats, "🗂️ 交通").await;
    assert_eq!(subs["field"], "subcategory");
    let (_, menu) = s.step(&subs, "📂 打车").await;
    assert!(menu["options"][2].as_str().unwrap().contains("交通 / 打车"));
    let (_, tags) = s.step(&menu, menu["options"][6].as_str().unwrap()).await;
    assert_eq!(tags["type"], "choose_many");
    let (_, menu) = s.step(&tags, "🏷️ 恋爱").await;
    let (_, done) = s.step(&menu, "✅ 确认记账").await;
    assert_done(&done, "交通/打车");
    assert_eq!(
        s.count("SELECT COUNT(*) FROM entry_tags WHERE tag = '恋爱'"),
        1
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
    assert_done(&v, "图片无法解码");

    let (status, v) = s
        .send(s.http.post(url("/step")).bearer_auth(TOKEN).body("{"))
        .await;
    assert_eq!(status, 400);
    assert_done(&v, "请求无效");

    let bad = json!({ "draft": "garbage", "field": "action" });
    let (status, v) = s.step(&bad, "确认").await;
    assert_eq!(status, 400);
    assert_done(&v, "请求无效");

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
}
