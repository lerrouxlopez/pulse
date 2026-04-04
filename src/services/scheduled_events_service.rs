use crate::db;
use crate::models::ScheduledEvent;
use crate::repositories::{events_repository, scheduled_events_repository, tournaments_repository};
use crate::state::AppState;
use rocket::State;

const CONTACT_TYPES: [&str; 2] = ["Contact", "Non-Contact"];
const STATUSES: [&str; 4] = ["Scheduled", "Ongoing", "Finished", "Cancelled"];

pub fn list(state: &State<AppState>, user_id: i64, tournament_id: i64) -> Result<Vec<ScheduledEvent>, String> {
    let conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    scheduled_events_repository::list(&conn, tournament_id).map_err(|_| "Storage error.".to_string())
}

pub fn get_by_id(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    id: i64,
) -> Result<Option<ScheduledEvent>, String> {
    let conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    scheduled_events_repository::get_by_id(&conn, tournament_id, id)
        .map_err(|_| "Storage error.".to_string())
}

pub fn create(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    event_id: i64,
    contact_type: &str,
    status: &str,
    location: Option<&str>,
    event_time: Option<&str>,
) -> Result<(), String> {
    let conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    if !CONTACT_TYPES.iter().any(|value| value.eq_ignore_ascii_case(contact_type)) {
        return Err("Invalid contact type.".to_string());
    }
    if !STATUSES.iter().any(|value| value.eq_ignore_ascii_case(status)) {
        return Err("Invalid status.".to_string());
    }
    let existing = scheduled_events_repository::list(&conn, tournament_id)
        .map_err(|_| "Storage error.".to_string())?;
    if existing.iter().any(|item| item.event_id == event_id) {
        return Err("Event is already scheduled for this tournament.".to_string());
    }
    let event_ids = events_repository::list(&conn, tournament_id)
        .map_err(|_| "Storage error.".to_string())?
        .into_iter()
        .map(|item| item.id)
        .collect::<Vec<_>>();
    if !event_ids.contains(&event_id) {
        return Err("Event is not included in this tournament.".to_string());
    }
    scheduled_events_repository::create(
        &conn,
        tournament_id,
        event_id,
        contact_type,
        status,
        location,
        event_time,
    )
        .map_err(|_| "Storage error.".to_string())?;
    Ok(())
}

pub fn update(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    id: i64,
    event_id: i64,
    contact_type: &str,
    status: &str,
    location: Option<&str>,
    event_time: Option<&str>,
) -> Result<(), String> {
    let conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    if !CONTACT_TYPES.iter().any(|value| value.eq_ignore_ascii_case(contact_type)) {
        return Err("Invalid contact type.".to_string());
    }
    if !STATUSES.iter().any(|value| value.eq_ignore_ascii_case(status)) {
        return Err("Invalid status.".to_string());
    }
    let existing = scheduled_events_repository::list(&conn, tournament_id)
        .map_err(|_| "Storage error.".to_string())?;
    if existing.iter().any(|item| item.event_id == event_id && item.id != id) {
        return Err("Event is already scheduled for this tournament.".to_string());
    }
    let event_ids = events_repository::list(&conn, tournament_id)
        .map_err(|_| "Storage error.".to_string())?
        .into_iter()
        .map(|item| item.id)
        .collect::<Vec<_>>();
    if !event_ids.contains(&event_id) {
        return Err("Event is not included in this tournament.".to_string());
    }
    let changed = scheduled_events_repository::update(
        &conn,
        tournament_id,
        id,
        event_id,
        contact_type,
        status,
        location,
        event_time,
    )
    .map_err(|_| "Storage error.".to_string())?;
    if changed == 0 {
        return Err("Event not found for this tournament.".to_string());
    }
    Ok(())
}

pub fn delete(
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
    let changed =
        scheduled_events_repository::delete(&conn, tournament_id, id).map_err(|_| "Storage error.".to_string())?;
    if changed == 0 {
        return Err("Event not found for this tournament.".to_string());
    }
    Ok(())
}

pub fn contact_types() -> Vec<&'static str> {
    CONTACT_TYPES.to_vec()
}

pub fn statuses() -> Vec<&'static str> {
    STATUSES.to_vec()
}
