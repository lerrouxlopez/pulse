use crate::services::{
    access_service, auth_service, matches_service, scheduled_events_service, tournament_service,
};
use crate::state::AppState;
use rocket::form::{Form, FromForm};
use rocket::http::{Cookie, CookieJar, Status};
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};
use serde::Serialize;

#[derive(Serialize)]
struct ScoreMatchOption {
    id: i64,
    label: String,
    status: String,
}

#[derive(Serialize)]
struct ScoreJudgeOption {
    id: i64,
    name: String,
}

#[derive(FromForm)]
pub struct ScoreAdjustForm {
    pub match_id: i64,
    pub fight_round: i64,
    pub side: String,
    pub judge_user_id: Option<i64>,
    pub delta: Option<i32>,
    pub value: Option<i32>,
}

#[derive(Serialize)]
struct ScoreRoundTable {
    round: i64,
    red_total: i64,
    blue_total: i64,
    judges: Vec<crate::models::MatchJudgeScore>,
}

#[get("/<slug>/scores?<match_id>&<round>&<judge_id>")]
pub fn scores_page(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    match_id: Option<i64>,
    round: Option<i64>,
    judge_id: Option<i64>,
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

    if !access_service::user_has_permission(state, user.id, tournament.id, "scores") {
        return Err(Redirect::to(uri!(
            crate::controllers::dashboard_controller::tournament_dashboard(slug = tournament.slug)
        )));
    }

    jar.add(Cookie::new("last_tournament_slug", tournament.slug.clone()));
    let tournament_slug = tournament.slug.clone();
    let can_admin = access_service::is_owner(state, user.id, tournament.id)
        || access_service::user_has_permission(state, user.id, tournament.id, "events");
    let mut selected_judge_id = if can_admin {
        judge_id.unwrap_or(user.id)
    } else {
        user.id
    };

    let mut options: Vec<ScoreMatchOption> = Vec::new();
    let scheduled_events =
        scheduled_events_service::list(state, user.id, tournament.id).unwrap_or_default();
    let mut event_map = std::collections::HashMap::new();
    for item in &scheduled_events {
        event_map.insert(item.id, item.event_name.clone());
    }

    let match_rows = if let Ok(mut conn) = crate::db::open_conn(&state.pool) {
        if can_admin {
            crate::repositories::matches_repository::list_by_tournament(&mut conn, tournament.id)
                .unwrap_or_default()
        } else {
            let match_ids = crate::repositories::match_judges_repository::list_match_ids_for_judge(
                &mut conn,
                tournament.id,
                user.id,
            )
            .unwrap_or_default();
            let mut rows = Vec::new();
            for id in match_ids {
                if let Ok(Some(row)) =
                    crate::repositories::matches_repository::get_by_id(&mut conn, tournament.id, id)
                {
                    rows.push(row);
                }
            }
            rows
        }
    } else {
        Vec::new()
    };

    for row in &match_rows {
        // Hide orphan matches (matches whose scheduled event was deleted).
        let Some(event_name) = event_map.get(&row.scheduled_event_id).cloned() else {
            continue;
        };
        let label = format!(
            "{}: {} vs {}",
            event_name,
            row.red.clone().unwrap_or_else(|| "TBD".to_string()),
            row.blue.clone().unwrap_or_else(|| "TBD".to_string())
        );
        options.push(ScoreMatchOption {
            id: row.id,
            label,
            status: row.status.clone(),
        });
    }

    let selected_match_id = match_id.or_else(|| options.first().map(|item| item.id));
    let mut selected_match = None;
    let mut selected_match_detail = None;
    if let Some(selected_id) = selected_match_id {
        selected_match = matches_service::get_match_row(state, user.id, tournament.id, selected_id)
            .ok()
            .flatten();
        selected_match_detail =
            matches_service::get_detail(state, user.id, tournament.id, selected_id)
                .ok()
                .flatten();
    }

    let judges: Vec<ScoreJudgeOption> = if can_admin {
        let mut assigned: Vec<ScoreJudgeOption> = Vec::new();
        if let Some(selected_id) = selected_match_id {
            if let Ok(mut conn) = crate::db::open_conn(&state.pool) {
                if let Ok(items) =
                    crate::repositories::match_judges_repository::list_assigned_judges(
                        &mut conn,
                        tournament.id,
                        selected_id,
                    )
                {
                    assigned = items
                        .into_iter()
                        .map(|(id, name)| ScoreJudgeOption { id, name })
                        .collect();
                }
            }
        }

        if assigned.is_empty() {
            // If the match has no judge assignments yet, keep admin scoring possible by listing all judges.
            matches_service::list_judges(state, tournament.id)
                .into_iter()
                .map(|item| ScoreJudgeOption {
                    id: item.id,
                    name: item.name,
                })
                .collect()
        } else {
            assigned
        }
    } else {
        Vec::new()
    };

    if can_admin && !judges.is_empty() && !judges.iter().any(|j| j.id == selected_judge_id) {
        selected_judge_id = judges[0].id;
    }

    let (rounds, allowed_scores, red_score, blue_score) = if let Some(ref match_row) =
        selected_match
    {
        let scheduled = scheduled_events_service::get_by_id(
            state,
            user.id,
            tournament.id,
            match_row.scheduled_event_id,
        )
        .ok()
        .flatten();
        let time_rule = scheduled
            .as_ref()
            .and_then(|item| scheduled_events_service::parse_time_rule(item.time_rule.as_deref()));
        let point_rule = scheduled.as_ref().and_then(|item| {
            scheduled_events_service::parse_point_rule(item.point_system.as_deref())
        });
        let min_score = point_rule.map(|rule| rule.min).unwrap_or(0);
        let max_score = point_rule.map(|rule| rule.max).unwrap_or(10);
        let base_rounds = time_rule.map(|rule| rule.rounds).unwrap_or(1).max(1);
        let is_extension = scheduled
            .as_ref()
            .and_then(|s| s.draw_system.as_deref())
            .unwrap_or("")
            .eq_ignore_ascii_case("Extension");
        let (max_scored_round, base_complete) = if let Ok(mut conn) =
            crate::db::open_conn(&state.pool)
        {
            let max_scored_round =
                crate::repositories::match_judges_repository::max_fight_round_for_match(
                    &mut conn,
                    tournament.id,
                    match_row.id,
                )
                .unwrap_or(1);
            let assigned_judges =
                crate::repositories::match_judges_repository::list_assigned_judges(
                    &mut conn,
                    tournament.id,
                    match_row.id,
                )
                .unwrap_or_default();
            let judge_count = assigned_judges.len() as i64;
            let mut base_complete = judge_count > 0;
            if base_complete {
                for r in 1..=base_rounds {
                    let count = crate::repositories::match_judges_repository::count_distinct_judges_with_valid_scores_for_match_round(
                        &mut conn,
                        tournament.id,
                        match_row.id,
                        r,
                        min_score,
                        max_score,
                    )
                    .unwrap_or(0);
                    if count != judge_count {
                        base_complete = false;
                        break;
                    }
                }
            }

            // If extension rounds were previously added prematurely, roll them back when loading the match.
            if is_extension
                && !base_complete
                && (match_row.fight_round.unwrap_or(1) > base_rounds
                    || max_scored_round > base_rounds)
            {
                let _ = crate::repositories::matches_repository::set_status_and_fight_round(
                    &mut conn,
                    tournament.id,
                    match_row.id,
                    &match_row.status,
                    base_rounds,
                );
                let _ = crate::repositories::match_judges_repository::delete_rounds_gt(
                    &mut conn,
                    tournament.id,
                    match_row.id,
                    base_rounds,
                );
                let mut sum_red: i64 = 0;
                let mut sum_blue: i64 = 0;
                for r in 1..=base_rounds {
                    if let Ok((red, blue)) =
                        crate::repositories::match_judges_repository::sum_for_match_round(
                            &mut conn,
                            tournament.id,
                            match_row.id,
                            r,
                        )
                    {
                        sum_red = sum_red.saturating_add(red);
                        sum_blue = sum_blue.saturating_add(blue);
                    }
                }
                let _ = crate::repositories::matches_repository::set_totals(
                    &mut conn,
                    tournament.id,
                    match_row.id,
                    sum_red.min(i64::from(i32::MAX)) as i32,
                    sum_blue.min(i64::from(i32::MAX)) as i32,
                );
                (base_rounds, base_complete)
            } else {
                (max_scored_round, base_complete)
            }
        } else {
            (1, false)
        };
        let rounds_total = if is_extension && !base_complete {
            // Hide extension rounds until all default rounds are fully scored.
            base_rounds
        } else {
            base_rounds
                .max(match_row.fight_round.unwrap_or(1))
                .max(max_scored_round)
        };
        let selected_round = round.unwrap_or(1).clamp(1, rounds_total);
        let allowed_scores: Vec<i32> = (min_score..=max_score).collect();

        let (red_score, blue_score) = {
            let mut conn = crate::db::open_conn(&state.pool).ok();
            let score = conn
                .as_mut()
                .and_then(|conn| {
                    crate::repositories::match_judges_repository::get_score(
                        conn,
                        tournament.id,
                        match_row.id,
                        selected_judge_id,
                        selected_round,
                    )
                    .ok()
                    .flatten()
                })
                .unwrap_or((min_score, min_score));
            score
        };

        (
            (1..=rounds_total).collect::<Vec<i64>>(),
            allowed_scores,
            red_score,
            blue_score,
        )
    } else {
        (Vec::new(), Vec::new(), 0, 0)
    };

    let round_tables: Vec<ScoreRoundTable> = if let Some(selected_id) = selected_match_id {
        if rounds.is_empty() {
            Vec::new()
        } else if let Ok(mut conn) = crate::db::open_conn(&state.pool) {
            let mut out = Vec::new();
            for r in &rounds {
                let judges = crate::repositories::match_judges_repository::list_by_match(
                    &mut conn,
                    tournament.id,
                    selected_id,
                    *r,
                )
                .unwrap_or_default();
                let (red_total, blue_total) =
                    crate::repositories::match_judges_repository::sum_for_match_round(
                        &mut conn,
                        tournament.id,
                        selected_id,
                        *r,
                    )
                    .unwrap_or((0, 0));
                out.push(ScoreRoundTable {
                    round: *r,
                    red_total,
                    blue_total,
                    judges,
                });
            }
            out
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    Ok(Template::render(
        "scores",
        context! {
            name: user.name,
            tournament_name: tournament.name,
            tournament_slug: tournament_slug,
            matches: options,
            selected_match_id: selected_match_id,
            selected_match_detail: selected_match_detail,
            selected_round: round.unwrap_or(1),
            can_admin: can_admin,
            judges: judges,
            selected_judge_id: selected_judge_id,
            rounds: rounds,
            allowed_scores: allowed_scores,
            red_score: red_score,
            blue_score: blue_score,
            round_tables: round_tables,
            active: "scores",
            is_setup: tournament.is_setup,
            allowed_pages: access_service::user_permissions(state, user.id, tournament.id),
        },
    ))
}

#[post("/<slug>/scores/adjust", data = "<form>")]
pub fn adjust_score(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    form: Form<ScoreAdjustForm>,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, user.id).ok_or(Status::NotFound)?;
    if !access_service::user_has_permission(state, user.id, tournament.id, "scores") {
        return Err(Status::Forbidden);
    }
    let can_admin = access_service::is_owner(state, user.id, tournament.id)
        || access_service::user_has_permission(state, user.id, tournament.id, "events");
    let judge_user_id = if can_admin {
        form.judge_user_id.unwrap_or(user.id)
    } else {
        user.id
    };

    let _ = matches_service::set_or_adjust_judge_score(
        state,
        user.id,
        tournament.id,
        form.match_id,
        judge_user_id,
        form.fight_round,
        &form.side,
        form.delta,
        form.value,
        can_admin,
    );

    // For contact matches: if all rounds have scores from all assigned judges, auto-finish the match
    // and persist the winner.
    let next_round = matches_service::try_finalize_contact_match_from_scores(
        state,
        user.id,
        tournament.id,
        form.match_id,
    );
    let redirect_round = match next_round {
        Ok(Some(value)) => Some(value),
        _ => Some(form.fight_round),
    };

    Ok(Redirect::to(uri!(scores_page(
        slug = slug,
        match_id = Some(form.match_id),
        round = redirect_round,
        judge_id = Some(judge_user_id),
    ))))
}
