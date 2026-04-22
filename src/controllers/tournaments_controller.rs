use crate::services::{auth_service, tournament_service};
use crate::state::AppState;
use rocket::form::{Form, FromForm};
use rocket::http::{CookieJar, Status};
use rocket::serde::json::Json;
use serde::Serialize;
use rocket::response::Redirect;
use rocket::State;

#[derive(FromForm)]
pub struct TournamentForm {
    pub name: String,
}

#[derive(Serialize)]
pub struct TournamentBrandingPayload {
    pub logo_url: Option<String>,
    pub theme_primary_color: String,
    pub theme_accent_color: String,
    pub theme_background_color: String,
    pub nav_background_color: String,
    pub nav_text_color: String,
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
    let name = form.name.trim();
    if name.is_empty() {
        return Err(Status::BadRequest);
    }
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
    let tournament =
        tournament_service::get_by_id_for_user(state, id, user.id).ok_or(Status::NotFound)?;
    Ok(Redirect::to(uri!(
        crate::controllers::dashboard_controller::tournament_dashboard(slug = tournament.slug)
    )))
}

#[get("/<slug>/branding.json")]
pub fn tournament_branding_json(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
) -> Result<Json<TournamentBrandingPayload>, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, user.id).ok_or(Status::NotFound)?;
    Ok(Json(TournamentBrandingPayload {
        logo_url: tournament.logo_url,
        theme_primary_color: tournament
            .theme_primary_color
            .unwrap_or_else(|| "#2d62ff".to_string()),
        theme_accent_color: tournament
            .theme_accent_color
            .unwrap_or_else(|| "#ff6b35".to_string()),
        theme_background_color: tournament
            .theme_background_color
            .unwrap_or_else(|| "#f2f3f5".to_string()),
        nav_background_color: tournament
            .nav_background_color
            .unwrap_or_else(|| "#0f1426".to_string()),
        nav_text_color: tournament
            .nav_text_color
            .unwrap_or_else(|| "#f5efe6".to_string()),
    }))
}
