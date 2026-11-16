use chrono::TimeZone;

use super::*;

fn now() -> DateTime<FixedOffset> {
    tz().with_ymd_and_hms(2026, 11, 16, 13, 42, 0).unwrap()
}

fn draft() -> Draft {
    Draft {
        id: "01".to_owned(),
        kind: Kind::Expense,
        amount_cents: 3250,
        category: "餐饮".to_owned(),
        subcategory: Some("咖啡饮品".to_owned()),
        account: Some("微信".to_owned()),
        merchant: Some("瑞幸咖啡".to_owned()),
        occurred_at: now(),
        note: None,
        tags: Vec::new(),
        warnings: Vec::new(),
    }
}

fn reply(draft: Draft, field: &str, value: &str) -> Reply {
    match step(draft, field, value, now()).unwrap() {
        Outcome::Reply(r) => r,
        Outcome::Confirm(_) => panic!("意外确认"),
    }
}

fn decoded(r: &Reply) -> Draft {
    Draft::decode(&r.draft).unwrap()
}

#[test]
fn menu_summary() {
    let r = menu(&draft(), now());
    assert_eq!(r.step, StepType::Choose);
    assert_eq!(r.field, "action");
    assert!(r.summary.is_empty());
    assert_eq!(
        r.options,
        vec![
            "💰 金额 · 32.50",
            "↔️ 类型 · 支出",
            "🗂️ 分类 · 餐饮 / 咖啡饮品",
            "💳 账户 · 微信",
            "🏪 商家 · 瑞幸咖啡",
            "🕒 时间 · 11-16 13:42",
            "🏷️ 标签 · 无",
            "📝 备注 · 无",
            "✅ 确认记账",
        ]
    );
}

#[test]
fn confirm_returns_draft() {
    let d = draft();
    match step(d.clone(), "action", "✅ 确认记账", now()).unwrap() {
        Outcome::Confirm(got) => assert_eq!(got, d),
        Outcome::Reply(_) => panic!("应当确认"),
    }
}

#[test]
fn two_level_category() {
    let r = reply(draft(), "action", "🗂️ 分类 · 餐饮 / 咖啡饮品");
    assert_eq!(r.field, "category");
    assert!(r.options.contains(&"🗂️ 交通".to_owned()));

    let r = reply(decoded(&r), "category", "🗂️ 交通");
    assert_eq!(r.field, "subcategory");
    assert_eq!(decoded(&r).subcategory, None);

    let r = reply(decoded(&r), "subcategory", "📂 打车");
    assert_eq!(r.field, "action");
    assert!(r.options.contains(&"🗂️ 分类 · 交通 / 打车".to_owned()));

    let r = reply(decoded(&r), "category", "🗂️ 其他");
    assert_eq!(r.field, "action", "无二级直接回主菜单");
}

#[test]
fn amount_input() {
    let r = reply(draft(), "action", "💰 金额 · 32.50");
    assert_eq!((r.step, r.default.as_str()), (StepType::Number, "32.50"));

    let r = reply(decoded(&r), "amount", "18.5");
    assert_eq!(decoded(&r).amount_cents, 1850);

    let r = reply(decoded(&r), "amount", "abc");
    assert_eq!(r.field, "amount", "非法金额重新输入");
    assert!(r.summary.starts_with("⚠ 金额无效"));

    for bad in ["0", "-1", "1.234", ".5", ""] {
        assert_eq!(parse_cents(bad), None, "{bad}");
    }
    assert_eq!(parse_cents("12,3"), Some(1230));
}

#[test]
fn tags_replace_and_clear() {
    let r = reply(draft(), "tags", "🏷️ 恋爱");
    assert_eq!(decoded(&r).tags, vec!["恋爱"]);
    assert!(r.options.contains(&"🏷️ 标签 · 恋爱".to_owned()));

    let r = reply(decoded(&r), "tags", "");
    assert!(decoded(&r).tags.is_empty());

    assert!(step(draft(), "tags", "不存在", now()).is_err());
}

#[test]
fn switch_kind_resets_category() {
    let r = reply(draft(), "kind", "↗️ 收入");
    assert_eq!(r.field, "category");
    let d = decoded(&r);
    assert_eq!(
        (d.kind, d.category.as_str(), d.subcategory),
        (Kind::Income, OTHER, None)
    );

    let r = reply(draft(), "kind", "↘️ 支出");
    assert_eq!(r.field, "action", "未变化直接回主菜单");
}

#[test]
fn more_fields() {
    let r = reply(draft(), "occurred_at", "2025-01-02T03:04:05Z");
    assert!(
        r.options.contains(&"🕒 时间 · 01-02 11:04".to_owned()),
        "转为 +08:00，菜单使用月日时间"
    );

    let r = reply(draft(), "note", "  和朋友  ");
    assert!(r.options.contains(&"📝 备注 · 和朋友".to_owned()));

    let r = reply(draft(), "merchant", "");
    assert!(r.options.contains(&"🏪 商家 · 无".to_owned()));
}

#[test]
fn rejects_invalid_input() {
    assert!(matches!(
        step(draft(), "nope", "", now()),
        Err(FlowError::UnknownField(_))
    ));
    assert!(step(draft(), "action", "删库", now()).is_err());
    assert!(step(draft(), "account", "现金", now()).is_err());
    assert!(Draft::decode("not-base64!").is_err());

    let mut d = draft();
    d.category = "不存在".to_owned();
    assert!(Draft::decode(&d.encode()).is_err());
}

#[test]
fn dynamic_actions_round_trip() {
    let d = draft();
    let menu = menu(&d, now());
    for (option, field) in menu.options.iter().zip([
        "amount",
        "kind",
        "category",
        "account",
        "merchant",
        "occurred_at",
        "tags",
        "note",
    ]) {
        assert_eq!(reply(d.clone(), "action", option).field, field);
    }
    assert!(
        matches!(step(d.clone(), "action", menu.options.last().unwrap(), now()).unwrap(), Outcome::Confirm(got) if got == d)
    );
    assert!(step(d, "action", "💰 金额 · 99.00", now()).is_err());
}

#[test]
fn decorated_choices_store_plain_values() {
    let r = reply(draft(), "category", "🗂️ 交通");
    assert!(r.options.contains(&"📂 打车".to_owned()));
    let r = reply(decoded(&r), "subcategory", "📂 打车");
    assert_eq!(decoded(&r).subcategory.as_deref(), Some("打车"));
    let r = reply(decoded(&r), "account", "💳 银行卡");
    assert_eq!(decoded(&r).account.as_deref(), Some("银行卡"));
    let r = reply(decoded(&r), "tags", "🏷️ 恋爱");
    assert_eq!(decoded(&r).tags, ["恋爱"]);
    let r = reply(decoded(&r), "kind", "↗️ 收入");
    assert_eq!(decoded(&r).kind, Kind::Income);
    assert!(step(draft(), "account", "🛑 微信", now()).is_err());
}

#[test]
fn warnings_are_not_displayed() {
    let mut d = draft();
    d.warnings.push("默认时间警告".to_owned());
    assert!(menu(&d, now()).summary.is_empty());
    let r = reply(d, "action", "💰 金额 · 32.50");
    assert!(!r.summary.contains("默认时间警告"));
}
