use chrono::{DateTime, TimeZone};

use super::*;
use crate::flow::tz;

fn draft() -> Draft {
    Draft {
        id: "01a1e344-a828-71f6-994d-4a31bf9cbb89".to_owned(),
        kind: Kind::Expense,
        amount_cents: 3250,
        category: "餐饮".to_owned(),
        subcategory: Some("咖啡饮品".to_owned()),
        account: Some("微信".to_owned()),
        merchant: Some("瑞幸咖啡".to_owned()),
        occurred_at: at(13, 42),
        note: None,
        tags: Vec::new(),
        warnings: Vec::new(),
    }
}

fn at(hour: u32, minute: u32) -> DateTime<chrono::FixedOffset> {
    tz().with_ymd_and_hms(2026, 11, 16, hour, minute, 0)
        .single()
        .expect("时间合法")
}

fn db(name: &str) -> (Db, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!("micjz-{name}-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&path);
    (Db::open(&path).expect("建库"), path)
}

#[test]
fn diff只有真改动的字段() {
    let before = draft();
    assert!(diff(&before, &before).is_empty());

    let mut after = before.clone();
    after.amount_cents = 2200;
    after.category = "交通".to_owned();
    after.subcategory = Some("打车".to_owned());
    let edits = diff(&before, &after);
    let fields: Vec<&str> = edits.iter().map(|e| e.field.as_str()).collect();
    assert_eq!(fields, ["amount", "category", "subcategory"]);
    assert_eq!(edits[0].before.as_deref(), Some("32.50"));
    assert_eq!(edits[0].after.as_deref(), Some("22.00"));
}

#[test]
fn 入账后宽视图带上原始识别与改动次数() {
    let (db, path) = db("wide");
    let d = draft();
    db.record_recognition(&Recognition {
        draft_id: &d.id,
        outcome: "ok",
        model: "deepseek-flash",
        elapsed_ms: 2100,
        image_sha256: "abc",
        image_bytes: 1024,
        raw: r#"{"ok":true,"amount":"32.50"}"#,
        reason: None,
        origin: Some(&d),
    })
    .expect("留痕");

    let mut corrected = d.clone();
    corrected.amount_cents = 2200;
    db.record_edits(&d.id, &diff(&d, &corrected))
        .expect("留痕改动");
    assert!(db.save(&corrected).expect("入账"));
    db.mark_confirmed(&d.id).expect("确认留痕");

    let conn = rusqlite::Connection::open(&path).expect("重新打开");
    let (raw, origin, edit_count, confirmed): (String, i64, i64, String) = conn
        .query_row(
            "SELECT raw, origin_amount_cents, edit_count, confirmed_at FROM v_bills WHERE id = ?1",
            params![d.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .expect("查宽视图");
    assert!(raw.contains("32.50"));
    assert_eq!(origin, 3250);
    assert_eq!(edit_count, 1);
    assert!(!confirmed.is_empty(), "确认时间应写上");

    let seqs: Vec<i64> = conn
        .prepare("SELECT seq FROM edits WHERE draft_id = ?1 ORDER BY seq")
        .expect("准备")
        .query_map(params![d.id], |r| r.get(0))
        .expect("查")
        .collect::<Result<Vec<_>, _>>()
        .expect("收集");
    assert_eq!(seqs, [1]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn 老库缺列时会补齐() {
    let path = std::env::temp_dir().join(format!("micjz-old-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&path);
    {
        let conn = rusqlite::Connection::open(&path).expect("建库");
        conn.execute_batch(
            "CREATE TABLE entries (
               id TEXT PRIMARY KEY, kind TEXT NOT NULL, amount_cents INTEGER NOT NULL,
               category TEXT NOT NULL, subcategory TEXT, account TEXT, merchant TEXT,
               occurred_at TEXT NOT NULL, note TEXT, created_at TEXT NOT NULL);",
        )
        .expect("建旧表");
    }
    let db = Db::open(&path).expect("打开旧库");
    assert!(db.save(&draft()).expect("入账"));
    let conn = rusqlite::Connection::open(&path).expect("重开");
    let source: String = conn
        .query_row("SELECT source FROM entries LIMIT 1", [], |r| r.get(0))
        .expect("新列已有默认值");
    assert_eq!(source, "micjz");
    let _ = std::fs::remove_file(&path);
}
