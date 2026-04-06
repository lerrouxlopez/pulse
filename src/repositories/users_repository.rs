use crate::models::{UserAuth, UserSummary};
use mysql::prelude::*;
use mysql::PooledConn;

pub fn create_user(
    conn: &mut PooledConn,
    name: &str,
    email: &str,
    password_hash: &str,
) -> mysql::Result<i64> {
    conn.exec_drop(
        "INSERT INTO users (name, email, password_hash) VALUES (?, ?, ?)",
        (name, email, password_hash),
    )?;
    Ok(conn.last_insert_id() as i64)
}

pub fn find_user_by_email(conn: &mut PooledConn, email: &str) -> mysql::Result<Option<UserAuth>> {
    let row: Option<(i64, String)> =
        conn.exec_first("SELECT id, password_hash FROM users WHERE email = ?", (email,))?;
    Ok(row.map(|(id, password_hash)| UserAuth { id, password_hash }))
}

pub fn find_user_by_id(conn: &mut PooledConn, user_id: i64) -> mysql::Result<Option<UserSummary>> {
    let row: Option<(i64, String)> =
        conn.exec_first("SELECT id, name FROM users WHERE id = ?", (user_id,))?;
    Ok(row.map(|(id, name)| UserSummary { id, name }))
}

pub fn find_user_id_by_email(conn: &mut PooledConn, email: &str) -> mysql::Result<Option<i64>> {
    conn.exec_first("SELECT id FROM users WHERE email = ?", (email,))
}

pub fn list_all(conn: &mut PooledConn) -> mysql::Result<Vec<(i64, String, String)>> {
    conn.exec_map(
        "SELECT id, name, email FROM users ORDER BY name",
        (),
        |(id, name, email)| (id, name, email),
    )
}

pub fn update_user(
    conn: &mut PooledConn,
    user_id: i64,
    name: &str,
    email: &str,
) -> mysql::Result<usize> {
    conn.exec_drop(
        "UPDATE users SET name = ?, email = ? WHERE id = ?",
        (name, email, user_id),
    )?;
    Ok(conn.affected_rows() as usize)
}
