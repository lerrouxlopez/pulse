use crate::models::Tournament;
use mysql::prelude::*;
use mysql::PooledConn;

pub fn get_by_id(conn: &mut PooledConn, tournament_id: i64) -> mysql::Result<Option<Tournament>> {
    let row: Option<(
        i64,
        String,
        String,
        i64,
        i64,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    )> = conn.exec_first(
        "SELECT id, name, COALESCE(slug, ''), CAST(is_setup AS SIGNED), user_id, started_at, logo_url, theme_primary_color, theme_accent_color, theme_background_color, nav_background_color, nav_text_color FROM tournaments WHERE id = ?",
        (tournament_id,),
    )?;
    Ok(row.map(map_tournament))
}

pub fn get_by_id_for_user(
    conn: &mut PooledConn,
    tournament_id: i64,
    user_id: i64,
) -> mysql::Result<Option<Tournament>> {
    let row: Option<(
        i64,
        String,
        String,
        i64,
        i64,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    )> = conn.exec_first(
        "SELECT t.id, t.name, COALESCE(t.slug, ''), CAST(t.is_setup AS SIGNED), t.user_id, t.started_at, t.logo_url, t.theme_primary_color, t.theme_accent_color, t.theme_background_color, t.nav_background_color, t.nav_text_color
         FROM tournaments t
         JOIN users u ON u.id = ?
         WHERE t.id = ?
           AND (
             (u.user_type = 'system' AND t.user_id = u.id)
             OR (u.user_type = 'tournament' AND u.tournament_id = t.id)
             OR u.tournament_id = t.id
           )",
        (user_id, tournament_id),
    )?;
    Ok(row.map(map_tournament))
}

pub fn list_by_user(conn: &mut PooledConn, user_id: i64) -> mysql::Result<Vec<Tournament>> {
    conn.exec_map(
        "SELECT t.id, t.name, COALESCE(t.slug, ''), CAST(t.is_setup AS SIGNED), t.user_id, t.started_at, t.logo_url, t.theme_primary_color, t.theme_accent_color, t.theme_background_color, t.nav_background_color, t.nav_text_color
         FROM tournaments t
         JOIN users u ON u.id = ?
         WHERE (u.user_type = 'system' AND t.user_id = u.id)
            OR (u.user_type = 'tournament' AND t.id = u.tournament_id)
         ORDER BY t.id DESC",
        (user_id,),
        |row: (
            i64,
            String,
            String,
            i64,
            i64,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        )| map_tournament(row),
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
    let row: Option<(
        i64,
        String,
        String,
        i64,
        i64,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    )> = conn.exec_first(
        "SELECT id, name, COALESCE(slug, ''), CAST(is_setup AS SIGNED), user_id, started_at, logo_url, theme_primary_color, theme_accent_color, theme_background_color, nav_background_color, nav_text_color
         FROM tournaments t
         WHERE t.slug = ?
            OR EXISTS (
              SELECT 1 FROM tournament_slug_aliases a
              WHERE a.tournament_id = t.id AND a.old_slug = ?
            )
         LIMIT 1",
        (slug, slug),
    )?;
    Ok(row.map(map_tournament))
}

pub fn get_by_slug_for_user(
    conn: &mut PooledConn,
    slug: &str,
    user_id: i64,
) -> mysql::Result<Option<Tournament>> {
    let row: Option<(
        i64,
        String,
        String,
        i64,
        i64,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    )> = conn.exec_first(
        "SELECT t.id, t.name, COALESCE(t.slug, ''), CAST(t.is_setup AS SIGNED), t.user_id, t.started_at, t.logo_url, t.theme_primary_color, t.theme_accent_color, t.theme_background_color, t.nav_background_color, t.nav_text_color
         FROM tournaments t
         JOIN users u ON u.id = ?
         WHERE (
             t.slug = ?
             OR EXISTS (
               SELECT 1 FROM tournament_slug_aliases a
               WHERE a.tournament_id = t.id AND a.old_slug = ?
             )
           )
           AND (
             (u.user_type = 'system' AND t.user_id = u.id)
             OR (u.user_type = 'tournament' AND u.tournament_id = t.id)
             OR u.tournament_id = t.id
           )",
        (user_id, slug, slug),
    )?;
    Ok(row.map(map_tournament))
}

pub fn slug_exists(conn: &mut PooledConn, slug: &str) -> mysql::Result<bool> {
    let row: Option<i64> =
        conn.exec_first("SELECT 1 FROM tournaments WHERE slug = ? LIMIT 1", (slug,))?;
    Ok(row.is_some())
}

pub fn user_has_access(
    conn: &mut PooledConn,
    tournament_id: i64,
    user_id: i64,
) -> mysql::Result<bool> {
    let row: Option<i64> = conn.exec_first(
        "SELECT 1
         FROM users u
         WHERE u.id = ?
           AND (
             (u.user_type = 'system' AND EXISTS (
                SELECT 1 FROM tournaments t WHERE t.id = ? AND t.user_id = u.id
             ))
             OR (u.user_type = 'tournament' AND u.tournament_id = ?)
             OR u.tournament_id = ?
           )",
        (user_id, tournament_id, tournament_id, tournament_id),
    )?;
    Ok(row.is_some())
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

pub fn create_slug_alias(
    conn: &mut PooledConn,
    tournament_id: i64,
    old_slug: &str,
) -> mysql::Result<()> {
    conn.exec_drop(
        "INSERT IGNORE INTO tournament_slug_aliases (tournament_id, old_slug) VALUES (?, ?)",
        (tournament_id, old_slug),
    )?;
    Ok(())
}

pub fn update_name(conn: &mut PooledConn, tournament_id: i64, name: &str) -> mysql::Result<()> {
    conn.exec_drop(
        "UPDATE tournaments SET name = ? WHERE id = ?",
        (name, tournament_id),
    )?;
    Ok(())
}

pub fn update_branding(
    conn: &mut PooledConn,
    tournament_id: i64,
    logo_url: Option<&str>,
    theme_primary_color: Option<&str>,
    theme_accent_color: Option<&str>,
    theme_background_color: Option<&str>,
    nav_background_color: Option<&str>,
    nav_text_color: Option<&str>,
) -> mysql::Result<()> {
    conn.exec_drop(
        "UPDATE tournaments
         SET logo_url = ?, theme_primary_color = ?, theme_accent_color = ?, theme_background_color = ?, nav_background_color = ?, nav_text_color = ?
         WHERE id = ?",
        (
            logo_url,
            theme_primary_color,
            theme_accent_color,
            theme_background_color,
            nav_background_color,
            nav_text_color,
            tournament_id,
        ),
    )?;
    Ok(())
}

fn map_tournament(
    (
        id,
        name,
        slug,
        is_setup_value,
        user_id,
        started_at,
        logo_url,
        theme_primary_color,
        theme_accent_color,
        theme_background_color,
        nav_background_color,
        nav_text_color,
    ): (
        i64,
        String,
        String,
        i64,
        i64,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ),
) -> Tournament {
    Tournament {
        id,
        name,
        slug,
        is_setup: is_setup_value != 0,
        user_id,
        started_at,
        logo_url,
        theme_primary_color,
        theme_accent_color,
        theme_background_color,
        nav_background_color,
        nav_text_color,
    }
}
