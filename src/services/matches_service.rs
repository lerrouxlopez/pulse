use crate::db;
use crate::models::{
    AccessUser, EventCompetitor, JudgeScoreCard, MatchCard, MatchDetail, MatchJudgeScore,
    ScheduledMatch,
};
use crate::repositories::{
    match_judges_repository, match_pause_votes_repository, matches_repository,
    scheduled_events_repository, teams_repository, tournaments_repository,
};
use crate::services::access_service;
use crate::state::AppState;
use mysql::prelude::Queryable;
use rocket::State;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

const MATCH_STATUSES: [&str; 4] = ["Scheduled", "Ongoing", "Forfeit", "Finished"];
const DRAW_SYSTEM_FIRST_POINT_ADVANTAGE: &str = "First point Advantage";

fn is_contact_first_point_advantage(scheduled: &crate::models::ScheduledEvent) -> bool {
    scheduled.contact_type.eq_ignore_ascii_case("Contact")
        && scheduled
            .draw_system
            .as_deref()
            .unwrap_or("")
            .eq_ignore_ascii_case(DRAW_SYSTEM_FIRST_POINT_ADVANTAGE)
}

#[derive(Clone)]
pub struct MatchJudgeInput {
    pub judge_user_id: i64,
    pub red_score: i32,
    pub blue_score: i32,
}

#[derive(Clone)]
pub struct PendingPauseVoteStatus {
    pub fight_round: i64,
    pub pause_seq: i64,
    pub judge_count: i64,
    pub votes_cast: i64,
    pub my_vote: Option<String>,
}

pub fn list(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    scheduled_event_id: i64,
) -> Result<Vec<ScheduledMatch>, String> {
    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    let mut matches = matches_repository::list(&mut conn, tournament_id, scheduled_event_id)
        .map_err(|_| "Storage error.".to_string())?;
    populate_judge_scores(&mut conn, tournament_id, &mut matches)?;
    Ok(matches)
}

pub fn list_cards(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
) -> Result<Vec<MatchCard>, String> {
    let events = crate::services::scheduled_events_service::list(state, user_id, tournament_id)?;
    let mut event_map = HashMap::new();
    let mut competitor_map: HashMap<i64, HashMap<i64, EventCompetitor>> = HashMap::new();

    for event in &events {
        event_map.insert(event.id, event);
        let competitors = list_competitors(state, user_id, tournament_id, event.id)?;
        competitor_map.insert(
            event.id,
            competitors
                .into_iter()
                .map(|competitor| (competitor.member_id, competitor))
                .collect(),
        );
    }

    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }

    let mut matches = matches_repository::list_by_tournament(&mut conn, tournament_id)
        .map_err(|_| "Storage error.".to_string())?;
    matches.sort_by_key(|item| {
        let priority = if item.status.eq_ignore_ascii_case("Ongoing") {
            0
        } else if item.status.eq_ignore_ascii_case("Scheduled") {
            1
        } else {
            2
        };
        (priority, item.id)
    });

    Ok(matches
        .into_iter()
        .filter_map(|item| {
            let event = event_map.get(&item.scheduled_event_id)?;
            let event_competitors = competitor_map.get(&item.scheduled_event_id);

            let red_competitor = item
                .red_member_id
                .and_then(|id| event_competitors.and_then(|lookup| lookup.get(&id)));
            let blue_competitor = item
                .blue_member_id
                .and_then(|id| event_competitors.and_then(|lookup| lookup.get(&id)));

            let red_name = red_competitor
                .map(|competitor| competitor.name.clone())
                .or_else(|| item.red.clone())
                .unwrap_or_else(|| "TBD".to_string());
            let blue_name = if item.is_bye {
                "BYE".to_string()
            } else {
                blue_competitor
                    .map(|competitor| competitor.name.clone())
                    .or_else(|| item.blue.clone())
                    .unwrap_or_else(|| "TBD".to_string())
            };

            let red_photo_url = red_competitor
                .and_then(|competitor| competitor.photo_url.clone())
                .filter(|url| !url.trim().is_empty())
                .unwrap_or_else(|| "/static/placeholders/player-1.svg".to_string());
            let blue_photo_url = if item.is_bye {
                "/static/placeholders/player-2.svg".to_string()
            } else {
                blue_competitor
                    .and_then(|competitor| competitor.photo_url.clone())
                    .filter(|url| !url.trim().is_empty())
                    .unwrap_or_else(|| "/static/placeholders/player-2.svg".to_string())
            };

            let status = if item.status.trim().is_empty() {
                event.status.clone()
            } else {
                item.status.clone()
            };

            if !(status.eq_ignore_ascii_case("Ongoing") || status.eq_ignore_ascii_case("Scheduled"))
            {
                return None;
            }

            Some(MatchCard {
                id: item.id,
                event_id: event.id,
                event_name: event.event_name.clone(),
                event_type: event.contact_type.clone(),
                division_name: event.division_name.clone(),
                weight_class_name: event.weight_class_label.clone(),
                status_class: status_class(&status).to_string(),
                status,
                red_name,
                blue_name,
                red_photo_url,
                blue_photo_url,
            })
        })
        .collect())
}

pub fn get_detail(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    match_id: i64,
) -> Result<Option<MatchDetail>, String> {
    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }

    let mut item = matches_repository::get_by_id(&mut conn, tournament_id, match_id)
        .map_err(|_| "Storage error.".to_string())?;
    let mut item = match item.take() {
        Some(item) => item,
        None => return Ok(None),
    };
    populate_judge_scores(&mut conn, tournament_id, std::slice::from_mut(&mut item))?;

    let event = crate::services::scheduled_events_service::get_by_id(
        state,
        user_id,
        tournament_id,
        item.scheduled_event_id,
    )?;
    let event = match event {
        Some(event) => event,
        None => return Ok(None),
    };

    let competitors = list_competitors(state, user_id, tournament_id, event.id)?;
    let competitor_map: HashMap<i64, EventCompetitor> = competitors
        .into_iter()
        .map(|competitor| (competitor.member_id, competitor))
        .collect();

    let red_competitor = item.red_member_id.and_then(|id| competitor_map.get(&id));
    let blue_competitor = item.blue_member_id.and_then(|id| competitor_map.get(&id));

    let red_name = red_competitor
        .map(|competitor| competitor.name.clone())
        .or_else(|| item.red.clone())
        .unwrap_or_else(|| "TBD".to_string());
    let blue_name = if item.is_bye {
        "BYE".to_string()
    } else {
        blue_competitor
            .map(|competitor| competitor.name.clone())
            .or_else(|| item.blue.clone())
            .unwrap_or_else(|| "TBD".to_string())
    };

    let red_photo_url = red_competitor
        .and_then(|competitor| competitor.photo_url.clone())
        .filter(|url| !url.trim().is_empty())
        .unwrap_or_else(|| "/static/placeholders/player-1.svg".to_string());
    let blue_photo_url = if item.is_bye {
        "/static/placeholders/player-2.svg".to_string()
    } else {
        blue_competitor
            .and_then(|competitor| competitor.photo_url.clone())
            .filter(|url| !url.trim().is_empty())
            .unwrap_or_else(|| "/static/placeholders/player-2.svg".to_string())
    };

    let status = if item.status.trim().is_empty() {
        event.status.clone()
    } else {
        item.status.clone()
    };
    let resolved_fight_round = item.fight_round.or(item.round);
    let resolved_fight_round_value = resolved_fight_round.unwrap_or(1).max(1);
    let round_label = resolved_fight_round
        .map(|round| format!("Round {}", round))
        .unwrap_or_else(|| "Round".to_string());

    Ok(Some(MatchDetail {
        id: item.id,
        event_id: event.id,
        event_name: event.event_name.clone(),
        event_type: event.contact_type.clone(),
        division_name: event.division_name.clone(),
        weight_class_name: event.weight_class_label.clone(),
        status_class: status_class(&status).to_string(),
        status,
        round_label,
        fight_round: resolved_fight_round_value,
        timer_started_at: item.timer_started_at,
        timer_duration_seconds: item.timer_duration_seconds,
        timer_is_running: item.timer_is_running,
        timer_last_completed_round: item.timer_last_completed_round.unwrap_or(0).max(0),
        red_name,
        blue_name,
        red_photo_url,
        blue_photo_url,
        red_total_score: item.red_total_score,
        blue_total_score: item.blue_total_score,
        location: item.location.clone().or(event.location.clone()),
        match_time: item.match_time.clone().or(event.event_time.clone()),
        judges: item
            .judge_scores
            .iter()
            .map(|judge| JudgeScoreCard {
                name: judge.judge_name.clone(),
                photo_url: judge
                    .judge_photo_url
                    .clone()
                    .filter(|url| !url.trim().is_empty())
                    .unwrap_or_else(|| "/static/placeholders/player-3.svg".to_string()),
                red_score: judge.red_score,
                blue_score: judge.blue_score,
            })
            .collect(),
    }))
}

pub fn get_match_row(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    match_id: i64,
) -> Result<Option<ScheduledMatch>, String> {
    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    let item = matches_repository::get_by_id(&mut conn, tournament_id, match_id)
        .map_err(|_| "Storage error.".to_string())?;
    Ok(item)
}

pub fn list_judges(state: &State<AppState>, tournament_id: i64) -> Vec<AccessUser> {
    access_service::list_access_users(state, tournament_id)
        .into_iter()
        .filter(|user| {
            user.role_name
                .as_deref()
                .map(|role| role.eq_ignore_ascii_case("judge"))
                .unwrap_or(false)
        })
        .collect()
}

pub fn list_competitors(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    scheduled_event_id: i64,
) -> Result<Vec<EventCompetitor>, String> {
    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    let scheduled =
        scheduled_events_repository::get_by_id(&mut conn, tournament_id, scheduled_event_id)
            .map_err(|_| "Storage error.".to_string())?;
    let (division_filter, weight_class_filter, is_contact) = scheduled
        .as_ref()
        .map(|event| {
            (
                event.division_id,
                event.weight_class_id,
                event.contact_type.eq_ignore_ascii_case("Contact"),
            )
        })
        .unwrap_or((None, None, false));
    let event_id = scheduled
        .as_ref()
        .map(|event| event.event_id)
        .unwrap_or(scheduled_event_id);
    let rows = teams_repository::list_event_competitors(&mut conn, tournament_id, event_id)
        .map_err(|_| "Storage error.".to_string())?;
    Ok(rows
        .into_iter()
        .filter(|(_, _, _, _, division_id, weight_class_id, _)| {
            if !is_contact {
                return true;
            }
            let division_ok = match division_filter {
                Some(required) => division_id.map(|id| id == required).unwrap_or(false),
                None => false,
            };
            let weight_ok = match (weight_class_filter, weight_class_id) {
                (Some(required), Some(current)) => required == *current,
                _ => false,
            };
            division_ok && weight_ok
        })
        .map(
            |(member_id, team_id, name, photo_url, _, _, _)| EventCompetitor {
                member_id,
                team_id,
                name,
                photo_url,
            },
        )
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
    red_total_score: i32,
    blue_total_score: i32,
) -> Result<(), String> {
    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    if !MATCH_STATUSES
        .iter()
        .any(|value| value.eq_ignore_ascii_case(status))
    {
        return Err("Invalid match status.".to_string());
    }
    let scheduled =
        scheduled_events_repository::get_by_id(&mut conn, tournament_id, scheduled_event_id)
            .map_err(|_| "Storage error.".to_string())?;
    if scheduled.is_none() {
        return Err("Event not found for this tournament.".to_string());
    }
    matches_repository::create(
        &mut conn,
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
        red_total_score,
        blue_total_score,
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
    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    if !MATCH_STATUSES
        .iter()
        .any(|value| value.eq_ignore_ascii_case(status))
    {
        return Err("Invalid match status.".to_string());
    }
    let existing = matches_repository::get_by_id(&mut conn, tournament_id, id)
        .map_err(|_| "Storage error.".to_string())?
        .ok_or_else(|| "Match not found for this event.".to_string())?;
    let changed = matches_repository::update(
        &mut conn,
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
        0,
        0,
    )
    .map_err(|_| "Storage error.".to_string())?;
    if changed == 0 {
        return Err("Match not found for this event.".to_string());
    }

    if !status.eq_ignore_ascii_case("Ongoing") {
        let _ = matches_repository::set_timer_state(
            &mut conn,
            tournament_id,
            scheduled_event_id,
            id,
            status,
            existing.fight_round,
            None,
            None,
            false,
            existing.timer_last_completed_round,
        );
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
    let changed = matches_repository::delete(&mut conn, tournament_id, id)
        .map_err(|_| "Storage error.".to_string())?;
    if changed == 0 {
        return Err("Match not found for this event.".to_string());
    }
    Ok(())
}

pub fn statuses() -> Vec<&'static str> {
    MATCH_STATUSES.to_vec()
}

pub fn toggle_match_timer(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    scheduled_event_id: i64,
    match_id: i64,
    fight_round: Option<i64>,
    auto_complete: bool,
) -> Result<(), String> {
    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }

    let scheduled =
        scheduled_events_repository::get_by_id(&mut conn, tournament_id, scheduled_event_id)
            .map_err(|_| "Storage error.".to_string())?
            .ok_or_else(|| "Event not found for this tournament.".to_string())?;

    let existing = matches_repository::get_by_id(&mut conn, tournament_id, match_id)
        .map_err(|_| "Storage error.".to_string())?
        .ok_or_else(|| "Match not found for this event.".to_string())?;
    if existing.scheduled_event_id != scheduled_event_id {
        return Err("Match not found for this event.".to_string());
    }

    let time_rule =
        crate::services::scheduled_events_service::parse_time_rule(scheduled.time_rule.as_deref());
    let max_fight_rounds = time_rule.map(|rule| rule.rounds).unwrap_or(1);
    let duration_seconds = time_rule.map(|rule| rule.seconds_per_round).unwrap_or(0);

    if existing.status.eq_ignore_ascii_case("Ongoing") {
        if auto_complete && existing.timer_is_running {
            let completed_round = existing.fight_round.unwrap_or(1);
            let changed = matches_repository::set_timer_state(
                &mut conn,
                tournament_id,
                scheduled_event_id,
                match_id,
                "Scheduled",
                existing.fight_round,
                existing.timer_started_at,
                existing.timer_duration_seconds,
                false,
                Some(completed_round),
            )
            .map_err(|_| "Storage error.".to_string())?;
            if changed == 0 {
                return Err("Match not found for this event.".to_string());
            }

            // First-point advantage contact events: if we just completed the last configured round,
            // finalize by highest score; if tied, use the first point winner as tie-breaker.
            if is_contact_first_point_advantage(&scheduled) && completed_round >= max_fight_rounds {
                let updated = matches_repository::get_by_id(&mut conn, tournament_id, match_id)
                    .map_err(|_| "Storage error.".to_string())?
                    .ok_or_else(|| "Match not found for this event.".to_string())?;
                let winner_side = if updated.red_total_score > updated.blue_total_score {
                    Some("red".to_string())
                } else if updated.blue_total_score > updated.red_total_score {
                    Some("blue".to_string())
                } else {
                    match_pause_votes_repository::first_applied_point_side(
                        &mut conn,
                        tournament_id,
                        match_id,
                    )
                    .map_err(|_| "Storage error.".to_string())?
                };
                if let Some(winner_side) = winner_side {
                    let _ = finalize_first_point_advantage_match(
                        &mut conn,
                        tournament_id,
                        scheduled_event_id,
                        &scheduled,
                        &updated,
                        winner_side.as_str(),
                    );
                }
            }
            return Ok(());
        }

        let changed = matches_repository::set_timer_state(
            &mut conn,
            tournament_id,
            scheduled_event_id,
            match_id,
            "Scheduled",
            existing.fight_round,
            None,
            None,
            false,
            existing.timer_last_completed_round,
        )
        .map_err(|_| "Storage error.".to_string())?;
        if changed == 0 {
            return Err("Match not found for this event.".to_string());
        }
        return Ok(());
    }

    let mut resolved_round = fight_round.unwrap_or(1);
    if resolved_round < 1 {
        resolved_round = 1;
    }
    if resolved_round > max_fight_rounds {
        resolved_round = max_fight_rounds;
    }

    let last_completed = existing.timer_last_completed_round.unwrap_or(0);
    if resolved_round <= last_completed {
        return Err(
            "Round is already completed. Select the next round to start again.".to_string(),
        );
    }

    let now_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs() as i64)
        .unwrap_or(0);

    let changed = matches_repository::set_timer_state(
        &mut conn,
        tournament_id,
        scheduled_event_id,
        match_id,
        "Ongoing",
        Some(resolved_round),
        Some(now_seconds),
        if duration_seconds > 0 {
            Some(duration_seconds)
        } else {
            None
        },
        true,
        existing.timer_last_completed_round,
    )
    .map_err(|_| "Storage error.".to_string())?;
    if changed == 0 {
        return Err("Match not found for this event.".to_string());
    }
    Ok(())
}

pub fn toggle_match_timer_pause(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    scheduled_event_id: i64,
    match_id: i64,
) -> Result<(), String> {
    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }

    let scheduled =
        scheduled_events_repository::get_by_id(&mut conn, tournament_id, scheduled_event_id)
            .map_err(|_| "Storage error.".to_string())?
            .ok_or_else(|| "Event not found for this tournament.".to_string())?;

    let existing = matches_repository::get_by_id(&mut conn, tournament_id, match_id)
        .map_err(|_| "Storage error.".to_string())?
        .ok_or_else(|| "Match not found for this event.".to_string())?;
    if existing.scheduled_event_id != scheduled_event_id {
        return Err("Match not found for this event.".to_string());
    }
    if !existing.status.eq_ignore_ascii_case("Ongoing") {
        return Err("Match timer is not started.".to_string());
    }

    let time_rule =
        crate::services::scheduled_events_service::parse_time_rule(scheduled.time_rule.as_deref());
    let duration_limit = time_rule.map(|rule| rule.seconds_per_round).unwrap_or(0);
    if duration_limit <= 0 {
        return Err("Timer is not configured for this event.".to_string());
    }

    let is_pause_vote_scoring = is_contact_first_point_advantage(&scheduled);
    let current_fight_round = existing.fight_round.unwrap_or(1).max(1);
    if is_pause_vote_scoring {
        let assigned =
            match_judges_repository::list_assigned_judges(&mut conn, tournament_id, match_id)
                .map_err(|_| "Storage error.".to_string())?;
        let judge_count = assigned.len() as i64;
        if judge_count < 3 || judge_count > 5 {
            return Err("Add between 3 and 5 judges.".to_string());
        }
        if judge_count % 2 == 0 {
            return Err("Add an odd number of judges to avoid tied votes.".to_string());
        }

        if existing.timer_is_running {
            // Pausing: create a new pending vote event for this round.
            if match_pause_votes_repository::latest_pending_vote_event(
                &mut conn,
                tournament_id,
                match_id,
                current_fight_round,
            )
            .map_err(|_| "Storage error.".to_string())?
            .is_some()
            {
                return Err("Previous pause vote is still pending.".to_string());
            }
            let next_seq = match_pause_votes_repository::next_pause_seq(
                &mut conn,
                tournament_id,
                match_id,
                current_fight_round,
            )
            .map_err(|_| "Storage error.".to_string())?;
            match_pause_votes_repository::create_vote_event(
                &mut conn,
                tournament_id,
                match_id,
                current_fight_round,
                next_seq,
            )
            .map_err(|_| "Storage error.".to_string())?;
        } else {
            // Resuming: require the pending vote to be complete, then apply exactly 1 point.
            if let Some(pending) = match_pause_votes_repository::latest_pending_vote_event(
                &mut conn,
                tournament_id,
                match_id,
                current_fight_round,
            )
            .map_err(|_| "Storage error.".to_string())?
            {
                let votes_cast = match_pause_votes_repository::count_votes(
                    &mut conn,
                    tournament_id,
                    match_id,
                    pending.fight_round,
                    pending.pause_seq,
                )
                .map_err(|_| "Storage error.".to_string())?;
                if votes_cast != judge_count {
                    return Err("Cannot resume: judge vote is incomplete.".to_string());
                }
                let (red_votes, blue_votes) = match_pause_votes_repository::tally_votes(
                    &mut conn,
                    tournament_id,
                    match_id,
                    pending.fight_round,
                    pending.pause_seq,
                )
                .map_err(|_| "Storage error.".to_string())?;

                let winner_side = if red_votes > blue_votes {
                    "red"
                } else if blue_votes > red_votes {
                    "blue"
                } else {
                    return Err("Cannot resume: vote is tied.".to_string());
                };

                let applied = match_pause_votes_repository::mark_applied(
                    &mut conn,
                    tournament_id,
                    match_id,
                    pending.fight_round,
                    pending.pause_seq,
                    winner_side,
                )
                .map_err(|_| "Storage error.".to_string())?;
                if applied > 0 {
                    let _ = matches_repository::increment_total(
                        &mut conn,
                        tournament_id,
                        match_id,
                        winner_side,
                    );
                }

                // If someone reached 5 points, finish the match and do not resume the timer.
                let updated = matches_repository::get_by_id(&mut conn, tournament_id, match_id)
                    .map_err(|_| "Storage error.".to_string())?
                    .ok_or_else(|| "Match not found for this event.".to_string())?;
                if updated.red_total_score >= 5 || updated.blue_total_score >= 5 {
                    let final_winner = if updated.red_total_score >= 5 {
                        "red"
                    } else {
                        "blue"
                    };
                    finalize_first_point_advantage_match(
                        &mut conn,
                        tournament_id,
                        scheduled_event_id,
                        &scheduled,
                        &updated,
                        final_winner,
                    )?;
                    return Ok(());
                }
            }
        }
    }

    let now_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs() as i64)
        .unwrap_or(0);

    let (timer_started_at, timer_duration_seconds, timer_is_running) = if existing.timer_is_running
    {
        let started_at = existing.timer_started_at.unwrap_or(now_seconds);
        let current_limit = existing.timer_duration_seconds.unwrap_or(duration_limit);
        let elapsed = now_seconds.saturating_sub(started_at);
        let elapsed_capped = if current_limit > 0 {
            elapsed.min(current_limit)
        } else {
            elapsed
        };
        (
            Some(started_at),
            Some(elapsed_capped),
            false, // paused
        )
    } else {
        // When paused we persist the elapsed seconds in `timer_duration_seconds` to freeze display.
        // Resuming turns `timer_duration_seconds` back into the round limit and rewrites started_at so
        // elapsed continues from the paused value.
        let elapsed_so_far = existing.timer_duration_seconds.unwrap_or(0).max(0);
        let started_at = now_seconds.saturating_sub(elapsed_so_far);
        (
            Some(started_at),
            Some(duration_limit),
            true, // running
        )
    };

    let changed = matches_repository::set_timer_state(
        &mut conn,
        tournament_id,
        scheduled_event_id,
        match_id,
        "Ongoing",
        existing.fight_round,
        timer_started_at,
        timer_duration_seconds,
        timer_is_running,
        existing.timer_last_completed_round,
    )
    .map_err(|_| "Storage error.".to_string())?;
    if changed == 0 {
        return Err("Match not found for this event.".to_string());
    }
    Ok(())
}

pub fn get_pending_pause_vote(
    state: &State<AppState>,
    actor_user_id: i64,
    tournament_id: i64,
    match_id: i64,
    judge_user_id: i64,
) -> Result<Option<PendingPauseVoteStatus>, String> {
    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let has_access =
        tournaments_repository::user_has_access(&mut conn, tournament_id, actor_user_id)
            .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }

    let match_row = matches_repository::get_by_id(&mut conn, tournament_id, match_id)
        .map_err(|_| "Storage error.".to_string())?
        .ok_or_else(|| "Match not found.".to_string())?;
    let scheduled = scheduled_events_repository::get_by_id(
        &mut conn,
        tournament_id,
        match_row.scheduled_event_id,
    )
    .map_err(|_| "Storage error.".to_string())?
    .ok_or_else(|| "Event not found.".to_string())?;

    if !is_contact_first_point_advantage(&scheduled) {
        return Ok(None);
    }
    if !match_row.status.eq_ignore_ascii_case("Ongoing") || match_row.timer_is_running {
        return Ok(None);
    }

    let fight_round = match_row.fight_round.unwrap_or(1).max(1);
    let Some(pending) = match_pause_votes_repository::latest_pending_vote_event(
        &mut conn,
        tournament_id,
        match_id,
        fight_round,
    )
    .map_err(|_| "Storage error.".to_string())?
    else {
        return Ok(None);
    };

    let assigned =
        match_judges_repository::list_assigned_judges(&mut conn, tournament_id, match_id)
            .map_err(|_| "Storage error.".to_string())?;
    let judge_count = assigned.len() as i64;
    let votes_cast = match_pause_votes_repository::count_votes(
        &mut conn,
        tournament_id,
        match_id,
        pending.fight_round,
        pending.pause_seq,
    )
    .map_err(|_| "Storage error.".to_string())?;
    let my_vote = match_pause_votes_repository::get_vote_for_judge(
        &mut conn,
        tournament_id,
        match_id,
        pending.fight_round,
        pending.pause_seq,
        judge_user_id,
    )
    .map_err(|_| "Storage error.".to_string())?;

    Ok(Some(PendingPauseVoteStatus {
        fight_round: pending.fight_round,
        pause_seq: pending.pause_seq,
        judge_count,
        votes_cast,
        my_vote,
    }))
}

pub fn submit_pause_vote(
    state: &State<AppState>,
    actor_user_id: i64,
    tournament_id: i64,
    match_id: i64,
    judge_user_id: i64,
    side: &str,
) -> Result<(), String> {
    let side = side.trim();
    if !(side.eq_ignore_ascii_case("red") || side.eq_ignore_ascii_case("blue")) {
        return Err("Invalid vote selection.".to_string());
    }

    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let has_access =
        tournaments_repository::user_has_access(&mut conn, tournament_id, actor_user_id)
            .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }

    let match_row = matches_repository::get_by_id(&mut conn, tournament_id, match_id)
        .map_err(|_| "Storage error.".to_string())?
        .ok_or_else(|| "Match not found.".to_string())?;
    let scheduled = scheduled_events_repository::get_by_id(
        &mut conn,
        tournament_id,
        match_row.scheduled_event_id,
    )
    .map_err(|_| "Storage error.".to_string())?
    .ok_or_else(|| "Event not found.".to_string())?;

    if !is_contact_first_point_advantage(&scheduled) {
        return Err("This match does not accept pause votes.".to_string());
    }
    if !match_row.status.eq_ignore_ascii_case("Ongoing") || match_row.timer_is_running {
        return Err("Match must be paused to accept votes.".to_string());
    }

    let fight_round = match_row.fight_round.unwrap_or(1).max(1);
    let pending = match_pause_votes_repository::latest_pending_vote_event(
        &mut conn,
        tournament_id,
        match_id,
        fight_round,
    )
    .map_err(|_| "Storage error.".to_string())?
    .ok_or_else(|| "No pending vote for this match.".to_string())?;

    let assigned =
        match_judges_repository::list_assigned_judges(&mut conn, tournament_id, match_id)
            .map_err(|_| "Storage error.".to_string())?;
    if !assigned.iter().any(|(id, _)| *id == judge_user_id) {
        return Err("Judge is not assigned to this match.".to_string());
    }

    match_pause_votes_repository::upsert_vote(
        &mut conn,
        tournament_id,
        match_id,
        pending.fight_round,
        pending.pause_seq,
        judge_user_id,
        if side.eq_ignore_ascii_case("red") {
            "red"
        } else {
            "blue"
        },
    )
    .map_err(|_| "Storage error.".to_string())?;

    Ok(())
}

pub fn set_or_adjust_judge_score(
    state: &State<AppState>,
    actor_user_id: i64,
    tournament_id: i64,
    match_id: i64,
    judge_user_id: i64,
    fight_round: i64,
    side: &str,
    delta: Option<i32>,
    value: Option<i32>,
    allow_unassigned: bool,
) -> Result<(), String> {
    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let has_access =
        tournaments_repository::user_has_access(&mut conn, tournament_id, actor_user_id)
            .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    let judge_has_access =
        tournaments_repository::user_has_access(&mut conn, tournament_id, judge_user_id)
            .map_err(|_| "Storage error.".to_string())?;
    if !judge_has_access {
        return Err("Selected judge is invalid.".to_string());
    }

    let match_row = matches_repository::get_by_id(&mut conn, tournament_id, match_id)
        .map_err(|_| "Storage error.".to_string())?
        .ok_or_else(|| "Match not found.".to_string())?;
    let scheduled = scheduled_events_repository::get_by_id(
        &mut conn,
        tournament_id,
        match_row.scheduled_event_id,
    )
    .map_err(|_| "Storage error.".to_string())?
    .ok_or_else(|| "Event not found.".to_string())?;

    if is_contact_first_point_advantage(&scheduled) {
        return Err("This event uses pause-vote scoring; judge round scores are disabled.".to_string());
    }

    let point_rule = crate::services::scheduled_events_service::parse_point_rule(
        scheduled.point_system.as_deref(),
    )
    .unwrap_or(crate::services::scheduled_events_service::PointRule { min: 0, max: 10 });
    let min_allowed = point_rule.min;
    let max_allowed = point_rule.max;

    let fight_round = if fight_round < 1 { 1 } else { fight_round };

    let judge_order = match_judges_repository::find_judge_order(
        &mut conn,
        tournament_id,
        match_id,
        judge_user_id,
    )
    .map_err(|_| "Storage error.".to_string())?
    .or_else(|| {
        if allow_unassigned {
            match_judges_repository::next_judge_order_for_match_round(
                &mut conn,
                tournament_id,
                match_id,
                fight_round,
            )
            .ok()
        } else {
            None
        }
    })
    .ok_or_else(|| "You are not assigned as a judge for this match.".to_string())?;

    let (existing_red, existing_blue) = match_judges_repository::get_score(
        &mut conn,
        tournament_id,
        match_id,
        judge_user_id,
        fight_round,
    )
    .map_err(|_| "Storage error.".to_string())?
    .unwrap_or((min_allowed, min_allowed));

    let normalize = |score: i32| {
        if score < min_allowed {
            min_allowed
        } else {
            score
        }
    };
    let clamp = |score: i32| score.clamp(min_allowed, max_allowed);

    let mut next_red = existing_red;
    let mut next_blue = existing_blue;

    let side = side.trim().to_lowercase();
    if let Some(value) = value {
        if value < min_allowed || value > max_allowed {
            return Err("Invalid score value.".to_string());
        }
        match side.as_str() {
            "red" => next_red = value,
            "blue" => next_blue = value,
            _ => return Err("Invalid side.".to_string()),
        }
    } else if let Some(delta) = delta {
        match side.as_str() {
            "red" => next_red = clamp(normalize(existing_red) + delta),
            "blue" => next_blue = clamp(normalize(existing_blue) + delta),
            _ => return Err("Invalid side.".to_string()),
        }
    } else {
        return Err("No score change provided.".to_string());
    }

    match_judges_repository::upsert_score(
        &mut conn,
        tournament_id,
        match_id,
        judge_user_id,
        fight_round,
        judge_order,
        next_red,
        next_blue,
    )
    .map_err(|_| "Storage error.".to_string())?;

    let (sum_red, sum_blue) = match_judges_repository::sum_for_match_round(
        &mut conn,
        tournament_id,
        match_id,
        fight_round,
    )
    .map_err(|_| "Storage error.".to_string())?;
    let _ = matches_repository::set_totals(
        &mut conn,
        tournament_id,
        match_id,
        sum_red.min(i64::from(i32::MAX)) as i32,
        sum_blue.min(i64::from(i32::MAX)) as i32,
    );

    Ok(())
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
    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    let existing = matches_repository::get_by_id(&mut conn, tournament_id, id)
        .map_err(|_| "Storage error.".to_string())?
        .ok_or_else(|| "Match not found for this event.".to_string())?;
    let changed = matches_repository::update(
        &mut conn,
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
        existing.red_total_score,
        existing.blue_total_score,
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
    judge_user_ids: Vec<i64>,
) -> Result<(), String> {
    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    if !MATCH_STATUSES
        .iter()
        .any(|value| value.eq_ignore_ascii_case(status))
    {
        return Err("Invalid match status.".to_string());
    }
    let existing = matches_repository::get_by_id(&mut conn, tournament_id, id)
        .map_err(|_| "Storage error.".to_string())?
        .ok_or_else(|| "Match not found for this event.".to_string())?;
    let scheduled =
        scheduled_events_repository::get_by_id(&mut conn, tournament_id, scheduled_event_id)
            .map_err(|_| "Storage error.".to_string())?;
    let is_pause_vote_scoring = scheduled
        .as_ref()
        .map(is_contact_first_point_advantage)
        .unwrap_or(false);

    let fight_round = existing.fight_round.or(existing.round).unwrap_or(1);
    let judge_scores = prepare_judge_scores_for_match_round(
        &mut conn,
        state,
        tournament_id,
        id,
        fight_round,
        &judge_user_ids,
    )?;
    let base_rounds = scheduled
        .as_ref()
        .and_then(|item| {
            crate::services::scheduled_events_service::parse_time_rule(item.time_rule.as_deref())
        })
        .map(|rule| rule.rounds)
        .unwrap_or(1)
        .max(1);
    let max_scored_round =
        match_judges_repository::max_fight_round_for_match(&mut conn, tournament_id, id)
            .map_err(|_| "Storage error.".to_string())?;
    let rounds_total = base_rounds
        .max(existing.fight_round.unwrap_or(1))
        .max(max_scored_round);

    if !(status.eq_ignore_ascii_case("Finished") || status.eq_ignore_ascii_case("Forfeit")) {
        let changed = matches_repository::update(
            &mut conn,
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
            existing.red_total_score,
            existing.blue_total_score,
        )
        .map_err(|_| "Storage error.".to_string())?;
        if changed == 0 {
            return Err("Match not found for this event.".to_string());
        }
        match_judges_repository::replace_for_match(
            &mut conn,
            tournament_id,
            id,
            fight_round,
            &judge_scores,
        )
        .map_err(|_| "Storage error.".to_string())?;
        if !is_pause_vote_scoring {
            let (sum_red, sum_blue) =
                total_scores_for_match(&mut conn, tournament_id, id, rounds_total)?;
            let _ = matches_repository::set_totals(
                &mut conn,
                tournament_id,
                id,
                sum_red.min(i64::from(i32::MAX)) as i32,
                sum_blue.min(i64::from(i32::MAX)) as i32,
            );
        }
        if !status.eq_ignore_ascii_case("Ongoing") {
            let _ = matches_repository::set_timer_state(
                &mut conn,
                tournament_id,
                scheduled_event_id,
                id,
                status,
                existing.fight_round,
                None,
                None,
                false,
                existing.timer_last_completed_round,
            );
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
            existing
                .red
                .clone()
                .filter(|value| !value.trim().is_empty()),
            existing.red_member_id,
        ),
        "blue" => (
            existing
                .blue
                .clone()
                .filter(|value| !value.trim().is_empty()),
            existing.blue_member_id,
        ),
        _ => return Err("Invalid winner selection.".to_string()),
    };

    let winner_label = winner_label.ok_or_else(|| "Winner not found.".to_string())?;

    let changed = matches_repository::update(
        &mut conn,
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
        existing.red_total_score,
        existing.blue_total_score,
    )
    .map_err(|_| "Storage error.".to_string())?;
    if changed == 0 {
        return Err("Match not found for this event.".to_string());
    }
    match_judges_repository::replace_for_match(
        &mut conn,
        tournament_id,
        id,
        fight_round,
        &judge_scores,
    )
    .map_err(|_| "Storage error.".to_string())?;
    if !is_pause_vote_scoring {
        let (sum_red, sum_blue) =
            total_scores_for_match(&mut conn, tournament_id, id, rounds_total)?;
        let _ = matches_repository::set_totals(
            &mut conn,
            tournament_id,
            id,
            sum_red.min(i64::from(i32::MAX)) as i32,
            sum_blue.min(i64::from(i32::MAX)) as i32,
        );
    }
    if !status.eq_ignore_ascii_case("Ongoing") {
        let _ = matches_repository::set_timer_state(
            &mut conn,
            tournament_id,
            scheduled_event_id,
            id,
            status,
            existing.fight_round,
            None,
            None,
            false,
            existing.timer_last_completed_round,
        );
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
        &mut conn,
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
            &mut conn,
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
            target_match.red_total_score,
            target_match.blue_total_score,
        )
        .map_err(|_| "Storage error.".to_string())?;
        if changed == 0 {
            return Err("Next round match not found.".to_string());
        }
    }

    if let Some(scheduled_event) = scheduled {
        if scheduled_event.contact_type.eq_ignore_ascii_case("Contact") {
            let is_final = conn
                .exec_first::<Option<i64>, _, _>(
                    "SELECT id FROM matches WHERE tournament_id = ? AND scheduled_event_id = ? AND round > ? LIMIT 1",
                    (tournament_id, scheduled_event_id, round),
                )
                .map_err(|_| "Storage error.".to_string())?
                .is_none();
            if is_final {
                let winner_member_id = match winner_side {
                    "red" => existing.red_member_id,
                    "blue" => existing.blue_member_id,
                    _ => None,
                };
                let _ = scheduled_events_repository::update_status_and_winner(
                    &mut conn,
                    tournament_id,
                    scheduled_event_id,
                    "Finished",
                    winner_member_id,
                );
            }
        }
    }

    Ok(())
}

fn total_scores_for_match(
    conn: &mut mysql::PooledConn,
    tournament_id: i64,
    match_id: i64,
    rounds_total: i64,
) -> Result<(i64, i64), String> {
    let mut sum_red: i64 = 0;
    let mut sum_blue: i64 = 0;
    for r in 1..=rounds_total {
        let (red, blue) =
            match_judges_repository::sum_for_match_round(conn, tournament_id, match_id, r)
                .map_err(|_| "Storage error.".to_string())?;
        sum_red = sum_red.saturating_add(red);
        sum_blue = sum_blue.saturating_add(blue);
    }
    Ok((sum_red, sum_blue))
}

pub fn try_finalize_contact_match_from_scores(
    state: &State<AppState>,
    actor_user_id: i64,
    tournament_id: i64,
    match_id: i64,
) -> Result<Option<i64>, String> {
    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let has_access =
        tournaments_repository::user_has_access(&mut conn, tournament_id, actor_user_id)
            .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }

    let match_row = matches_repository::get_by_id(&mut conn, tournament_id, match_id)
        .map_err(|_| "Storage error.".to_string())?
        .ok_or_else(|| "Match not found.".to_string())?;

    if match_row.status.eq_ignore_ascii_case("Finished")
        || match_row.status.eq_ignore_ascii_case("Forfeit")
    {
        return Ok(None);
    }

    let scheduled = scheduled_events_repository::get_by_id(
        &mut conn,
        tournament_id,
        match_row.scheduled_event_id,
    )
    .map_err(|_| "Storage error.".to_string())?
    .ok_or_else(|| "Event not found.".to_string())?;

    if !scheduled.contact_type.eq_ignore_ascii_case("Contact") {
        return Ok(None);
    }

    if is_contact_first_point_advantage(&scheduled) {
        // First-point advantage contact events are scored via pause-votes, not round scorecards.
        return Ok(None);
    }

    let point_rule = crate::services::scheduled_events_service::parse_point_rule(
        scheduled.point_system.as_deref(),
    )
    .unwrap_or(crate::services::scheduled_events_service::PointRule { min: 0, max: 10 });
    let min_allowed = point_rule.min;
    let max_allowed = point_rule.max;

    let base_rounds =
        crate::services::scheduled_events_service::parse_time_rule(scheduled.time_rule.as_deref())
            .map(|rule| rule.rounds)
            .unwrap_or(1)
            .max(1);
    let max_scored_round =
        match_judges_repository::max_fight_round_for_match(&mut conn, tournament_id, match_id)
            .map_err(|_| "Storage error.".to_string())?;
    let current_rounds = base_rounds
        .max(match_row.fight_round.unwrap_or(1))
        .max(max_scored_round);

    let assigned =
        match_judges_repository::list_assigned_judges(&mut conn, tournament_id, match_id)
            .map_err(|_| "Storage error.".to_string())?;
    let judge_user_ids: Vec<i64> = assigned.into_iter().map(|(id, _)| id).collect();
    if judge_user_ids.is_empty() {
        return Ok(None);
    }

    let judge_count = judge_user_ids.len() as i64;
    let round_complete = |conn: &mut mysql::PooledConn, round: i64| -> Result<bool, String> {
        let count =
            match_judges_repository::count_distinct_judges_with_valid_scores_for_match_round(
                conn,
                tournament_id,
                match_id,
                round,
                min_allowed,
                max_allowed,
            )
            .map_err(|_| "Storage error.".to_string())?;
        Ok(count == judge_count)
    };

    let is_extension = scheduled
        .draw_system
        .as_deref()
        .unwrap_or("")
        .eq_ignore_ascii_case("Extension");

    // For Extension draw-system, never add (or keep) extension rounds until all default rounds are fully scored.
    let mut base_complete = true;
    for r in 1..=base_rounds {
        if !round_complete(&mut conn, r)? {
            base_complete = false;
            break;
        }
    }

    // If extension rounds were previously added prematurely (before completing default rounds),
    // remove them by rolling back the match fight_round and deleting any extension score rows.
    if is_extension
        && !base_complete
        && (match_row.fight_round.unwrap_or(1) > base_rounds || max_scored_round > base_rounds)
    {
        let _ = matches_repository::set_status_and_fight_round(
            &mut conn,
            tournament_id,
            match_id,
            &match_row.status,
            base_rounds,
        )
        .map_err(|_| "Storage error.".to_string())?;
        let _ = match_judges_repository::delete_rounds_gt(
            &mut conn,
            tournament_id,
            match_id,
            base_rounds,
        )
        .map_err(|_| "Storage error.".to_string())?;
        let (sum_red, sum_blue) =
            total_scores_for_match(&mut conn, tournament_id, match_id, base_rounds)?;
        let _ = matches_repository::set_totals(
            &mut conn,
            tournament_id,
            match_id,
            sum_red.min(i64::from(i32::MAX)) as i32,
            sum_blue.min(i64::from(i32::MAX)) as i32,
        );
        return Ok(None);
    }

    // Do not finalize/extend until all currently-relevant rounds are fully scored by all assigned judges.
    for r in 1..=current_rounds {
        if !round_complete(&mut conn, r)? {
            return Ok(None);
        }
    }

    let (sum_red, sum_blue) =
        total_scores_for_match(&mut conn, tournament_id, match_id, current_rounds)?;
    let winner_side = if sum_red > sum_blue {
        "red"
    } else if sum_blue > sum_red {
        "blue"
    } else {
        if is_extension {
            if !base_complete {
                return Ok(None);
            }
            let next_round = current_rounds + 1;
            let status = if match_row.status.eq_ignore_ascii_case("Ongoing") {
                match_row.status.as_str()
            } else {
                "Ongoing"
            };
            let _ = matches_repository::set_status_and_fight_round(
                &mut conn,
                tournament_id,
                match_id,
                status,
                next_round,
            )
            .map_err(|_| "Storage error.".to_string())?;
            return Ok(Some(next_round));
        }
        return Ok(None);
    };

    // Reuse the existing finalize logic (bracket progression + scheduled event winner).
    let scheduled_event_id = match_row.scheduled_event_id;
    let location = match_row.location.clone();
    let match_time = match_row.match_time.clone();
    drop(conn);

    update_contact_match(
        state,
        actor_user_id,
        tournament_id,
        match_id,
        scheduled_event_id,
        "Finished",
        location.as_deref(),
        match_time.as_deref(),
        Some(winner_side),
        judge_user_ids,
    )?;

    Ok(None)
}

fn finalize_first_point_advantage_match(
    conn: &mut mysql::PooledConn,
    tournament_id: i64,
    scheduled_event_id: i64,
    scheduled_event: &crate::models::ScheduledEvent,
    match_row: &ScheduledMatch,
    winner_side: &str,
) -> Result<(), String> {
    if !is_contact_first_point_advantage(scheduled_event) {
        return Err("Match is not configured for first-point advantage scoring.".to_string());
    }

    let winner_side = winner_side.trim();
    let (winner_label, winner_id) = match winner_side {
        "red" => (
            match_row
                .red
                .clone()
                .filter(|value| !value.trim().is_empty()),
            match_row.red_member_id,
        ),
        "blue" => (
            match_row
                .blue
                .clone()
                .filter(|value| !value.trim().is_empty()),
            match_row.blue_member_id,
        ),
        _ => return Err("Invalid winner selection.".to_string()),
    };
    let winner_label = winner_label.ok_or_else(|| "Winner not found.".to_string())?;

    let changed = matches_repository::update(
        conn,
        tournament_id,
        match_row.id,
        scheduled_event_id,
        match_row.mat.as_deref(),
        match_row.category.as_deref(),
        match_row.red.as_deref(),
        match_row.blue.as_deref(),
        "Finished",
        match_row.location.as_deref(),
        match_row.match_time.as_deref(),
        match_row.round,
        match_row.slot,
        match_row.red_member_id,
        match_row.blue_member_id,
        match_row.is_bye,
        Some(winner_side),
        match_row.red_total_score,
        match_row.blue_total_score,
    )
    .map_err(|_| "Storage error.".to_string())?;
    if changed == 0 {
        return Err("Match not found for this event.".to_string());
    }

    // Stop timer if still running/paused mid-round.
    let _ = matches_repository::set_timer_state(
        conn,
        tournament_id,
        scheduled_event_id,
        match_row.id,
        "Finished",
        match_row.fight_round,
        None,
        None,
        false,
        match_row.timer_last_completed_round,
    );

    let round = match match_row.round {
        Some(value) => value,
        None => return Ok(()),
    };
    let slot = match match_row.slot {
        Some(value) => value,
        None => return Ok(()),
    };

    // Advance winner into next bracket match, if one exists.
    let next_round = round + 1;
    let next_slot = (slot + 1) / 2;
    let mut target = matches_repository::get_by_round_slot(
        conn,
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
            conn,
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
            target_match.red_total_score,
            target_match.blue_total_score,
        )
        .map_err(|_| "Storage error.".to_string())?;
        if changed == 0 {
            return Err("Next round match not found.".to_string());
        }
    }

    // If this was the final match for the scheduled event, finalize the scheduled event outcome too.
    let is_final = conn
        .exec_first::<Option<i64>, _, _>(
            "SELECT id FROM matches WHERE tournament_id = ? AND scheduled_event_id = ? AND round > ? LIMIT 1",
            (tournament_id, scheduled_event_id, round),
        )
        .map_err(|_| "Storage error.".to_string())?
        .is_none();
    if is_final {
        let winner_member_id = match winner_side {
            "red" => match_row.red_member_id,
            "blue" => match_row.blue_member_id,
            _ => None,
        };
        let _ = scheduled_events_repository::update_status_and_winner(
            conn,
            tournament_id,
            scheduled_event_id,
            "Finished",
            winner_member_id,
        );
    }

    Ok(())
}

fn prepare_judge_scores_for_match_round(
    conn: &mut mysql::PooledConn,
    state: &State<AppState>,
    tournament_id: i64,
    match_id: i64,
    fight_round: i64,
    judge_user_ids: &[i64],
) -> Result<Vec<MatchJudgeScore>, String> {
    if judge_user_ids.is_empty() {
        return Ok(Vec::new());
    }
    if judge_user_ids.len() < 3 || judge_user_ids.len() > 5 {
        return Err("Add between 3 and 5 judges.".to_string());
    }

    let judge_users = list_judges(state, tournament_id);
    let judge_map: HashMap<i64, AccessUser> = judge_users
        .into_iter()
        .map(|judge| (judge.id, judge))
        .collect();

    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();

    for (index, judge_user_id) in judge_user_ids.iter().copied().enumerate() {
        if !seen.insert(judge_user_id) {
            return Err("Duplicate judges are not allowed.".to_string());
        }
        let judge_user = judge_map
            .get(&judge_user_id)
            .ok_or_else(|| "Selected judge is invalid.".to_string())?;
        let (red_score, blue_score) = match_judges_repository::get_score(
            conn,
            tournament_id,
            match_id,
            judge_user_id,
            fight_round,
        )
        .map_err(|_| "Storage error.".to_string())?
        .unwrap_or((0, 0));

        result.push(MatchJudgeScore {
            judge_user_id,
            judge_name: judge_user.name.clone(),
            judge_photo_url: judge_user.photo_url.clone(),
            red_score,
            blue_score,
            judge_order: (index as i32) + 1,
        });
    }

    Ok(result)
}

pub fn ensure_bracket_for_contact_event(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    scheduled_event_id: i64,
    _event_id: i64,
) -> Result<(), String> {
    let competitors = list_competitors(state, user_id, tournament_id, scheduled_event_id)?;
    if competitors.is_empty() {
        return Ok(());
    }
    let existing = list(state, user_id, tournament_id, scheduled_event_id)?;
    if !existing.is_empty() {
        let mut existing_ids = std::collections::HashSet::new();
        for item in &existing {
            if let Some(id) = item.red_member_id {
                existing_ids.insert(id);
            }
            if let Some(id) = item.blue_member_id {
                existing_ids.insert(id);
            }
        }
        let competitor_ids: std::collections::HashSet<i64> =
            competitors.iter().map(|c| c.member_id).collect();
        let has_new_competitors = !competitor_ids.is_subset(&existing_ids);
        let has_locked_matches = existing.iter().any(|m| {
            m.status.eq_ignore_ascii_case("Finished")
                || m.status.eq_ignore_ascii_case("Forfeit")
                || m.winner_side.is_some()
        });
        if !has_new_competitors {
            return Ok(());
        }
        if has_locked_matches {
            return Ok(());
        }
        let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
        match_judges_repository::delete_by_scheduled_event(
            &mut conn,
            tournament_id,
            scheduled_event_id,
        )
        .map_err(|_| "Storage error.".to_string())?;
        matches_repository::delete_by_scheduled_event(&mut conn, tournament_id, scheduled_event_id)
            .map_err(|_| "Storage error.".to_string())?;
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
                        0,
                        0,
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
                        0,
                        0,
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
                        0,
                        0,
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
                        0,
                        0,
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
                        0,
                        0,
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
                        0,
                        0,
                    )?;
                    slot += 1;
                    BracketParticipant::ByeCarry(
                        blue_competitor.name.clone(),
                        Some(blue_competitor.member_id),
                    )
                }
                (Some(BracketParticipant::Winner(red_label)), None) => {
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
                        0,
                        0,
                    )?;
                    slot += 1;
                    BracketParticipant::Winner(red_label.clone())
                }
                (None, Some(BracketParticipant::Winner(blue_label))) => {
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
                        0,
                        0,
                    )?;
                    slot += 1;
                    BracketParticipant::Winner(blue_label.clone())
                }
                (
                    Some(BracketParticipant::Winner(red_label)),
                    Some(BracketParticipant::ByeCarry(blue_label, blue_id)),
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
                        *blue_id,
                        false,
                        0,
                        0,
                    )?;
                    next_match_number += 1;
                    slot += 1;
                    BracketParticipant::Winner(format!("Winner of Match {}", match_number))
                }
                (
                    Some(BracketParticipant::ByeCarry(red_label, red_id)),
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
                        *red_id,
                        None,
                        false,
                        0,
                        0,
                    )?;
                    next_match_number += 1;
                    slot += 1;
                    BracketParticipant::Winner(format!("Winner of Match {}", match_number))
                }
                (
                    Some(BracketParticipant::ByeCarry(red_label, red_id)),
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
                        *red_id,
                        Some(blue_competitor.member_id),
                        false,
                        0,
                        0,
                    )?;
                    next_match_number += 1;
                    slot += 1;
                    BracketParticipant::Winner(format!("Winner of Match {}", match_number))
                }
                (
                    Some(BracketParticipant::Competitor(red_competitor)),
                    Some(BracketParticipant::ByeCarry(blue_label, blue_id)),
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
                        *blue_id,
                        false,
                        0,
                        0,
                    )?;
                    next_match_number += 1;
                    slot += 1;
                    BracketParticipant::Winner(format!("Winner of Match {}", match_number))
                }
                (
                    Some(BracketParticipant::ByeCarry(red_label, red_id)),
                    Some(BracketParticipant::ByeCarry(blue_label, blue_id)),
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
                        *red_id,
                        *blue_id,
                        false,
                        0,
                        0,
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
                        0,
                        0,
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
                        0,
                        0,
                    )?;
                    slot += 1;
                    BracketParticipant::ByeCarry(blue_label.clone(), *blue_id)
                }
                (
                    Some(BracketParticipant::ByeCarry(red_label, red_id)),
                    Some(BracketParticipant::Unknown),
                )
                | (
                    Some(BracketParticipant::Unknown),
                    Some(BracketParticipant::ByeCarry(red_label, red_id)),
                ) => BracketParticipant::ByeCarry(red_label.clone(), *red_id),
                (
                    Some(BracketParticipant::Winner(red_label)),
                    Some(BracketParticipant::Unknown),
                )
                | (
                    Some(BracketParticipant::Unknown),
                    Some(BracketParticipant::Winner(red_label)),
                ) => BracketParticipant::Winner(red_label.clone()),
                (
                    Some(BracketParticipant::Competitor(red_competitor)),
                    Some(BracketParticipant::Unknown),
                )
                | (
                    Some(BracketParticipant::Unknown),
                    Some(BracketParticipant::Competitor(red_competitor)),
                ) => BracketParticipant::Competitor(red_competitor.clone()),
                (Some(BracketParticipant::Unknown), Some(BracketParticipant::Unknown))
                | (Some(BracketParticipant::Unknown), None)
                | (None, Some(BracketParticipant::Unknown))
                | (None, None) => BracketParticipant::Unknown,
            };
            next_round.push(next_participant);
        }

        current_round = next_round;
        round += 1;
    }

    Ok(())
}

pub fn reset_automatic_matchmaking(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    scheduled_event_id: i64,
) -> Result<(), String> {
    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }

    let scheduled =
        scheduled_events_repository::get_by_id(&mut conn, tournament_id, scheduled_event_id)
            .map_err(|_| "Storage error.".to_string())?
            .ok_or_else(|| "Event not found for this tournament.".to_string())?;

    if !scheduled.contact_type.eq_ignore_ascii_case("Contact") {
        return Err("Automatic matchmaking is only available for contact events.".to_string());
    }

    match_judges_repository::delete_by_scheduled_event(
        &mut conn,
        tournament_id,
        scheduled_event_id,
    )
    .map_err(|_| "Storage error.".to_string())?;
    matches_repository::delete_by_scheduled_event(&mut conn, tournament_id, scheduled_event_id)
        .map_err(|_| "Storage error.".to_string())?;

    let _ = scheduled_events_repository::update_status_and_winner(
        &mut conn,
        tournament_id,
        scheduled_event_id,
        "Scheduled",
        None,
    );

    // Rebuild the bracket (premade matches) using the same logic as the event page.
    ensure_bracket_for_contact_event(
        state,
        user_id,
        tournament_id,
        scheduled_event_id,
        scheduled.event_id,
    )
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
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let swap_index = (state % ((index + 1) as u64)) as usize;
        competitors.swap(index, swap_index);
    }
    competitors
}

fn status_class(status: &str) -> &'static str {
    if status.eq_ignore_ascii_case("Finished") {
        "status-ok"
    } else if status.eq_ignore_ascii_case("Ongoing") {
        "status-live"
    } else {
        "status-ready"
    }
}

fn populate_judge_scores(
    conn: &mut mysql::PooledConn,
    tournament_id: i64,
    matches: &mut [ScheduledMatch],
) -> Result<(), String> {
    for item in matches.iter_mut() {
        let fight_round = item.fight_round.or(item.round).unwrap_or(1);
        item.judge_scores =
            match_judges_repository::list_by_match(conn, tournament_id, item.id, fight_round)
                .map_err(|_| "Storage error.".to_string())?;
    }
    Ok(())
}

fn prepare_judge_scores(
    state: &State<AppState>,
    tournament_id: i64,
    judges: &[MatchJudgeInput],
) -> Result<Vec<MatchJudgeScore>, String> {
    if judges.is_empty() {
        return Ok(Vec::new());
    }
    if judges.len() < 3 || judges.len() > 5 {
        return Err("Add between 3 and 5 judges.".to_string());
    }

    let judge_users = list_judges(state, tournament_id);
    let judge_map: HashMap<i64, AccessUser> = judge_users
        .into_iter()
        .map(|judge| (judge.id, judge))
        .collect();
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();

    for (index, judge) in judges.iter().enumerate() {
        if !seen.insert(judge.judge_user_id) {
            return Err("Duplicate judges are not allowed.".to_string());
        }
        let judge_user = judge_map
            .get(&judge.judge_user_id)
            .ok_or_else(|| "Selected judge is invalid.".to_string())?;
        if judge.red_score < 0 || judge.blue_score < 0 {
            return Err("Judge scores must be zero or higher.".to_string());
        }
        result.push(MatchJudgeScore {
            judge_user_id: judge.judge_user_id,
            judge_name: judge_user.name.clone(),
            judge_photo_url: judge_user.photo_url.clone(),
            red_score: judge.red_score,
            blue_score: judge.blue_score,
            judge_order: (index as i32) + 1,
        });
    }

    Ok(result)
}
