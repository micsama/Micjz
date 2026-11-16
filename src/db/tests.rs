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
fn final_values_and_recognition_are_linked_without_edits() {
    let (db, path) = db("wide");
    let d = draft();
    db.record_recognition(&Recognition {
        draft_id: &d.id,
        outcome: "ok",
        model: "mock",
        elapsed_ms: 1,
        image_sha256: "abc",
        image_bytes: 10,
        raw: "original-response",
        reason: None,
        origin: Some(&d),
        image_path: "image.png",
        prompt: "actual prompt",
        parameters: "{\"reasoning_effort\":\"none\"}",
        timings: "{}",
    })
    .unwrap();
    let mut corrected = d.clone();
    corrected.amount_cents = 2200;
    assert!(db.save(&corrected).unwrap());
    assert!(!db.save(&corrected).unwrap());
    let conn = Connection::open(path).unwrap();
    let (amount, origin, raw): (i64, String, String) = conn.query_row(
        "SELECT e.amount_cents,r.origin,r.raw FROM entries e JOIN recognitions r ON e.draft_id=r.draft_id", [],
        |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
    assert_eq!(amount, 2200);
    assert!(origin.contains("3250"));
    assert_eq!(raw, "original-response");
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE name='edits'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}
