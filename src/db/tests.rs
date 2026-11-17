use chrono::TimeZone;

use super::*;

/// 蓝图里的余额查询，和 docs/blueprints/entries.md 一致。
const BALANCES: &str = "SELECT a.name, coalesce(sum(CASE
         WHEN e.kind IN ('income', 'adjust') THEN e.amount_cents
         WHEN e.kind = 'expense' THEN -e.amount_cents
         WHEN e.account_id = a.id THEN -e.amount_cents
         ELSE e.amount_cents
       END), 0) AS balance_cents
FROM accounts a LEFT JOIN entries e ON a.id IN (e.account_id, e.to_account_id)
GROUP BY a.id ORDER BY a.position";
use crate::{flow::tz, recognize::Outcome};

fn recognized(draft: Draft) -> Recognized {
    Recognized {
        draft,
        outcome: Outcome::Ok,
        reason: None,
        model: "mock".to_owned(),
        model_ms: 1,
        raw: "original-response".to_owned(),
        prompt: "actual prompt".to_owned(),
        parameters: "{}".to_owned(),
    }
}

#[test]
fn confirm_links_recognition_and_saves_once() {
    let path = std::env::temp_dir().join(format!("micjz-db-{}.db", uuid::Uuid::now_v7()));
    let db = Db::open(&path).unwrap();
    let now = tz().with_ymd_and_hms(2026, 11, 16, 13, 42, 0).unwrap();
    let mut d = Draft::blank(now, &db.catalog().unwrap());
    d.amount_cents = Some(3250);
    d.tag_ids = vec![db.catalog().unwrap().tags[0].id];
    let image = Image {
        sha256: "abc",
        bytes: 10,
        path: "images/abc.png",
    };
    db.record_recognition(&recognized(d.clone()), &image, 5)
        .unwrap();
    assert_eq!(db.recognition(&d.id).unwrap(), Some((Outcome::Ok, None)));
    assert_eq!(db.recognition("nope").unwrap(), None);

    assert!(db.save(&d, 2200).unwrap());
    assert!(!db.save(&d, 2200).unwrap(), "重复确认不重复入账");

    let conn = Connection::open(&path).unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    let (id, amount, origin, account, occurred_at): (String, i64, String, String, String) = conn
        .query_row(
            "SELECT e.id, e.amount_cents, r.origin, a.name, e.occurred_at
             FROM entries e JOIN recognitions r ON e.recognition_id = r.draft_id
             JOIN accounts a ON a.id = e.account_id",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .unwrap();
    assert_ne!(id, d.id, "账目有自己的 id");
    assert_eq!(amount, 2200, "入账用用户最终值");
    assert_eq!(occurred_at, "2026-11-16T05:42:00Z", "时间存 UTC");
    assert!(origin.contains("3250"), "识别记录保留原始草稿");
    assert_eq!(account, "微信", "空白草稿用默认账户");
    let tags: i64 = conn
        .query_row("SELECT count(*) FROM entry_tags", [], |r| r.get(0))
        .unwrap();
    assert_eq!(tags, 1);
    conn.execute("DELETE FROM entries", []).unwrap();
    let tags: i64 = conn
        .query_row("SELECT count(*) FROM entry_tags", [], |r| r.get(0))
        .unwrap();
    assert_eq!(tags, 0, "删账目时标签跟着删");
    let _ = std::fs::remove_file(path);
}

/// 四类账目的约束和余额：由数据库保证，不靠写入路径。
#[test]
fn entry_kinds_and_balances() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    conn.execute_batch(SCHEMA).unwrap();
    conn.execute_batch(catalog::SEED).unwrap();
    // 账户 1 微信、2 银行卡；分类 drink（支出）、salary（收入）。
    let insert = |id: &str, kind: &str, amount: &str, category: &str, account: i64, to: &str| {
        conn.execute(
            &format!(
                "INSERT INTO entries (id, kind, amount_cents, category_id, account_id,
                                      to_account_id, occurred_at, created_at)
                 VALUES ('{id}', '{kind}', {amount},
                         (SELECT id FROM categories WHERE key = '{category}'), {account}, {to},
                         't', 't')"
            ),
            [],
        )
    };
    insert("a", "adjust", "10000", "", 2, "NULL").unwrap();
    insert("b", "income", "5000", "salary", 2, "NULL").unwrap();
    insert("c", "expense", "1200", "drink", 1, "NULL").unwrap();
    insert("d", "transfer", "3000", "", 2, "1").unwrap();
    insert("e", "adjust", "-500", "", 1, "NULL").unwrap();

    for (why, kind, amount, category, account, to) in [
        ("支出金额须为正", "expense", "-1", "drink", 1, "NULL"),
        ("平账差额不能为 0", "adjust", "0", "", 1, "NULL"),
        ("金额必须是整数", "expense", "1.5", "drink", 1, "NULL"),
        ("支出必须有分类", "expense", "1", "", 1, "NULL"),
        ("平账没有分类", "adjust", "1", "drink", 1, "NULL"),
        ("转账要有对方账户", "transfer", "1", "", 1, "NULL"),
        ("不能转给自己", "transfer", "1", "", 1, "1"),
        ("支出没有对方账户", "expense", "1", "drink", 1, "2"),
    ] {
        assert!(
            insert("x", kind, amount, category, account, to).is_err(),
            "{why}"
        );
    }

    let balances: Vec<(String, i64)> = conn
        .prepare(BALANCES)
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        balances,
        [
            ("微信".to_owned(), 3000 - 1200 - 500),
            ("银行卡".to_owned(), 10000 + 5000 - 3000)
        ]
    );
}
