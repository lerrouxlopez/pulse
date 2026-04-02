use rusqlite::{params, Connection};

pub fn add_user(conn: &Connection, tournament_id: i64, user_id: i64) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO tournament_users (tournament_id, user_id) VALUES (?1, ?2)",
        params![tournament_id, user_id],
    )?;
    Ok(())
}
