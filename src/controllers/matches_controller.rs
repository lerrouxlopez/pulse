use crate::services::{access_service, auth_service, matches_service, tournament_service};
use crate::state::AppState;
use rocket::http::{Cookie, CookieJar};
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};

#[get("/<slug>/matches")]
pub fn matches_page(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
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
    if !access_service::user_has_permission(state, user.id, tournament.id, "events") {
        return Err(Redirect::to(uri!(
            crate::controllers::dashboard_controller::tournament_dashboard(slug = tournament.slug)
        )));
    }

    jar.add(Cookie::new("last_tournament_slug", tournament.slug.clone()));

    let matches = matches_service::list_cards(state, user.id, tournament.id).unwrap_or_default();

    Ok(Template::render(
        "matches",
        context! {
            name: user.name,
            tournament_name: tournament.name,
            tournament_slug: tournament.slug,
            matches: matches,
            active: "matches",
            is_setup: tournament.is_setup,
            allowed_pages: access_service::user_permissions(state, user.id, tournament.id),
        },
    ))
}

#[get("/<slug>/matches/<id>")]
pub fn match_page(
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
    if !access_service::user_has_permission(state, user.id, tournament.id, "events") {
        return Err(Redirect::to(uri!(
            crate::controllers::dashboard_controller::tournament_dashboard(slug = tournament.slug)
        )));
    }

    jar.add(Cookie::new("last_tournament_slug", tournament.slug.clone()));

    let match_detail = match matches_service::get_detail(state, user.id, tournament.id, id) {
        Ok(Some(item)) => item,
        _ => {
            return Err(Redirect::to(uri!(
                crate::controllers::matches_controller::matches_page(slug = tournament.slug)
            )))
        }
    };

    Ok(Template::render(
        "match",
        context! {
            name: user.name,
            tournament_name: tournament.name,
            tournament_slug: tournament.slug,
            match_item: match_detail,
            active: "matches",
            is_setup: tournament.is_setup,
            allowed_pages: access_service::user_permissions(state, user.id, tournament.id),
        },
    ))
}
