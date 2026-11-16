//! 账户目录的读写规则，供 HTTP 和命令行共用。
use super::{CardType, account_ids, check_card_number, check_name};
use crate::db::{
    Error,
    entries::{self, required_nullable},
};
use rusqlite::{Connection, Transaction, params};
use serde::{Deserialize, Serialize};
#[derive(Serialize)]
pub struct Account {
    pub id: i64,
    pub name: String,
    pub brand: Option<String>,
    pub(super) card: Option<Card>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Card {
    pub(super) r#type: CardType,
    pub(super) number: String,
}
pub fn list(conn: &Connection) -> Result<Vec<Account>, Error> {
    Ok(conn
        .prepare("SELECT id,name,brand,card_type,card_number FROM accounts ORDER BY position,id")?
        .query_map([], |r| {
            let t: Option<String> = r.get(3)?;
            let card = match t.as_deref() {
                Some("debit") => Some(CardType::Debit),
                Some("credit") => Some(CardType::Credit),
                None => None,
                _ => return Err(rusqlite::Error::InvalidQuery),
            };
            Ok(Account {
                id: r.get(0)?,
                name: r.get(1)?,
                brand: r.get(2)?,
                card: card
                    .map(|t| {
                        Ok::<_, rusqlite::Error>(Card {
                            r#type: t,
                            number: r.get(4)?,
                        })
                    })
                    .transpose()?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccountInput {
    name: String,
    #[serde(deserialize_with = "required_nullable")]
    brand: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    card: Option<Card>,
}
impl AccountInput {
    fn parse(mut self) -> Result<Self, Error> {
        self.name = check_name(&self.name)
            .map_err(|e| Error::Invalid(e.to_string()))?
            .to_owned();
        self.brand = entries::text(self.brand, 20)?;
        if let Some(c) = &mut self.card {
            c.number = check_card_number(&c.number).map_err(|e| Error::Invalid(e.to_string()))?;
        }
        Ok(self)
    }
}
pub fn require(conn: &Connection, id: i64) -> Result<(), Error> {
    if !conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM accounts WHERE id=?1)",
        [id],
        |r| r.get::<_, bool>(0),
    )? {
        return Err(Error::Missing("账户不存在".into()));
    }
    Ok(())
}
pub fn save(tx: &Transaction<'_>, id: Option<i64>, v: AccountInput) -> Result<Account, Error> {
    let v = v.parse()?;
    if let Some(id) = id {
        require(tx, id)?;
    }
    if tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM accounts WHERE name=?1 AND (?2 IS NULL OR id<>?2))",
        params![v.name, id],
        |r| r.get::<_, bool>(0),
    )? {
        return Err(Error::Conflict("已经有同名账户".into()));
    }
    let ct = v.card.as_ref().map(|c| c.r#type.as_str());
    let number = v.card.as_ref().map(|c| &c.number);
    let id = if let Some(id) = id {
        tx.execute(
            "UPDATE accounts SET name=?2,brand=?3,card_type=?4,card_number=?5 WHERE id=?1",
            params![id, v.name, v.brand, ct, number],
        )?;
        id
    } else {
        tx.execute(
            "INSERT INTO accounts (name, brand, card_type, card_number, position)
             VALUES (?1, ?2, ?3, ?4, (SELECT coalesce(max(position), 0)+1 FROM accounts))",
            params![v.name, v.brand, ct, number],
        )?;
        tx.last_insert_rowid()
    };
    Ok(Account {
        id,
        name: v.name,
        brand: v.brand,
        card: v.card,
    })
}

pub fn delete(tx: &Transaction<'_>, id: i64) -> Result<(), Error> {
    require(tx, id)?;
    let used: i64 = tx.query_row(
        "SELECT count(*) FROM entries WHERE account_id=?1 OR to_account_id=?1",
        [id],
        |r| r.get(0),
    )?;
    if used > 0 {
        return Err(Error::Conflict(format!("还有 {used} 笔账目在用，不能删除")));
    }
    if account_ids(tx)?.len() == 1 {
        return Err(Error::Conflict("至少保留一个账户".into()));
    }
    tx.execute("DELETE FROM accounts WHERE id=?1", [id])?;
    Ok(())
}
