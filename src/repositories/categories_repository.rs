use crate::models::NamedItem;
use rusqlite::{params, Connection};

pub fn list(conn: &Connection, tournament_id: i64) -> rusqlite::Result<Vec<NamedItem>> {
    let mut stmt = conn.prepare("SELECT id, name FROM categories WHERE tournament_id = ?1 ORDER BY id")?;
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
        "INSERT INTO categories (tournament_id, name) VALUES (?1, ?2)",
        params![tournament_id, name],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update(
    conn: &Connection,
    tournament_id: i64,
    id: i64,
    name: &str,
) -> rusqlite::Result<usize> {
    let changed = conn.execute(
        "UPDATE categories SET name = ?1 WHERE id = ?2 AND tournament_id = ?3",
        params![name, id, tournament_id],
    )?;
    Ok(changed)
}

pub fn delete(conn: &Connection, tournament_id: i64, id: i64) -> rusqlite::Result<usize> {
    let changed = conn.execute(
        "DELETE FROM categories WHERE id = ?1 AND tournament_id = ?2",
        params![id, tournament_id],
    )?;
    Ok(changed)
}
