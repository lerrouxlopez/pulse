use crate::db;
use crate::models::{EventCompetitor, ScheduledMatch};
use crate::repositories::{
    matches_repository, scheduled_events_repository, teams_repository, tournaments_repository,
};
use crate::state::AppState;
use std::time::{SystemTime, UNIX_EPOCH};
use rocket::State;

const MATCH_STATUSES: [&str; 3] = ["Scheduled", "Forfeit", "Finished"];

pub fn list(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    scheduled_event_id: i64,
) -> Result<Vec<ScheduledMatch>, String> {
    let conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    matches_repository::list(&conn, tournament_id, scheduled_event_id)
        .map_err(|_| "Storage error.".to_string())
}

pub fn list_competitors(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    event_id: i64,
) -> Result<Vec<EventCompetitor>, String> {
    let conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    let rows = teams_repository::list_event_competitors(&conn, tournament_id, event_id)
        .map_err(|_| "Storage error.".to_string())?;
    Ok(rows
        .into_iter()
        .map(|(member_id, team_id, name, photo_url)| EventCompetitor {
            member_id,
            team_id,
            name,
            photo_url,
        })
        .collect())
}

pub fn create(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    scheduled_event_id: i64,
    mat: Option<&str>,
    category: Option<&str>,
    red: Option<&str>,
    blue: Option<&str>,
    status: &str,
    location: Option<&str>,
    match_time: Option<&str>,
    round: Option<i64>,
    slot: Option<i64>,
    red_member_id: Option<i64>,
    blue_member_id: Option<i64>,
    is_bye: bool,
) -> Result<(), String> {
    let conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    if !MATCH_STATUSES.iter().any(|value| value.eq_ignore_ascii_case(status)) {
        return Err("Invalid match status.".to_string());
    }
    let scheduled = scheduled_events_repository::get_by_id(&conn, tournament_id, scheduled_event_id)
        .map_err(|_| "Storage error.".to_string())?;
    if scheduled.is_none() {
        return Err("Event not found for this tournament.".to_string());
    }
    matches_repository::create(
        &conn,
        tournament_id,
        scheduled_event_id,
        mat,
        category,
        red,
        blue,
        status,
        location,
        match_time,
        round,
        slot,
        red_member_id,
        blue_member_id,
        is_bye,
    )
    .map_err(|_| "Storage error.".to_string())?;
    Ok(())
}

pub fn update(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    id: i64,
    scheduled_event_id: i64,
    mat: Option<&str>,
    category: Option<&str>,
    red: Option<&str>,
    blue: Option<&str>,
    status: &str,
    location: Option<&str>,
    match_time: Option<&str>,
    round: Option<i64>,
    slot: Option<i64>,
    red_member_id: Option<i64>,
    blue_member_id: Option<i64>,
    is_bye: bool,
) -> Result<(), String> {
    let conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    if !MATCH_STATUSES.iter().any(|value| value.eq_ignore_ascii_case(status)) {
        return Err("Invalid match status.".to_string());
    }
    let changed = matches_repository::update(
        &conn,
        tournament_id,
        id,
        scheduled_event_id,
        mat,
        category,
        red,
        blue,
        status,
        location,
        match_time,
        round,
        slot,
        red_member_id,
        blue_member_id,
        is_bye,
        None,
    )
    .map_err(|_| "Storage error.".to_string())?;
    if changed == 0 {
        return Err("Match not found for this event.".to_string());
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
    let changed = matches_repository::delete(&conn, tournament_id, id)
        .map_err(|_| "Storage error.".to_string())?;
    if changed == 0 {
        return Err("Match not found for this event.".to_string());
    }
    Ok(())
}

pub fn statuses() -> Vec<&'static str> {
    MATCH_STATUSES.to_vec()
}

pub fn update_schedule(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    id: i64,
    scheduled_event_id: i64,
    location: Option<&str>,
    match_time: Option<&str>,
) -> Result<(), String> {
    let conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    let existing = matches_repository::get_by_id(&conn, tournament_id, id)
        .map_err(|_| "Storage error.".to_string())?
        .ok_or_else(|| "Match not found for this event.".to_string())?;
    let changed = matches_repository::update(
        &conn,
        tournament_id,
        id,
        scheduled_event_id,
        existing.mat.as_deref(),
        existing.category.as_deref(),
        existing.red.as_deref(),
        existing.blue.as_deref(),
        &existing.status,
        location,
        match_time,
        existing.round,
        existing.slot,
        existing.red_member_id,
        existing.blue_member_id,
        existing.is_bye,
        existing.winner_side.as_deref(),
    )
    .map_err(|_| "Storage error.".to_string())?;
    if changed == 0 {
        return Err("Match not found for this event.".to_string());
    }
    Ok(())
}

pub fn update_contact_match(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    id: i64,
    scheduled_event_id: i64,
    status: &str,
    location: Option<&str>,
    match_time: Option<&str>,
    winner_side: Option<&str>,
) -> Result<(), String> {
    let conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    if !MATCH_STATUSES.iter().any(|value| value.eq_ignore_ascii_case(status)) {
        return Err("Invalid match status.".to_string());
    }
    let existing = matches_repository::get_by_id(&conn, tournament_id, id)
        .map_err(|_| "Storage error.".to_string())?
        .ok_or_else(|| "Match not found for this event.".to_string())?;

    if !(status.eq_ignore_ascii_case("Finished") || status.eq_ignore_ascii_case("Forfeit")) {
        let changed = matches_repository::update(
            &conn,
            tournament_id,
            id,
            scheduled_event_id,
            existing.mat.as_deref(),
            existing.category.as_deref(),
            existing.red.as_deref(),
            existing.blue.as_deref(),
            status,
            location,
            match_time,
            existing.round,
            existing.slot,
            existing.red_member_id,
            existing.blue_member_id,
            existing.is_bye,
            None,
        )
        .map_err(|_| "Storage error.".to_string())?;
        if changed == 0 {
            return Err("Match not found for this event.".to_string());
        }
        return Ok(());
    }

    let resolved_winner = if existing.is_bye && winner_side.is_none() {
        Some("red")
    } else {
        winner_side
    };
    let winner_side = resolved_winner.ok_or_else(|| "Winner is required.".to_string())?;
    let winner_side = winner_side.trim();
    let winner_side_value = Some(winner_side);

    let (winner_label, winner_id) = match winner_side {
        "red" => (
            existing.red.clone().filter(|value| !value.trim().is_empty()),
            existing.red_member_id,
        ),
        "blue" => (
            existing.blue.clone().filter(|value| !value.trim().is_empty()),
            existing.blue_member_id,
        ),
        _ => return Err("Invalid winner selection.".to_string()),
    };

    let winner_label = winner_label.ok_or_else(|| "Winner not found.".to_string())?;

    let changed = matches_repository::update(
        &conn,
        tournament_id,
        id,
        scheduled_event_id,
        existing.mat.as_deref(),
        existing.category.as_deref(),
        existing.red.as_deref(),
        existing.blue.as_deref(),
        status,
        location,
        match_time,
        existing.round,
        existing.slot,
        existing.red_member_id,
        existing.blue_member_id,
        existing.is_bye,
        winner_side_value,
    )
    .map_err(|_| "Storage error.".to_string())?;
    if changed == 0 {
        return Err("Match not found for this event.".to_string());
    }

    let round = match existing.round {
        Some(value) => value,
        None => return Ok(()),
    };
    let slot = match existing.slot {
        Some(value) => value,
        None => return Ok(()),
    };

    let next_round = round + 1;
    let next_slot = (slot + 1) / 2;
    let mut target = matches_repository::get_by_round_slot(
        &conn,
        tournament_id,
        scheduled_event_id,
        next_round,
        next_slot,
    )
    .map_err(|_| "Storage error.".to_string())?;
    if let Some(ref mut target_match) = target {
        if slot % 2 == 1 {
            target_match.red = Some(winner_label.clone());
            target_match.red_member_id = Some(winner_id).flatten();
        } else {
            target_match.blue = Some(winner_label.clone());
            target_match.blue_member_id = Some(winner_id).flatten();
        }
        let changed = matches_repository::update(
            &conn,
            tournament_id,
            target_match.id,
            scheduled_event_id,
            target_match.mat.as_deref(),
            target_match.category.as_deref(),
            target_match.red.as_deref(),
            target_match.blue.as_deref(),
            &target_match.status,
            target_match.location.as_deref(),
            target_match.match_time.as_deref(),
            target_match.round,
            target_match.slot,
            target_match.red_member_id,
            target_match.blue_member_id,
            target_match.is_bye,
            target_match.winner_side.as_deref(),
        )
        .map_err(|_| "Storage error.".to_string())?;
        if changed == 0 {
            return Err("Next round match not found.".to_string());
        }
    }

    Ok(())
}

pub fn ensure_bracket_for_contact_event(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    scheduled_event_id: i64,
    event_id: i64,
) -> Result<(), String> {
    let existing = list(state, user_id, tournament_id, scheduled_event_id)?;
    if !existing.is_empty() {
        return Ok(());
    }
    let competitors = list_competitors(state, user_id, tournament_id, event_id)?;
    if competitors.is_empty() {
        return Ok(());
    }

    let mut current_round: Vec<BracketParticipant> = randomize_competitors(competitors)
        .into_iter()
        .map(BracketParticipant::Competitor)
        .collect();
    let mut round = 1i64;
    let mut next_match_number = 1i64;

    while current_round.len() > 1 {
        let mut next_round = Vec::new();
        let mut slot = 1i64;
        let mut index = 0usize;

        while index < current_round.len() {
            let red = current_round.get(index).cloned();
            let blue = current_round.get(index + 1).cloned();
            index += 2;

            let next_participant = match (&red, &blue) {
                (
                    Some(BracketParticipant::Competitor(red_competitor)),
                    Some(BracketParticipant::Competitor(blue_competitor)),
                ) => {
                    let match_number = next_match_number;
                    create(
                        state,
                        user_id,
                        tournament_id,
                        scheduled_event_id,
                        None,
                        None,
                        Some(&red_competitor.name),
                        Some(&blue_competitor.name),
                        "Scheduled",
                        None,
                        None,
                        Some(round),
                        Some(slot),
                        Some(red_competitor.member_id),
                        Some(blue_competitor.member_id),
                        false,
                    )?;
                    next_match_number += 1;
                    slot += 1;
                    BracketParticipant::Winner(format!("Winner of Match {}", match_number))
                }
                (
                    Some(BracketParticipant::Competitor(red_competitor)),
                    Some(BracketParticipant::Winner(blue_label)),
                ) => {
                    let match_number = next_match_number;
                    create(
                        state,
                        user_id,
                        tournament_id,
                        scheduled_event_id,
                        None,
                        None,
                        Some(&red_competitor.name),
                        Some(blue_label),
                        "Scheduled",
                        None,
                        None,
                        Some(round),
                        Some(slot),
                        Some(red_competitor.member_id),
                        None,
                        false,
                    )?;
                    next_match_number += 1;
                    slot += 1;
                    BracketParticipant::Winner(format!("Winner of Match {}", match_number))
                }
                (
                    Some(BracketParticipant::Winner(red_label)),
                    Some(BracketParticipant::Competitor(blue_competitor)),
                ) => {
                    let match_number = next_match_number;
                    create(
                        state,
                        user_id,
                        tournament_id,
                        scheduled_event_id,
                        None,
                        None,
                        Some(red_label),
                        Some(&blue_competitor.name),
                        "Scheduled",
                        None,
                        None,
                        Some(round),
                        Some(slot),
                        None,
                        Some(blue_competitor.member_id),
                        false,
                    )?;
                    next_match_number += 1;
                    slot += 1;
                    BracketParticipant::Winner(format!("Winner of Match {}", match_number))
                }
                (
                    Some(BracketParticipant::Winner(red_label)),
                    Some(BracketParticipant::Winner(blue_label)),
                ) => {
                    let match_number = next_match_number;
                    create(
                        state,
                        user_id,
                        tournament_id,
                        scheduled_event_id,
                        None,
                        None,
                        Some(red_label),
                        Some(blue_label),
                        "Scheduled",
                        None,
                        None,
                        Some(round),
                        Some(slot),
                        None,
                        None,
                        false,
                    )?;
                    next_match_number += 1;
                    slot += 1;
                    BracketParticipant::Winner(format!("Winner of Match {}", match_number))
                }
                (Some(BracketParticipant::Competitor(red_competitor)), None) => {
                    create(
                        state,
                        user_id,
                        tournament_id,
                        scheduled_event_id,
                        None,
                        None,
                        Some(&format!("{} - bye", red_competitor.name)),
                        None,
                        "Finished",
                        None,
                        None,
                        Some(round),
                        Some(slot),
                        Some(red_competitor.member_id),
                        None,
                        true,
                    )?;
                    slot += 1;
                    BracketParticipant::ByeCarry(
                        red_competitor.name.clone(),
                        Some(red_competitor.member_id),
                    )
                }
                (None, Some(BracketParticipant::Competitor(blue_competitor))) => {
                    create(
                        state,
                        user_id,
                        tournament_id,
                        scheduled_event_id,
                        None,
                        None,
                        Some(&format!("{} - bye", blue_competitor.name)),
                        None,
                        "Finished",
                        None,
                        None,
                        Some(round),
                        Some(slot),
                        Some(blue_competitor.member_id),
                        None,
                        true,
                    )?;
                    slot += 1;
                    BracketParticipant::ByeCarry(
                        blue_competitor.name.clone(),
                        Some(blue_competitor.member_id),
                    )
                }
                (
                    Some(BracketParticipant::Winner(red_label)),
                    None,
                ) => {
                    create(
                        state,
                        user_id,
                        tournament_id,
                        scheduled_event_id,
                        None,
                        None,
                        Some(&format!("{} - bye", red_label)),
                        None,
                        "Finished",
                        None,
                        None,
                        Some(round),
                        Some(slot),
                        None,
                        None,
                        true,
                    )?;
                    slot += 1;
                    BracketParticipant::Winner(red_label.clone())
                }
                (
                    None,
                    Some(BracketParticipant::Winner(blue_label)),
                ) => {
                    create(
                        state,
                        user_id,
                        tournament_id,
                        scheduled_event_id,
                        None,
                        None,
                        Some(&format!("{} - bye", blue_label)),
                        None,
                        "Finished",
                        None,
                        None,
                        Some(round),
                        Some(slot),
                        None,
                        None,
                        true,
                    )?;
                    slot += 1;
                    BracketParticipant::Winner(blue_label.clone())
                }
                (Some(BracketParticipant::Winner(red_label)), Some(BracketParticipant::ByeCarry(blue_label, blue_id))) => {
                    let match_number = next_match_number;
                    create(
                        state,
                        user_id,
                        tournament_id,
                        scheduled_event_id,
                        None,
                        None,
                        Some(red_label),
                        Some(blue_label),
                        "Scheduled",
                        None,
                        None,
                        Some(round),
                        Some(slot),
                        None,
                        *blue_id,
                        false,
                    )?;
                    next_match_number += 1;
                    slot += 1;
                    BracketParticipant::Winner(format!("Winner of Match {}", match_number))
                }
                (Some(BracketParticipant::ByeCarry(red_label, red_id)), Some(BracketParticipant::Winner(blue_label))) => {
                    let match_number = next_match_number;
                    create(
                        state,
                        user_id,
                        tournament_id,
                        scheduled_event_id,
                        None,
                        None,
                        Some(red_label),
                        Some(blue_label),
                        "Scheduled",
                        None,
                        None,
                        Some(round),
                        Some(slot),
                        *red_id,
                        None,
                        false,
                    )?;
                    next_match_number += 1;
                    slot += 1;
                    BracketParticipant::Winner(format!("Winner of Match {}", match_number))
                }
                (Some(BracketParticipant::ByeCarry(red_label, red_id)), Some(BracketParticipant::Competitor(blue_competitor))) => {
                    let match_number = next_match_number;
                    create(
                        state,
                        user_id,
                        tournament_id,
                        scheduled_event_id,
                        None,
                        None,
                        Some(red_label),
                        Some(&blue_competitor.name),
                        "Scheduled",
                        None,
                        None,
                        Some(round),
                        Some(slot),
                        *red_id,
                        Some(blue_competitor.member_id),
                        false,
                    )?;
                    next_match_number += 1;
                    slot += 1;
                    BracketParticipant::Winner(format!("Winner of Match {}", match_number))
                }
                (Some(BracketParticipant::Competitor(red_competitor)), Some(BracketParticipant::ByeCarry(blue_label, blue_id))) => {
                    let match_number = next_match_number;
                    create(
                        state,
                        user_id,
                        tournament_id,
                        scheduled_event_id,
                        None,
                        None,
                        Some(&red_competitor.name),
                        Some(blue_label),
                        "Scheduled",
                        None,
                        None,
                        Some(round),
                        Some(slot),
                        Some(red_competitor.member_id),
                        *blue_id,
                        false,
                    )?;
                    next_match_number += 1;
                    slot += 1;
                    BracketParticipant::Winner(format!("Winner of Match {}", match_number))
                }
                (Some(BracketParticipant::ByeCarry(red_label, red_id)), Some(BracketParticipant::ByeCarry(blue_label, blue_id))) => {
                    let match_number = next_match_number;
                    create(
                        state,
                        user_id,
                        tournament_id,
                        scheduled_event_id,
                        None,
                        None,
                        Some(red_label),
                        Some(blue_label),
                        "Scheduled",
                        None,
                        None,
                        Some(round),
                        Some(slot),
                        *red_id,
                        *blue_id,
                        false,
                    )?;
                    next_match_number += 1;
                    slot += 1;
                    BracketParticipant::Winner(format!("Winner of Match {}", match_number))
                }
                (Some(BracketParticipant::ByeCarry(red_label, red_id)), None) => {
                    create(
                        state,
                        user_id,
                        tournament_id,
                        scheduled_event_id,
                        None,
                        None,
                        Some(&format!("{} - bye", red_label)),
                        None,
                        "Finished",
                        None,
                        None,
                        Some(round),
                        Some(slot),
                        *red_id,
                        None,
                        true,
                    )?;
                    slot += 1;
                    BracketParticipant::ByeCarry(red_label.clone(), *red_id)
                }
                (None, Some(BracketParticipant::ByeCarry(blue_label, blue_id))) => {
                    create(
                        state,
                        user_id,
                        tournament_id,
                        scheduled_event_id,
                        None,
                        None,
                        Some(&format!("{} - bye", blue_label)),
                        None,
                        "Finished",
                        None,
                        None,
                        Some(round),
                        Some(slot),
                        *blue_id,
                        None,
                        true,
                    )?;
                    slot += 1;
                    BracketParticipant::ByeCarry(blue_label.clone(), *blue_id)
                }
                (Some(BracketParticipant::ByeCarry(red_label, red_id)), Some(BracketParticipant::Unknown))
                | (Some(BracketParticipant::Unknown), Some(BracketParticipant::ByeCarry(red_label, red_id))) => {
                    BracketParticipant::ByeCarry(red_label.clone(), *red_id)
                }
                (Some(BracketParticipant::Winner(red_label)), Some(BracketParticipant::Unknown))
                | (Some(BracketParticipant::Unknown), Some(BracketParticipant::Winner(red_label))) => {
                    BracketParticipant::Winner(red_label.clone())
                }
                (Some(BracketParticipant::Competitor(red_competitor)), Some(BracketParticipant::Unknown))
                | (Some(BracketParticipant::Unknown), Some(BracketParticipant::Competitor(red_competitor))) => {
                    BracketParticipant::Competitor(red_competitor.clone())
                }
                (Some(BracketParticipant::Unknown), Some(BracketParticipant::Unknown))
                | (Some(BracketParticipant::Unknown), None)
                | (None, Some(BracketParticipant::Unknown))
                | (None, None) => {
                    BracketParticipant::Unknown
                }
            };
            next_round.push(next_participant);
        }

        current_round = next_round;
        round += 1;
    }

    Ok(())
}

#[derive(Clone)]
enum BracketParticipant {
    Competitor(EventCompetitor),
    Winner(String),
    ByeCarry(String, Option<i64>),
    Unknown,
}

fn randomize_competitors(mut competitors: Vec<EventCompetitor>) -> Vec<EventCompetitor> {
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos() as u64)
        .unwrap_or(1);
    let mut state = seed | 1;
    if competitors.len() <= 1 {
        return competitors;
    }
    for index in (1..competitors.len()).rev() {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        let swap_index = (state % ((index + 1) as u64)) as usize;
        competitors.swap(index, swap_index);
    }
    competitors
}
