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
    pub point_system: Option<String>,
    pub time_rule: Option<String>,
    pub division_id: Option<i64>,
    pub weight_class_id: Option<i64>,
}

#[derive(FromForm)]
pub struct MatchForm {
    pub mat: Option<String>,
    pub category: Option<String>,
    pub red: Option<String>,
    pub blue: Option<String>,
    pub status: Option<String>,
    pub winner: Option<String>,
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
    let divisions = settings_service::list(state, tournament.id, SettingsEntity::Division);
    let weight_classes = settings_service::list(state, tournament.id, SettingsEntity::WeightClass);
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
            divisions: divisions,
            weight_classes: weight_classes,
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
        matches_service::list_competitors(state, user.id, tournament.id, event.id)
            .unwrap_or_default();
    let is_contact = event.contact_type.eq_ignore_ascii_case("Contact");
    let max_round = matches
        .iter()
        .filter_map(|item| item.round)
        .max()
        .unwrap_or(1);
    let rounds: Vec<i64> = (1..=max_round).collect();

    #[derive(Serialize, Clone)]
    struct BracketMatchView {
        id: i64,
        round: i64,
        slot: i64,
        is_bye: bool,
        has_next: bool,
        header_label: String,
        top_label: String,
        bottom_label: String,
        top_photo: String,
        bottom_photo: String,
        winner_side: Option<String>,
        x: f32,
        y: f32,
    }

    #[derive(Serialize)]
    struct BracketRoundView {
        index: i64,
        round: i64,
        title: String,
        x: f32,
    }

    #[derive(Serialize)]
    struct BracketConnector {
        path: String,
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
    let mut bracket_matches: Vec<BracketMatchView> = Vec::new();
    let mut bracket_connectors: Vec<BracketConnector> = Vec::new();
    let mut contact_match_rows: Vec<ContactMatchRow> = Vec::new();
    let box_width: f32 = 220.0;
    let box_height: f32 = 90.0;
    let header_height: f32 = 24.0;
    let box_total_height: f32 = box_height + header_height;
    let gap: f32 = 60.0;
    let round_gap: f32 = 280.0;
    let margin_left: f32 = 32.0;
    let margin_top: f32 = 48.0;
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
    let mut rounds_map: HashMap<i64, Vec<&ScheduledMatch>> = HashMap::new();
    for item in &matches {
        let round = item.round.unwrap_or(1);
        rounds_map.entry(round).or_default().push(item);
    }
    for items in rounds_map.values_mut() {
        items.sort_by(|a, b| a.slot.unwrap_or(0).cmp(&b.slot.unwrap_or(0)));
    }

    fn format_match_time(raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        let mut parts = trimmed.split('T');
        let date_part = parts.next().unwrap_or("");
        let time_part = parts.next().unwrap_or("");
        let mut date_iter = date_part.split('-');
        let year = date_iter.next().unwrap_or("");
        let month = date_iter.next().unwrap_or("");
        let day = date_iter.next().unwrap_or("");
        let mut time_iter = time_part.split(':');
        let hour_str = time_iter.next().unwrap_or("");
        let minute = time_iter.next().unwrap_or("");
        let hour: u32 = hour_str.parse().ok()?;
        let (display_hour, suffix) = match hour {
            0 => (12, "AM"),
            1..=11 => (hour, "AM"),
            12 => (12, "PM"),
            _ => (hour - 12, "PM"),
        };
        if year.is_empty() || month.is_empty() || day.is_empty() || minute.is_empty() {
            return None;
        }
        Some(format!(
            "{}/{}/{} {:02}:{:02}{}",
            month,
            day,
            year,
            display_hour,
            minute,
            suffix
        ))
    }

    let round1_count = rounds_map.get(&1).map(|r| r.len()).unwrap_or(0);
    let total_height =
        round1_count as f32 * box_total_height + (round1_count.saturating_sub(1) as f32) * gap;
    let canvas_height = total_height + margin_top * 2.0 + header_height;

    let mut center_map: HashMap<(i64, i64), f32> = HashMap::new();

    for (index, round) in rounds.iter().enumerate() {
        let round_x = margin_left + (index as f32) * round_gap;
        let title = if *round == 1 {
            "Round 1".to_string()
        } else if *round == max_round {
            "Final".to_string()
        } else if *round == max_round - 1 {
            "Semifinals".to_string()
        } else {
            "Round".to_string()
        };
        bracket_rounds.push(BracketRoundView {
            index: (index as i64) + 1,
            round: *round,
            title,
            x: round_x,
        });

        let items = rounds_map.get(round).cloned().unwrap_or_default();
        for (i, item) in items.iter().enumerate() {
            let slot = item.slot.unwrap_or(0);
            let red = item.red_member_id.and_then(|id| competitor_map.get(&id).cloned());
            let blue = item.blue_member_id.and_then(|id| competitor_map.get(&id).cloned());
            let red_name = red.clone().map(|(name, _)| name).unwrap_or_default();
            let blue_name = blue.clone().map(|(name, _)| name).unwrap_or_default();
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
                    .unwrap_or_else(|| "Advancing".to_string())
            };
            let bottom_label = if item.is_bye {
                "BYE".to_string()
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
                    .unwrap_or_else(|| "Advancing".to_string())
            };
            let top_photo = if *round == 1 {
                red.map(|(_, photo)| photo).unwrap_or_default()
            } else {
                String::new()
            };
            let bottom_photo = if *round == 1 {
                blue.map(|(_, photo)| photo).unwrap_or_default()
            } else {
                String::new()
            };
            let match_number = match_number_by_id.get(&item.id).copied();
            let formatted_time = item
                .match_time
                .as_deref()
                .and_then(format_match_time);
            let header_label = if item.is_bye {
                "BYE".to_string()
            } else if *round == max_round {
                format!("Finals Match (Match {})", match_number.unwrap_or(0))
            } else if *round == max_round - 1 {
                format!("Semi Finals Match (Match {})", match_number.unwrap_or(0))
            } else {
                format!("Match {}", match_number.unwrap_or(0))
            };
            let header_label = if let Some(time_label) = formatted_time {
                format!("{} - {}", header_label, time_label)
            } else {
                header_label
            };
            let center_y = if *round == 1 {
                margin_top + (i as f32) * (box_total_height + gap) + (box_total_height / 2.0)
            } else {
                let feeder_a = (slot - 1) * 2 + 1;
                let feeder_b = feeder_a + 1;
                let prev_a = center_map
                    .get(&(*round - 1, feeder_a))
                    .copied()
                    .unwrap_or(margin_top + box_height / 2.0);
                let prev_b = center_map.get(&(*round - 1, feeder_b)).copied();
                prev_b.map(|value| (prev_a + value) / 2.0).unwrap_or(prev_a)
            };
            center_map.insert((*round, slot), center_y);

            let matchup_label = format!("{} vs {}", top_label, bottom_label);

            bracket_matches.push(BracketMatchView {
                id: item.id,
                round: *round,
                slot,
                is_bye: item.is_bye,
                has_next: *round < max_round,
                header_label,
                top_label: top_label.clone(),
                bottom_label: bottom_label.clone(),
                top_photo,
                bottom_photo,
                winner_side: item.winner_side.clone(),
                x: round_x,
                y: center_y - box_total_height / 2.0,
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
    }

    let mut final_center_y = margin_top + box_total_height / 2.0;
    if let Some(final_match) = bracket_matches
        .iter()
        .filter(|m| m.round == max_round)
        .min_by(|a, b| a.slot.cmp(&b.slot))
    {
        final_center_y = final_match.y + box_total_height / 2.0;
    }
    let champion_x = margin_left + (rounds.len() as f32) * round_gap;
    let champion_width = box_width + 40.0;
    let champion_height = 48.0;

    let mut match_lookup: HashMap<(i64, i64), BracketMatchView> = HashMap::new();
    for item in &bracket_matches {
        match_lookup.insert((item.round, item.slot), item.clone());
    }
    for item in &bracket_matches {
        if item.round >= max_round {
            continue;
        }
        let next_slot = (item.slot + 1) / 2;
        let target = match_lookup.get(&(item.round + 1, next_slot));
        if let Some(target_match) = target {
            let start_x = item.x + box_width;
            let start_y = item.y + box_total_height / 2.0;
            let end_x = target_match.x;
            let end_y = target_match.y + box_total_height / 2.0;
            let mid_x = (start_x + end_x) / 2.0;
            let path = format!(
                "M {} {} L {} {} L {} {} L {} {}",
                start_x, start_y, mid_x, start_y, mid_x, end_y, end_x, end_y
            );
            bracket_connectors.push(BracketConnector { path });
        }
    }

    let canvas_width = champion_x + champion_width + margin_left;

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
            bracket_rounds: bracket_rounds,
            bracket_matches: bracket_matches,
            bracket_connectors: bracket_connectors,
            bracket_canvas_width: canvas_width,
            bracket_canvas_height: canvas_height,
            bracket_box_width: box_width,
            bracket_box_height: box_height,
            bracket_header_height: header_height,
            bracket_box_total_height: box_total_height,
            champion_x: champion_x,
            champion_y: final_center_y - (champion_height / 2.0),
            champion_width: champion_width,
            champion_height: champion_height,
            contact_match_rows: contact_match_rows,
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
    let point_system = form
        .point_system
        .as_deref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    let time_rule = form
        .time_rule
        .as_deref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    match scheduled_events_service::create(
        state,
        user.id,
        tournament.id,
        form.event_id,
        &form.contact_type,
        &form.status,
        location,
        event_time,
        point_system,
        time_rule,
        form.division_id,
        form.weight_class_id,
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
    let point_system = form
        .point_system
        .as_deref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    let time_rule = form
        .time_rule
        .as_deref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
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
        point_system,
        time_rule,
        form.division_id,
        form.weight_class_id,
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
            let status = match form.status.as_deref() {
                Some(value) if !value.trim().is_empty() => value.trim(),
                _ => "Scheduled",
            };
            matches_service::update_contact_match(
                state,
                user.id,
                tournament.id,
                id,
                event_id,
                status,
                location,
                match_time,
                form.winner.as_deref().map(|value| value.trim()).filter(|value| !value.is_empty()),
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
