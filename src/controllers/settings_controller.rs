use crate::services::{auth_service, settings_service, tournament_service};
use crate::services::settings_service::SettingsEntity;
use crate::state::AppState;
use rocket::form::{Form, FromForm};
use rocket::http::{Cookie, CookieJar, Status};
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};

#[derive(FromForm)]
pub struct NameForm {
    pub name: String,
}

#[derive(FromForm)]
pub struct InviteForm {
    pub email: String,
}

#[derive(FromForm)]
pub struct SettingsOptionsForm {
    pub options: Vec<String>,
}
#[get("/<slug>/settings?<error>&<success>&<tab>")]
pub fn settings_page(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
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

    let tournament = match tournament_service::get_by_slug_for_user(state, &slug, user.id) {
        Some(tournament) => tournament,
        None => {
            return Err(Redirect::to(uri!(
                crate::controllers::dashboard_controller::dashboard
            )))
        }
    };

    jar.add(Cookie::new("last_tournament_slug", tournament.slug.clone()));

    let divisions = settings_service::list(state, tournament.id, SettingsEntity::Division);
    let categories = settings_service::list(state, tournament.id, SettingsEntity::Category);
    let weight_classes = settings_service::list(state, tournament.id, SettingsEntity::WeightClass);
    let events = settings_service::list(state, tournament.id, SettingsEntity::Event);
    let access_users = tournament_service::list_access_users(state, tournament.id);
    let can_complete_setup =
        !divisions.is_empty() && !categories.is_empty() && !weight_classes.is_empty() && !events.is_empty();
    let category_names: Vec<String> = categories.iter().map(|item| item.name.to_lowercase()).collect();
    let event_names: Vec<String> = events.iter().map(|item| item.name.to_lowercase()).collect();
    let weight_names: Vec<String> = weight_classes.iter().map(|item| item.name.to_lowercase()).collect();

    let active_tab = tab.unwrap_or_else(|| "divisions".to_string());
    Ok(Template::render(
        "settings",
        context! {
            name: user.name,
            tournament_name: tournament.name,
            tournament_slug: tournament.slug,
            is_setup: tournament.is_setup,
            divisions: divisions,
            categories: categories,
            category_names: category_names,
            weight_classes: weight_classes,
            weight_names: weight_names,
            events: events,
            event_names: event_names,
            access_users: access_users,
            error: error,
            success: success,
            active: "settings",
            active_tab: active_tab,
            can_complete_setup: can_complete_setup,
        },
    ))
}

#[post("/<slug>/settings/setup/complete")]
pub fn complete_setup(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, _user.id)
        .ok_or(Status::NotFound)?;
    let divisions = settings_service::list(state, tournament.id, SettingsEntity::Division);
    let categories = settings_service::list(state, tournament.id, SettingsEntity::Category);
    let weight_classes = settings_service::list(state, tournament.id, SettingsEntity::WeightClass);
    let events = settings_service::list(state, tournament.id, SettingsEntity::Event);
    let can_complete_setup =
        !divisions.is_empty() && !categories.is_empty() && !weight_classes.is_empty() && !events.is_empty();

    if !can_complete_setup {
        return Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some("Add at least one item in each tab before completing setup.".to_string()),
            success = Option::<String>::None,
            tab = Option::<String>::None
        ))));
    }

    if tournament_service::mark_setup_complete(state, tournament.id) {
        Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Option::<String>::None,
            success = Some("Tournament setup completed.".to_string()),
            tab = Option::<String>::None
        ))))
    } else {
        Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some("Unable to update setup status.".to_string()),
            success = Option::<String>::None,
            tab = Option::<String>::None
        ))))
    }
}

#[post("/<slug>/settings/divisions", data = "<form>")]
pub fn create_division(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    form: Form<NameForm>,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, _user.id)
        .ok_or(Status::NotFound)?;
    match settings_service::create(
        state,
        _user.id,
        tournament.id,
        SettingsEntity::Division,
        &form.name,
    ) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Option::<String>::None,
            success = Some("Division added.".to_string()),
            tab = Some("divisions".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("divisions".to_string())
        )))),
    }
}

#[post("/<slug>/settings/divisions/<id>/update", data = "<form>")]
pub fn update_division(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    id: i64,
    form: Form<NameForm>,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, _user.id)
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
            slug = slug,
            error = Option::<String>::None,
            success = Some("Division updated.".to_string()),
            tab = Some("divisions".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("divisions".to_string())
        )))),
    }
}

#[post("/<slug>/settings/divisions/<id>/delete")]
pub fn delete_division(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    id: i64,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, _user.id)
        .ok_or(Status::NotFound)?;
    match settings_service::delete(
        state,
        _user.id,
        tournament.id,
        SettingsEntity::Division,
        id,
    ) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Option::<String>::None,
            success = Some("Division deleted.".to_string()),
            tab = Some("divisions".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("divisions".to_string())
        )))),
    }
}

#[post("/<slug>/settings/categories", data = "<form>")]
pub fn create_category(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    form: Form<NameForm>,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, _user.id)
        .ok_or(Status::NotFound)?;
    match settings_service::create(
        state,
        _user.id,
        tournament.id,
        SettingsEntity::Category,
        &form.name,
    ) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Option::<String>::None,
            success = Some("Category added.".to_string()),
            tab = Some("categories".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("categories".to_string())
        )))),
    }
}

#[post("/<slug>/settings/categories/bulk", data = "<form>")]
pub fn create_category_options(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    form: Form<SettingsOptionsForm>,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, _user.id)
        .ok_or(Status::NotFound)?;
    if form.options.is_empty() {
        return Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some("Select at least one category to add.".to_string()),
            success = Option::<String>::None,
            tab = Some("categories".to_string())
        ))));
    }

    let existing = settings_service::list(state, tournament.id, SettingsEntity::Category);
    let mut existing_names: Vec<String> =
        existing.iter().map(|item| item.name.to_lowercase()).collect();

    let mut added = 0usize;
    for option in &form.options {
        let trimmed = option.trim();
        if trimmed.is_empty() {
            continue;
        }
        let key = trimmed.to_lowercase();
        if existing_names.contains(&key) {
            continue;
        }
        if settings_service::create(
            state,
            _user.id,
            tournament.id,
            SettingsEntity::Category,
            trimmed,
        )
        .is_ok()
        {
            added += 1;
            existing_names.push(key);
        }
    }

    if added == 0 {
        Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some("Selected categories already exist.".to_string()),
            success = Option::<String>::None,
            tab = Some("categories".to_string())
        ))))
    } else {
        Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Option::<String>::None,
            success = Some(format!("Added {} category(ies).", added)),
            tab = Some("categories".to_string())
        ))))
    }
}

#[post("/<slug>/settings/categories/<id>/update", data = "<form>")]
pub fn update_category(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    id: i64,
    form: Form<NameForm>,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, _user.id)
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
            slug = slug,
            error = Option::<String>::None,
            success = Some("Category updated.".to_string()),
            tab = Some("categories".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("categories".to_string())
        )))),
    }
}

#[post("/<slug>/settings/categories/<id>/delete")]
pub fn delete_category(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    id: i64,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, _user.id)
        .ok_or(Status::NotFound)?;
    match settings_service::delete(
        state,
        _user.id,
        tournament.id,
        SettingsEntity::Category,
        id,
    ) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Option::<String>::None,
            success = Some("Category deleted.".to_string()),
            tab = Some("categories".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("categories".to_string())
        )))),
    }
}

#[post("/<slug>/settings/weight-classes", data = "<form>")]
pub fn create_weight_class(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    form: Form<NameForm>,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, _user.id)
        .ok_or(Status::NotFound)?;
    match settings_service::create(
        state,
        _user.id,
        tournament.id,
        SettingsEntity::WeightClass,
        &form.name,
    ) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Option::<String>::None,
            success = Some("Weight class added.".to_string()),
            tab = Some("weight".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("weight".to_string())
        )))),
    }
}

#[post("/<slug>/settings/weight-classes/bulk", data = "<form>")]
pub fn create_weight_options(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    form: Form<SettingsOptionsForm>,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, _user.id)
        .ok_or(Status::NotFound)?;
    if form.options.is_empty() {
        return Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some("Select at least one weight class to add.".to_string()),
            success = Option::<String>::None,
            tab = Some("weight".to_string())
        ))));
    }

    let existing = settings_service::list(state, tournament.id, SettingsEntity::WeightClass);
    let mut existing_names: Vec<String> =
        existing.iter().map(|item| item.name.to_lowercase()).collect();

    let mut added = 0usize;
    for option in &form.options {
        let trimmed = option.trim();
        if trimmed.is_empty() {
            continue;
        }
        let key = trimmed.to_lowercase();
        if existing_names.contains(&key) {
            continue;
        }
        if settings_service::create(
            state,
            _user.id,
            tournament.id,
            SettingsEntity::WeightClass,
            trimmed,
        )
        .is_ok()
        {
            added += 1;
            existing_names.push(key);
        }
    }

    if added == 0 {
        Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some("Selected weight classes already exist.".to_string()),
            success = Option::<String>::None,
            tab = Some("weight".to_string())
        ))))
    } else {
        Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Option::<String>::None,
            success = Some(format!("Added {} weight class(es).", added)),
            tab = Some("weight".to_string())
        ))))
    }
}

#[post("/<slug>/settings/weight-classes/<id>/update", data = "<form>")]
pub fn update_weight_class(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    id: i64,
    form: Form<NameForm>,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, _user.id)
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
            slug = slug,
            error = Option::<String>::None,
            success = Some("Weight class updated.".to_string()),
            tab = Some("weight".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("weight".to_string())
        )))),
    }
}

#[post("/<slug>/settings/weight-classes/<id>/delete")]
pub fn delete_weight_class(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    id: i64,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, _user.id)
        .ok_or(Status::NotFound)?;
    match settings_service::delete(
        state,
        _user.id,
        tournament.id,
        SettingsEntity::WeightClass,
        id,
    ) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Option::<String>::None,
            success = Some("Weight class deleted.".to_string()),
            tab = Some("weight".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("weight".to_string())
        )))),
    }
}

#[post("/<slug>/settings/events", data = "<form>")]
pub fn create_event(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    form: Form<NameForm>,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, _user.id)
        .ok_or(Status::NotFound)?;
    match settings_service::create(
        state,
        _user.id,
        tournament.id,
        SettingsEntity::Event,
        &form.name,
    ) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Option::<String>::None,
            success = Some("Event added.".to_string()),
            tab = Some("events".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("events".to_string())
        )))),
    }
}

#[post("/<slug>/settings/events/bulk", data = "<form>")]
pub fn create_event_options(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    form: Form<SettingsOptionsForm>,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, _user.id)
        .ok_or(Status::NotFound)?;
    if form.options.is_empty() {
        return Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some("Select at least one event to add.".to_string()),
            success = Option::<String>::None,
            tab = Some("events".to_string())
        ))));
    }

    let existing = settings_service::list(state, tournament.id, SettingsEntity::Event);
    let mut existing_names: Vec<String> =
        existing.iter().map(|item| item.name.to_lowercase()).collect();

    let mut added = 0usize;
    for option in &form.options {
        let trimmed = option.trim();
        if trimmed.is_empty() {
            continue;
        }
        let key = trimmed.to_lowercase();
        if existing_names.contains(&key) {
            continue;
        }
        if settings_service::create(
            state,
            _user.id,
            tournament.id,
            SettingsEntity::Event,
            trimmed,
        )
        .is_ok()
        {
            added += 1;
            existing_names.push(key);
        }
    }

    if added == 0 {
        Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some("Selected events already exist.".to_string()),
            success = Option::<String>::None,
            tab = Some("events".to_string())
        ))))
    } else {
        Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Option::<String>::None,
            success = Some(format!("Added {} event(s).", added)),
            tab = Some("events".to_string())
        ))))
    }
}

#[post("/<slug>/settings/events/<id>/update", data = "<form>")]
pub fn update_event(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    id: i64,
    form: Form<NameForm>,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, _user.id)
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
            slug = slug,
            error = Option::<String>::None,
            success = Some("Event updated.".to_string()),
            tab = Some("events".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("events".to_string())
        )))),
    }
}

#[post("/<slug>/settings/events/<id>/delete")]
pub fn delete_event(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    id: i64,
) -> Result<Redirect, Status> {
    let _user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, _user.id)
        .ok_or(Status::NotFound)?;
    match settings_service::delete(
        state,
        _user.id,
        tournament.id,
        SettingsEntity::Event,
        id,
    ) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Option::<String>::None,
            success = Some("Event deleted.".to_string()),
            tab = Some("events".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("events".to_string())
        )))),
    }
}

#[post("/<slug>/settings/invite", data = "<form>")]
pub fn invite_user(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    form: Form<InviteForm>,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, user.id)
        .ok_or(Status::NotFound)?;
    match tournament_service::invite_user_by_email(state, user.id, tournament.id, &form.email) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            slug = tournament.slug,
            error = Option::<String>::None,
            success = Some("Invite sent.".to_string()),
            tab = Some("access".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            slug = tournament.slug,
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("access".to_string())
        )))),
    }
}
