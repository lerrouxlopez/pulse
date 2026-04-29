use crate::services::{access_service, auth_service, scheduled_events_service, tournament_service};
use crate::state::AppState;
use rocket::http::{Cookie, CookieJar, Status};
use rocket::response::Redirect;
use rocket::serde::json::Json;
use rocket::State;
use rocket_dyn_templates::{context, Template};
use serde::Serialize;

#[derive(Serialize)]
struct EventWinnerRow {
    event_name: String,
    division_name: Option<String>,
    weight_class_label: Option<String>,
    winner_name: String,
    winner_team: Option<String>,
}

#[derive(Serialize, Clone)]
struct MatchResultRow {
    match_id: i64,
    event_name: String,
    label: String,
    status: String,
    winner: Option<String>,
    red_total_score: i32,
    blue_total_score: i32,
}

#[derive(Serialize, Clone)]
struct TeamChampionRow {
    team_id: i64,
    team_name: String,
    wins: i64,
}

#[get("/<slug>/results?<page>&<q>&<error>&<success>", rank = 50)]
pub fn results_page(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    page: Option<usize>,
    q: Option<String>,
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

    // Results is a "dashboard-level" page: any user who can access the tournament dashboard can view it.
    if !access_service::user_has_permission(state, user.id, tournament.id, "dashboard") {
        return Err(Redirect::to(uri!(
            crate::controllers::dashboard_controller::tournament_dashboard(slug = tournament.slug)
        )));
    }

    jar.add(Cookie::new("last_tournament_slug", tournament.slug.clone()));

    let outcomes =
        scheduled_events_service::list_outcomes(state, user.id, tournament.id).unwrap_or_default();

    let mut team_name_by_id: std::collections::HashMap<i64, String> =
        std::collections::HashMap::new();
    let mut member_team_by_id: std::collections::HashMap<i64, i64> =
        std::collections::HashMap::new();

    let mut match_results_all: Vec<MatchResultRow>;
    let event_winners: Vec<EventWinnerRow>;
    let champion_teams: Vec<TeamChampionRow>;
    let team_leaderboard: Vec<TeamChampionRow>;

    if let Ok(mut conn) = crate::db::open_conn(&state.pool) {
        let teams = crate::repositories::teams_repository::list_teams(&mut conn, tournament.id)
            .unwrap_or_default();
        for team in teams {
            team_name_by_id.insert(team.id, team.name);
        }

        let members = crate::repositories::teams_repository::list_members(&mut conn, tournament.id)
            .unwrap_or_default();
        for member in members {
            member_team_by_id.insert(member.id, member.team_id);
        }

        let mut winners_by_event_id: std::collections::HashMap<i64, Vec<i64>> =
            std::collections::HashMap::new();
        if let Ok(rows) =
            crate::repositories::scheduled_event_winners_repository::list_all_winners_for_tournament(
                &mut conn,
                tournament.id,
            )
        {
            for (scheduled_event_id, winner_member_id) in rows {
                winners_by_event_id
                    .entry(scheduled_event_id)
                    .or_default()
                    .push(winner_member_id);
            }
        }

        // Champion team = number of event wins by team members.
        let mut wins_by_team: std::collections::HashMap<i64, i64> =
            std::collections::HashMap::new();
        for outcome in &outcomes {
            let winner_member_ids: Vec<i64> = if let Some(list) = winners_by_event_id.get(&outcome.id) {
                list.clone()
            } else if let Some(member_id) = outcome.winner_member_id {
                vec![member_id]
            } else {
                Vec::new()
            };
            for member_id in winner_member_ids {
                let Some(team_id) = member_team_by_id.get(&member_id).copied() else {
                    continue;
                };
                *wins_by_team.entry(team_id).or_insert(0) += 1;
            }
        }

        let mut leaderboard: Vec<TeamChampionRow> = wins_by_team
            .iter()
            .filter_map(|(team_id, wins)| {
                let name = team_name_by_id.get(team_id)?.clone();
                Some(TeamChampionRow {
                    team_id: *team_id,
                    team_name: name,
                    wins: *wins,
                })
            })
            .collect();
        leaderboard.sort_by(|a, b| {
            b.wins
                .cmp(&a.wins)
                .then_with(|| a.team_name.cmp(&b.team_name))
        });

        let top_wins = leaderboard.first().map(|row| row.wins).unwrap_or(0);
        let champs: Vec<TeamChampionRow> = if top_wins > 0 {
            leaderboard
                .iter()
                .cloned()
                .filter(|row| row.wins == top_wins)
                .collect()
        } else {
            Vec::new()
        };

        team_leaderboard = leaderboard;
        champion_teams = champs;

        event_winners = outcomes
            .iter()
            .map(|item| {
                let winner_team = item
                    .winner_member_id
                    .and_then(|member_id| member_team_by_id.get(&member_id).copied())
                    .and_then(|team_id| team_name_by_id.get(&team_id).cloned());
                EventWinnerRow {
                    event_name: item.event_name.clone(),
                    division_name: item.division_name.clone(),
                    weight_class_label: item
                        .weight_class_label
                        .clone()
                        .or(item.weight_class_name.clone()),
                    winner_name: item.winner_name.clone().unwrap_or_default(),
                    winner_team,
                }
            })
            .collect();

        let mut matches =
            crate::repositories::matches_repository::list_by_tournament(&mut conn, tournament.id)
                .unwrap_or_default();

        let mut event_name_by_scheduled_id: std::collections::HashMap<i64, String> =
            std::collections::HashMap::new();
        let mut is_contact_by_scheduled_id: std::collections::HashMap<i64, bool> =
            std::collections::HashMap::new();
        for item in
            scheduled_events_service::list(state, user.id, tournament.id).unwrap_or_default()
        {
            event_name_by_scheduled_id.insert(item.id, item.event_name);
            is_contact_by_scheduled_id.insert(item.id, item.contact_type.eq_ignore_ascii_case("Contact"));
        }

        // Only show matches with final-ish state.
        matches.retain(|m| {
            m.status.eq_ignore_ascii_case("Finished")
                || m.status.eq_ignore_ascii_case("Forfeit")
                || m.winner_side.is_some()
        });

        match_results_all = matches
            .into_iter()
            .filter_map(|m| {
                let winner = match m.winner_side.as_deref().map(|s| s.to_lowercase()) {
                    Some(side) if side == "red" => m.red.clone().filter(|v| !v.trim().is_empty()),
                    Some(side) if side == "blue" => m.blue.clone().filter(|v| !v.trim().is_empty()),
                    _ => None,
                };
                let red = m.red.clone().unwrap_or_else(|| "TBD".to_string());
                let blue = if m.is_bye {
                    "BYE".to_string()
                } else {
                    m.blue.clone().unwrap_or_else(|| "TBD".to_string())
                };
                let is_contact = is_contact_by_scheduled_id
                    .get(&m.scheduled_event_id)
                    .copied()
                    .unwrap_or(true);
                let label = if is_contact {
                    format!("{} vs {}", red, blue)
                } else {
                    red.clone()
                };
                // Hide orphan matches (matches whose scheduled event was deleted).
                let event_name = event_name_by_scheduled_id
                    .get(&m.scheduled_event_id)?
                    .clone();
                Some(MatchResultRow {
                    match_id: m.id,
                    event_name,
                    label,
                    status: m.status,
                    winner,
                    red_total_score: m.red_total_score,
                    blue_total_score: m.blue_total_score,
                })
            })
            .collect();
    } else {
        match_results_all = Vec::new();
        event_winners = outcomes
            .iter()
            .map(|item| EventWinnerRow {
                event_name: item.event_name.clone(),
                division_name: item.division_name.clone(),
                weight_class_label: item
                    .weight_class_label
                    .clone()
                    .or(item.weight_class_name.clone()),
                winner_name: item.winner_name.clone().unwrap_or_default(),
                winner_team: None,
            })
            .collect();
        champion_teams = Vec::new();
        team_leaderboard = Vec::new();
    }

    let query = q
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let query_lc = query.as_deref().map(|s| s.to_ascii_lowercase());

    if let Some(ref q_lc) = query_lc {
        match_results_all.retain(|row| {
            let mut haystack = String::new();
            haystack.push_str(&row.event_name);
            haystack.push(' ');
            haystack.push_str(&row.label);
            haystack.push(' ');
            haystack.push_str(&row.status);
            if let Some(ref w) = row.winner {
                haystack.push(' ');
                haystack.push_str(w);
            }
            haystack.to_ascii_lowercase().contains(q_lc)
        });
    }

    let page_size: usize = 20;
    let match_results_total: usize = match_results_all.len();
    let match_results_pages: usize = ((match_results_total + page_size - 1) / page_size).max(1);
    let requested_page = page.unwrap_or(1).max(1);
    let match_results_page: usize = requested_page.min(match_results_pages);
    let start = (match_results_page - 1) * page_size;
    let end = (start + page_size).min(match_results_total);
    let match_results: Vec<MatchResultRow> = if start >= end {
        Vec::new()
    } else {
        match_results_all[start..end].to_vec()
    };

    let window: usize = 7;
    let half = window / 2;
    let mut from = match_results_page.saturating_sub(half);
    if from == 0 {
        from = 1;
    }
    let to = (from + window - 1).min(match_results_pages);
    if to.saturating_sub(from) + 1 < window {
        from = to.saturating_sub(window - 1).max(1);
    }
    let match_results_page_numbers: Vec<usize> = (from..=to).collect();
    let has_prev = match_results_page > 1;
    let has_next = match_results_page < match_results_pages;
    let allowed_pages = access_service::user_permissions(state, user.id, tournament.id);
    let sidebar_nav_items =
        access_service::sidebar_nav_items(&allowed_pages, tournament.is_setup, Some(&tournament.slug));
    let can_reset_results = access_service::is_owner(state, user.id, tournament.id);

    Ok(Template::render(
        "results",
        context! {
            name: user.name,
            tournament_name: tournament.name,
            tournament_slug: tournament.slug,
            active: "results",
            is_setup: tournament.is_setup,
            allowed_pages: allowed_pages,
            sidebar_nav_items: sidebar_nav_items,
            champion_teams: champion_teams,
            team_leaderboard: team_leaderboard,
            event_winners: event_winners,
            match_results: match_results,
            match_results_total: match_results_total,
            match_results_page: match_results_page,
            match_results_pages: match_results_pages,
            match_results_page_numbers: match_results_page_numbers,
            match_results_has_prev: has_prev,
            match_results_has_next: has_next,
            match_results_query: query,
            error: error,
            success: success,
            can_reset_results: can_reset_results,
        },
    ))
}

#[post("/<slug>/results/reset", rank = 50)]
pub fn reset_results(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, user.id).ok_or(Status::NotFound)?;
    if !access_service::is_owner(state, user.id, tournament.id) {
        return Err(Status::Forbidden);
    }

    let mut conn = crate::db::open_conn(&state.pool).map_err(|_| Status::InternalServerError)?;
    let scheduled_events =
        crate::repositories::scheduled_events_repository::list(&mut conn, tournament.id)
            .unwrap_or_default();

    for scheduled in scheduled_events {
        let matches =
            crate::repositories::matches_repository::list(&mut conn, tournament.id, scheduled.id)
                .unwrap_or_default();

        if scheduled.contact_type.eq_ignore_ascii_case("Contact") {
            let max_round = matches.iter().filter_map(|m| m.round).max();
            let final_match = max_round.and_then(|mr| {
                matches
                    .iter()
                    .filter(|m| m.round == Some(mr))
                    .max_by_key(|m| m.slot.unwrap_or(0))
            });
            let winner_member_id = final_match.and_then(|m| {
                match m
                    .winner_side
                    .as_deref()
                    .unwrap_or("")
                    .trim()
                    .to_ascii_lowercase()
                    .as_str()
                {
                    "red" => m.red_member_id,
                    "blue" => m.blue_member_id,
                    _ => None,
                }
            });

            let winners: Vec<i64> = winner_member_id.into_iter().collect();
            let _ = crate::repositories::scheduled_event_winners_repository::replace_winners(
                &mut conn,
                tournament.id,
                scheduled.id,
                &winners,
            );
            let status = if !winners.is_empty() {
                "Finished"
            } else {
                scheduled.status.as_str()
            };
            let _ = crate::repositories::scheduled_events_repository::update_status_and_winner(
                &mut conn,
                tournament.id,
                scheduled.id,
                status,
                winners.first().copied(),
            );
            continue;
        }

        let judge_user_ids =
            crate::repositories::scheduled_event_judges_repository::list_assigned_judges(
                &mut conn,
                tournament.id,
                scheduled.id,
            )
            .unwrap_or_default();
        let judge_count = judge_user_ids.len() as i64;

        let mut winners: Vec<i64> = Vec::new();
        let is_ready = judge_count >= 3
            && judge_count <= 5
            && judge_count % 2 == 1
            && !matches.is_empty()
            && matches.iter().all(|m| m.status.eq_ignore_ascii_case("Finished"))
            && matches.iter().all(|m| {
                let scored =
                    crate::repositories::match_judges_repository::count_distinct_judges_with_valid_red_score_for_match_round(
                        &mut conn,
                        tournament.id,
                        m.id,
                        1,
                        5,
                        10,
                    )
                    .unwrap_or(0);
                scored == judge_count
            });

        if is_ready {
            let mut best_total: Option<i64> = None;
            for perf in &matches {
                let Ok((sum_red, _sum_blue)) =
                    crate::repositories::match_judges_repository::sum_for_match_round(
                        &mut conn,
                        tournament.id,
                        perf.id,
                        1,
                    )
                else {
                    continue;
                };
                let Some(member_id) = perf.red_member_id else {
                    continue;
                };
                match best_total {
                    None => {
                        best_total = Some(sum_red);
                        winners.clear();
                        winners.push(member_id);
                    }
                    Some(best) if sum_red > best => {
                        best_total = Some(sum_red);
                        winners.clear();
                        winners.push(member_id);
                    }
                    Some(best) if sum_red == best => {
                        winners.push(member_id);
                    }
                    _ => {}
                }
            }
            winners.sort();
            winners.dedup();
        }

        let _ = crate::repositories::scheduled_event_winners_repository::replace_winners(
            &mut conn,
            tournament.id,
            scheduled.id,
            &winners,
        );
        let status = if !winners.is_empty() {
            "Finished"
        } else {
            scheduled.status.as_str()
        };
        let _ = crate::repositories::scheduled_events_repository::update_status_and_winner(
            &mut conn,
            tournament.id,
            scheduled.id,
            status,
            winners.first().copied(),
        );
    }

    Ok(Redirect::to(uri!(results_page(
        slug = slug,
        page = Option::<usize>::None,
        q = Option::<String>::None,
        error = Option::<String>::None,
        success = Some("Results recalculated.".to_string())
    ))))
}

#[derive(Serialize)]
pub(crate) struct RoundJudgeRow {
    judge_name: String,
    judge_photo_url: Option<String>,
    red_score: i32,
    blue_score: i32,
    judge_order: i32,
}

#[derive(Serialize)]
pub(crate) struct MatchRoundRow {
    fight_round: i64,
    judges: Vec<RoundJudgeRow>,
    red_total: i64,
    blue_total: i64,
}

#[derive(Serialize)]
pub(crate) struct MatchResultDetail {
    match_id: i64,
    event_name: String,
    label: String,
    status: String,
    winner: Option<String>,
    rounds: Vec<MatchRoundRow>,
}

#[get("/<slug>/results/matches/<match_id>")]
pub fn match_result_detail(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    match_id: i64,
) -> Result<Json<MatchResultDetail>, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, user.id).ok_or(Status::NotFound)?;
    if !access_service::user_has_permission(state, user.id, tournament.id, "dashboard") {
        return Err(Status::Forbidden);
    }

    let mut conn = crate::db::open_conn(&state.pool).map_err(|_| Status::InternalServerError)?;
    let match_row =
        crate::repositories::matches_repository::get_by_id(&mut conn, tournament.id, match_id)
            .map_err(|_| Status::InternalServerError)?
            .ok_or(Status::NotFound)?;
    let scheduled_event = scheduled_events_service::get_by_id(
        state,
        user.id,
        tournament.id,
        match_row.scheduled_event_id,
    )
    .map_err(|_| Status::InternalServerError)?
    .ok_or(Status::NotFound)?;

    let base_rounds =
        scheduled_events_service::parse_time_rule(scheduled_event.time_rule.as_deref())
            .map(|rule| rule.rounds)
            .unwrap_or(1)
            .max(1);
    let point_rule =
        scheduled_events_service::parse_point_rule(scheduled_event.point_system.as_deref())
            .unwrap_or(scheduled_events_service::PointRule { min: 0, max: 10 });
    let min_allowed = point_rule.min;
    let max_allowed = point_rule.max;

    let mut max_scored_round =
        crate::repositories::match_judges_repository::max_fight_round_for_match(
            &mut conn,
            tournament.id,
            match_id,
        )
        .map_err(|_| Status::InternalServerError)?;
    let assigned = crate::repositories::match_judges_repository::list_assigned_judges(
        &mut conn,
        tournament.id,
        match_id,
    )
    .map_err(|_| Status::InternalServerError)?;
    let judge_count = assigned.len() as i64;
    let mut base_complete = judge_count > 0;
    if base_complete {
        for r in 1..=base_rounds {
            let count = crate::repositories::match_judges_repository::count_distinct_judges_with_valid_scores_for_match_round(
                &mut conn,
                tournament.id,
                match_id,
                r,
                min_allowed,
                max_allowed,
            )
            .map_err(|_| Status::InternalServerError)?;
            if count != judge_count {
                base_complete = false;
                break;
            }
        }
    }

    let is_extension = scheduled_event
        .draw_system
        .as_deref()
        .unwrap_or("")
        .eq_ignore_ascii_case("Extension");

    // If extension rounds were previously added prematurely, roll them back when viewing results.
    let mut match_fight_round = match_row.fight_round.unwrap_or(1);
    if is_extension
        && !base_complete
        && (match_fight_round > base_rounds || max_scored_round > base_rounds)
    {
        let _ = crate::repositories::matches_repository::set_status_and_fight_round(
            &mut conn,
            tournament.id,
            match_id,
            &match_row.status,
            base_rounds,
        );
        let _ = crate::repositories::match_judges_repository::delete_rounds_gt(
            &mut conn,
            tournament.id,
            match_id,
            base_rounds,
        );
        let mut sum_red: i64 = 0;
        let mut sum_blue: i64 = 0;
        for r in 1..=base_rounds {
            if let Ok((red, blue)) =
                crate::repositories::match_judges_repository::sum_for_match_round(
                    &mut conn,
                    tournament.id,
                    match_id,
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
            match_id,
            sum_red.min(i64::from(i32::MAX)) as i32,
            sum_blue.min(i64::from(i32::MAX)) as i32,
        );
        match_fight_round = base_rounds;
        max_scored_round = base_rounds;
    }

    let rounds_total = if is_extension && !base_complete {
        base_rounds
    } else {
        base_rounds.max(match_fight_round).max(max_scored_round)
    };

    let winner_side = match_row
        .winner_side
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let winner = if winner_side == "red" {
        match_row.red.clone().filter(|v| !v.trim().is_empty())
    } else if winner_side == "blue" {
        match_row.blue.clone().filter(|v| !v.trim().is_empty())
    } else {
        None
    };
    let red = match_row.red.clone().unwrap_or_else(|| "TBD".to_string());
    let blue = if match_row.is_bye {
        "BYE".to_string()
    } else {
        match_row.blue.clone().unwrap_or_else(|| "TBD".to_string())
    };
    let label = if scheduled_event.contact_type.eq_ignore_ascii_case("Contact") {
        format!("{} vs {}", red, blue)
    } else {
        red.clone()
    };

    let mut rounds: Vec<MatchRoundRow> = Vec::new();
    for r in 1..=rounds_total {
        let judges = crate::repositories::match_judges_repository::list_by_match(
            &mut conn,
            tournament.id,
            match_id,
            r,
        )
        .map_err(|_| Status::InternalServerError)?;
        let (red_total, blue_total) =
            crate::repositories::match_judges_repository::sum_for_match_round(
                &mut conn,
                tournament.id,
                match_id,
                r,
            )
            .map_err(|_| Status::InternalServerError)?;
        rounds.push(MatchRoundRow {
            fight_round: r,
            judges: judges
                .into_iter()
                .map(|j| RoundJudgeRow {
                    judge_name: j.judge_name,
                    judge_photo_url: j.judge_photo_url,
                    red_score: j.red_score,
                    blue_score: j.blue_score,
                    judge_order: j.judge_order,
                })
                .collect(),
            red_total,
            blue_total,
        });
    }

    Ok(Json(MatchResultDetail {
        match_id,
        event_name: scheduled_event.event_name,
        label,
        status: match_row.status,
        winner,
        rounds,
    }))
}
