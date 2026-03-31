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

    let tournament = match tournament_service::get_or_create(state) {
        Some(tournament) => tournament,
        None => {
            return Err(Redirect::to(uri!(
                crate::controllers::auth_controller::auth_page(
                    error = Some("Unable to load tournament.".to_string()),
                    success = Option::<String>::None
                )
            )))
        }
    };

    if !tournament.is_setup {
        return Err(Redirect::to(uri!(crate::controllers::settings_controller::settings_page(
            error = Option::<String>::None,
            success = Option::<String>::None,
            tab = Option::<String>::None
        ))));
    }

    let matches = match_service::list_featured_matches();
    Ok(Template::render(
        "dashboard",
        context! {
            name: user.name,
            matches: matches,
            is_setup: tournament.is_setup,
            active: "dashboard",
        },
    ))
}
