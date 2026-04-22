use crate::db;
use crate::models::{NamedItem, Team, TeamMember};
use crate::repositories::{teams_repository, tournaments_repository, weight_classes_repository};
use crate::state::AppState;
use mysql::prelude::*;
use rocket::State;
use std::fs;
use std::path::Path;

pub fn list(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
) -> Result<Vec<Team>, String> {
    let mut conn = db::open_conn(&state.pool).map_err(|err| format!("Storage error: {err}"))?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|err| format!("Storage error: {err}"))?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }

    let mut teams =
        teams_repository::list_teams(&mut conn, tournament_id).map_err(|err| format!("Storage error: {err}"))?;
    let mut members =
        teams_repository::list_members(&mut conn, tournament_id).map_err(|err| format!("Storage error: {err}"))?;
    let member_categories = teams_repository::list_member_categories(&mut conn, tournament_id)
        .map_err(|err| format!("Storage error: {err}"))?;
    let member_events = teams_repository::list_member_events(&mut conn, tournament_id)
        .map_err(|err| format!("Storage error: {err}"))?;
    for member in members.iter_mut() {
        if member.photo_url.is_none() || avatar_missing(member.photo_url.as_deref()) {
            if let Ok(url) =
                ensure_avatar_for_member(&mut conn, tournament_id, member.id, &member.name)
            {
                member.photo_url = Some(url);
            }
        }
        member.category_ids = collect_member_ids(&member_categories, member.id);
        member.event_ids = collect_member_ids(&member_events, member.id);
    }
    let team_divisions = teams_repository::list_team_divisions(&mut conn, tournament_id)
        .map_err(|err| format!("Storage error: {err}"))?;
    let team_categories = teams_repository::list_team_categories(&mut conn, tournament_id)
        .map_err(|err| format!("Storage error: {err}"))?;
    let team_events = teams_repository::list_team_events(&mut conn, tournament_id)
        .map_err(|err| format!("Storage error: {err}"))?;
    let mut division_name_map = std::collections::HashMap::new();
    for (_, item) in team_divisions.iter() {
        division_name_map.insert(item.id, item.name.clone());
    }

    for team in teams.iter_mut() {
        team.members = members
            .iter()
            .filter(|member| member.team_id == team.id)
            .map(|member| TeamMember {
                id: member.id,
                name: member.name.clone(),
                team_id: member.team_id,
                notes: member.notes.clone(),
                weight_class: member.weight_class.clone(),
                weight_class_id: member.weight_class_id,
                division_id: member.division_id,
                division_name: member
                    .division_id
                    .and_then(|id| division_name_map.get(&id).cloned()),
                category_ids: member.category_ids.clone(),
                event_ids: member.event_ids.clone(),
                photo_url: member.photo_url.clone(),
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

pub fn get_team(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    team_id: i64,
) -> Result<Option<Team>, String> {
    let teams = list(state, user_id, tournament_id)?;
    Ok(teams.into_iter().find(|team| team.id == team_id))
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
    let mut conn = db::open_conn(&state.pool).map_err(|err| format!("Storage error: {err}"))?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|err| format!("Storage error: {err}"))?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    let team_id = teams_repository::create_team(&mut conn, tournament_id, trimmed, logo_url)
        .map_err(|err| format!("Storage error: {err}"))?;
    sync_team_divisions(&mut conn, tournament_id, team_id, division_ids)?;
    sync_team_categories(&mut conn, tournament_id, team_id, category_ids)?;
    sync_team_events(&mut conn, tournament_id, team_id, event_ids)?;
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
    let mut conn = db::open_conn(&state.pool).map_err(|err| format!("Storage error: {err}"))?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|err| format!("Storage error: {err}"))?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    let exists =
        teams_repository::team_exists(&mut conn, tournament_id, id).map_err(|err| format!("Storage error: {err}"))?;
    if !exists {
        return Err("Team not found for this tournament.".to_string());
    }
    let _ = teams_repository::update_team(&mut conn, tournament_id, id, trimmed, logo_url)
        .map_err(|err| format!("Storage error: {err}"))?;
    sync_team_divisions(&mut conn, tournament_id, id, division_ids)?;
    sync_team_categories(&mut conn, tournament_id, id, category_ids)?;
    sync_team_events(&mut conn, tournament_id, id, event_ids)?;
    Ok(())
}

pub fn delete_team(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    id: i64,
) -> Result<(), String> {
    let mut conn = db::open_conn(&state.pool).map_err(|err| format!("Storage error: {err}"))?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|err| format!("Storage error: {err}"))?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    let _ = conn.exec_drop(
        "DELETE FROM team_members WHERE team_id = ? AND tournament_id = ?",
        (id, tournament_id),
    );
    let _ = conn.exec_drop(
        "DELETE FROM team_divisions WHERE team_id = ? AND tournament_id = ?",
        (id, tournament_id),
    );
    let _ = conn.exec_drop(
        "DELETE FROM team_categories WHERE team_id = ? AND tournament_id = ?",
        (id, tournament_id),
    );
    let _ = conn.exec_drop(
        "DELETE FROM team_events WHERE team_id = ? AND tournament_id = ?",
        (id, tournament_id),
    );
    let changed = teams_repository::delete_team(&mut conn, tournament_id, id)
        .map_err(|err| format!("Storage error: {err}"))?;
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
    notes: Option<&str>,
    weight_class: Option<&str>,
    division_id: Option<i64>,
    category_ids: &[i64],
    event_ids: &[i64],
    photo_url: Option<&str>,
) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Player name is required.".to_string());
    }
    let mut conn = db::open_conn(&state.pool).map_err(|err| format!("Storage error: {err}"))?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|err| format!("Storage error: {err}"))?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    validate_member_selection(
        &mut conn,
        tournament_id,
        team_id,
        division_id,
        category_ids,
        event_ids,
    )?;
    let weight_class_id = resolve_weight_class_id(&mut conn, tournament_id, weight_class)?;
    let member_id = teams_repository::create_member(
        &mut conn,
        tournament_id,
        team_id,
        trimmed,
        notes,
        weight_class,
        weight_class_id,
        division_id,
        photo_url,
    )
    .map_err(|err| format!("Storage error: {err}"))?;
    sync_member_categories(&mut conn, tournament_id, team_id, member_id, category_ids)?;
    sync_member_events(&mut conn, tournament_id, team_id, member_id, event_ids)?;
    if photo_url.is_none() {
        let _ = ensure_avatar_for_member(&mut conn, tournament_id, member_id, trimmed);
    }
    Ok(())
}

pub fn delete_member(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    member_id: i64,
) -> Result<(), String> {
    let mut conn = db::open_conn(&state.pool).map_err(|err| format!("Storage error: {err}"))?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|err| format!("Storage error: {err}"))?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    let _ = teams_repository::clear_member_categories(&mut conn, tournament_id, member_id);
    let _ = teams_repository::clear_member_events(&mut conn, tournament_id, member_id);
    let changed = teams_repository::delete_member(&mut conn, tournament_id, member_id)
        .map_err(|err| format!("Storage error: {err}"))?;
    if changed == 0 {
        return Err("Member not found for this tournament.".to_string());
    }
    Ok(())
}

pub fn get_member_team_id(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    member_id: i64,
) -> Result<i64, String> {
    let mut conn = db::open_conn(&state.pool).map_err(|err| format!("Storage error: {err}"))?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|err| format!("Storage error: {err}"))?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    let member = teams_repository::get_member(&mut conn, tournament_id, member_id)
        .map_err(|err| format!("Storage error: {err}"))?
        .ok_or_else(|| "Player not found for this tournament.".to_string())?;
    Ok(member.team_id)
}

pub fn update_member(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    member_id: i64,
    name: Option<&str>,
    notes: Option<&str>,
    weight_class: Option<&str>,
    division_id: Option<i64>,
    category_ids: Option<Vec<i64>>,
    event_ids: Option<Vec<i64>>,
    clear_notes: bool,
    clear_weight_class: bool,
    clear_division: bool,
    clear_categories: bool,
    clear_events: bool,
    photo_url: Option<&str>,
    clear_photo: bool,
) -> Result<(), String> {
    let mut conn = db::open_conn(&state.pool).map_err(|err| format!("Storage error: {err}"))?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|err| format!("Storage error: {err}"))?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    let existing = teams_repository::get_member(&mut conn, tournament_id, member_id)
        .map_err(|err| format!("Storage error: {err}"))?
        .ok_or_else(|| "Player not found for this tournament.".to_string())?;

    let next_name = name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or(existing.name);
    let next_notes = if clear_notes {
        None
    } else {
        notes
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .or(existing.notes.clone())
    };
    let next_weight = if clear_weight_class {
        None
    } else {
        weight_class
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .or(existing.weight_class.clone())
    };
    let next_division_id = if clear_division {
        None
    } else {
        division_id.or(existing.division_id)
    };
    let existing_category_ids = collect_member_ids(
        &teams_repository::list_member_categories(&mut conn, tournament_id)
            .map_err(|err| format!("Storage error: {err}"))?,
        existing.id,
    );
    let existing_event_ids = collect_member_ids(
        &teams_repository::list_member_events(&mut conn, tournament_id)
            .map_err(|err| format!("Storage error: {err}"))?,
        existing.id,
    );
    let next_category_ids = if clear_categories {
        Vec::new()
    } else {
        category_ids.unwrap_or(existing_category_ids)
    };
    let next_event_ids = if clear_events {
        Vec::new()
    } else {
        event_ids.unwrap_or(existing_event_ids)
    };
    let next_weight_id = if clear_weight_class {
        None
    } else {
        resolve_weight_class_id(&mut conn, tournament_id, next_weight.as_deref())?
    };
    validate_member_selection(
        &mut conn,
        tournament_id,
        existing.team_id,
        next_division_id,
        &next_category_ids,
        &next_event_ids,
    )?;
    let next_photo = if clear_photo {
        None
    } else if let Some(url) = photo_url {
        Some(url.to_string())
    } else {
        existing.photo_url.clone()
    };

    let changed = teams_repository::update_member(
        &mut conn,
        tournament_id,
        member_id,
        &next_name,
        next_notes.as_deref(),
        next_weight.as_deref(),
        next_weight_id,
        next_division_id,
        next_photo.as_deref(),
    )
    .map_err(|err| format!("Storage error: {err}"))?;
    sync_member_categories(
        &mut conn,
        tournament_id,
        existing.team_id,
        member_id,
        &next_category_ids,
    )?;
    sync_member_events(
        &mut conn,
        tournament_id,
        existing.team_id,
        member_id,
        &next_event_ids,
    )?;
    if changed == 0 {
        return Err("Player not found for this tournament.".to_string());
    }
    Ok(())
}

fn resolve_weight_class_id(
    conn: &mut mysql::PooledConn,
    tournament_id: i64,
    weight_class: Option<&str>,
) -> Result<Option<i64>, String> {
    let trimmed = match weight_class {
        Some(value) => value.trim(),
        None => return Ok(None),
    };
    if trimmed.is_empty() {
        return Ok(None);
    }
    let found = weight_classes_repository::get_by_name(conn, tournament_id, trimmed)
        .map_err(|err| format!("Storage error: {err}"))?;
    match found {
        Some(item) => Ok(Some(item.id)),
        None => Err("Weight class not found.".to_string()),
    }
}

fn validate_member_selection(
    conn: &mut mysql::PooledConn,
    tournament_id: i64,
    team_id: i64,
    division_id: Option<i64>,
    category_ids: &[i64],
    event_ids: &[i64],
) -> Result<(), String> {
    if let Some(division_id) = division_id {
        let team_divisions = teams_repository::list_team_divisions(conn, tournament_id)
            .map_err(|err| format!("Storage error: {err}"))?;
        let is_allowed = team_divisions
            .iter()
            .any(|(owner_id, item)| *owner_id == team_id && item.id == division_id);
        if !is_allowed {
            return Err("Division is not assigned to this team.".to_string());
        }
    }
    let team_categories = teams_repository::list_team_categories(conn, tournament_id)
        .map_err(|err| format!("Storage error: {err}"))?;
    for category_id in category_ids {
        let is_allowed = team_categories
            .iter()
            .any(|(owner_id, item)| *owner_id == team_id && item.id == *category_id);
        if !is_allowed {
            return Err("Category is not assigned to this team.".to_string());
        }
    }
    let team_events = teams_repository::list_team_events(conn, tournament_id)
        .map_err(|err| format!("Storage error: {err}"))?;
    for event_id in event_ids {
        let is_allowed = team_events
            .iter()
            .any(|(owner_id, item)| *owner_id == team_id && item.id == *event_id);
        if !is_allowed {
            return Err("Event is not assigned to this team.".to_string());
        }
    }
    Ok(())
}

fn collect_member_ids(items: &[(i64, NamedItem)], member_id: i64) -> Vec<i64> {
    items
        .iter()
        .filter(|(owner_id, _)| *owner_id == member_id)
        .map(|(_, item)| item.id)
        .collect()
}

fn sync_member_categories(
    conn: &mut mysql::PooledConn,
    tournament_id: i64,
    team_id: i64,
    member_id: i64,
    category_ids: &[i64],
) -> Result<(), String> {
    teams_repository::clear_member_categories(conn, tournament_id, member_id)
        .map_err(|err| format!("Storage error: {err}"))?;
    for category_id in category_ids {
        teams_repository::add_member_category(
            conn,
            tournament_id,
            team_id,
            member_id,
            *category_id,
        )
        .map_err(|err| format!("Storage error: {err}"))?;
    }
    Ok(())
}

fn sync_member_events(
    conn: &mut mysql::PooledConn,
    tournament_id: i64,
    team_id: i64,
    member_id: i64,
    event_ids: &[i64],
) -> Result<(), String> {
    teams_repository::clear_member_events(conn, tournament_id, member_id)
        .map_err(|err| format!("Storage error: {err}"))?;
    for event_id in event_ids {
        teams_repository::add_member_event(conn, tournament_id, team_id, member_id, *event_id)
            .map_err(|err| format!("Storage error: {err}"))?;
    }
    Ok(())
}

fn ensure_avatar_for_member(
    conn: &mut mysql::PooledConn,
    tournament_id: i64,
    member_id: i64,
    name: &str,
) -> Result<String, String> {
    let avatars_dir = Path::new("static").join("avatars");
    fs::create_dir_all(&avatars_dir).map_err(|err| format!("Storage error: {err}"))?;
    let safe_name: String = name
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == ' ')
        .collect();
    let hash = simple_hash(&format!("{}-{}", member_id, safe_name));
    let filename = format!("avatar-{}-{}.svg", member_id, hash);
    let filepath = avatars_dir.join(filename);
    if !filepath.exists() {
        let initials = initials_for(name);
        let color = avatar_color(hash);
        let svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"256\" height=\"256\" viewBox=\"0 0 256 256\"><rect width=\"256\" height=\"256\" rx=\"48\" fill=\"{}\"/><text x=\"50%\" y=\"52%\" text-anchor=\"middle\" dominant-baseline=\"middle\" font-family=\"'Space Grotesk', sans-serif\" font-size=\"96\" fill=\"#FFFFFF\">{}</text></svg>",
            color,
            initials
        );
        fs::write(&filepath, svg).map_err(|err| format!("Storage error: {err}"))?;
    }
    let public_path = format!(
        "/static/avatars/{}",
        filepath.file_name().unwrap().to_string_lossy()
    );
    let _ =
        teams_repository::update_member_photo(conn, tournament_id, member_id, Some(&public_path));
    Ok(public_path)
}

fn avatar_missing(photo_url: Option<&str>) -> bool {
    let Some(path) = photo_url else {
        return true;
    };
    if !path.starts_with("/static/avatars/") {
        return false;
    }
    let filename = path.trim_start_matches("/static/avatars/");
    let filepath = Path::new("static").join("avatars").join(filename);
    !filepath.exists()
}

fn initials_for(name: &str) -> String {
    let mut parts = name
        .split_whitespace()
        .filter(|part| !part.is_empty())
        .take(2)
        .collect::<Vec<_>>();
    if parts.is_empty() {
        parts.push("P");
    }
    parts
        .iter()
        .filter_map(|part| part.chars().next())
        .map(|ch| ch.to_ascii_uppercase())
        .collect()
}

fn simple_hash(value: &str) -> u64 {
    let mut hash: u64 = 0;
    for byte in value.as_bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(*byte as u64);
    }
    hash
}

fn avatar_color(seed: u64) -> &'static str {
    const COLORS: [&str; 8] = [
        "#2563EB", "#DC2626", "#059669", "#7C3AED", "#EA580C", "#0F766E", "#D97706", "#3B82F6",
    ];
    let idx = (seed % COLORS.len() as u64) as usize;
    COLORS[idx]
}

pub fn get_team_logo(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    team_id: i64,
) -> Result<Option<String>, String> {
    let mut conn = db::open_conn(&state.pool).map_err(|err| format!("Storage error: {err}"))?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|err| format!("Storage error: {err}"))?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    teams_repository::get_team_logo(&mut conn, tournament_id, team_id)
        .map_err(|err| format!("Storage error: {err}"))
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
    conn: &mut mysql::PooledConn,
    tournament_id: i64,
    team_id: i64,
    division_ids: &[i64],
) -> Result<(), String> {
    teams_repository::clear_team_divisions(conn, tournament_id, team_id)
        .map_err(|err| format!("Storage error: {err}"))?;
    for division_id in division_ids {
        teams_repository::add_team_division(conn, tournament_id, team_id, *division_id)
            .map_err(|err| format!("Storage error: {err}"))?;
    }
    Ok(())
}

fn sync_team_categories(
    conn: &mut mysql::PooledConn,
    tournament_id: i64,
    team_id: i64,
    category_ids: &[i64],
) -> Result<(), String> {
    teams_repository::clear_team_categories(conn, tournament_id, team_id)
        .map_err(|err| format!("Storage error: {err}"))?;
    for category_id in category_ids {
        teams_repository::add_team_category(conn, tournament_id, team_id, *category_id)
            .map_err(|err| format!("Storage error: {err}"))?;
    }
    Ok(())
}

fn sync_team_events(
    conn: &mut mysql::PooledConn,
    tournament_id: i64,
    team_id: i64,
    event_ids: &[i64],
) -> Result<(), String> {
    teams_repository::clear_team_events(conn, tournament_id, team_id)
        .map_err(|err| format!("Storage error: {err}"))?;
    for event_id in event_ids {
        teams_repository::add_team_event(conn, tournament_id, team_id, *event_id)
            .map_err(|err| format!("Storage error: {err}"))?;
    }
    Ok(())
}
