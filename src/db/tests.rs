use chrono::TimeZone;

use super::*;
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
    let mut d = Draft::blank(tz().with_ymd_and_hms(2026, 11, 16, 13, 42, 0).unwrap());
    d.amount_cents = Some(3250);
    d.tags = vec!["恋爱".to_owned()];
    let image = Image {
        sha256: "abc",
        bytes: 10,
        path: "images/abc.png",
    };
    db.record_recognition(&recognized(d.clone()), &image, "{}")
        .unwrap();

    assert!(db.save(&d, 2200).unwrap());
    assert!(!db.save(&d, 2200).unwrap(), "重复确认不重复入账");

    let conn = Connection::open(&path).unwrap();
    let (amount, origin, confirmed): (i64, String, Option<String>) = conn
        .query_row(
            "SELECT e.amount_cents, r.origin, r.confirmed_at
             FROM entries e JOIN recognitions r ON e.draft_id = r.draft_id",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(amount, 2200, "入账用用户最终值");
    assert!(origin.contains("3250"), "识别记录保留原始草稿");
    assert!(confirmed.is_some());
    let tags: i64 = conn
        .query_row("SELECT count(*) FROM entry_tags", [], |r| r.get(0))
        .unwrap();
    assert_eq!(tags, 1);
    let _ = std::fs::remove_file(path);
}
