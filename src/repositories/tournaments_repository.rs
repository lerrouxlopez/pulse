use crate::models::{Tournament, UserSummary};
use mysql::prelude::*;
use mysql::PooledConn;

pub fn get_by_id(conn: &mut PooledConn, tournament_id: i64) -> mysql::Result<Option<Tournament>> {
    let row: Option<(i64, String, String, i64, i64, Option<String>)> = conn.exec_first(
        "SELECT id, name, COALESCE(slug, ''), CAST(is_setup AS SIGNED), user_id, started_at FROM tournaments WHERE id = ?",
        (tournament_id,),
    )?;
    Ok(row.map(
        |(id, name, slug, is_setup_value, user_id, started_at)| Tournament {
            id,
            name,
            slug,
            is_setup: is_setup_value != 0,
            user_id,
            started_at,
        },
    ))
}

pub fn get_by_id_for_user(
    conn: &mut PooledConn,
    tournament_id: i64,
    user_id: i64,
) -> mysql::Result<Option<Tournament>> {
    let row: Option<(i64, String, String, i64, i64, Option<String>)> = conn.exec_first(
        "SELECT id, name, COALESCE(slug, ''), CAST(is_setup AS SIGNED), user_id, started_at FROM tournaments
         WHERE id = ?
           AND (user_id = ? OR EXISTS (
             SELECT 1 FROM tournament_users WHERE tournament_id = ? AND user_id = ?
           ))",
        (tournament_id, user_id, tournament_id, user_id),
    )?;
    Ok(row.map(
        |(id, name, slug, is_setup_value, user_id, started_at)| Tournament {
            id,
            name,
            slug,
            is_setup: is_setup_value != 0,
            user_id,
            started_at,
        },
    ))
}

pub fn list_by_user(conn: &mut PooledConn, user_id: i64) -> mysql::Result<Vec<Tournament>> {
    conn.exec_map(
        "SELECT DISTINCT t.id, t.name, COALESCE(t.slug, ''), CAST(t.is_setup AS SIGNED), t.user_id, t.started_at
         FROM tournaments t
         LEFT JOIN tournament_users tu ON tu.tournament_id = t.id
         WHERE t.user_id = ? OR tu.user_id = ?
         ORDER BY t.id DESC",
        (user_id, user_id),
        |(id, name, slug, is_setup_value, user_id, started_at): (
            i64,
            String,
            String,
            i64,
            i64,
            Option<String>,
        )| Tournament {
            id,
            name,
            slug,
            is_setup: is_setup_value != 0,
            user_id,
            started_at,
        },
    )
}

pub fn create(conn: &mut PooledConn, user_id: i64, name: &str, slug: &str) -> mysql::Result<i64> {
    conn.exec_drop(
        "INSERT INTO tournaments (user_id, name, slug, is_setup) VALUES (?, ?, ?, 0)",
        (user_id, name, slug),
    )?;
    Ok(conn.last_insert_id() as i64)
}

pub fn set_setup(conn: &mut PooledConn, tournament_id: i64, is_setup: bool) -> mysql::Result<()> {
    conn.exec_drop(
        "UPDATE tournaments SET is_setup = ? WHERE id = ?",
        (if is_setup { 1 } else { 0 }, tournament_id),
    )?;
    Ok(())
}

pub fn get_by_slug(conn: &mut PooledConn, slug: &str) -> mysql::Result<Option<Tournament>> {
    let row: Option<(i64, String, String, i64, i64, Option<String>)> = conn.exec_first(
        "SELECT id, name, COALESCE(slug, ''), CAST(is_setup AS SIGNED), user_id, started_at FROM tournaments WHERE slug = ?",
        (slug,),
    )?;
    Ok(row.map(
        |(id, name, slug, is_setup_value, user_id, started_at)| Tournament {
            id,
            name,
            slug,
            is_setup: is_setup_value != 0,
            user_id,
            started_at,
        },
    ))
}

pub fn get_by_slug_for_user(
    conn: &mut PooledConn,
    slug: &str,
    user_id: i64,
) -> mysql::Result<Option<Tournament>> {
    let row: Option<(i64, String, String, i64, i64, Option<String>)> = conn.exec_first(
        "SELECT id, name, COALESCE(slug, ''), CAST(is_setup AS SIGNED), user_id, started_at FROM tournaments
         WHERE slug = ?
           AND (user_id = ? OR EXISTS (
             SELECT 1 FROM tournament_users WHERE tournament_id = tournaments.id AND user_id = ?
           ))",
        (slug, user_id, user_id),
    )?;
    Ok(row.map(
        |(id, name, slug, is_setup_value, user_id, started_at)| Tournament {
            id,
            name,
            slug,
            is_setup: is_setup_value != 0,
            user_id,
            started_at,
        },
    ))
}

pub fn slug_exists(conn: &mut PooledConn, slug: &str) -> mysql::Result<bool> {
    let row: Option<i64> = conn.exec_first(
        "SELECT 1 FROM tournaments WHERE slug = ? LIMIT 1",
        (slug,),
    )?;
    Ok(row.is_some())
}

pub fn user_has_access(
    conn: &mut PooledConn,
    tournament_id: i64,
    user_id: i64,
) -> mysql::Result<bool> {
    let row: Option<i64> = conn.exec_first(
        "SELECT 1
         WHERE EXISTS (SELECT 1 FROM tournaments WHERE id = ? AND user_id = ?)
            OR EXISTS (SELECT 1 FROM tournament_users WHERE tournament_id = ? AND user_id = ?)",
        (tournament_id, user_id, tournament_id, user_id),
    )?;
    Ok(row.is_some())
}

pub fn list_access_users(
    conn: &mut PooledConn,
    tournament_id: i64,
) -> mysql::Result<Vec<UserSummary>> {
    conn.exec_map(
        "SELECT id, name FROM users WHERE id = (SELECT user_id FROM tournaments WHERE id = ?1)
         UNION
         SELECT users.id, users.name
         FROM users
         INNER JOIN tournament_users ON tournament_users.user_id = users.id
         WHERE tournament_users.tournament_id = ?1
         ORDER BY name",
        (tournament_id,),
        |(id, name)| UserSummary { id, name },
    )
}

pub fn list_missing_slugs(conn: &mut PooledConn) -> mysql::Result<Vec<(i64, String)>> {
    conn.exec_map(
        "SELECT id, name FROM tournaments WHERE slug IS NULL OR slug = ''",
        (),
        |(id, name)| (id, name),
    )
}

pub fn update_slug(conn: &mut PooledConn, tournament_id: i64, slug: &str) -> mysql::Result<()> {
    conn.exec_drop(
        "UPDATE tournaments SET slug = ? WHERE id = ?",
        (slug, tournament_id),
    )?;
    Ok(())
}
