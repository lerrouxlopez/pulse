use crate::services::{auth_service, settings_service, teams_service, tournament_service};
use crate::services::settings_service::SettingsEntity;
use crate::state::AppState;
use rocket::form::{Form, FromForm};
use rocket::fs::TempFile;
use rocket::http::{CookieJar, Status};
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(FromForm)]
pub struct TeamForm<'r> {
    pub name: String,
    pub logo_file: Option<TempFile<'r>>,
    pub division_ids: Option<Vec<i64>>,
    pub category_ids: Option<Vec<i64>>,
    pub event_ids: Option<Vec<i64>>,
}

#[derive(FromForm)]
pub struct MemberForm {
    pub name: String,
}

#[get("/teams?<error>&<success>")]
pub fn teams_page(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
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

    let tournament_id = jar
        .get("tournament_id")
        .and_then(|cookie| cookie.value().parse::<i64>().ok());
    let tournament = match tournament_id
        .and_then(|id| tournament_service::get_by_id_for_user(state, id, user.id))
    {
        Some(tournament) => tournament,
        None => {
            return Err(Redirect::to(uri!(
                crate::controllers::dashboard_controller::dashboard
            )))
        }
    };

    let teams = teams_service::list(state, user.id, tournament.id).unwrap_or_default();
    let divisions = settings_service::list(state, tournament.id, SettingsEntity::Division);
    let categories = settings_service::list(state, tournament.id, SettingsEntity::Category);
    let events = settings_service::list(state, tournament.id, SettingsEntity::Event);

    Ok(Template::render(
        "teams",
        context! {
            name: user.name,
            tournament_name: tournament.name,
            teams: teams,
            divisions: divisions,
            categories: categories,
            events: events,
            error: error,
            success: success,
            active: "teams",
            is_setup: tournament.is_setup,
        },
    ))
}

#[post("/teams", data = "<form>")]
pub async fn create_team(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    mut form: Form<TeamForm<'_>>,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament_id = jar
        .get("tournament_id")
        .and_then(|cookie| cookie.value().parse::<i64>().ok())
        .ok_or(Status::BadRequest)?;
    let tournament = tournament_service::get_by_id_for_user(state, tournament_id, user.id)
        .ok_or(Status::NotFound)?;
    let logo_url = save_logo(&mut form.logo_file).await.map_err(|_| Status::InternalServerError)?;
    let division_ids = form.division_ids.clone().unwrap_or_default();
    let category_ids = form.category_ids.clone().unwrap_or_default();
    let event_ids = form.event_ids.clone().unwrap_or_default();
    match teams_service::create_team(
        state,
        user.id,
        tournament.id,
        &form.name,
        logo_url.as_deref(),
        &division_ids,
        &category_ids,
        &event_ids,
    ) {
        Ok(_) => Ok(Redirect::to(uri!(teams_page(
            error = Option::<String>::None,
            success = Some("Team added.".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(teams_page(
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}

#[post("/teams/<id>/update", data = "<form>")]
pub async fn update_team(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    id: i64,
    mut form: Form<TeamForm<'_>>,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament_id = jar
        .get("tournament_id")
        .and_then(|cookie| cookie.value().parse::<i64>().ok())
        .ok_or(Status::BadRequest)?;
    let tournament = tournament_service::get_by_id_for_user(state, tournament_id, user.id)
        .ok_or(Status::NotFound)?;
    let uploaded_logo = save_logo(&mut form.logo_file).await.map_err(|_| Status::InternalServerError)?;
    let logo_url = if uploaded_logo.is_some() {
        uploaded_logo
    } else {
        teams_service::get_team_logo(state, user.id, tournament.id, id)
            .map_err(|_| Status::InternalServerError)?
    };
    let division_ids = form.division_ids.clone().unwrap_or_default();
    let category_ids = form.category_ids.clone().unwrap_or_default();
    let event_ids = form.event_ids.clone().unwrap_or_default();
    match teams_service::update_team(
        state,
        user.id,
        tournament.id,
        id,
        &form.name,
        logo_url.as_deref(),
        &division_ids,
        &category_ids,
        &event_ids,
    ) {
        Ok(_) => Ok(Redirect::to(uri!(teams_page(
            error = Option::<String>::None,
            success = Some("Team updated.".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(teams_page(
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}

#[post("/teams/<id>/delete")]
pub fn delete_team(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    id: i64,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament_id = jar
        .get("tournament_id")
        .and_then(|cookie| cookie.value().parse::<i64>().ok())
        .ok_or(Status::BadRequest)?;
    let tournament = tournament_service::get_by_id_for_user(state, tournament_id, user.id)
        .ok_or(Status::NotFound)?;
    match teams_service::delete_team(state, user.id, tournament.id, id) {
        Ok(_) => Ok(Redirect::to(uri!(teams_page(
            error = Option::<String>::None,
            success = Some("Team deleted.".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(teams_page(
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}

#[post("/teams/<team_id>/members", data = "<form>")]
pub fn add_member(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    team_id: i64,
    form: Form<MemberForm>,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament_id = jar
        .get("tournament_id")
        .and_then(|cookie| cookie.value().parse::<i64>().ok())
        .ok_or(Status::BadRequest)?;
    let tournament = tournament_service::get_by_id_for_user(state, tournament_id, user.id)
        .ok_or(Status::NotFound)?;
    match teams_service::add_member(state, user.id, tournament.id, team_id, &form.name) {
        Ok(_) => Ok(Redirect::to(uri!(teams_page(
            error = Option::<String>::None,
            success = Some("Member added.".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(teams_page(
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}

#[post("/teams/members/<member_id>/delete")]
pub fn delete_member(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    member_id: i64,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament_id = jar
        .get("tournament_id")
        .and_then(|cookie| cookie.value().parse::<i64>().ok())
        .ok_or(Status::BadRequest)?;
    let tournament = tournament_service::get_by_id_for_user(state, tournament_id, user.id)
        .ok_or(Status::NotFound)?;
    match teams_service::delete_member(state, user.id, tournament.id, member_id) {
        Ok(_) => Ok(Redirect::to(uri!(teams_page(
            error = Option::<String>::None,
            success = Some("Member removed.".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(teams_page(
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}

async fn save_logo(file: &mut Option<TempFile<'_>>) -> Result<Option<String>, std::io::Error> {
    let Some(upload) = file else {
        return Ok(None);
    };
    if upload.len() == 0 {
        return Ok(None);
    }
    let uploads_dir = Path::new("static").join("uploads");
    std::fs::create_dir_all(&uploads_dir)?;

    let extension = upload
        .content_type()
        .and_then(|ct| ct.extension().map(|ext| format!(".{}", ext)))
        .unwrap_or_else(|| ".png".to_string());
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let filename = format!("team-logo-{}{}", timestamp, extension);
    let filepath = uploads_dir.join(filename);
    upload.persist_to(&filepath).await?;
    let public_path = format!("/static/uploads/{}", filepath.file_name().unwrap().to_string_lossy());
    Ok(Some(public_path))
}
