use crate::models::MatchRow;
use crate::services::{auth_service, match_service, tournament_service};
use crate::state::AppState;
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

    let tournaments = tournament_service::list_by_user(state, user.id);
    let tournament_id = jar
        .get("tournament_id")
        .and_then(|cookie| cookie.value().parse::<i64>().ok());
    let tournament = tournament_id
        .and_then(|id| tournament_service::get_by_id_for_user(state, id, user.id));

    if let Some(tournament) = tournament {
        if !tournament.is_setup {
            return Err(Redirect::to(uri!(crate::controllers::settings_controller::settings_page(
                error = Option::<String>::None,
                success = Option::<String>::None,
                tab = Option::<String>::None
            ))));
        }

        let matches = match_service::list_featured_matches();
        return Ok(Template::render(
            "dashboard",
            context! {
                name: user.name,
                matches: matches,
                is_setup: tournament.is_setup,
                active: "dashboard",
                tournaments: tournaments,
                show_tournament_modal: false,
                current_tournament_name: tournament.name,
            },
        ));
    }

    if tournament_id.is_some() {
        jar.remove(rocket::http::Cookie::from("tournament_id"));
    }

    Ok(Template::render(
        "dashboard",
        context! {
            name: user.name,
            matches: Vec::<MatchRow>::new(),
            is_setup: false,
            active: "dashboard",
            tournaments: tournaments,
            show_tournament_modal: true,
            current_tournament_name: Option::<String>::None,
        },
    ))
}
