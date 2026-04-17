use crate::models::{CurrentUser, UserAuth, UserSummary};
use mysql::prelude::*;
use mysql::PooledConn;

pub fn create_system_user(
    conn: &mut PooledConn,
    name: &str,
    email: &str,
    password_hash: &str,
) -> mysql::Result<i64> {
    conn.exec_drop(
        "INSERT INTO users (name, email, password_hash, user_type, tournament_id, photo_url) VALUES (?, ?, ?, 'system', 0, NULL)",
        (name, email, password_hash),
    )?;
    Ok(conn.last_insert_id() as i64)
}

pub fn create_tournament_user(
    conn: &mut PooledConn,
    tournament_id: i64,
    name: &str,
    email: &str,
    password_hash: &str,
    photo_url: Option<&str>,
) -> mysql::Result<i64> {
    conn.exec_drop(
        "INSERT INTO users (name, email, password_hash, user_type, tournament_id, photo_url) VALUES (?, ?, ?, 'tournament', ?, ?)",
        (name, email, password_hash, tournament_id, photo_url),
    )?;
    Ok(conn.last_insert_id() as i64)
}

pub fn find_system_user_by_email(
    conn: &mut PooledConn,
    email: &str,
) -> mysql::Result<Option<UserAuth>> {
    let row: Option<(i64, String)> = conn.exec_first(
        "SELECT id, password_hash FROM users WHERE email = ? AND user_type = 'system' AND tournament_id = 0",
        (email,),
    )?;
    Ok(row.map(|(id, password_hash)| UserAuth { id, password_hash }))
}

pub fn find_tournament_user_by_email(
    conn: &mut PooledConn,
    tournament_id: i64,
    email: &str,
) -> mysql::Result<Option<UserAuth>> {
    let row: Option<(i64, String)> = conn.exec_first(
        "SELECT id, password_hash FROM users
         WHERE email = ?
           AND tournament_id = ?
           AND (user_type = 'tournament' OR user_type IS NULL OR user_type = '')",
        (email, tournament_id),
    )?;
    Ok(row.map(|(id, password_hash)| UserAuth { id, password_hash }))
}

pub fn find_user_by_id(conn: &mut PooledConn, user_id: i64) -> mysql::Result<Option<UserSummary>> {
    let row: Option<(i64, String)> =
        conn.exec_first("SELECT id, name FROM users WHERE id = ?", (user_id,))?;
    Ok(row.map(|(id, name)| UserSummary { id, name }))
}

pub fn find_user_profile_by_id(
    conn: &mut PooledConn,
    user_id: i64,
) -> mysql::Result<Option<CurrentUser>> {
    let row: Option<(i64, String, String, i64, Option<String>)> = conn.exec_first(
        "SELECT id, name, user_type, tournament_id, photo_url FROM users WHERE id = ?",
        (user_id,),
    )?;
    Ok(row.map(
        |(id, name, user_type, tournament_id, photo_url)| CurrentUser {
            id,
            name,
            user_type,
            tournament_id,
            photo_url,
        },
    ))
}

pub fn update_user(
    conn: &mut PooledConn,
    user_id: i64,
    name: &str,
    email: &str,
    photo_url: Option<&str>,
) -> mysql::Result<usize> {
    conn.exec_drop(
        "UPDATE users SET name = ?, email = ?, photo_url = ? WHERE id = ?",
        (name, email, photo_url, user_id),
    )?;
    Ok(conn.affected_rows() as usize)
}
