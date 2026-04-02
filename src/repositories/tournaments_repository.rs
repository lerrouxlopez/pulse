use crate::models::{Tournament, UserSummary};
use rusqlite::{params, Connection};

pub fn get_by_id(conn: &Connection, tournament_id: i64) -> rusqlite::Result<Option<Tournament>> {
    let mut stmt =
        conn.prepare("SELECT id, name, slug, is_setup, user_id, started_at FROM tournaments WHERE id = ?1")?;
    let mut rows = stmt.query(params![tournament_id])?;
    if let Some(row) = rows.next()? {
        let is_setup_value: i64 = row.get(3)?;
        Ok(Some(Tournament {
            id: row.get(0)?,
            name: row.get(1)?,
            slug: row.get(2)?,
            is_setup: is_setup_value != 0,
            user_id: row.get(4)?,
            started_at: row.get(5)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn get_by_id_for_user(
    conn: &Connection,
    tournament_id: i64,
    user_id: i64,
) -> rusqlite::Result<Option<Tournament>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, slug, is_setup, user_id, started_at FROM tournaments
         WHERE id = ?1
           AND (user_id = ?2 OR EXISTS (
             SELECT 1 FROM tournament_users WHERE tournament_id = ?1 AND user_id = ?2
           ))",
    )?;
    let mut rows = stmt.query(params![tournament_id, user_id])?;
    if let Some(row) = rows.next()? {
        let is_setup_value: i64 = row.get(3)?;
        Ok(Some(Tournament {
            id: row.get(0)?,
            name: row.get(1)?,
            slug: row.get(2)?,
            is_setup: is_setup_value != 0,
            user_id: row.get(4)?,
            started_at: row.get(5)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn list_by_user(conn: &Connection, user_id: i64) -> rusqlite::Result<Vec<Tournament>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT t.id, t.name, t.slug, t.is_setup, t.user_id, t.started_at
         FROM tournaments t
         LEFT JOIN tournament_users tu ON tu.tournament_id = t.id
         WHERE t.user_id = ?1 OR tu.user_id = ?1
         ORDER BY t.id DESC",
    )?;
    let rows = stmt.query_map(params![user_id], |row| {
        let is_setup_value: i64 = row.get(3)?;
        Ok(Tournament {
            id: row.get(0)?,
            name: row.get(1)?,
            slug: row.get(2)?,
            is_setup: is_setup_value != 0,
            user_id: row.get(4)?,
            started_at: row.get(5)?,
        })
    })?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok(items)
}

pub fn create(conn: &Connection, user_id: i64, name: &str, slug: &str) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO tournaments (user_id, name, slug, is_setup) VALUES (?1, ?2, ?3, 0)",
        params![user_id, name, slug],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn set_setup(conn: &Connection, tournament_id: i64, is_setup: bool) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE tournaments SET is_setup = ?1 WHERE id = ?2",
        params![if is_setup { 1 } else { 0 }, tournament_id],
    )?;
    Ok(())
}

pub fn get_by_slug(conn: &Connection, slug: &str) -> rusqlite::Result<Option<Tournament>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, slug, is_setup, user_id, started_at FROM tournaments WHERE slug = ?1",
    )?;
    let mut rows = stmt.query(params![slug])?;
    if let Some(row) = rows.next()? {
        let is_setup_value: i64 = row.get(3)?;
        Ok(Some(Tournament {
            id: row.get(0)?,
            name: row.get(1)?,
            slug: row.get(2)?,
            is_setup: is_setup_value != 0,
            user_id: row.get(4)?,
            started_at: row.get(5)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn get_by_slug_for_user(
    conn: &Connection,
    slug: &str,
    user_id: i64,
) -> rusqlite::Result<Option<Tournament>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, slug, is_setup, user_id, started_at FROM tournaments
         WHERE slug = ?1
           AND (user_id = ?2 OR EXISTS (
             SELECT 1 FROM tournament_users WHERE tournament_id = tournaments.id AND user_id = ?2
           ))",
    )?;
    let mut rows = stmt.query(params![slug, user_id])?;
    if let Some(row) = rows.next()? {
        let is_setup_value: i64 = row.get(3)?;
        Ok(Some(Tournament {
            id: row.get(0)?,
            name: row.get(1)?,
            slug: row.get(2)?,
            is_setup: is_setup_value != 0,
            user_id: row.get(4)?,
            started_at: row.get(5)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn slug_exists(conn: &Connection, slug: &str) -> rusqlite::Result<bool> {
    let mut stmt = conn.prepare("SELECT 1 FROM tournaments WHERE slug = ?1 LIMIT 1")?;
    let mut rows = stmt.query(params![slug])?;
    Ok(rows.next()?.is_some())
}

pub fn user_has_access(
    conn: &Connection,
    tournament_id: i64,
    user_id: i64,
) -> rusqlite::Result<bool> {
    let mut stmt = conn.prepare(
        "SELECT 1
         WHERE EXISTS (SELECT 1 FROM tournaments WHERE id = ?1 AND user_id = ?2)
            OR EXISTS (SELECT 1 FROM tournament_users WHERE tournament_id = ?1 AND user_id = ?2)",
    )?;
    let mut rows = stmt.query(params![tournament_id, user_id])?;
    Ok(rows.next()?.is_some())
}

pub fn list_access_users(conn: &Connection, tournament_id: i64) -> rusqlite::Result<Vec<UserSummary>> {
    let mut stmt = conn.prepare(
        "SELECT id, name FROM users WHERE id = (SELECT user_id FROM tournaments WHERE id = ?1)
         UNION
         SELECT users.id, users.name
         FROM users
         INNER JOIN tournament_users ON tournament_users.user_id = users.id
         WHERE tournament_users.tournament_id = ?1
         ORDER BY name",
    )?;
    let rows = stmt.query_map(params![tournament_id], |row| {
        Ok(UserSummary {
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

pub fn list_missing_slugs(conn: &Connection) -> rusqlite::Result<Vec<(i64, String)>> {
    let mut stmt =
        conn.prepare("SELECT id, name FROM tournaments WHERE slug IS NULL OR slug = ''")?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok(items)
}

pub fn update_slug(conn: &Connection, tournament_id: i64, slug: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE tournaments SET slug = ?1 WHERE id = ?2",
        params![slug, tournament_id],
    )?;
    Ok(())
}
