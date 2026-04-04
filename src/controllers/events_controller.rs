use crate::models::ScheduledMatch;
use crate::services::{auth_service, matches_service, scheduled_events_service, settings_service, tournament_service};
use crate::services::settings_service::SettingsEntity;
use crate::state::AppState;
use rocket::form::{Form, FromForm};
use rocket::http::{Cookie, CookieJar, Status};
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};
use serde::Serialize;
use std::collections::HashMap;

#[derive(FromForm)]
pub struct EventForm {
    pub event_id: i64,
    pub contact_type: String,
    pub status: String,
    pub location: Option<String>,
    pub event_time: Option<String>,
}

#[derive(FromForm)]
pub struct MatchForm {
    pub mat: Option<String>,
    pub category: Option<String>,
    pub red: Option<String>,
    pub blue: Option<String>,
    pub status: Option<String>,
    pub location: Option<String>,
    pub match_time: Option<String>,
}

#[get("/<slug>/events?<error>&<success>")]
pub fn events_page(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    error: Option<String>,
    success: Option<String>,
) -> Result<Template, Redirect> {
    let user = match auth_service::current_user(state, jar) {
        Some(user) => user,
        None => {
            return Err(Redirect::to(uri!(
                crate::controllers::auth_controller::auth_page(
                    error = Option::<String>::None,
                    success = Option::<String>::None
                )
            )))
        }
    };

    let tournament = match tournament_service::get_by_slug_for_user(state, &slug, user.id) {
        Some(tournament) => tournament,
        None => {
            return Err(Redirect::to(uri!(
                crate::controllers::dashboard_controller::dashboard
            )))
        }
    };

    jar.add(Cookie::new("last_tournament_slug", tournament.slug.clone()));

    let events = scheduled_events_service::list(state, user.id, tournament.id).unwrap_or_default();
    let event_options = settings_service::list(state, tournament.id, SettingsEntity::Event);
    let scheduled_ids: Vec<i64> = events.iter().map(|item| item.event_id).collect();
    let contact_types = scheduled_events_service::contact_types();
    let statuses = scheduled_events_service::statuses();

    Ok(Template::render(
        "events",
        context! {
            name: user.name,
            tournament_name: tournament.name,
            tournament_slug: tournament.slug,
            events: events,
            event_options: event_options,
            scheduled_ids: scheduled_ids,
            contact_types: contact_types,
            statuses: statuses,
            error: error,
            success: success,
            active: "events",
            is_setup: tournament.is_setup,
        },
    ))
}

#[get("/<slug>/events/<id>")]
pub fn event_profile(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    id: i64,
) -> Result<Template, Redirect> {
    let user = match auth_service::current_user(state, jar) {
        Some(user) => user,
        None => {
            return Err(Redirect::to(uri!(
                crate::controllers::auth_controller::auth_page(
                    error = Option::<String>::None,
                    success = Option::<String>::None
                )
            )))
        }
    };

    let tournament = match tournament_service::get_by_slug_for_user(state, &slug, user.id) {
        Some(tournament) => tournament,
        None => {
            return Err(Redirect::to(uri!(
                crate::controllers::dashboard_controller::dashboard
            )))
        }
    };

    let event = match scheduled_events_service::get_by_id(state, user.id, tournament.id, id) {
        Ok(Some(event)) => event,
        _ => {
            return Err(Redirect::to(uri!(events_page(
                slug = slug,
                error = Some("Event not found.".to_string()),
                success = Option::<String>::None
            ))))
        }
    };

    if event.contact_type.eq_ignore_ascii_case("Contact") {
        let _ = matches_service::ensure_bracket_for_contact_event(
            state,
            user.id,
            tournament.id,
            event.id,
            event.event_id,
        );
    }

    let mut matches = matches_service::list(state, user.id, tournament.id, id).unwrap_or_default();
    if event.contact_type.eq_ignore_ascii_case("Contact") {
        matches.sort_by(|a, b| {
            let ra = a.round.unwrap_or(1);
            let rb = b.round.unwrap_or(1);
            let sa = a.slot.unwrap_or(0);
            let sb = b.slot.unwrap_or(0);
            ra.cmp(&rb).then(sa.cmp(&sb))
        });
    }
    let match_statuses = matches_service::statuses();
    let competitors =
        matches_service::list_competitors(state, user.id, tournament.id, event.event_id)
            .unwrap_or_default();
    let is_contact = event.contact_type.eq_ignore_ascii_case("Contact");
    let max_round = matches
        .iter()
        .filter_map(|item| item.round)
        .max()
        .unwrap_or(1);
    let rounds: Vec<i64> = (1..=max_round).collect();
    let bracket_rows = 1_i64 << (max_round as u32 + 1);
    let champion_col = rounds.len() as i64 + 1;

    #[derive(Serialize)]
    struct BracketMatchView {
        id: i64,
        round: i64,
        slot: i64,
        row_start: i64,
        row_span: i64,
        is_upper: bool,
        has_prev: bool,
        has_next: bool,
        is_bye: bool,
        status: String,
        location: Option<String>,
        match_time: Option<String>,
        red_member_id: Option<i64>,
        blue_member_id: Option<i64>,
        red_name: String,
        blue_name: String,
        red_photo: String,
        blue_photo: String,
        advancement_label: String,
        matchup_label: String,
        header_label: String,
        top_label: String,
        bottom_label: String,
        stage_class: String,
    }

    #[derive(Serialize)]
    struct BracketRoundView {
        index: i64,
        round: i64,
        title: String,
        matches: Vec<BracketMatchView>,
    }

    #[derive(Serialize)]
    struct ContactMatchRow {
        id: i64,
        round: Option<i64>,
        matchup_label: String,
        match_time: Option<String>,
        location: Option<String>,
        status: String,
    }

    let mut competitor_map: HashMap<i64, (String, String)> = HashMap::new();
    for competitor in &competitors {
        competitor_map.insert(
            competitor.member_id,
            (
                competitor.name.clone(),
                competitor.photo_url.clone().unwrap_or_default(),
            ),
        );
    }

    let mut bracket_rounds: Vec<BracketRoundView> = Vec::new();
    let mut contact_match_rows: Vec<ContactMatchRow> = Vec::new();
    let mut champion_row: i64 = bracket_rows / 2;
    let mut match_number_by_id: HashMap<i64, i64> = HashMap::new();
    let mut ordered_for_numbers: Vec<&ScheduledMatch> = matches.iter().collect();
    ordered_for_numbers.sort_by(|a, b| {
        let ra = a.round.unwrap_or(1);
        let rb = b.round.unwrap_or(1);
        let sa = a.slot.unwrap_or(0);
        let sb = b.slot.unwrap_or(0);
        ra.cmp(&rb).then(sa.cmp(&sb))
    });
    let mut next_match_number = 1i64;
    for item in ordered_for_numbers {
        if !item.is_bye {
            match_number_by_id.insert(item.id, next_match_number);
            next_match_number += 1;
        }
    }
    for (index, round) in rounds.iter().enumerate() {
        let mut round_matches: Vec<BracketMatchView> = Vec::new();
        for item in matches.iter().filter(|item| item.round.unwrap_or(1) == *round) {
            let slot = item.slot.unwrap_or(0);
            let round_span = 1_i64 << (*round as u32);
            let row_start = (slot * round_span) + (round_span / 2) + 1;
            let red = item.red_member_id.and_then(|id| competitor_map.get(&id).cloned());
            let blue = item.blue_member_id.and_then(|id| competitor_map.get(&id).cloned());
            let red_name = red.clone().map(|(name, _)| name).unwrap_or_default();
            let blue_name = blue.clone().map(|(name, _)| name).unwrap_or_default();
            let matchup_label = if !red_name.is_empty() && !blue_name.is_empty() {
                format!("{} vs {}", red_name, blue_name)
            } else if let (Some(red_label), Some(blue_label)) = (&item.red, &item.blue) {
                format!("{} vs {}", red_label, blue_label)
            } else if !red_name.is_empty() && item.is_bye {
                format!("{} - bye", red_name)
            } else if let Some(red_label) = &item.red {
                red_label.clone()
            } else if !red_name.is_empty() {
                red_name.clone()
            } else if !blue_name.is_empty() {
                blue_name.clone()
            } else {
                "TBD vs TBD".to_string()
            };
            let top_label = if *round == 1 {
                if !red_name.is_empty() {
                    red_name.clone()
                } else {
                    item.red.clone().unwrap_or_else(|| "TBD".to_string())
                }
            } else {
                item.red
                    .clone()
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| "TBD".to_string())
            };
            let bottom_label = if item.is_bye {
                String::new()
            } else if *round == 1 {
                if !blue_name.is_empty() {
                    blue_name.clone()
                } else {
                    item.blue.clone().unwrap_or_else(|| "TBD".to_string())
                }
            } else {
                item.blue
                    .clone()
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| "TBD".to_string())
            };
            let match_number = match_number_by_id.get(&item.id).copied();
            let header_label = if item.is_bye {
                "BYE".to_string()
            } else if *round == max_round {
                format!("Finals Match (Match {})", match_number.unwrap_or(0))
            } else if *round == max_round - 1 {
                format!("Semi Finals Match (Match {})", match_number.unwrap_or(0))
            } else {
                format!("Match {}", match_number.unwrap_or(0))
            };
            let stage_class = if *round == 1 {
                "stage-round-1".to_string()
            } else if *round == max_round {
                "stage-finals".to_string()
            } else if *round == max_round - 1 {
                "stage-semi-finals".to_string()
            } else {
                "stage-round-2".to_string()
            };
            round_matches.push(BracketMatchView {
                id: item.id,
                round: *round,
                slot,
                row_start,
                row_span: round_span,
                is_upper: slot % 2 == 0,
                has_prev: *round > 1,
                has_next: *round < max_round,
                is_bye: item.is_bye,
                status: item.status.clone(),
                location: item.location.clone(),
                match_time: item.match_time.clone(),
                red_member_id: item.red_member_id,
                blue_member_id: item.blue_member_id,
                red_name,
                blue_name,
                red_photo: red.map(|(_, photo)| photo).unwrap_or_default(),
                blue_photo: blue.map(|(_, photo)| photo).unwrap_or_default(),
                advancement_label: top_label.clone(),
                matchup_label: matchup_label.clone(),
                header_label,
                top_label,
                bottom_label,
                stage_class,
            });
            contact_match_rows.push(ContactMatchRow {
                id: item.id,
                round: item.round,
                matchup_label,
                match_time: item.match_time.clone(),
                location: item.location.clone(),
                status: item.status.clone(),
            });
        }
        if *round == max_round {
            if let Some(final_match) = round_matches.first() {
                champion_row = final_match.row_start;
            }
        }
        let title = if *round == 1 {
            "Round 1".to_string()
        } else if *round == max_round {
            "Finals".to_string()
        } else if *round == max_round - 1 {
            "Semi Finals".to_string()
        } else {
            format!("Round {}", round)
        };
        bracket_rounds.push(BracketRoundView {
            index: (index as i64) + 1,
            round: *round,
            title,
            matches: round_matches,
        });
    }

    Ok(Template::render(
        "event_profile",
        context! {
            name: user.name,
            tournament_name: tournament.name,
            tournament_slug: tournament.slug,
            event: event,
            matches: matches,
            match_statuses: match_statuses,
            competitors: competitors,
            is_contact: is_contact,
            rounds: rounds,
            bracket_rows: bracket_rows,
            bracket_rounds: bracket_rounds,
            contact_match_rows: contact_match_rows,
            champion_row: champion_row,
            champion_col: champion_col,
            active: "events",
            is_setup: tournament.is_setup,
        },
    ))
}

#[post("/<slug>/events", data = "<form>")]
pub fn create_event(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    form: Form<EventForm>,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, user.id)
        .ok_or(Status::NotFound)?;
    let location = form.location.as_deref().map(|value| value.trim()).filter(|value| !value.is_empty());
    let event_time = form.event_time.as_deref().map(|value| value.trim()).filter(|value| !value.is_empty());
    match scheduled_events_service::create(
        state,
        user.id,
        tournament.id,
        form.event_id,
        &form.contact_type,
        &form.status,
        location,
        event_time,
    ) {
        Ok(_) => Ok(Redirect::to(uri!(events_page(
            slug = slug,
            error = Option::<String>::None,
            success = Some("Event scheduled.".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(events_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}

#[post("/<slug>/events/<id>/update", data = "<form>")]
pub fn update_event(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    id: i64,
    form: Form<EventForm>,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, user.id)
        .ok_or(Status::NotFound)?;
    let location = form.location.as_deref().map(|value| value.trim()).filter(|value| !value.is_empty());
    let event_time = form.event_time.as_deref().map(|value| value.trim()).filter(|value| !value.is_empty());
    match scheduled_events_service::update(
        state,
        user.id,
        tournament.id,
        id,
        form.event_id,
        &form.contact_type,
        &form.status,
        location,
        event_time,
    ) {
        Ok(_) => Ok(Redirect::to(uri!(events_page(
            slug = slug,
            error = Option::<String>::None,
            success = Some("Event updated.".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(events_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}

#[post("/<slug>/events/<id>/delete")]
pub fn delete_event(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    id: i64,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, user.id)
        .ok_or(Status::NotFound)?;
    match scheduled_events_service::delete(state, user.id, tournament.id, id) {
        Ok(_) => Ok(Redirect::to(uri!(events_page(
            slug = slug,
            error = Option::<String>::None,
            success = Some("Event deleted.".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(events_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}

#[post("/<slug>/events/<event_id>/matches", data = "<form>")]
pub fn create_match(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    event_id: i64,
    form: Form<MatchForm>,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, user.id)
        .ok_or(Status::NotFound)?;
    let event = scheduled_events_service::get_by_id(state, user.id, tournament.id, event_id)
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound)?;
    if event.contact_type.eq_ignore_ascii_case("Contact") {
        return Ok(Redirect::to(uri!(event_profile(
            slug = slug,
            id = event_id
        ))));
    }
    let mat = form.mat.as_deref().map(|value| value.trim()).filter(|value| !value.is_empty());
    let category = form.category.as_deref().map(|value| value.trim()).filter(|value| !value.is_empty());
    let red = form.red.as_deref().map(|value| value.trim()).filter(|value| !value.is_empty());
    let blue = form.blue.as_deref().map(|value| value.trim()).filter(|value| !value.is_empty());
    let location = form.location.as_deref().map(|value| value.trim()).filter(|value| !value.is_empty());
    let match_time = form.match_time.as_deref().map(|value| value.trim()).filter(|value| !value.is_empty());
    let status = match form.status.as_deref() {
        Some(value) if !value.trim().is_empty() => value.trim(),
        _ => {
            return Ok(Redirect::to(uri!(event_profile(
                slug = slug,
                id = event_id
            ))));
        }
    };
    match matches_service::create(
        state,
        user.id,
        tournament.id,
        event_id,
        mat,
        category,
        red,
        blue,
        status,
        location,
        match_time,
        None,
        None,
        None,
        None,
        false,
    ) {
        Ok(_) => Ok(Redirect::to(uri!(event_profile(
            slug = slug,
            id = event_id
        )))),
        Err(message) => Ok(Redirect::to(uri!(events_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}

#[post("/<slug>/events/<event_id>/matches/<id>/update", data = "<form>")]
pub fn update_match(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    event_id: i64,
    id: i64,
    form: Form<MatchForm>,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, user.id)
        .ok_or(Status::NotFound)?;
    let event = scheduled_events_service::get_by_id(state, user.id, tournament.id, event_id)
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound)?;
    let mat = form.mat.as_deref().map(|value| value.trim()).filter(|value| !value.is_empty());
    let category = form.category.as_deref().map(|value| value.trim()).filter(|value| !value.is_empty());
    let red = form.red.as_deref().map(|value| value.trim()).filter(|value| !value.is_empty());
    let blue = form.blue.as_deref().map(|value| value.trim()).filter(|value| !value.is_empty());
    let location = form.location.as_deref().map(|value| value.trim()).filter(|value| !value.is_empty());
    let match_time = form.match_time.as_deref().map(|value| value.trim()).filter(|value| !value.is_empty());
    let result = if event.contact_type.eq_ignore_ascii_case("Contact") {
        matches_service::update_schedule(
            state,
            user.id,
            tournament.id,
            id,
            event_id,
            location,
            match_time,
        )
    } else {
        let status = match form.status.as_deref() {
            Some(value) if !value.trim().is_empty() => value.trim(),
            _ => "Scheduled",
        };
        matches_service::update(
            state,
            user.id,
            tournament.id,
            id,
            event_id,
            mat,
            category,
            red,
            blue,
            status,
            location,
            match_time,
            None,
            None,
            None,
            None,
            false,
        )
    };
    match result {
        Ok(_) => Ok(Redirect::to(uri!(event_profile(
            slug = slug,
            id = event_id
        )))),
        Err(message) => Ok(Redirect::to(uri!(events_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}

#[post("/<slug>/events/<event_id>/matches/<id>/delete")]
pub fn delete_match(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    event_id: i64,
    id: i64,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, user.id)
        .ok_or(Status::NotFound)?;
    match matches_service::delete(state, user.id, tournament.id, id) {
        Ok(_) => Ok(Redirect::to(uri!(event_profile(
            slug = slug,
            id = event_id
        )))),
        Err(message) => Ok(Redirect::to(uri!(events_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}
