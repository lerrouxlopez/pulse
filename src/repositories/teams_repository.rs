use crate::models::{Team, TeamMember};
use rusqlite::{params, Connection};

pub fn list_teams(conn: &Connection, tournament_id: i64) -> rusqlite::Result<Vec<Team>> {
    let mut stmt = conn.prepare(
        "SELECT id, name FROM teams WHERE tournament_id = ?1 ORDER BY id",
    )?;
    let rows = stmt.query_map(params![tournament_id], |row| {
        Ok(Team {
            id: row.get(0)?,
            name: row.get(1)?,
            members: Vec::new(),
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
        "SELECT id, name, team_id FROM team_members WHERE tournament_id = ?1 ORDER BY id",
    )?;
    let rows = stmt.query_map(params![tournament_id], |row| {
        Ok(TeamMember {
            id: row.get(0)?,
            name: row.get(1)?,
            team_id: row.get(2)?,
        })
    })?;
    let mut members = Vec::new();
    for row in rows {
        members.push(row?);
    }
    Ok(members)
}

pub fn create_team(conn: &Connection, tournament_id: i64, name: &str) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO teams (tournament_id, name) VALUES (?1, ?2)",
        params![tournament_id, name],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_team(
    conn: &Connection,
    tournament_id: i64,
    id: i64,
    name: &str,
) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE teams SET name = ?1 WHERE id = ?2 AND tournament_id = ?3",
        params![name, id, tournament_id],
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
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO team_members (tournament_id, team_id, name) VALUES (?1, ?2, ?3)",
        params![tournament_id, team_id, name],
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
