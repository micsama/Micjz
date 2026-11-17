use chrono::TimeZone;

use super::*;
use crate::catalog;

fn now() -> DateTime<FixedOffset> {
    tz().with_ymd_and_hms(2026, 11, 16, 13, 42, 0).unwrap()
}

fn draft() -> Draft {
    let cat = catalog::defaults();
    Draft {
        id: "01".to_owned(),
        kind: Kind::Expense,
        amount_cents: Some(3250),
        category_id: cat.by_key(Kind::Expense, "coffee").unwrap(),
        account_id: cat.account_named("微信").unwrap(),
        merchant: Some("瑞幸咖啡".to_owned()),
        occurred_at: now(),
        note: None,
        tag_ids: Vec::new(),
    }
}

/// 断言用的名字。
fn path(d: &Draft) -> String {
    d.category_path(&catalog::defaults())
}

fn account(d: &Draft) -> String {
    d.account_name(&catalog::defaults()).to_owned()
}

fn tags(d: &Draft) -> Vec<String> {
    let cat = catalog::defaults();
    d.tag_names(&cat).into_iter().map(str::to_owned).collect()
}

fn reply(draft: Draft, field: &str, value: &str) -> Reply {
    match step(draft, field, value, now(), &catalog::defaults()).unwrap() {
        Outcome::Reply(r) => r,
        Outcome::Confirm { .. } => panic!("意外确认"),
    }
}

fn decoded(r: &Reply) -> Draft {
    Draft::decode(&r.draft, &catalog::defaults()).unwrap()
}

#[test]
fn menu_summary() {
    let r = menu(&draft(), now(), &catalog::defaults());
    assert_eq!(r.step, StepType::Choose);
    assert_eq!(r.field, "action");
    assert!(r.summary.is_empty());
    assert_eq!(
        r.options,
        vec![
            "💰 金额 · 32.50",
            "↔️ 类型 · 支出",
            "🍜 餐饮 / ☕️ 咖啡饮品",
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
    match step(
        d.clone(),
        "action",
        "✅ 确认记账",
        now(),
        &catalog::defaults(),
    )
    .unwrap()
    {
        Outcome::Confirm {
            draft,
            amount_cents,
        } => assert_eq!((draft, amount_cents), (d, 3250)),
        Outcome::Reply(_) => panic!("应当确认"),
    }
}

#[test]
fn two_level_category() {
    let r = reply(draft(), "action", "🍜 餐饮 / ☕️ 咖啡饮品");
    assert_eq!(r.field, "category");
    assert!(r.options.contains(&"🚇 交通".to_owned()));

    let r = reply(decoded(&r), "category", "🚇 交通");
    assert_eq!(r.field, "subcategory");
    assert_eq!(path(&decoded(&r)), "🚇 交通", "没选二级前停在一级");

    let r = reply(decoded(&r), "subcategory", "🚕 打车");
    assert_eq!(r.field, "action");
    assert!(r.options.contains(&"🚇 交通 / 🚕 打车".to_owned()));

    let r = reply(decoded(&r), "category", "✨ 其他");
    assert_eq!(r.field, "action", "无二级直接回主菜单");
}

#[test]
fn amount_input() {
    let r = reply(draft(), "action", "💰 金额 · 32.50");
    assert_eq!((r.step, r.default.as_str()), (StepType::Number, "32.50"));

    let r = reply(decoded(&r), "amount", "18.5");
    assert_eq!(decoded(&r).amount_cents, Some(1850));

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
    let r = reply(draft(), "tags", "恋爱");
    assert_eq!(tags(&decoded(&r)), ["恋爱"]);
    assert!(r.options.contains(&"🏷️ 标签 · 恋爱".to_owned()));

    let r = reply(decoded(&r), "tags", "");
    assert!(decoded(&r).tag_ids.is_empty());

    assert!(step(draft(), "tags", "不存在", now(), &catalog::defaults()).is_err());
}

#[test]
fn switch_kind_resets_category() {
    let r = reply(draft(), "kind", "↗️ 收入");
    assert_eq!(r.field, "category");
    let d = decoded(&r);
    assert_eq!((d.kind, path(&d).as_str()), (Kind::Income, "✨ 其他"));

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
        step(draft(), "nope", "", now(), &catalog::defaults()),
        Err(FlowError::UnknownField(_))
    ));
    assert!(step(draft(), "action", "删库", now(), &catalog::defaults()).is_err());
    assert!(step(draft(), "account", "现金", now(), &catalog::defaults()).is_err());
    assert!(Draft::decode("not-base64!", &catalog::defaults()).is_err());

    let mut d = draft();
    d.category_id = 9999;
    assert!(Draft::decode(&d.encode(), &catalog::defaults()).is_err());
    let mut d = draft();
    d.tag_ids = vec![9999];
    assert!(Draft::decode(&d.encode(), &catalog::defaults()).is_err());
}

#[test]
fn dynamic_actions_round_trip() {
    let d = draft();
    let menu = menu(&d, now(), &catalog::defaults());
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
        matches!(step(d.clone(), "action", menu.options.last().unwrap(), now(), &catalog::defaults()).unwrap(), Outcome::Confirm { draft, .. } if draft == d)
    );
    assert!(step(d, "action", "💰 金额 · 99.00", now(), &catalog::defaults()).is_err());
}

#[test]
fn decorated_choices_store_plain_values() {
    let r = reply(draft(), "category", "🚇 交通");
    assert!(r.options.contains(&"🚕 打车".to_owned()));
    let r = reply(decoded(&r), "subcategory", "🚕 打车");
    assert_eq!(path(&decoded(&r)), "🚇 交通/🚕 打车");
    let r = reply(decoded(&r), "account", "银行卡");
    assert_eq!(account(&decoded(&r)), "银行卡");
    let r = reply(decoded(&r), "tags", "恋爱");
    assert_eq!(tags(&decoded(&r)), ["恋爱"]);
    let r = reply(decoded(&r), "kind", "↗️ 收入");
    assert_eq!(decoded(&r).kind, Kind::Income);
    assert!(step(draft(), "account", "🛑 微信", now(), &catalog::defaults()).is_err());
}

#[test]
fn missing_amount_blocks_confirm() {
    let mut d = draft();
    d.amount_cents = None;
    let m = menu(&d, now(), &catalog::defaults());
    assert!(m.summary.is_empty(), "不在顶部提示");
    assert!(m.options.contains(&"💰 金额 · 请补充".to_owned()));

    let r = reply(d.clone(), "action", "✅ 确认记账");
    assert_eq!(
        (r.step, r.field.as_str()),
        (StepType::Number, "amount"),
        "没金额不能确认，转去填金额"
    );
    assert!(r.summary.contains("¥请补充"));
    assert_eq!(r.default, "0", "数字框预填 0");

    let r = reply(decoded(&r), "amount", "0");
    assert!(r.summary.starts_with("⚠ 金额无效"), "0 不能入账，重新输入");

    let r = reply(decoded(&r), "amount", "12");
    assert!(r.summary.is_empty());
    assert!(matches!(
        step(
            decoded(&r),
            "action",
            "✅ 确认记账",
            now(),
            &catalog::defaults()
        )
        .unwrap(),
        Outcome::Confirm {
            amount_cents: 1200,
            ..
        }
    ));
}

#[test]
fn draft_validation() {
    let mut d = draft();
    d.amount_cents = Some(0);
    assert!(Draft::decode(&d.encode(), &catalog::defaults()).is_err());
    let mut d = draft();
    d.kind = Kind::Income;
    assert!(
        Draft::decode(&d.encode(), &catalog::defaults()).is_err(),
        "分类须与类型一致"
    );
    let mut d = draft();
    d.amount_cents = None;
    assert!(Draft::decode(&d.encode(), &catalog::defaults()).is_ok());
}
