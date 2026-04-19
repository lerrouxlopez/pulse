use crate::models::Role;
use mysql::prelude::*;
use mysql::PooledConn;

pub fn list(conn: &mut PooledConn, tournament_id: i64) -> mysql::Result<Vec<Role>> {
    conn.exec_map(
        "SELECT id, name, CAST(is_owner AS SIGNED)
         FROM tournament_roles
         WHERE tournament_id = ?
         ORDER BY id",
        (tournament_id,),
        |(id, name, is_owner_value): (i64, String, i64)| Role {
            id,
            name,
            is_owner: is_owner_value != 0,
            permissions: Vec::new(),
        },
    )
}

pub fn get_owner_role_id(conn: &mut PooledConn, tournament_id: i64) -> mysql::Result<Option<i64>> {
    conn.exec_first(
        "SELECT id FROM tournament_roles WHERE tournament_id = ? AND is_owner = 1 LIMIT 1",
        (tournament_id,),
    )
}

pub fn create(
    conn: &mut PooledConn,
    tournament_id: i64,
    name: &str,
    is_owner: bool,
) -> mysql::Result<i64> {
    conn.exec_drop(
        "INSERT INTO tournament_roles (tournament_id, name, is_owner) VALUES (?, ?, ?)",
        (tournament_id, name, if is_owner { 1 } else { 0 }),
    )?;
    Ok(conn.last_insert_id() as i64)
}

pub fn delete(conn: &mut PooledConn, tournament_id: i64, role_id: i64) -> mysql::Result<usize> {
    conn.exec_drop(
        "DELETE FROM tournament_roles WHERE id = ? AND tournament_id = ? AND is_owner = 0",
        (role_id, tournament_id),
    )?;
    Ok(conn.affected_rows() as usize)
}
