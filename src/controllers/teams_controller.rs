use crate::services::{auth_service, teams_service, tournament_service};
use crate::state::AppState;
use rocket::form::{Form, FromForm};
use rocket::http::{CookieJar, Status};
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};

#[derive(FromForm)]
pub struct TeamForm {
    pub name: String,
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

    Ok(Template::render(
        "teams",
        context! {
            name: user.name,
            tournament_name: tournament.name,
            teams: teams,
            error: error,
            success: success,
            active: "teams",
            is_setup: tournament.is_setup,
        },
    ))
}

#[post("/teams", data = "<form>")]
pub fn create_team(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    form: Form<TeamForm>,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament_id = jar
        .get("tournament_id")
        .and_then(|cookie| cookie.value().parse::<i64>().ok())
        .ok_or(Status::BadRequest)?;
    let tournament = tournament_service::get_by_id_for_user(state, tournament_id, user.id)
        .ok_or(Status::NotFound)?;
    match teams_service::create_team(state, user.id, tournament.id, &form.name) {
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
pub fn update_team(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    id: i64,
    form: Form<TeamForm>,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament_id = jar
        .get("tournament_id")
        .and_then(|cookie| cookie.value().parse::<i64>().ok())
        .ok_or(Status::BadRequest)?;
    let tournament = tournament_service::get_by_id_for_user(state, tournament_id, user.id)
        .ok_or(Status::NotFound)?;
    match teams_service::update_team(state, user.id, tournament.id, id, &form.name) {
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
