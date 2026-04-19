use mysql::prelude::*;
use mysql::PooledConn;

pub fn add_user(conn: &mut PooledConn, tournament_id: i64, user_id: i64) -> mysql::Result<()> {
    conn.exec_drop(
        "INSERT IGNORE INTO tournament_users (tournament_id, user_id) VALUES (?, ?)",
        (tournament_id, user_id),
    )?;
    Ok(())
}

pub fn remove_user(
    conn: &mut PooledConn,
    tournament_id: i64,
    user_id: i64,
) -> mysql::Result<()> {
    conn.exec_drop(
        "DELETE FROM tournament_users WHERE tournament_id = ? AND user_id = ?",
        (tournament_id, user_id),
    )?;
    Ok(())
}
