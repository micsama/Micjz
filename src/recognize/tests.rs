use chrono::TimeZone;

use super::*;

fn now() -> DateTime<FixedOffset> {
    flow::tz()
        .with_ymd_and_hms(2026, 11, 16, 13, 42, 0)
        .unwrap()
}

fn output(json: &str) -> Output {
    serde_json::from_str(json).unwrap()
}

#[test]
fn full_output() {
    let d = build_draft(
        output(
            r#"{"ok":true,"reason":null,"kind":"expense","amount":"32.50","merchant":"瑞幸咖啡",
                "account":"微信","occurred_at":"2026-11-16 12:05","category":"coffee"}"#,
        ),
        now(),
    )
    .unwrap();
    assert_eq!(d.amount_cents, 3250);
    assert_eq!(d.subcategory.as_deref(), Some("咖啡饮品"));
    assert_eq!(d.occurred_at.to_rfc3339(), "2026-11-16T12:05:00+08:00");
    assert!(d.warnings.is_empty());
}

#[test]
fn falls_back_with_warnings() {
    let d = build_draft(
        output(
            r#"{"ok":true,"kind":"expense","amount":18.5,"account":"支付宝",
                "category":null}"#,
        ),
        now(),
    )
    .unwrap();
    assert_eq!(d.amount_cents, 1850);
    assert_eq!((d.category.as_str(), d.subcategory), (OTHER, None));
    assert_eq!(d.account, None, "不在列表的账户留空");
    assert_eq!(d.occurred_at, now());
    assert!(d.warnings.is_empty());
}

#[test]
fn rejects_non_payment_and_missing_amount() {
    assert!(matches!(
        build_draft(output(r#"{"ok":false,"reason":"聊天截图"}"#), now()),
        Err(RecognizeError::NotPayment(r)) if r == "聊天截图"
    ));
    assert!(matches!(
        build_draft(output(r#"{"ok":true,"amount":null}"#), now()),
        Err(RecognizeError::NoAmount)
    ));
}

#[test]
fn detects_image_type() {
    assert_eq!(mime(b"\x89PNG\r\n\x1a\nxxxx"), Some("image/png"));
    assert_eq!(mime(b"\xff\xd8\xff\xe0"), Some("image/jpeg"));
    assert_eq!(mime(b"GIF89a"), None);
}
