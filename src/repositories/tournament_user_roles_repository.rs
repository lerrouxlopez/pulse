use mysql::prelude::*;
use mysql::PooledConn;

pub fn set_user_role(
    conn: &mut PooledConn,
    tournament_id: i64,
    user_id: i64,
    role_id: i64,
) -> mysql::Result<()> {
    conn.exec_drop(
        "DELETE FROM tournament_user_roles WHERE tournament_id = ? AND user_id = ?",
        (tournament_id, user_id),
    )?;
    conn.exec_drop(
        "INSERT INTO tournament_user_roles (tournament_id, user_id, role_id) VALUES (?, ?, ?)",
        (tournament_id, user_id, role_id),
    )?;
    Ok(())
}

pub fn get_user_role(
    conn: &mut PooledConn,
    tournament_id: i64,
    user_id: i64,
) -> mysql::Result<Option<i64>> {
    conn.exec_first(
        "SELECT role_id FROM tournament_user_roles WHERE tournament_id = ? AND user_id = ?",
        (tournament_id, user_id),
    )
}

pub fn remove_user(
    conn: &mut PooledConn,
    tournament_id: i64,
    user_id: i64,
) -> mysql::Result<()> {
    conn.exec_drop(
        "DELETE FROM tournament_user_roles WHERE tournament_id = ? AND user_id = ?",
        (tournament_id, user_id),
    )?;
    Ok(())
}
