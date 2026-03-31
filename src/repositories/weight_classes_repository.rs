use crate::models::NamedItem;
use rusqlite::{params, Connection};

pub fn list(conn: &Connection, tournament_id: i64) -> rusqlite::Result<Vec<NamedItem>> {
    let mut stmt = conn.prepare("SELECT id, name FROM weight_classes WHERE tournament_id = ?1 ORDER BY id")?;
    let rows = stmt.query_map(params![tournament_id], |row| {
        Ok(NamedItem {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    })?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok(items)
}

pub fn create(conn: &Connection, tournament_id: i64, name: &str) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO weight_classes (tournament_id, name) VALUES (?1, ?2)",
        params![tournament_id, name],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update(conn: &Connection, id: i64, name: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE weight_classes SET name = ?1 WHERE id = ?2",
        params![name, id],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM weight_classes WHERE id = ?1", params![id])?;
    Ok(())
}
