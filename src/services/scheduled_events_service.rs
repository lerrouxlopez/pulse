use crate::db;
use crate::models::ScheduledEvent;
use crate::repositories::{
    divisions_repository, events_repository, scheduled_events_repository, tournaments_repository,
    weight_classes_repository,
};
use crate::state::AppState;
use rocket::State;

const CONTACT_TYPES: [&str; 2] = ["Contact", "Non-Contact"];
const STATUSES: [&str; 4] = ["Scheduled", "Ongoing", "Finished", "Cancelled"];
const POINT_SYSTEMS: [&str; 2] = ["5-10 points", "Must 8/10 points"];
const TIME_RULES: [&str; 2] = ["1 round | 2 minutes", "3 rounds | 1 minute"];

#[derive(Debug, Clone, Copy)]
pub struct TimeRule {
    pub rounds: i64,
    pub seconds_per_round: i64,
}

#[derive(Debug, Clone, Copy)]
pub struct PointRule {
    pub min: i32,
    pub max: i32,
}

fn format_weight_class_label(value: &Option<String>) -> Option<String> {
    value.as_ref().map(|name| {
        let trimmed = name.split(':').next().unwrap_or(name).trim();
        if trimmed.is_empty() {
            name.trim().to_string()
        } else {
            trimmed.to_string()
        }
    })
}

pub fn list(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
) -> Result<Vec<ScheduledEvent>, String> {
    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    let mut events = scheduled_events_repository::list(&mut conn, tournament_id)
        .map_err(|_| "Storage error.".to_string())?;
    for item in events.iter_mut() {
        item.weight_class_label = format_weight_class_label(&item.weight_class_name);
    }
    Ok(events)
}

pub fn get_by_id(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    id: i64,
) -> Result<Option<ScheduledEvent>, String> {
    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    let mut event = scheduled_events_repository::get_by_id(&mut conn, tournament_id, id)
        .map_err(|_| "Storage error.".to_string())?;
    if let Some(item) = event.as_mut() {
        item.weight_class_label = format_weight_class_label(&item.weight_class_name);
    }
    Ok(event)
}

pub fn list_outcomes(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
) -> Result<Vec<ScheduledEvent>, String> {
    let mut events = list(state, user_id, tournament_id)?;
    events.retain(|item| {
        item.status.eq_ignore_ascii_case("Finished")
            && item.winner_member_id.is_some()
            && item
                .winner_name
                .as_ref()
                .map(|name| !name.trim().is_empty())
                .unwrap_or(false)
    });
    Ok(events)
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
    point_system: Option<&str>,
    time_rule: Option<&str>,
    division_id: Option<i64>,
    weight_class_id: Option<i64>,
) -> Result<(), String> {
    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    if !CONTACT_TYPES
        .iter()
        .any(|value| value.eq_ignore_ascii_case(contact_type))
    {
        return Err("Invalid contact type.".to_string());
    }
    if !STATUSES
        .iter()
        .any(|value| value.eq_ignore_ascii_case(status))
    {
        return Err("Invalid status.".to_string());
    }
    let existing = scheduled_events_repository::list(&mut conn, tournament_id)
        .map_err(|_| "Storage error.".to_string())?;
    let is_contact = contact_type.eq_ignore_ascii_case("Contact");
    if existing.iter().any(|item| item.event_id == event_id) {
        return Err("Event is already scheduled for this tournament.".to_string());
    }
    if is_contact {
        let division_id = division_id.ok_or_else(|| "Division is required.".to_string())?;
        let weight_class_id =
            weight_class_id.ok_or_else(|| "Weight class is required.".to_string())?;
        if !POINT_SYSTEMS
            .iter()
            .any(|value| point_system.unwrap_or("").eq_ignore_ascii_case(value))
        {
            return Err("Invalid point system.".to_string());
        }
        if !TIME_RULES
            .iter()
            .any(|value| time_rule.unwrap_or("").eq_ignore_ascii_case(value))
        {
            return Err("Invalid time rule.".to_string());
        }
        if divisions_repository::get_by_id(&mut conn, tournament_id, division_id)
            .map_err(|_| "Storage error.".to_string())?
            .is_none()
        {
            return Err("Division not found.".to_string());
        }
        if weight_classes_repository::get_by_id(&mut conn, tournament_id, weight_class_id)
            .map_err(|_| "Storage error.".to_string())?
            .is_none()
        {
            return Err("Weight class not found.".to_string());
        }
    }
    let event_ids = events_repository::list(&mut conn, tournament_id)
        .map_err(|_| "Storage error.".to_string())?
        .into_iter()
        .map(|item| item.id)
        .collect::<Vec<_>>();
    if !event_ids.contains(&event_id) {
        return Err("Event is not included in this tournament.".to_string());
    }
    scheduled_events_repository::create(
        &mut conn,
        tournament_id,
        event_id,
        contact_type,
        status,
        location,
        event_time,
        if is_contact { point_system } else { None },
        if is_contact { time_rule } else { None },
        if is_contact { division_id } else { None },
        if is_contact { weight_class_id } else { None },
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
    point_system: Option<&str>,
    time_rule: Option<&str>,
    division_id: Option<i64>,
    weight_class_id: Option<i64>,
) -> Result<(), String> {
    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    if !CONTACT_TYPES
        .iter()
        .any(|value| value.eq_ignore_ascii_case(contact_type))
    {
        return Err("Invalid contact type.".to_string());
    }
    if !STATUSES
        .iter()
        .any(|value| value.eq_ignore_ascii_case(status))
    {
        return Err("Invalid status.".to_string());
    }
    let existing = scheduled_events_repository::list(&mut conn, tournament_id)
        .map_err(|_| "Storage error.".to_string())?;
    let is_contact = contact_type.eq_ignore_ascii_case("Contact");
    if existing
        .iter()
        .any(|item| item.id != id && item.event_id == event_id)
    {
        return Err("Event is already scheduled for this tournament.".to_string());
    }
    if is_contact {
        let division_id = division_id.ok_or_else(|| "Division is required.".to_string())?;
        let weight_class_id =
            weight_class_id.ok_or_else(|| "Weight class is required.".to_string())?;
        if !POINT_SYSTEMS
            .iter()
            .any(|value| point_system.unwrap_or("").eq_ignore_ascii_case(value))
        {
            return Err("Invalid point system.".to_string());
        }
        if !TIME_RULES
            .iter()
            .any(|value| time_rule.unwrap_or("").eq_ignore_ascii_case(value))
        {
            return Err("Invalid time rule.".to_string());
        }
        if divisions_repository::get_by_id(&mut conn, tournament_id, division_id)
            .map_err(|_| "Storage error.".to_string())?
            .is_none()
        {
            return Err("Division not found.".to_string());
        }
        if weight_classes_repository::get_by_id(&mut conn, tournament_id, weight_class_id)
            .map_err(|_| "Storage error.".to_string())?
            .is_none()
        {
            return Err("Weight class not found.".to_string());
        }
    }
    let event_ids = events_repository::list(&mut conn, tournament_id)
        .map_err(|_| "Storage error.".to_string())?
        .into_iter()
        .map(|item| item.id)
        .collect::<Vec<_>>();
    if !event_ids.contains(&event_id) {
        return Err("Event is not included in this tournament.".to_string());
    }
    let changed = scheduled_events_repository::update(
        &mut conn,
        tournament_id,
        id,
        event_id,
        contact_type,
        status,
        location,
        event_time,
        if is_contact { point_system } else { None },
        if is_contact { time_rule } else { None },
        if is_contact { division_id } else { None },
        if is_contact { weight_class_id } else { None },
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
    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    let changed = scheduled_events_repository::delete(&mut conn, tournament_id, id)
        .map_err(|_| "Storage error.".to_string())?;
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

fn extract_first_number(value: &str) -> Option<i64> {
    let mut digits = String::new();
    let mut started = false;
    for ch in value.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
            started = true;
        } else if started {
            break;
        }
    }
    if digits.is_empty() {
        return None;
    }
    digits.parse::<i64>().ok()
}

pub fn parse_time_rule(value: Option<&str>) -> Option<TimeRule> {
    let value = value?.trim();
    if value.is_empty() {
        return None;
    }
    // Expected format: "<rounds> round(s) | <minutes> minute(s)".
    let mut parts = value.split('|').map(|part| part.trim());
    let rounds_part = parts.next()?;
    let minutes_part = parts.next()?;
    let rounds = extract_first_number(rounds_part)?;
    let minutes = extract_first_number(minutes_part)?;
    if rounds <= 0 || minutes <= 0 {
        return None;
    }
    Some(TimeRule {
        rounds,
        seconds_per_round: minutes * 60,
    })
}

pub fn parse_point_rule(value: Option<&str>) -> Option<PointRule> {
    let value = value?.trim();
    if value.is_empty() {
        return None;
    }
    if value.eq_ignore_ascii_case("Must 8/10 points") {
        return Some(PointRule { min: 8, max: 10 });
    }
    if value.eq_ignore_ascii_case("5-10 points") {
        return Some(PointRule { min: 5, max: 10 });
    }
    None
}
