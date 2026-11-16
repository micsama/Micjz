use chrono::TimeZone;

use super::*;
use crate::catalog;

fn now() -> DateTime<FixedOffset> {
    flow::tz()
        .with_ymd_and_hms(2026, 11, 16, 13, 42, 0)
        .unwrap()
}

fn draft(json: &str) -> Draft {
    build_draft(
        Output::from_json(&serde_json::from_str(json).unwrap()),
        now(),
        &catalog::defaults(),
    )
}

#[test]
fn full_output() {
    let d = draft(
        r#"{"ok":true,"kind":"expense","amount":"32.50","merchant":"瑞幸咖啡","item":"生椰拿铁",
            "account":"微信","occurred_at":"2026-11-16 12:05","category":"coffee"}"#,
    );
    assert_eq!(d.amount_cents, Some(3250));
    assert_eq!(
        (d.category.as_str(), d.subcategory.as_deref()),
        ("🍜 餐饮", Some("☕️ 咖啡饮品"))
    );
    assert_eq!(d.occurred_at.to_rfc3339(), "2026-11-16T12:05:00+08:00");
    assert_eq!(d.note.as_deref(), Some("生椰拿铁"), "商品名进备注");
}

#[test]
fn invalid_fields_become_empty() {
    let d = draft(
        r#"{"ok":true,"amount":"abc","merchant":123,"account":"支付宝",
            "occurred_at":"昨天","category":"nope","item":"  "}"#,
    );
    assert_eq!(d.amount_cents, None, "金额不合规置空，让用户填");
    assert_eq!((d.category.as_str(), d.subcategory), ("✨ 其他", None));
    assert_eq!(d.account, "微信", "不在列表里的账户用默认账户");
    assert_eq!(d.merchant.as_deref(), Some("123"));
    assert_eq!(d.occurred_at, now());
    assert_eq!(d.note, None);
}

#[test]
fn recent_time_uses_now() {
    let d = draft(r#"{"ok":true,"amount":1,"occurred_at":"2026-11-16 13:30"}"#);
    assert_eq!(d.occurred_at, now(), "30 分钟内用识别时刻");
    assert_eq!(d.amount_cents, Some(100), "数字金额也收");
}

#[test]
fn income_key_must_match_kind() {
    let d = draft(r#"{"ok":true,"kind":"income","amount":"5","category":"refund"}"#);
    assert_eq!((d.kind, d.category.as_str()), (Kind::Income, "↩️ 退款"));
    let d = draft(r#"{"ok":true,"kind":"income","amount":"5","category":"coffee"}"#);
    assert_eq!(d.category, "✨ 其他", "支出 key 不能用在收入上");
}

#[test]
fn unparseable_output_fails_whole() {
    assert!(parse_output("not json").is_err());
    let wrap = |content: &str| json!({"choices":[{"message":{"content":content}}]}).to_string();
    assert!(parse_output(&wrap("[1]")).is_err());
    let out = parse_output(&wrap(r#"{"ok":false,"reason":"聊天截图"}"#)).unwrap();
    assert!(!out.ok);
    assert_eq!(out.reason.as_deref(), Some("聊天截图"));
}

#[tokio::test]
async fn mock_always_ok() {
    let r = Recognizer::Mock
        .recognize(
            b"\x89PNG\r\n\x1a\n",
            "image/png",
            now(),
            &catalog::defaults(),
        )
        .await;
    assert_eq!(r.outcome, Outcome::Ok);
    assert!(r.prompt.contains("coffee: 🍜 餐饮/☕️ 咖啡饮品"));
    assert!(
        r.prompt.contains("看不出省略：\n微信\n银行卡"),
        "账户逐行列出"
    );
    assert!(r.parameters.contains("json_object"));
}

#[test]
fn detects_image_type() {
    assert_eq!(mime(b"\x89PNG\r\n\x1a\nxxxx"), Some("image/png"));
    assert_eq!(mime(b"\xff\xd8\xff\xe0"), Some("image/jpeg"));
    assert_eq!(mime(b"GIF89a"), None);
}
