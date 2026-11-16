use super::*;

fn db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    conn.execute_batch(crate::db::SCHEMA).unwrap();
    seed(&conn).unwrap();
    conn
}

fn run(conn: &mut Connection, cmd: &str) -> anyhow::Result<String> {
    let args: Vec<String> = cmd.split(' ').map(str::to_owned).collect();
    cli(conn, &args)
}

#[test]
fn seed_once() {
    let conn = db();
    let cat = load(&conn).unwrap();
    assert_eq!(
        cat.key_lines(Kind::Expense),
        defaults().key_lines(Kind::Expense)
    );
    assert_eq!(cat.accounts, defaults().accounts);
    assert_eq!(
        cat.by_key(Kind::Expense, "coffee"),
        Some(("餐饮", Some("咖啡饮品")))
    );
    assert_eq!(
        cat.by_key(Kind::Expense, "food"),
        None,
        "有二级的一级不给模型"
    );
    seed(&conn).unwrap();
    assert_eq!(load(&conn).unwrap().accounts.len(), cat.accounts.len());
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
    assert!(run(&mut conn, "category rename 收入 其他 杂项").is_err());

    run(&mut conn, "account add 现金").unwrap();
    assert!(run(&mut conn, "account add 现金").is_err());
    run(&mut conn, "tag rename 恋爱 约会").unwrap();
    assert_eq!(load(&conn).unwrap().tags, ["约会"]);
    assert!(run(&mut conn, "nope").is_err());
}

#[test]
fn used_ones_cannot_be_removed() {
    let mut conn = db();
    conn.execute_batch(
        "INSERT INTO entries (id, kind, amount_cents, category_id, account_id, occurred_at,
                              created_at)
         SELECT 'e1', 'expense', 100, c.id, a.id, 'now', 'now'
         FROM categories c, accounts a WHERE c.key = 'coffee' AND a.name = '微信';
         INSERT INTO entry_tags SELECT 'e1', id FROM tags WHERE name = '恋爱';",
    )
    .unwrap();
    let err = run(&mut conn, "category rm 支出 餐饮/咖啡饮品").unwrap_err();
    assert!(err.to_string().contains("1 笔账"), "{err}");
    assert!(run(&mut conn, "account rm 微信").is_err());
    assert!(run(&mut conn, "tag rm 恋爱").is_err());
    run(&mut conn, "category rename 支出 餐饮/咖啡饮品 咖啡").unwrap();
    assert!(
        run(&mut conn, "category list")
            .unwrap()
            .contains("咖啡  coffee  1 笔")
    );
}
