use crate::db;
use crate::models::{NamedItem, Team, TeamMember};
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
    let team_divisions =
        teams_repository::list_team_divisions(&conn, tournament_id).map_err(|_| "Storage error.")?;
    let team_categories =
        teams_repository::list_team_categories(&conn, tournament_id).map_err(|_| "Storage error.")?;
    let team_events =
        teams_repository::list_team_events(&conn, tournament_id).map_err(|_| "Storage error.")?;
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
        team.divisions = collect_team_items(&team_divisions, team.id);
        team.categories = collect_team_items(&team_categories, team.id);
        team.events = collect_team_items(&team_events, team.id);
        team.division_ids = team.divisions.iter().map(|item| item.id).collect();
        team.category_ids = team.categories.iter().map(|item| item.id).collect();
        team.event_ids = team.events.iter().map(|item| item.id).collect();
    }
    Ok(teams)
}

pub fn create_team(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    name: &str,
    logo_url: Option<&str>,
    division_ids: &[i64],
    category_ids: &[i64],
    event_ids: &[i64],
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
    let team_id = teams_repository::create_team(&conn, tournament_id, trimmed, logo_url)
        .map_err(|_| "Storage error.")?;
    sync_team_divisions(&conn, tournament_id, team_id, division_ids)?;
    sync_team_categories(&conn, tournament_id, team_id, category_ids)?;
    sync_team_events(&conn, tournament_id, team_id, event_ids)?;
    Ok(())
}

pub fn update_team(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    id: i64,
    name: &str,
    logo_url: Option<&str>,
    division_ids: &[i64],
    category_ids: &[i64],
    event_ids: &[i64],
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
    let changed = teams_repository::update_team(&conn, tournament_id, id, trimmed, logo_url)
        .map_err(|_| "Storage error.")?;
    if changed == 0 {
        return Err("Team not found for this tournament.".to_string());
    }
    sync_team_divisions(&conn, tournament_id, id, division_ids)?;
    sync_team_categories(&conn, tournament_id, id, category_ids)?;
    sync_team_events(&conn, tournament_id, id, event_ids)?;
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
    let _ = conn.execute(
        "DELETE FROM team_divisions WHERE team_id = ?1 AND tournament_id = ?2",
        rusqlite::params![id, tournament_id],
    );
    let _ = conn.execute(
        "DELETE FROM team_categories WHERE team_id = ?1 AND tournament_id = ?2",
        rusqlite::params![id, tournament_id],
    );
    let _ = conn.execute(
        "DELETE FROM team_events WHERE team_id = ?1 AND tournament_id = ?2",
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

pub fn get_team_logo(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    team_id: i64,
) -> Result<Option<String>, String> {
    let conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
    let tournament_exists = tournaments_repository::get_by_id_for_user(&conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?
        .is_some();
    if !tournament_exists {
        return Err("Tournament not found.".to_string());
    }
    teams_repository::get_team_logo(&conn, tournament_id, team_id)
        .map_err(|_| "Storage error.".to_string())
}

fn collect_team_items(items: &[(i64, NamedItem)], team_id: i64) -> Vec<NamedItem> {
    items
        .iter()
        .filter(|(owner_id, _)| *owner_id == team_id)
        .map(|(_, item)| NamedItem {
            id: item.id,
            name: item.name.clone(),
        })
        .collect()
}

fn sync_team_divisions(
    conn: &rusqlite::Connection,
    tournament_id: i64,
    team_id: i64,
    division_ids: &[i64],
) -> Result<(), String> {
    teams_repository::clear_team_divisions(conn, tournament_id, team_id)
        .map_err(|_| "Storage error.".to_string())?;
    for division_id in division_ids {
        teams_repository::add_team_division(conn, tournament_id, team_id, *division_id)
            .map_err(|_| "Storage error.".to_string())?;
    }
    Ok(())
}

fn sync_team_categories(
    conn: &rusqlite::Connection,
    tournament_id: i64,
    team_id: i64,
    category_ids: &[i64],
) -> Result<(), String> {
    teams_repository::clear_team_categories(conn, tournament_id, team_id)
        .map_err(|_| "Storage error.".to_string())?;
    for category_id in category_ids {
        teams_repository::add_team_category(conn, tournament_id, team_id, *category_id)
            .map_err(|_| "Storage error.".to_string())?;
    }
    Ok(())
}

fn sync_team_events(
    conn: &rusqlite::Connection,
    tournament_id: i64,
    team_id: i64,
    event_ids: &[i64],
) -> Result<(), String> {
    teams_repository::clear_team_events(conn, tournament_id, team_id)
        .map_err(|_| "Storage error.".to_string())?;
    for event_id in event_ids {
        teams_repository::add_team_event(conn, tournament_id, team_id, *event_id)
            .map_err(|_| "Storage error.".to_string())?;
    }
    Ok(())
}
