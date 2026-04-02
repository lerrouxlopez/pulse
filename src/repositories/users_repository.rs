use crate::models::{UserAuth, UserSummary};
use rusqlite::{params, Connection};

pub fn create_user(
    conn: &Connection,
    name: &str,
    email: &str,
    password_hash: &str,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO users (name, email, password_hash) VALUES (?1, ?2, ?3)",
        params![name, email, password_hash],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn find_user_by_email(conn: &Connection, email: &str) -> rusqlite::Result<Option<UserAuth>> {
    let mut stmt = conn.prepare("SELECT id, password_hash FROM users WHERE email = ?1")?;
    let mut rows = stmt.query(params![email])?;
    if let Some(row) = rows.next()? {
        Ok(Some(UserAuth {
            id: row.get(0)?,
            password_hash: row.get(1)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn find_user_by_id(conn: &Connection, user_id: i64) -> rusqlite::Result<Option<UserSummary>> {
    let mut stmt = conn.prepare("SELECT id, name FROM users WHERE id = ?1")?;
    let mut rows = stmt.query(params![user_id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(UserSummary {
            id: row.get(0)?,
            name: row.get(1)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn find_user_id_by_email(conn: &Connection, email: &str) -> rusqlite::Result<Option<i64>> {
    let mut stmt = conn.prepare("SELECT id FROM users WHERE email = ?1")?;
    let mut rows = stmt.query(params![email])?;
    if let Some(row) = rows.next()? {
        Ok(Some(row.get(0)?))
    } else {
        Ok(None)
    }
}
