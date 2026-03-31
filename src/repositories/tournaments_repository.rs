use crate::models::Tournament;
use rusqlite::{params, Connection};

pub fn get_by_id(conn: &Connection, tournament_id: i64) -> rusqlite::Result<Option<Tournament>> {
    let mut stmt =
        conn.prepare("SELECT id, name, is_setup, user_id, started_at FROM tournaments WHERE id = ?1")?;
    let mut rows = stmt.query(params![tournament_id])?;
    if let Some(row) = rows.next()? {
        let is_setup_value: i64 = row.get(2)?;
        Ok(Some(Tournament {
            id: row.get(0)?,
            name: row.get(1)?,
            is_setup: is_setup_value != 0,
            user_id: row.get(3)?,
            started_at: row.get(4)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn get_by_id_for_user(
    conn: &Connection,
    tournament_id: i64,
    user_id: i64,
) -> rusqlite::Result<Option<Tournament>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, is_setup, user_id, started_at FROM tournaments WHERE id = ?1 AND user_id = ?2",
    )?;
    let mut rows = stmt.query(params![tournament_id, user_id])?;
    if let Some(row) = rows.next()? {
        let is_setup_value: i64 = row.get(2)?;
        Ok(Some(Tournament {
            id: row.get(0)?,
            name: row.get(1)?,
            is_setup: is_setup_value != 0,
            user_id: row.get(3)?,
            started_at: row.get(4)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn list_by_user(conn: &Connection, user_id: i64) -> rusqlite::Result<Vec<Tournament>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, is_setup, user_id, started_at FROM tournaments WHERE user_id = ?1 ORDER BY id DESC",
    )?;
    let rows = stmt.query_map(params![user_id], |row| {
        let is_setup_value: i64 = row.get(2)?;
        Ok(Tournament {
            id: row.get(0)?,
            name: row.get(1)?,
            is_setup: is_setup_value != 0,
            user_id: row.get(3)?,
            started_at: row.get(4)?,
        })
    })?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok(items)
}

pub fn create(conn: &Connection, user_id: i64, name: &str) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO tournaments (user_id, name, is_setup) VALUES (?1, ?2, 0)",
        params![user_id, name],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn set_setup(conn: &Connection, tournament_id: i64, is_setup: bool) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE tournaments SET is_setup = ?1 WHERE id = ?2",
        params![if is_setup { 1 } else { 0 }, tournament_id],
    )?;
    Ok(())
}
