use crate::services::auth_service;
use crate::state::AppState;
use rocket::http::CookieJar;
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};

#[get("/")]
pub fn index(state: &State<AppState>, jar: &CookieJar<'_>) -> Result<Template, Redirect> {
    if auth_service::current_user(state, jar).is_some() {
        return Err(Redirect::to(uri!(
            crate::controllers::dashboard_controller::dashboard
        )));
    }

    Ok(Template::render("index", context! {}))
}
