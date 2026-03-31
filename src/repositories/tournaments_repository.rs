use crate::models::Tournament;
use rusqlite::{params, Connection};

pub fn get_first(conn: &Connection) -> rusqlite::Result<Option<Tournament>> {
    let mut stmt = conn.prepare("SELECT id, name, is_setup FROM tournaments ORDER BY id LIMIT 1")?;
    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        let is_setup_value: i64 = row.get(2)?;
        Ok(Some(Tournament {
            id: row.get(0)?,
            name: row.get(1)?,
            is_setup: is_setup_value != 0,
        }))
    } else {
        Ok(None)
    }
}

pub fn create(conn: &Connection, name: &str) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO tournaments (name, is_setup) VALUES (?1, 0)",
        params![name],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_by_id(conn: &Connection, tournament_id: i64) -> rusqlite::Result<Option<Tournament>> {
    let mut stmt = conn.prepare("SELECT id, name, is_setup FROM tournaments WHERE id = ?1")?;
    let mut rows = stmt.query(params![tournament_id])?;
    if let Some(row) = rows.next()? {
        let is_setup_value: i64 = row.get(2)?;
        Ok(Some(Tournament {
            id: row.get(0)?,
            name: row.get(1)?,
            is_setup: is_setup_value != 0,
        }))
    } else {
        Ok(None)
    }
}

pub fn set_setup(conn: &Connection, tournament_id: i64, is_setup: bool) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE tournaments SET is_setup = ?1 WHERE id = ?2",
        params![if is_setup { 1 } else { 0 }, tournament_id],
    )?;
    Ok(())
}
