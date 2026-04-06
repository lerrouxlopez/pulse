use crate::models::NamedItem;
use mysql::prelude::*;
use mysql::{PooledConn, TxOpts};

pub fn list(conn: &mut PooledConn, tournament_id: i64) -> mysql::Result<Vec<NamedItem>> {
    conn.exec_map(
        "SELECT id, name FROM events WHERE tournament_id = ? ORDER BY id",
        (tournament_id,),
        |(id, name)| NamedItem { id, name },
    )
}

pub fn create(conn: &mut PooledConn, tournament_id: i64, name: &str) -> mysql::Result<i64> {
    conn.exec_drop(
        "INSERT INTO events (tournament_id, name) VALUES (?, ?)",
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
        "UPDATE events SET name = ? WHERE id = ? AND tournament_id = ?",
        (name, id, tournament_id),
    )?;
    Ok(conn.affected_rows() as usize)
}

pub fn delete(conn: &mut PooledConn, tournament_id: i64, id: i64) -> mysql::Result<usize> {
    let mut tx = conn.start_transaction(TxOpts::default())?;
    tx.exec_drop(
        "DELETE FROM team_events WHERE event_id = ? AND tournament_id = ?",
        (id, tournament_id),
    )?;
    tx.exec_drop(
        "DELETE FROM events WHERE id = ? AND tournament_id = ?",
        (id, tournament_id),
    )?;
    let affected = tx.affected_rows() as usize;
    tx.commit()?;
    Ok(affected)
}
