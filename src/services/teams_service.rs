use crate::db;
use crate::models::{NamedItem, Team, TeamMember};
use crate::repositories::{teams_repository, tournaments_repository};
use crate::state::AppState;
use rocket::State;
use std::fs;
use std::path::Path;

pub fn list(state: &State<AppState>, user_id: i64, tournament_id: i64) -> Result<Vec<Team>, String> {
    let conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }

    let mut teams = teams_repository::list_teams(&conn, tournament_id).map_err(|_| "Storage error.")?;
    let mut members =
        teams_repository::list_members(&conn, tournament_id).map_err(|_| "Storage error.")?;
    for member in members.iter_mut() {
        if member.photo_url.is_none() {
            if let Ok(url) = ensure_avatar_for_member(&conn, tournament_id, member.id, &member.name) {
                member.photo_url = Some(url);
            }
        }
    }
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
                notes: member.notes.clone(),
                rank: member.rank.clone(),
                weight_class: member.weight_class.clone(),
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
    let conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
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
    let has_access = tournaments_repository::user_has_access(&conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
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
    let has_access = tournaments_repository::user_has_access(&conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
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
    notes: Option<&str>,
    rank: Option<&str>,
    weight_class: Option<&str>,
    photo_url: Option<&str>,
) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Player name is required.".to_string());
    }
    let conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    let member_id = teams_repository::create_member(
        &conn,
        tournament_id,
        team_id,
        trimmed,
        notes,
        rank,
        weight_class,
        photo_url,
    )
    .map_err(|_| "Storage error.")?;
    if photo_url.is_none() {
        let _ = ensure_avatar_for_member(&conn, tournament_id, member_id, trimmed);
    }
    Ok(())
}

pub fn delete_member(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    member_id: i64,
) -> Result<(), String> {
    let conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    let changed =
        teams_repository::delete_member(&conn, tournament_id, member_id).map_err(|_| "Storage error.")?;
    if changed == 0 {
        return Err("Member not found for this tournament.".to_string());
    }
    Ok(())
}

pub fn update_member(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    member_id: i64,
    name: Option<&str>,
    notes: Option<&str>,
    rank: Option<&str>,
    weight_class: Option<&str>,
    clear_notes: bool,
    clear_rank: bool,
    clear_weight_class: bool,
    photo_url: Option<&str>,
    clear_photo: bool,
) -> Result<(), String> {
    let conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    let existing = teams_repository::get_member(&conn, tournament_id, member_id)
        .map_err(|_| "Storage error.".to_string())?
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
    let next_rank = if clear_rank {
        None
    } else {
        rank
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .or(existing.rank.clone())
    };
    let next_weight = if clear_weight_class {
        None
    } else {
        weight_class
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .or(existing.weight_class.clone())
    };
    let next_photo = if clear_photo {
        None
    } else if let Some(url) = photo_url {
        Some(url.to_string())
    } else {
        existing.photo_url.clone()
    };

    let changed = teams_repository::update_member(
        &conn,
        tournament_id,
        member_id,
        &next_name,
        next_notes.as_deref(),
        next_rank.as_deref(),
        next_weight.as_deref(),
        next_photo.as_deref(),
    )
        .map_err(|_| "Storage error.".to_string())?;
    if changed == 0 {
        return Err("Player not found for this tournament.".to_string());
    }
    Ok(())
}

fn ensure_avatar_for_member(
    conn: &rusqlite::Connection,
    tournament_id: i64,
    member_id: i64,
    name: &str,
) -> Result<String, String> {
    let avatars_dir = Path::new("static").join("avatars");
    fs::create_dir_all(&avatars_dir).map_err(|_| "Storage error.".to_string())?;
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
        fs::write(&filepath, svg).map_err(|_| "Storage error.".to_string())?;
    }
    let public_path = format!("/static/avatars/{}", filepath.file_name().unwrap().to_string_lossy());
    let _ = teams_repository::update_member_photo(conn, tournament_id, member_id, Some(&public_path));
    Ok(public_path)
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
    let conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
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
