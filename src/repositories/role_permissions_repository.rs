use mysql::prelude::*;
use mysql::PooledConn;

pub fn list_by_role(conn: &mut PooledConn, role_id: i64) -> mysql::Result<Vec<String>> {
    conn.exec_map(
        "SELECT permission_key FROM role_permissions WHERE role_id = ? ORDER BY id",
        (role_id,),
        |permission_key| permission_key,
    )
}

pub fn replace_for_role(
    conn: &mut PooledConn,
    role_id: i64,
    permissions: &[String],
) -> mysql::Result<()> {
    conn.exec_drop("DELETE FROM role_permissions WHERE role_id = ?", (role_id,))?;
    for permission in permissions {
        conn.exec_drop(
            "INSERT INTO role_permissions (role_id, permission_key) VALUES (?, ?)",
            (role_id, permission),
        )?;
    }
    Ok(())
}
