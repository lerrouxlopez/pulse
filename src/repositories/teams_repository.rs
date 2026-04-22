use crate::models::{NamedItem, Team, TeamMember};
use mysql::prelude::*;
use mysql::PooledConn;

pub fn list_teams(conn: &mut PooledConn, tournament_id: i64) -> mysql::Result<Vec<Team>> {
    conn.exec_map(
        "SELECT id, name, logo_url FROM teams WHERE tournament_id = ? ORDER BY id",
        (tournament_id,),
        |(id, name, logo_url)| Team {
            id,
            name,
            logo_url,
            members: Vec::new(),
            divisions: Vec::new(),
            categories: Vec::new(),
            events: Vec::new(),
            division_ids: Vec::new(),
            category_ids: Vec::new(),
            event_ids: Vec::new(),
        },
    )
}

pub fn list_members(conn: &mut PooledConn, tournament_id: i64) -> mysql::Result<Vec<TeamMember>> {
    conn.exec_map(
        "SELECT id, name, team_id, notes, weight_class, weight_class_id, division_id, photo_url
         FROM team_members
         WHERE tournament_id = ?
         ORDER BY id",
        (tournament_id,),
        |(id, name, team_id, notes, weight_class, weight_class_id, division_id, photo_url)| {
            TeamMember {
                id,
                name,
                team_id,
                notes,
                weight_class,
                weight_class_id,
                division_id,
                division_name: None,
                category_ids: Vec::new(),
                event_ids: Vec::new(),
                photo_url,
            }
        },
    )
}

pub fn create_team(
    conn: &mut PooledConn,
    tournament_id: i64,
    name: &str,
    logo_url: Option<&str>,
) -> mysql::Result<i64> {
    conn.exec_drop(
        "INSERT INTO teams (tournament_id, name, logo_url) VALUES (?, ?, ?)",
        (tournament_id, name, logo_url),
    )?;
    Ok(conn.last_insert_id() as i64)
}

pub fn update_team(
    conn: &mut PooledConn,
    tournament_id: i64,
    id: i64,
    name: &str,
    logo_url: Option<&str>,
) -> mysql::Result<usize> {
    conn.exec_drop(
        "UPDATE teams SET name = ?, logo_url = ? WHERE id = ? AND tournament_id = ?",
        (name, logo_url, id, tournament_id),
    )?;
    Ok(conn.affected_rows() as usize)
}

pub fn team_exists(conn: &mut PooledConn, tournament_id: i64, id: i64) -> mysql::Result<bool> {
    let count: Option<i64> = conn.exec_first(
        "SELECT COUNT(*) FROM teams WHERE id = ? AND tournament_id = ?",
        (id, tournament_id),
    )?;
    Ok(count.unwrap_or(0) > 0)
}

pub fn member_exists(conn: &mut PooledConn, tournament_id: i64, member_id: i64) -> mysql::Result<bool> {
    let count: Option<i64> = conn.exec_first(
        "SELECT COUNT(*) FROM teams_members WHERE id = ? AND tournament_id = ?",
        (member_id, tournament_id),
    )?;
    Ok(count.unwrap_or(0) > 0)
}

pub fn delete_team(conn: &mut PooledConn, tournament_id: i64, id: i64) -> mysql::Result<usize> {
    conn.exec_drop(
        "DELETE FROM teams WHERE id = ? AND tournament_id = ?",
        (id, tournament_id),
    )?;
    Ok(conn.affected_rows() as usize)
}

pub fn create_member(
    conn: &mut PooledConn,
    tournament_id: i64,
    team_id: i64,
    name: &str,
    notes: Option<&str>,
    weight_class: Option<&str>,
    weight_class_id: Option<i64>,
    division_id: Option<i64>,
    photo_url: Option<&str>,
) -> mysql::Result<i64> {
    conn.exec_drop(
        "INSERT INTO team_members (tournament_id, team_id, name, notes, weight_class, weight_class_id, division_id, photo_url)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        (
            tournament_id,
            team_id,
            name,
            notes,
            weight_class,
            weight_class_id,
            division_id,
            photo_url,
        ),
    )?;
    Ok(conn.last_insert_id() as i64)
}

pub fn delete_member(conn: &mut PooledConn, tournament_id: i64, id: i64) -> mysql::Result<usize> {
    conn.exec_drop(
        "DELETE FROM team_members WHERE id = ? AND tournament_id = ?",
        (id, tournament_id),
    )?;
    Ok(conn.affected_rows() as usize)
}

pub fn update_member(
    conn: &mut PooledConn,
    tournament_id: i64,
    member_id: i64,
    name: &str,
    notes: Option<&str>,
    weight_class: Option<&str>,
    weight_class_id: Option<i64>,
    division_id: Option<i64>,
    photo_url: Option<&str>,
) -> mysql::Result<usize> {
    conn.exec_drop(
        "UPDATE team_members
         SET name = ?, notes = ?, weight_class = ?, weight_class_id = ?, division_id = ?, photo_url = ?
         WHERE id = ? AND tournament_id = ?",
        (
            name,
            notes,
            weight_class,
            weight_class_id,
            division_id,
            photo_url,
            member_id,
            tournament_id,
        ),
    )?;
    Ok(conn.affected_rows() as usize)
}

pub fn update_member_photo(
    conn: &mut PooledConn,
    tournament_id: i64,
    member_id: i64,
    photo_url: Option<&str>,
) -> mysql::Result<usize> {
    conn.exec_drop(
        "UPDATE team_members SET photo_url = ? WHERE id = ? AND tournament_id = ?",
        (photo_url, member_id, tournament_id),
    )?;
    Ok(conn.affected_rows() as usize)
}

pub fn get_member(
    conn: &mut PooledConn,
    tournament_id: i64,
    member_id: i64,
) -> mysql::Result<Option<TeamMember>> {
    let row: Option<(
        i64,
        String,
        i64,
        Option<String>,
        Option<String>,
        Option<i64>,
        Option<i64>,
        Option<String>,
    )> = conn.exec_first(
        "SELECT id, name, team_id, notes, weight_class, weight_class_id, division_id, photo_url
             FROM team_members
             WHERE id = ? AND tournament_id = ?",
        (member_id, tournament_id),
    )?;
    Ok(row.map(
        |(id, name, team_id, notes, weight_class, weight_class_id, division_id, photo_url)| {
            TeamMember {
                id,
                name,
                team_id,
                notes,
                weight_class,
                weight_class_id,
                division_id,
                division_name: None,
                category_ids: Vec::new(),
                event_ids: Vec::new(),
                photo_url,
            }
        },
    ))
}

pub fn get_team_logo(
    conn: &mut PooledConn,
    tournament_id: i64,
    team_id: i64,
) -> mysql::Result<Option<String>> {
    conn.exec_first(
        "SELECT logo_url FROM teams WHERE id = ? AND tournament_id = ?",
        (team_id, tournament_id),
    )
}

pub fn list_team_divisions(
    conn: &mut PooledConn,
    tournament_id: i64,
) -> mysql::Result<Vec<(i64, NamedItem)>> {
    conn.exec_map(
        "SELECT td.team_id, d.id, d.name
         FROM team_divisions td
         JOIN divisions d ON d.id = td.division_id
         WHERE td.tournament_id = ?
         ORDER BY d.id",
        (tournament_id,),
        |(team_id, id, name)| (team_id, NamedItem { id, name }),
    )
}

pub fn list_team_categories(
    conn: &mut PooledConn,
    tournament_id: i64,
) -> mysql::Result<Vec<(i64, NamedItem)>> {
    conn.exec_map(
        "SELECT tc.team_id, c.id, c.name
         FROM team_categories tc
         JOIN categories c ON c.id = tc.category_id
         WHERE tc.tournament_id = ?
         ORDER BY c.id",
        (tournament_id,),
        |(team_id, id, name)| (team_id, NamedItem { id, name }),
    )
}

pub fn list_team_events(
    conn: &mut PooledConn,
    tournament_id: i64,
) -> mysql::Result<Vec<(i64, NamedItem)>> {
    conn.exec_map(
        "SELECT te.team_id, e.id, e.name
         FROM team_events te
         JOIN events e ON e.id = te.event_id
         WHERE te.tournament_id = ?
         ORDER BY e.id",
        (tournament_id,),
        |(team_id, id, name)| (team_id, NamedItem { id, name }),
    )
}

pub fn list_event_competitors(
    conn: &mut PooledConn,
    tournament_id: i64,
    event_id: i64,
) -> mysql::Result<
    Vec<(
        i64,
        i64,
        String,
        Option<String>,
        Option<i64>,
        Option<i64>,
        Option<String>,
    )>,
> {
    conn.exec_map(
        "SELECT tm.id, tm.team_id, tm.name, tm.photo_url, tm.division_id, tm.weight_class_id, tm.weight_class
         FROM team_members tm
         JOIN team_member_events tme ON tme.member_id = tm.id
         WHERE tm.tournament_id = ? AND tme.tournament_id = ? AND tme.event_id = ?
         ORDER BY tm.id",
        (tournament_id, tournament_id, event_id),
        |(id, team_id, name, photo_url, division_id, weight_class_id, weight_class)| {
            (id, team_id, name, photo_url, division_id, weight_class_id, weight_class)
        },
    )
}

pub fn list_member_categories(
    conn: &mut PooledConn,
    tournament_id: i64,
) -> mysql::Result<Vec<(i64, NamedItem)>> {
    conn.exec_map(
        "SELECT tmc.member_id, c.id, c.name
         FROM team_member_categories tmc
         JOIN categories c ON c.id = tmc.category_id
         WHERE tmc.tournament_id = ?
         ORDER BY c.id",
        (tournament_id,),
        |(member_id, id, name)| (member_id, NamedItem { id, name }),
    )
}

pub fn list_member_events(
    conn: &mut PooledConn,
    tournament_id: i64,
) -> mysql::Result<Vec<(i64, NamedItem)>> {
    conn.exec_map(
        "SELECT tme.member_id, e.id, e.name
         FROM team_member_events tme
         JOIN events e ON e.id = tme.event_id
         WHERE tme.tournament_id = ?
         ORDER BY e.id",
        (tournament_id,),
        |(member_id, id, name)| (member_id, NamedItem { id, name }),
    )
}

pub fn clear_member_categories(
    conn: &mut PooledConn,
    tournament_id: i64,
    member_id: i64,
) -> mysql::Result<()> {
    conn.exec_drop(
        "DELETE FROM team_member_categories WHERE member_id = ? AND tournament_id = ?",
        (member_id, tournament_id),
    )?;
    Ok(())
}

pub fn add_member_category(
    conn: &mut PooledConn,
    tournament_id: i64,
    team_id: i64,
    member_id: i64,
    category_id: i64,
) -> mysql::Result<()> {
    conn.exec_drop(
        "INSERT INTO team_member_categories (tournament_id, team_id, member_id, category_id)
         VALUES (?, ?, ?, ?)",
        (tournament_id, team_id, member_id, category_id),
    )?;
    Ok(())
}

pub fn clear_member_events(
    conn: &mut PooledConn,
    tournament_id: i64,
    member_id: i64,
) -> mysql::Result<()> {
    conn.exec_drop(
        "DELETE FROM team_member_events WHERE member_id = ? AND tournament_id = ?",
        (member_id, tournament_id),
    )?;
    Ok(())
}

pub fn add_member_event(
    conn: &mut PooledConn,
    tournament_id: i64,
    team_id: i64,
    member_id: i64,
    event_id: i64,
) -> mysql::Result<()> {
    conn.exec_drop(
        "INSERT INTO team_member_events (tournament_id, team_id, member_id, event_id)
         VALUES (?, ?, ?, ?)",
        (tournament_id, team_id, member_id, event_id),
    )?;
    Ok(())
}

pub fn clear_team_divisions(
    conn: &mut PooledConn,
    tournament_id: i64,
    team_id: i64,
) -> mysql::Result<()> {
    conn.exec_drop(
        "DELETE FROM team_divisions WHERE team_id = ? AND tournament_id = ?",
        (team_id, tournament_id),
    )?;
    Ok(())
}

pub fn add_team_division(
    conn: &mut PooledConn,
    tournament_id: i64,
    team_id: i64,
    division_id: i64,
) -> mysql::Result<()> {
    conn.exec_drop(
        "INSERT INTO team_divisions (tournament_id, team_id, division_id) VALUES (?, ?, ?)",
        (tournament_id, team_id, division_id),
    )?;
    Ok(())
}

pub fn clear_team_categories(
    conn: &mut PooledConn,
    tournament_id: i64,
    team_id: i64,
) -> mysql::Result<()> {
    conn.exec_drop(
        "DELETE FROM team_categories WHERE team_id = ? AND tournament_id = ?",
        (team_id, tournament_id),
    )?;
    Ok(())
}

pub fn add_team_category(
    conn: &mut PooledConn,
    tournament_id: i64,
    team_id: i64,
    category_id: i64,
) -> mysql::Result<()> {
    conn.exec_drop(
        "INSERT INTO team_categories (tournament_id, team_id, category_id) VALUES (?, ?, ?)",
        (tournament_id, team_id, category_id),
    )?;
    Ok(())
}

pub fn clear_team_events(
    conn: &mut PooledConn,
    tournament_id: i64,
    team_id: i64,
) -> mysql::Result<()> {
    conn.exec_drop(
        "DELETE FROM team_events WHERE team_id = ? AND tournament_id = ?",
        (team_id, tournament_id),
    )?;
    Ok(())
}

pub fn add_team_event(
    conn: &mut PooledConn,
    tournament_id: i64,
    team_id: i64,
    event_id: i64,
) -> mysql::Result<()> {
    conn.exec_drop(
        "INSERT INTO team_events (tournament_id, team_id, event_id) VALUES (?, ?, ?)",
        (tournament_id, team_id, event_id),
    )?;
    Ok(())
}
