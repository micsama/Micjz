use super::*;

fn db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    conn.execute_batch(crate::db::SCHEMA).unwrap();
    conn.execute_batch(SEED).unwrap();
    conn
}

fn run(conn: &mut Connection, cmd: &str) -> anyhow::Result<String> {
    let args: Vec<String> = cmd.split(' ').map(str::to_owned).collect();
    cli(conn, &args)
}

#[test]
fn seed_data() {
    let cat = load(&db()).unwrap();
    assert_eq!(cat.fallback(Kind::Expense), "其他");
    assert_eq!(cat.fallback(Kind::Income), "其他");
    assert_eq!(cat.account_names().collect::<Vec<_>>(), ["微信", "银行卡"]);
    assert_eq!(cat.default_account(), "微信");
    assert_eq!(
        cat.by_key(Kind::Expense, "coffee"),
        Some(("餐饮", Some("咖啡饮品")))
    );
    assert_eq!(
        cat.by_key(Kind::Expense, "food"),
        None,
        "有二级的一级不给模型"
    );
}

#[test]
fn add_rename_remove() {
    let mut conn = db();
    run(&mut conn, "category add 支出 学习 study").unwrap();
    run(&mut conn, "category add 支出 学习/书籍 books").unwrap();
    assert!(
        run(&mut conn, "category add 支出 学习/课程 books").is_err(),
        "key 唯一"
    );
    assert!(
        run(&mut conn, "category add 支出 学习/课程 Course").is_err(),
        "key 小写"
    );
    assert!(
        run(&mut conn, "category add 支出 学习/书籍 books2").is_err(),
        "同级不重名"
    );
    run(&mut conn, "category rename 支出 学习/书籍 图书").unwrap();
    let cat = load(&conn).unwrap();
    assert_eq!(
        cat.by_key(Kind::Expense, "books"),
        Some(("学习", Some("图书")))
    );

    assert!(run(&mut conn, "category rm 支出 学习").is_err(), "先删二级");
    run(&mut conn, "category rm 支出 学习/图书").unwrap();
    run(&mut conn, "category rm 支出 学习").unwrap();
    assert!(run(&mut conn, "category rm 支出 其他").is_err());
    run(&mut conn, "category rename 收入 其他 杂项").unwrap();
    assert_eq!(
        load(&conn).unwrap().fallback(Kind::Income),
        "杂项",
        "兜底按 key 认，可以改名"
    );
    assert!(run(&mut conn, "category rm 收入 杂项").is_err());

    run(&mut conn, "account add 现金").unwrap();
    assert!(run(&mut conn, "account add 现金").is_err());
    run(&mut conn, "account add 招商银行 信用卡 6225888800001234").unwrap();
    assert!(run(&mut conn, "account add 某卡 储蓄卡 12ab").is_err());
    run(&mut conn, "account move 招商银行 1").unwrap();
    let cat = load(&conn).unwrap();
    assert_eq!(cat.default_account(), "招商银行", "第一位是默认账户");
    assert_eq!(
        cat.account_lines(),
        "招商银行（信用卡 尾号 1234）\n微信\n银行卡\n现金",
        "模型只看尾号"
    );
    assert!(
        run(&mut conn, "account list")
            .unwrap()
            .contains("招商银行  信用卡 6225888800001234  0 笔")
    );
    for name in ["微信", "银行卡", "现金"] {
        run(&mut conn, &format!("account rm {name}")).unwrap();
    }
    assert!(
        run(&mut conn, "account rm 招商银行").is_err(),
        "最后一个账户不能删"
    );

    run(&mut conn, "category move 支出 餐饮/聚餐 1").unwrap();
    run(&mut conn, "category move 支出 其他 1").unwrap();
    let cat = load(&conn).unwrap();
    let first = &cat.categories(Kind::Expense)[0];
    assert_eq!(first.name, "其他");
    assert_eq!(
        cat.category(Kind::Expense, "餐饮").unwrap().subs[0].1,
        "聚餐"
    );
    assert!(run(&mut conn, "category move 支出 其他 0").is_err());
    run(&mut conn, "tag rename 恋爱 约会").unwrap();
    assert_eq!(load(&conn).unwrap().tags, ["约会"]);
    assert!(run(&mut conn, "nope").is_err());
}

#[test]
fn used_ones_cannot_be_removed() {
    let mut conn = db();
    conn.execute_batch(
        "INSERT INTO recognitions (draft_id, outcome, model, upload_ms, model_ms, image_sha256,
                                   image_bytes, image_path, prompt, parameters, raw, origin)
         VALUES ('e1', 'ok', 'm', 0, 0, 's', 0, 'p', 'p', '{}', '', '{}');
         INSERT INTO entries (id, kind, amount_cents, category_id, account_id, occurred_at,
                              created_at)
         SELECT 'e1', 'expense', 100, c.id, a.id, 'now', 'now'
         FROM categories c, accounts a WHERE c.key = 'coffee' AND a.name = '微信';
         INSERT INTO entry_tags SELECT 'e1', id FROM tags WHERE name = '恋爱';
         INSERT INTO accounts (name, position) VALUES ('现金', 9);
         INSERT INTO entries (id, kind, amount_cents, account_id, to_account_id, occurred_at,
                              created_at)
         SELECT 'e2', 'transfer', 100, a.id, b.id, 'now', 'now'
         FROM accounts a, accounts b WHERE a.name = '银行卡' AND b.name = '现金';",
    )
    .unwrap();
    let err = run(&mut conn, "category rm 支出 餐饮/咖啡饮品").unwrap_err();
    assert!(err.to_string().contains("1 笔账"), "{err}");
    assert!(run(&mut conn, "account rm 微信").is_err());
    assert!(run(&mut conn, "tag rm 恋爱").is_err());
    assert!(
        run(&mut conn, "account rm 现金").is_err(),
        "转入账户也算在用"
    );
    assert!(
        run(&mut conn, "account list")
            .unwrap()
            .contains("现金  1 笔")
    );
    run(&mut conn, "category rename 支出 餐饮/咖啡饮品 咖啡").unwrap();
    assert!(
        run(&mut conn, "category list")
            .unwrap()
            .contains("咖啡  coffee  1 笔")
    );
}
