use crate::models::NamedItem;
use mysql::prelude::*;
use mysql::PooledConn;

pub fn list(conn: &mut PooledConn, tournament_id: i64) -> mysql::Result<Vec<NamedItem>> {
    conn.exec_map(
        "SELECT id, name FROM divisions WHERE tournament_id = ? ORDER BY id",
        (tournament_id,),
        |(id, name)| NamedItem { id, name },
    )
}

pub fn create(conn: &mut PooledConn, tournament_id: i64, name: &str) -> mysql::Result<i64> {
    conn.exec_drop(
        "INSERT INTO divisions (tournament_id, name) VALUES (?, ?)",
        (tournament_id, name),
    )?;
    Ok(conn.last_insert_id() as i64)
}

pub fn update(
    conn: &mut PooledConn,
    tournament_id: i64,
    id: i64,
    name: &str,
) -> mysql::Result<usize> {
    conn.exec_drop(
        "UPDATE divisions SET name = ? WHERE id = ? AND tournament_id = ?",
        (name, id, tournament_id),
    )?;
    Ok(conn.affected_rows() as usize)
}

pub fn delete(conn: &mut PooledConn, tournament_id: i64, id: i64) -> mysql::Result<usize> {
    conn.exec_drop(
        "DELETE FROM divisions WHERE id = ? AND tournament_id = ?",
        (id, tournament_id),
    )?;
    Ok(conn.affected_rows() as usize)
}

pub fn get_by_id(conn: &mut PooledConn, tournament_id: i64, id: i64) -> mysql::Result<Option<NamedItem>> {
    let row: Option<(i64, String)> =
        conn.exec_first("SELECT id, name FROM divisions WHERE id = ? AND tournament_id = ?", (id, tournament_id))?;
    Ok(row.map(|(id, name)| NamedItem { id, name }))
}
