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

#[get("/settings?<error>&<success>&<tab>")]
pub fn settings_page(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    error: Option<String>,
    success: Option<String>,
    tab: Option<String>,
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

    let tournament_id = match jar.get("tournament_id") {
        Some(cookie) => cookie.value().parse::<i64>().ok(),
        None => None,
    };
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

    let divisions = settings_service::list(state, tournament.id, SettingsEntity::Division);
    let categories = settings_service::list(state, tournament.id, SettingsEntity::Category);
    let weight_classes = settings_service::list(state, tournament.id, SettingsEntity::WeightClass);
    let events = settings_service::list(state, tournament.id, SettingsEntity::Event);
    let can_complete_setup =
        !divisions.is_empty() && !categories.is_empty() && !weight_classes.is_empty() && !events.is_empty();

    let active_tab = tab.unwrap_or_else(|| "divisions".to_string());
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
            active_tab: active_tab,
            can_complete_setup: can_complete_setup,
        },
    ))
}

#[post("/settings/setup/complete")]
pub fn complete_setup(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament_id = jar
        .get("tournament_id")
        .and_then(|cookie| cookie.value().parse::<i64>().ok())
        .ok_or(Status::BadRequest)?;
    let tournament = tournament_service::get_by_id_for_user(state, tournament_id, _user.id)
        .ok_or(Status::NotFound)?;
    let divisions = settings_service::list(state, tournament.id, SettingsEntity::Division);
    let categories = settings_service::list(state, tournament.id, SettingsEntity::Category);
    let weight_classes = settings_service::list(state, tournament.id, SettingsEntity::WeightClass);
    let events = settings_service::list(state, tournament.id, SettingsEntity::Event);
    let can_complete_setup =
        !divisions.is_empty() && !categories.is_empty() && !weight_classes.is_empty() && !events.is_empty();

    if !can_complete_setup {
        return Ok(Redirect::to(uri!(settings_page(
            error = Some("Add at least one item in each tab before completing setup.".to_string()),
            success = Option::<String>::None,
            tab = Option::<String>::None
        ))));
    }

    if tournament_service::mark_setup_complete(state, tournament.id) {
        Ok(Redirect::to(uri!(settings_page(
            error = Option::<String>::None,
            success = Some("Tournament setup completed.".to_string()),
            tab = Option::<String>::None
        ))))
    } else {
        Ok(Redirect::to(uri!(settings_page(
            error = Some("Unable to update setup status.".to_string()),
            success = Option::<String>::None,
            tab = Option::<String>::None
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
    let tournament_id = jar
        .get("tournament_id")
        .and_then(|cookie| cookie.value().parse::<i64>().ok())
        .ok_or(Status::BadRequest)?;
    let tournament = tournament_service::get_by_id_for_user(state, tournament_id, _user.id)
        .ok_or(Status::NotFound)?;
    match settings_service::create(
        state,
        _user.id,
        tournament.id,
        SettingsEntity::Division,
        &form.name,
    ) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            error = Option::<String>::None,
            success = Some("Division added.".to_string()),
            tab = Some("divisions".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("divisions".to_string())
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
    let tournament_id = jar
        .get("tournament_id")
        .and_then(|cookie| cookie.value().parse::<i64>().ok())
        .ok_or(Status::BadRequest)?;
    let tournament = tournament_service::get_by_id_for_user(state, tournament_id, _user.id)
        .ok_or(Status::NotFound)?;
    match settings_service::update(
        state,
        _user.id,
        tournament.id,
        SettingsEntity::Division,
        id,
        &form.name,
    ) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            error = Option::<String>::None,
            success = Some("Division updated.".to_string()),
            tab = Some("divisions".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("divisions".to_string())
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
    let tournament_id = jar
        .get("tournament_id")
        .and_then(|cookie| cookie.value().parse::<i64>().ok())
        .ok_or(Status::BadRequest)?;
    let tournament = tournament_service::get_by_id_for_user(state, tournament_id, _user.id)
        .ok_or(Status::NotFound)?;
    match settings_service::delete(
        state,
        _user.id,
        tournament.id,
        SettingsEntity::Division,
        id,
    ) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            error = Option::<String>::None,
            success = Some("Division deleted.".to_string()),
            tab = Some("divisions".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("divisions".to_string())
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
    let tournament_id = jar
        .get("tournament_id")
        .and_then(|cookie| cookie.value().parse::<i64>().ok())
        .ok_or(Status::BadRequest)?;
    let tournament = tournament_service::get_by_id_for_user(state, tournament_id, _user.id)
        .ok_or(Status::NotFound)?;
    match settings_service::create(
        state,
        _user.id,
        tournament.id,
        SettingsEntity::Category,
        &form.name,
    ) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            error = Option::<String>::None,
            success = Some("Category added.".to_string()),
            tab = Some("categories".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("categories".to_string())
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
    let tournament_id = jar
        .get("tournament_id")
        .and_then(|cookie| cookie.value().parse::<i64>().ok())
        .ok_or(Status::BadRequest)?;
    let tournament = tournament_service::get_by_id_for_user(state, tournament_id, _user.id)
        .ok_or(Status::NotFound)?;
    match settings_service::update(
        state,
        _user.id,
        tournament.id,
        SettingsEntity::Category,
        id,
        &form.name,
    ) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            error = Option::<String>::None,
            success = Some("Category updated.".to_string()),
            tab = Some("categories".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("categories".to_string())
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
    let tournament_id = jar
        .get("tournament_id")
        .and_then(|cookie| cookie.value().parse::<i64>().ok())
        .ok_or(Status::BadRequest)?;
    let tournament = tournament_service::get_by_id_for_user(state, tournament_id, _user.id)
        .ok_or(Status::NotFound)?;
    match settings_service::delete(
        state,
        _user.id,
        tournament.id,
        SettingsEntity::Category,
        id,
    ) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            error = Option::<String>::None,
            success = Some("Category deleted.".to_string()),
            tab = Some("categories".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("categories".to_string())
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
    let tournament_id = jar
        .get("tournament_id")
        .and_then(|cookie| cookie.value().parse::<i64>().ok())
        .ok_or(Status::BadRequest)?;
    let tournament = tournament_service::get_by_id_for_user(state, tournament_id, _user.id)
        .ok_or(Status::NotFound)?;
    match settings_service::create(
        state,
        _user.id,
        tournament.id,
        SettingsEntity::WeightClass,
        &form.name,
    ) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            error = Option::<String>::None,
            success = Some("Weight class added.".to_string()),
            tab = Some("weight".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("weight".to_string())
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
    let tournament_id = jar
        .get("tournament_id")
        .and_then(|cookie| cookie.value().parse::<i64>().ok())
        .ok_or(Status::BadRequest)?;
    let tournament = tournament_service::get_by_id_for_user(state, tournament_id, _user.id)
        .ok_or(Status::NotFound)?;
    match settings_service::update(
        state,
        _user.id,
        tournament.id,
        SettingsEntity::WeightClass,
        id,
        &form.name,
    ) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            error = Option::<String>::None,
            success = Some("Weight class updated.".to_string()),
            tab = Some("weight".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("weight".to_string())
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
    let tournament_id = jar
        .get("tournament_id")
        .and_then(|cookie| cookie.value().parse::<i64>().ok())
        .ok_or(Status::BadRequest)?;
    let tournament = tournament_service::get_by_id_for_user(state, tournament_id, _user.id)
        .ok_or(Status::NotFound)?;
    match settings_service::delete(
        state,
        _user.id,
        tournament.id,
        SettingsEntity::WeightClass,
        id,
    ) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            error = Option::<String>::None,
            success = Some("Weight class deleted.".to_string()),
            tab = Some("weight".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("weight".to_string())
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
    let tournament_id = jar
        .get("tournament_id")
        .and_then(|cookie| cookie.value().parse::<i64>().ok())
        .ok_or(Status::BadRequest)?;
    let tournament = tournament_service::get_by_id_for_user(state, tournament_id, _user.id)
        .ok_or(Status::NotFound)?;
    match settings_service::create(
        state,
        _user.id,
        tournament.id,
        SettingsEntity::Event,
        &form.name,
    ) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            error = Option::<String>::None,
            success = Some("Event added.".to_string()),
            tab = Some("events".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("events".to_string())
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
    let tournament_id = jar
        .get("tournament_id")
        .and_then(|cookie| cookie.value().parse::<i64>().ok())
        .ok_or(Status::BadRequest)?;
    let tournament = tournament_service::get_by_id_for_user(state, tournament_id, _user.id)
        .ok_or(Status::NotFound)?;
    match settings_service::update(
        state,
        _user.id,
        tournament.id,
        SettingsEntity::Event,
        id,
        &form.name,
    ) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            error = Option::<String>::None,
            success = Some("Event updated.".to_string()),
            tab = Some("events".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("events".to_string())
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
    let tournament_id = jar
        .get("tournament_id")
        .and_then(|cookie| cookie.value().parse::<i64>().ok())
        .ok_or(Status::BadRequest)?;
    let tournament = tournament_service::get_by_id_for_user(state, tournament_id, _user.id)
        .ok_or(Status::NotFound)?;
    match settings_service::delete(
        state,
        _user.id,
        tournament.id,
        SettingsEntity::Event,
        id,
    ) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            error = Option::<String>::None,
            success = Some("Event deleted.".to_string()),
            tab = Some("events".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("events".to_string())
        )))),
    }
}
