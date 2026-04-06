use crate::models::MatchRow;
use crate::services::{access_service, auth_service, match_service, scheduled_events_service, tournament_service};
use crate::state::AppState;
use rocket::http::Cookie;
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};

#[get("/dashboard")]
pub fn dashboard(state: &State<AppState>, jar: &rocket::http::CookieJar<'_>) -> Result<Template, Redirect> {
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

    if let Some(last_slug) = jar.get("last_tournament_slug").map(|cookie| cookie.value().to_string()) {
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
        },
    ))
}

#[get("/<slug>/dashboard")]
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
        None => return Err(Redirect::to(uri!(crate::controllers::dashboard_controller::dashboard))),
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
    if !access_service::user_has_permission(state, user.id, tournament.id, "dashboard") {
        if allowed_pages.iter().any(|item| item.eq_ignore_ascii_case("events")) {
            return Err(Redirect::to(uri!(
                crate::controllers::events_controller::events_page(
                    slug = tournament.slug,
                    error = Option::<String>::None,
                    success = Option::<String>::None
                )
            )));
        }
        if allowed_pages.iter().any(|item| item.eq_ignore_ascii_case("teams")) {
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
    let matches = match_service::list_featured_matches();
    let outcomes = scheduled_events_service::list_outcomes(state, user.id, tournament.id).unwrap_or_default();
    Ok(Template::render(
        "dashboard",
        context! {
            name: user.name,
            matches: matches,
            outcomes: outcomes,
            is_setup: tournament.is_setup,
            active: "dashboard",
            tournaments: tournaments,
            show_tournament_modal: false,
            current_tournament_name: tournament.name,
            tournament_slug: tournament.slug,
            allowed_pages: allowed_pages,
        },
    ))
}
