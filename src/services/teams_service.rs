use crate::db;
use crate::models::{Team, TeamMember};
use crate::repositories::{teams_repository, tournaments_repository};
use crate::state::AppState;
use rocket::State;

pub fn list(state: &State<AppState>, user_id: i64, tournament_id: i64) -> Result<Vec<Team>, String> {
    let conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
    let tournament_exists = tournaments_repository::get_by_id_for_user(&conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?
        .is_some();
    if !tournament_exists {
        return Err("Tournament not found.".to_string());
    }

    let mut teams = teams_repository::list_teams(&conn, tournament_id).map_err(|_| "Storage error.")?;
    let members = teams_repository::list_members(&conn, tournament_id).map_err(|_| "Storage error.")?;
    for team in teams.iter_mut() {
        team.members = members
            .iter()
            .filter(|member| member.team_id == team.id)
            .map(|member| TeamMember {
                id: member.id,
                name: member.name.clone(),
                team_id: member.team_id,
            })
            .collect();
    }
    Ok(teams)
}

pub fn create_team(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    name: &str,
) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Team name is required.".to_string());
    }
    let conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
    let tournament_exists = tournaments_repository::get_by_id_for_user(&conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?
        .is_some();
    if !tournament_exists {
        return Err("Tournament not found.".to_string());
    }
    teams_repository::create_team(&conn, tournament_id, trimmed).map_err(|_| "Storage error.")?;
    Ok(())
}

pub fn update_team(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    id: i64,
    name: &str,
) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Team name is required.".to_string());
    }
    let conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
    let tournament_exists = tournaments_repository::get_by_id_for_user(&conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?
        .is_some();
    if !tournament_exists {
        return Err("Tournament not found.".to_string());
    }
    let changed =
        teams_repository::update_team(&conn, tournament_id, id, trimmed).map_err(|_| "Storage error.")?;
    if changed == 0 {
        return Err("Team not found for this tournament.".to_string());
    }
    Ok(())
}

pub fn delete_team(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    id: i64,
) -> Result<(), String> {
    let conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
    let tournament_exists = tournaments_repository::get_by_id_for_user(&conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?
        .is_some();
    if !tournament_exists {
        return Err("Tournament not found.".to_string());
    }
    let _ = conn.execute(
        "DELETE FROM team_members WHERE team_id = ?1 AND tournament_id = ?2",
        rusqlite::params![id, tournament_id],
    );
    let changed =
        teams_repository::delete_team(&conn, tournament_id, id).map_err(|_| "Storage error.")?;
    if changed == 0 {
        return Err("Team not found for this tournament.".to_string());
    }
    Ok(())
}

pub fn add_member(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    team_id: i64,
    name: &str,
) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Member name is required.".to_string());
    }
    let conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
    let tournament_exists = tournaments_repository::get_by_id_for_user(&conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?
        .is_some();
    if !tournament_exists {
        return Err("Tournament not found.".to_string());
    }
    teams_repository::create_member(&conn, tournament_id, team_id, trimmed)
        .map_err(|_| "Storage error.")?;
    Ok(())
}

pub fn delete_member(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    member_id: i64,
) -> Result<(), String> {
    let conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
    let tournament_exists = tournaments_repository::get_by_id_for_user(&conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?
        .is_some();
    if !tournament_exists {
        return Err("Tournament not found.".to_string());
    }
    let changed =
        teams_repository::delete_member(&conn, tournament_id, member_id).map_err(|_| "Storage error.")?;
    if changed == 0 {
        return Err("Member not found for this tournament.".to_string());
    }
    Ok(())
}
