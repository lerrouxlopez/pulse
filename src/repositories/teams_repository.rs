use crate::models::{NamedItem, Team, TeamMember};
use rusqlite::{params, Connection};

pub fn list_teams(conn: &Connection, tournament_id: i64) -> rusqlite::Result<Vec<Team>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, logo_url FROM teams WHERE tournament_id = ?1 ORDER BY id",
    )?;
    let rows = stmt.query_map(params![tournament_id], |row| {
        Ok(Team {
            id: row.get(0)?,
            name: row.get(1)?,
            logo_url: row.get(2)?,
            members: Vec::new(),
            divisions: Vec::new(),
            categories: Vec::new(),
            events: Vec::new(),
            division_ids: Vec::new(),
            category_ids: Vec::new(),
            event_ids: Vec::new(),
        })
    })?;
    let mut teams = Vec::new();
    for row in rows {
        teams.push(row?);
    }
    Ok(teams)
}

pub fn list_members(conn: &Connection, tournament_id: i64) -> rusqlite::Result<Vec<TeamMember>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, team_id, notes, weight_class, division_id, photo_url
         FROM team_members
         WHERE tournament_id = ?1
         ORDER BY id",
    )?;
    let rows = stmt.query_map(params![tournament_id], |row| {
        Ok(TeamMember {
            id: row.get(0)?,
            name: row.get(1)?,
            team_id: row.get(2)?,
            notes: row.get(3)?,
            weight_class: row.get(4)?,
            division_id: row.get(5)?,
            division_name: None,
            category_ids: Vec::new(),
            event_ids: Vec::new(),
            photo_url: row.get(6)?,
        })
    })?;
    let mut members = Vec::new();
    for row in rows {
        members.push(row?);
    }
    Ok(members)
}

pub fn create_team(
    conn: &Connection,
    tournament_id: i64,
    name: &str,
    logo_url: Option<&str>,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO teams (tournament_id, name, logo_url) VALUES (?1, ?2, ?3)",
        params![tournament_id, name, logo_url],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_team(
    conn: &Connection,
    tournament_id: i64,
    id: i64,
    name: &str,
    logo_url: Option<&str>,
) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE teams SET name = ?1, logo_url = ?2 WHERE id = ?3 AND tournament_id = ?4",
        params![name, logo_url, id, tournament_id],
    )
}

pub fn delete_team(conn: &Connection, tournament_id: i64, id: i64) -> rusqlite::Result<usize> {
    conn.execute(
        "DELETE FROM teams WHERE id = ?1 AND tournament_id = ?2",
        params![id, tournament_id],
    )
}

pub fn create_member(
    conn: &Connection,
    tournament_id: i64,
    team_id: i64,
    name: &str,
    notes: Option<&str>,
    weight_class: Option<&str>,
    division_id: Option<i64>,
    photo_url: Option<&str>,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO team_members (tournament_id, team_id, name, notes, weight_class, division_id, photo_url)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            tournament_id,
            team_id,
            name,
            notes,
            weight_class,
            division_id,
            photo_url
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_member(
    conn: &Connection,
    tournament_id: i64,
    id: i64,
) -> rusqlite::Result<usize> {
    conn.execute(
        "DELETE FROM team_members WHERE id = ?1 AND tournament_id = ?2",
        params![id, tournament_id],
    )
}

pub fn update_member(
    conn: &Connection,
    tournament_id: i64,
    member_id: i64,
    name: &str,
    notes: Option<&str>,
    weight_class: Option<&str>,
    division_id: Option<i64>,
    photo_url: Option<&str>,
) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE team_members
         SET name = ?1, notes = ?2, weight_class = ?3, division_id = ?4, photo_url = ?5
         WHERE id = ?6 AND tournament_id = ?7",
        params![
            name,
            notes,
            weight_class,
            division_id,
            photo_url,
            member_id,
            tournament_id
        ],
    )
}

pub fn update_member_photo(
    conn: &Connection,
    tournament_id: i64,
    member_id: i64,
    photo_url: Option<&str>,
) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE team_members SET photo_url = ?1 WHERE id = ?2 AND tournament_id = ?3",
        params![photo_url, member_id, tournament_id],
    )
}

pub fn get_member(
    conn: &Connection,
    tournament_id: i64,
    member_id: i64,
) -> rusqlite::Result<Option<TeamMember>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, team_id, notes, weight_class, division_id, photo_url
         FROM team_members
         WHERE id = ?1 AND tournament_id = ?2",
    )?;
    let mut rows = stmt.query(params![member_id, tournament_id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(TeamMember {
            id: row.get(0)?,
            name: row.get(1)?,
            team_id: row.get(2)?,
            notes: row.get(3)?,
            weight_class: row.get(4)?,
            division_id: row.get(5)?,
            division_name: None,
            category_ids: Vec::new(),
            event_ids: Vec::new(),
            photo_url: row.get(6)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn get_team_logo(
    conn: &Connection,
    tournament_id: i64,
    team_id: i64,
) -> rusqlite::Result<Option<String>> {
    let mut stmt =
        conn.prepare("SELECT logo_url FROM teams WHERE id = ?1 AND tournament_id = ?2")?;
    let mut rows = stmt.query(params![team_id, tournament_id])?;
    if let Some(row) = rows.next()? {
        Ok(row.get(0)?)
    } else {
        Ok(None)
    }
}

pub fn list_team_divisions(
    conn: &Connection,
    tournament_id: i64,
) -> rusqlite::Result<Vec<(i64, NamedItem)>> {
    let mut stmt = conn.prepare(
        "SELECT td.team_id, d.id, d.name
         FROM team_divisions td
         JOIN divisions d ON d.id = td.division_id
         WHERE td.tournament_id = ?1
         ORDER BY d.id",
    )?;
    let rows = stmt.query_map(params![tournament_id], |row| {
        Ok((
            row.get(0)?,
            NamedItem {
                id: row.get(1)?,
                name: row.get(2)?,
            },
        ))
    })?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok(items)
}

pub fn list_team_categories(
    conn: &Connection,
    tournament_id: i64,
) -> rusqlite::Result<Vec<(i64, NamedItem)>> {
    let mut stmt = conn.prepare(
        "SELECT tc.team_id, c.id, c.name
         FROM team_categories tc
         JOIN categories c ON c.id = tc.category_id
         WHERE tc.tournament_id = ?1
         ORDER BY c.id",
    )?;
    let rows = stmt.query_map(params![tournament_id], |row| {
        Ok((
            row.get(0)?,
            NamedItem {
                id: row.get(1)?,
                name: row.get(2)?,
            },
        ))
    })?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok(items)
}

pub fn list_team_events(
    conn: &Connection,
    tournament_id: i64,
) -> rusqlite::Result<Vec<(i64, NamedItem)>> {
    let mut stmt = conn.prepare(
        "SELECT te.team_id, e.id, e.name
         FROM team_events te
         JOIN events e ON e.id = te.event_id
         WHERE te.tournament_id = ?1
         ORDER BY e.id",
    )?;
    let rows = stmt.query_map(params![tournament_id], |row| {
        Ok((
            row.get(0)?,
            NamedItem {
                id: row.get(1)?,
                name: row.get(2)?,
            },
        ))
    })?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok(items)
}

pub fn list_member_categories(
    conn: &Connection,
    tournament_id: i64,
) -> rusqlite::Result<Vec<(i64, NamedItem)>> {
    let mut stmt = conn.prepare(
        "SELECT tmc.member_id, c.id, c.name
         FROM team_member_categories tmc
         JOIN categories c ON c.id = tmc.category_id
         WHERE tmc.tournament_id = ?1
         ORDER BY c.id",
    )?;
    let rows = stmt.query_map(params![tournament_id], |row| {
        Ok((
            row.get(0)?,
            NamedItem {
                id: row.get(1)?,
                name: row.get(2)?,
            },
        ))
    })?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok(items)
}

pub fn list_member_events(
    conn: &Connection,
    tournament_id: i64,
) -> rusqlite::Result<Vec<(i64, NamedItem)>> {
    let mut stmt = conn.prepare(
        "SELECT tme.member_id, e.id, e.name
         FROM team_member_events tme
         JOIN events e ON e.id = tme.event_id
         WHERE tme.tournament_id = ?1
         ORDER BY e.id",
    )?;
    let rows = stmt.query_map(params![tournament_id], |row| {
        Ok((
            row.get(0)?,
            NamedItem {
                id: row.get(1)?,
                name: row.get(2)?,
            },
        ))
    })?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok(items)
}

pub fn clear_member_categories(
    conn: &Connection,
    tournament_id: i64,
    member_id: i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM team_member_categories WHERE member_id = ?1 AND tournament_id = ?2",
        params![member_id, tournament_id],
    )?;
    Ok(())
}

pub fn add_member_category(
    conn: &Connection,
    tournament_id: i64,
    team_id: i64,
    member_id: i64,
    category_id: i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO team_member_categories (tournament_id, team_id, member_id, category_id)
         VALUES (?1, ?2, ?3, ?4)",
        params![tournament_id, team_id, member_id, category_id],
    )?;
    Ok(())
}

pub fn clear_member_events(
    conn: &Connection,
    tournament_id: i64,
    member_id: i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM team_member_events WHERE member_id = ?1 AND tournament_id = ?2",
        params![member_id, tournament_id],
    )?;
    Ok(())
}

pub fn add_member_event(
    conn: &Connection,
    tournament_id: i64,
    team_id: i64,
    member_id: i64,
    event_id: i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO team_member_events (tournament_id, team_id, member_id, event_id)
         VALUES (?1, ?2, ?3, ?4)",
        params![tournament_id, team_id, member_id, event_id],
    )?;
    Ok(())
}

pub fn clear_team_divisions(
    conn: &Connection,
    tournament_id: i64,
    team_id: i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM team_divisions WHERE team_id = ?1 AND tournament_id = ?2",
        params![team_id, tournament_id],
    )?;
    Ok(())
}

pub fn add_team_division(
    conn: &Connection,
    tournament_id: i64,
    team_id: i64,
    division_id: i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO team_divisions (tournament_id, team_id, division_id) VALUES (?1, ?2, ?3)",
        params![tournament_id, team_id, division_id],
    )?;
    Ok(())
}

pub fn clear_team_categories(
    conn: &Connection,
    tournament_id: i64,
    team_id: i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM team_categories WHERE team_id = ?1 AND tournament_id = ?2",
        params![team_id, tournament_id],
    )?;
    Ok(())
}

pub fn add_team_category(
    conn: &Connection,
    tournament_id: i64,
    team_id: i64,
    category_id: i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO team_categories (tournament_id, team_id, category_id) VALUES (?1, ?2, ?3)",
        params![tournament_id, team_id, category_id],
    )?;
    Ok(())
}

pub fn clear_team_events(
    conn: &Connection,
    tournament_id: i64,
    team_id: i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM team_events WHERE team_id = ?1 AND tournament_id = ?2",
        params![team_id, tournament_id],
    )?;
    Ok(())
}

pub fn add_team_event(
    conn: &Connection,
    tournament_id: i64,
    team_id: i64,
    event_id: i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO team_events (tournament_id, team_id, event_id) VALUES (?1, ?2, ?3)",
        params![tournament_id, team_id, event_id],
    )?;
    Ok(())
}
