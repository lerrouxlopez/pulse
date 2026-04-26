use crate::models::MatchRow;
use crate::repositories::dashboard_repository;
use crate::services::{
    access_service, auth_service, scheduled_events_service, tournament_service,
};
use crate::state::AppState;
use rocket::http::Cookie;
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};
use serde::Serialize;

#[get("/dashboard")]
pub fn dashboard(
    state: &State<AppState>,
    jar: &rocket::http::CookieJar<'_>,
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

    if user.user_type.eq_ignore_ascii_case("tournament") {
        if let Some(tournament) = tournament_service::get_by_id(state, user.tournament_id) {
            jar.add(Cookie::new("last_tournament_slug", tournament.slug.clone()));
            return Err(Redirect::to(uri!(
                crate::controllers::dashboard_controller::tournament_dashboard(
                    slug = tournament.slug
                )
            )));
        }
        return Err(Redirect::to(uri!(
            crate::controllers::auth_controller::auth_page(
                error = Some("Tournament not found.".to_string()),
                success = Option::<String>::None
            )
        )));
    }

    if let Some(last_slug) = jar
        .get("last_tournament_slug")
        .map(|cookie| cookie.value().to_string())
    {
        if tournament_service::get_by_slug_for_user(state, &last_slug, user.id).is_some() {
            return Err(Redirect::to(uri!(
                crate::controllers::dashboard_controller::tournament_dashboard(slug = last_slug)
            )));
        }
    }

    let tournaments = tournament_service::list_by_user(state, user.id);

    Ok(Template::render(
        "dashboard",
        context! {
            name: user.name,
            matches: Vec::<MatchRow>::new(),
            outcomes: Vec::<crate::models::ScheduledEvent>::new(),
            is_setup: false,
            active: "dashboard",
            tournaments: tournaments,
            show_tournament_modal: true,
            current_tournament_name: Option::<String>::None,
            tournament_slug: Option::<String>::None,
            allowed_pages: Vec::<String>::new(),
            sidebar_nav_items: Vec::<crate::services::access_service::SidebarNavItem>::new(),
            is_system_user: true,
        },
    ))
}

#[get("/<slug>/dashboard", rank = 50)]
pub fn tournament_dashboard(
    state: &State<AppState>,
    jar: &rocket::http::CookieJar<'_>,
    slug: String,
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

    if !tournament.is_setup {
        return Err(Redirect::to(uri!(
            crate::controllers::settings_controller::settings_page(
                slug = tournament.slug,
                error = Option::<String>::None,
                success = Option::<String>::None,
                tab = Option::<String>::None
            )
        )));
    }

    let allowed_pages = access_service::user_permissions(state, user.id, tournament.id);
    let sidebar_nav_items =
        access_service::sidebar_nav_items(&allowed_pages, tournament.is_setup, Some(&tournament.slug));
    if !access_service::user_has_permission(state, user.id, tournament.id, "dashboard") {
        if allowed_pages
            .iter()
            .any(|item| item.eq_ignore_ascii_case("events"))
        {
            return Err(Redirect::to(uri!(
                crate::controllers::events_controller::events_page(
                    slug = tournament.slug,
                    error = Option::<String>::None,
                    success = Option::<String>::None
                )
            )));
        }
        if allowed_pages
            .iter()
            .any(|item| item.eq_ignore_ascii_case("teams"))
        {
            return Err(Redirect::to(uri!(
                crate::controllers::teams_controller::teams_page(
                    slug = tournament.slug,
                    error = Option::<String>::None,
                    success = Option::<String>::None
                )
            )));
        }
        return Err(Redirect::to(uri!(
            crate::controllers::settings_controller::settings_page(
                slug = tournament.slug,
                error = Some("Access denied.".to_string()),
                success = Option::<String>::None,
                tab = Option::<String>::None
            )
        )));
    }

    let tournaments = tournament_service::list_by_user(state, user.id);
    let outcomes =
        scheduled_events_service::list_outcomes(state, user.id, tournament.id).unwrap_or_default();
    let outcomes_count = outcomes.len();

    #[derive(Serialize)]
    struct Series {
        labels: Vec<String>,
        values: Vec<u64>,
    }

    #[derive(Serialize)]
    struct Activity {
        labels: Vec<String>,
        events: Vec<u64>,
        matches: Vec<u64>,
    }

    #[derive(Serialize)]
    struct DashboardJson {
        events_by_status: Series,
        events_by_type: Series,
        matches_by_status: Series,
        matches_series: Series,
        members_by_division: Series,
        members_by_weight_class: Series,
        members_by_category: Series,
        events_per_division: Series,
        events_per_weight_class: Series,
        events_per_category: Series,
        participants_by_event: Series,
        activity: Activity,
    }

    let mut conn = match crate::db::open_conn(&state.pool) {
        Ok(conn) => conn,
        Err(err) => {
            return Ok(Template::render(
                "dashboard_tournament",
                context! {
                    name: user.name,
                    tournament_name: tournament.name,
                    tournament_slug: tournament.slug,
                    active: "dashboard",
                    allowed_pages: allowed_pages,
                    sidebar_nav_items: sidebar_nav_items,
                    counts: dashboard_repository::DashboardCounts { teams: 0, members: 0, scheduled_events: 0, matches: 0 },
                    counts_events_finished: 0,
                    counts_matches_finished: 0,
                    outcomes: outcomes,
                    outcomes_count: outcomes_count,
                    upcoming: Vec::<dashboard_repository::UpcomingScheduledEventRow>::new(),
                    recent_matches: Vec::<dashboard_repository::RecentMatchRow>::new(),
                    dashboard_json: serde_json::to_string(&DashboardJson {
                        events_by_status: Series { labels: vec![], values: vec![] },
                        events_by_type: Series { labels: vec![], values: vec![] },
                        matches_by_status: Series { labels: vec![], values: vec![] },
                        matches_series: Series { labels: vec![], values: vec![] },
                        members_by_division: Series { labels: vec![], values: vec![] },
                        members_by_weight_class: Series { labels: vec![], values: vec![] },
                        members_by_category: Series { labels: vec![], values: vec![] },
                        events_per_division: Series { labels: vec![], values: vec![] },
                        events_per_weight_class: Series { labels: vec![], values: vec![] },
                        events_per_category: Series { labels: vec![], values: vec![] },
                        participants_by_event: Series { labels: vec![], values: vec![] },
                        activity: Activity { labels: vec![], events: vec![], matches: vec![] },
                    }).unwrap_or_else(|_| "{}".to_string()),
                    error: format!("Storage error: {err}"),
                },
            ));
        }
    };

    let counts = dashboard_repository::counts(&mut conn, tournament.id).unwrap_or(
        dashboard_repository::DashboardCounts {
            teams: 0,
            members: 0,
            scheduled_events: 0,
            matches: 0,
        },
    );

    let events_by_status = dashboard_repository::scheduled_events_by_status(&mut conn, tournament.id)
        .unwrap_or_default();
    let events_by_type =
        dashboard_repository::scheduled_events_by_contact_type(&mut conn, tournament.id)
            .unwrap_or_default();
    let matches_by_status = dashboard_repository::matches_by_status(&mut conn, tournament.id)
        .unwrap_or_default();

    let events_series = dashboard_repository::scheduled_events_timeseries(&mut conn, tournament.id, 30)
        .unwrap_or_default();
    let matches_series = dashboard_repository::matches_timeseries(&mut conn, tournament.id, 30)
        .unwrap_or_default();

    const DASHBOARD_TOP_N: u64 = 12;
    const DASHBOARD_TOP_EVENTS: u64 = 10;

    let members_by_division =
        dashboard_repository::members_by_division(&mut conn, tournament.id, DASHBOARD_TOP_N)
            .unwrap_or_default();
    let members_by_weight_class =
        dashboard_repository::members_by_weight_class(&mut conn, tournament.id, DASHBOARD_TOP_N)
            .unwrap_or_default();
    let members_by_category =
        dashboard_repository::members_by_category(&mut conn, tournament.id, DASHBOARD_TOP_N)
            .unwrap_or_default();

    let events_per_division =
        dashboard_repository::events_per_division(&mut conn, tournament.id, DASHBOARD_TOP_N)
            .unwrap_or_default();
    let events_per_weight_class =
        dashboard_repository::events_per_weight_class(&mut conn, tournament.id, DASHBOARD_TOP_N)
            .unwrap_or_default();
    let events_per_category =
        dashboard_repository::events_per_category(&mut conn, tournament.id, DASHBOARD_TOP_N)
            .unwrap_or_default();

    let participants_by_event =
        dashboard_repository::participants_by_event(&mut conn, tournament.id, DASHBOARD_TOP_EVENTS)
            .unwrap_or_default();

    let upcoming = dashboard_repository::upcoming_scheduled_events(&mut conn, tournament.id, 8)
        .unwrap_or_default();
    let recent_matches = dashboard_repository::recent_matches(&mut conn, tournament.id, 10)
        .unwrap_or_default();

    let counts_events_finished = events_by_status
        .iter()
        .find(|(status, _)| status.eq_ignore_ascii_case("Finished"))
        .map(|(_, count)| *count)
        .unwrap_or(0) as i64;
    let counts_matches_finished = matches_by_status
        .iter()
        .find(|(status, _)| status.eq_ignore_ascii_case("Finished"))
        .map(|(_, count)| *count)
        .unwrap_or(0) as i64;

    let split = |rows: Vec<(String, u64)>| -> Series {
        let mut labels = Vec::new();
        let mut values = Vec::new();
        for (k, v) in rows {
            labels.push(k);
            values.push(v);
        }
        Series { labels, values }
    };

    let events_series_split = split(events_series.clone());
    let matches_series_split = split(matches_series.clone());

    let activity_labels = if events_series_split.labels.len() >= matches_series_split.labels.len() {
        events_series_split.labels.clone()
    } else {
        matches_series_split.labels.clone()
    };
    let mut events_map = std::collections::HashMap::<String, u64>::new();
    for (day, count) in events_series {
        events_map.insert(day, count);
    }
    let mut matches_map = std::collections::HashMap::<String, u64>::new();
    for (day, count) in matches_series {
        matches_map.insert(day, count);
    }
    let mut activity_events = Vec::new();
    let mut activity_matches = Vec::new();
    for day in &activity_labels {
        activity_events.push(*events_map.get(day).unwrap_or(&0));
        activity_matches.push(*matches_map.get(day).unwrap_or(&0));
    }

    let dashboard_json = serde_json::to_string(&DashboardJson {
        events_by_status: split(events_by_status),
        events_by_type: split(events_by_type),
        matches_by_status: split(matches_by_status),
        matches_series: matches_series_split,
        members_by_division: split(members_by_division),
        members_by_weight_class: split(members_by_weight_class),
        members_by_category: split(members_by_category),
        events_per_division: split(events_per_division),
        events_per_weight_class: split(events_per_weight_class),
        events_per_category: split(events_per_category),
        participants_by_event: split(participants_by_event),
        activity: Activity {
            labels: activity_labels,
            events: activity_events,
            matches: activity_matches,
        },
    })
    .unwrap_or_else(|_| "{}".to_string());

    Ok(Template::render(
        "dashboard_tournament",
        context! {
            name: user.name,
            tournament_name: tournament.name,
            tournament_slug: tournament.slug,
            outcomes: outcomes,
            outcomes_count: outcomes_count,
            counts: counts,
            counts_events_finished: counts_events_finished,
            counts_matches_finished: counts_matches_finished,
            upcoming: upcoming,
            recent_matches: recent_matches,
            active: "dashboard",
            tournaments: tournaments,
            allowed_pages: allowed_pages,
            sidebar_nav_items: sidebar_nav_items,
            dashboard_json: dashboard_json,
            error: Option::<String>::None,
        },
    ))
}
