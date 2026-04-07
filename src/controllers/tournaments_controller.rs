use crate::services::{auth_service, tournament_service};
use crate::state::AppState;
use rocket::form::{Form, FromForm};
use rocket::http::{CookieJar, Status};
use rocket::response::Redirect;
use rocket::State;

#[derive(FromForm)]
pub struct TournamentForm {
    pub name: String,
}

#[post("/tournaments/new", data = "<form>")]
pub fn create_tournament(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    form: Form<TournamentForm>,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    if !user.user_type.eq_ignore_ascii_case("system") {
        return Err(Status::Unauthorized);
    }
    let name = if form.name.trim().is_empty() {
        "New Tournament"
    } else {
        form.name.trim()
    };
    let tournament =
        tournament_service::create(state, user.id, name).ok_or(Status::InternalServerError)?;
    Ok(Redirect::to(uri!(
        crate::controllers::settings_controller::settings_page(
            slug = tournament.slug,
            error = Option::<String>::None,
            success = Option::<String>::None,
            tab = Option::<String>::None
        )
    )))
}

#[post("/tournaments/select/<id>")]
pub fn select_tournament(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    id: i64,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    if !user.user_type.eq_ignore_ascii_case("system") {
        return Err(Status::Unauthorized);
    }
    let tournament = tournament_service::get_by_id_for_user(state, id, user.id)
        .ok_or(Status::NotFound)?;
    Ok(Redirect::to(uri!(
        crate::controllers::dashboard_controller::tournament_dashboard(
            slug = tournament.slug
        )
    )))
}
