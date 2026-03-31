use crate::services::{auth_service, settings_service, tournament_service};
use crate::services::settings_service::SettingsEntity;
use crate::state::AppState;
use rocket::form::{Form, FromForm};
use rocket::http::{CookieJar, Status};
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};

#[derive(FromForm)]
pub struct NameForm {
    pub name: String,
}

#[get("/settings?<error>&<success>")]
pub fn settings_page(
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

    let tournament = match tournament_service::get_or_create(state) {
        Some(tournament) => tournament,
        None => return Err(Redirect::to(uri!(crate::controllers::dashboard_controller::dashboard))),
    };

    let divisions = settings_service::list(state, tournament.id, SettingsEntity::Division);
    let categories = settings_service::list(state, tournament.id, SettingsEntity::Category);
    let weight_classes = settings_service::list(state, tournament.id, SettingsEntity::WeightClass);
    let events = settings_service::list(state, tournament.id, SettingsEntity::Event);

    Ok(Template::render(
        "settings",
        context! {
            name: user.name,
            tournament_name: tournament.name,
            is_setup: tournament.is_setup,
            divisions: divisions,
            categories: categories,
            weight_classes: weight_classes,
            events: events,
            error: error,
            success: success,
            active: "settings",
        },
    ))
}

#[post("/settings/setup/complete")]
pub fn complete_setup(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament = tournament_service::get_or_create(state).ok_or(Status::InternalServerError)?;
    if tournament_service::mark_setup_complete(state, tournament.id) {
        Ok(Redirect::to(uri!(settings_page(
            error = Option::<String>::None,
            success = Some("Tournament setup completed.".to_string())
        ))))
    } else {
        Ok(Redirect::to(uri!(settings_page(
            error = Some("Unable to update setup status.".to_string()),
            success = Option::<String>::None
        ))))
    }
}

#[post("/settings/divisions", data = "<form>")]
pub fn create_division(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    form: Form<NameForm>,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament = tournament_service::get_or_create(state).ok_or(Status::InternalServerError)?;
    match settings_service::create(state, tournament.id, SettingsEntity::Division, &form.name) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            error = Option::<String>::None,
            success = Some("Division added.".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}

#[post("/settings/divisions/<id>/update", data = "<form>")]
pub fn update_division(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    id: i64,
    form: Form<NameForm>,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    match settings_service::update(state, SettingsEntity::Division, id, &form.name) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            error = Option::<String>::None,
            success = Some("Division updated.".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}

#[post("/settings/divisions/<id>/delete")]
pub fn delete_division(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    id: i64,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    match settings_service::delete(state, SettingsEntity::Division, id) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            error = Option::<String>::None,
            success = Some("Division deleted.".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}

#[post("/settings/categories", data = "<form>")]
pub fn create_category(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    form: Form<NameForm>,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament = tournament_service::get_or_create(state).ok_or(Status::InternalServerError)?;
    match settings_service::create(state, tournament.id, SettingsEntity::Category, &form.name) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            error = Option::<String>::None,
            success = Some("Category added.".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}

#[post("/settings/categories/<id>/update", data = "<form>")]
pub fn update_category(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    id: i64,
    form: Form<NameForm>,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    match settings_service::update(state, SettingsEntity::Category, id, &form.name) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            error = Option::<String>::None,
            success = Some("Category updated.".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}

#[post("/settings/categories/<id>/delete")]
pub fn delete_category(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    id: i64,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    match settings_service::delete(state, SettingsEntity::Category, id) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            error = Option::<String>::None,
            success = Some("Category deleted.".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}

#[post("/settings/weight-classes", data = "<form>")]
pub fn create_weight_class(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    form: Form<NameForm>,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament = tournament_service::get_or_create(state).ok_or(Status::InternalServerError)?;
    match settings_service::create(state, tournament.id, SettingsEntity::WeightClass, &form.name) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            error = Option::<String>::None,
            success = Some("Weight class added.".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}

#[post("/settings/weight-classes/<id>/update", data = "<form>")]
pub fn update_weight_class(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    id: i64,
    form: Form<NameForm>,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    match settings_service::update(state, SettingsEntity::WeightClass, id, &form.name) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            error = Option::<String>::None,
            success = Some("Weight class updated.".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}

#[post("/settings/weight-classes/<id>/delete")]
pub fn delete_weight_class(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    id: i64,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    match settings_service::delete(state, SettingsEntity::WeightClass, id) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            error = Option::<String>::None,
            success = Some("Weight class deleted.".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}

#[post("/settings/events", data = "<form>")]
pub fn create_event(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    form: Form<NameForm>,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament = tournament_service::get_or_create(state).ok_or(Status::InternalServerError)?;
    match settings_service::create(state, tournament.id, SettingsEntity::Event, &form.name) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            error = Option::<String>::None,
            success = Some("Event added.".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}

#[post("/settings/events/<id>/update", data = "<form>")]
pub fn update_event(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    id: i64,
    form: Form<NameForm>,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    match settings_service::update(state, SettingsEntity::Event, id, &form.name) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            error = Option::<String>::None,
            success = Some("Event updated.".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}

#[post("/settings/events/<id>/delete")]
pub fn delete_event(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    id: i64,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    match settings_service::delete(state, SettingsEntity::Event, id) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            error = Option::<String>::None,
            success = Some("Event deleted.".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}
