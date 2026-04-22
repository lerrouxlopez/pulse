use crate::db;
use crate::services::settings_service::SettingsEntity;
use crate::services::{access_service, auth_service, settings_service, tournament_service};
use crate::state::AppState;
use image::{imageops::FilterType, GenericImageView};
use mysql::prelude::*;
use rocket::form::{Form, FromForm};
use rocket::fs::TempFile;
use rocket::http::{Cookie, CookieJar, Status};
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(FromForm)]
pub struct NameForm {
    pub name: String,
}

#[derive(FromForm)]
pub struct RoleForm {
    pub name: String,
}

#[derive(FromForm)]
pub struct RolePermissionsForm {
    pub permissions: Vec<String>,
}

#[derive(FromForm)]
pub struct UserRoleForm {
    pub user_id: i64,
    pub role_id: i64,
}

#[derive(FromForm)]
pub struct CreateUserForm<'r> {
    pub name: String,
    pub email: String,
    pub password: String,
    pub role_id: Option<i64>,
    pub photo_file: Option<TempFile<'r>>,
}

#[derive(FromForm)]
pub struct UpdateUserForm<'r> {
    pub name: String,
    pub email: String,
    pub role_id: Option<i64>,
    pub photo_file: Option<TempFile<'r>>,
    pub password: Option<String>,
    pub password_confirm: Option<String>,
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
    let _ = access_service::ensure_owner_role(state, tournament.id);
    let _ = access_service::assign_owner(state, tournament.id, tournament.user_id);
    if !access_service::user_has_permission(state, user.id, tournament.id, "settings") {
        return Err(Redirect::to(uri!(
            crate::controllers::dashboard_controller::tournament_dashboard(slug = tournament.slug)
        )));
    }

    jar.add(Cookie::new("last_tournament_slug", tournament.slug.clone()));
    let tournament_login_path = format!("/t/{}/login", tournament.slug);

    let divisions = settings_service::list(state, tournament.id, SettingsEntity::Division);
    let categories = settings_service::list(state, tournament.id, SettingsEntity::Category);
    let weight_classes = settings_service::list(state, tournament.id, SettingsEntity::WeightClass);
    let events = settings_service::list(state, tournament.id, SettingsEntity::Event);
    let access_users = access_service::list_access_users(state, tournament.id);
    let roles = access_service::list_roles(state, tournament.id);
    let permissions = access_service::permissions();
    let can_complete_setup = !divisions.is_empty()
        && !categories.is_empty()
        && !weight_classes.is_empty()
        && !events.is_empty();
    let category_names: Vec<String> = categories
        .iter()
        .map(|item| item.name.to_lowercase())
        .collect();
    let event_names: Vec<String> = events.iter().map(|item| item.name.to_lowercase()).collect();
    let weight_names: Vec<String> = weight_classes
        .iter()
        .map(|item| item.name.to_lowercase())
        .collect();

    let active_tab = tab.unwrap_or_else(|| "divisions".to_string());
    let is_owner = access_service::is_owner(state, user.id, tournament.id);
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
            roles: roles,
            permissions: permissions,
            error: error,
            success: success,
            active: "settings",
            active_tab: active_tab,
            can_complete_setup: can_complete_setup,
            allowed_pages: access_service::user_permissions(state, user.id, tournament.id),
            is_owner: is_owner,
            tournament_owner_id: tournament.user_id,
            tournament_login_path: tournament_login_path,
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
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, _user.id).ok_or(Status::NotFound)?;
    let divisions = settings_service::list(state, tournament.id, SettingsEntity::Division);
    let categories = settings_service::list(state, tournament.id, SettingsEntity::Category);
    let weight_classes = settings_service::list(state, tournament.id, SettingsEntity::WeightClass);
    let events = settings_service::list(state, tournament.id, SettingsEntity::Event);
    let can_complete_setup = !divisions.is_empty()
        && !categories.is_empty()
        && !weight_classes.is_empty()
        && !events.is_empty();

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
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, _user.id).ok_or(Status::NotFound)?;
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
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, _user.id).ok_or(Status::NotFound)?;
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
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, _user.id).ok_or(Status::NotFound)?;
    match settings_service::delete(state, _user.id, tournament.id, SettingsEntity::Division, id) {
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
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, _user.id).ok_or(Status::NotFound)?;
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
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, _user.id).ok_or(Status::NotFound)?;
    if form.options.is_empty() {
        return Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some("Select at least one category to add.".to_string()),
            success = Option::<String>::None,
            tab = Some("categories".to_string())
        ))));
    }

    let existing = settings_service::list(state, tournament.id, SettingsEntity::Category);
    let mut existing_names: Vec<String> = existing
        .iter()
        .map(|item| item.name.to_lowercase())
        .collect();

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
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, _user.id).ok_or(Status::NotFound)?;
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
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, _user.id).ok_or(Status::NotFound)?;
    match settings_service::delete(state, _user.id, tournament.id, SettingsEntity::Category, id) {
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
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, _user.id).ok_or(Status::NotFound)?;
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
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, _user.id).ok_or(Status::NotFound)?;
    if form.options.is_empty() {
        return Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some("Select at least one weight class to add.".to_string()),
            success = Option::<String>::None,
            tab = Some("weight".to_string())
        ))));
    }

    let existing = settings_service::list(state, tournament.id, SettingsEntity::WeightClass);
    let mut existing_names: Vec<String> = existing
        .iter()
        .map(|item| item.name.to_lowercase())
        .collect();

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
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, _user.id).ok_or(Status::NotFound)?;
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
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, _user.id).ok_or(Status::NotFound)?;
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
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, _user.id).ok_or(Status::NotFound)?;
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
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, _user.id).ok_or(Status::NotFound)?;
    if form.options.is_empty() {
        return Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some("Select at least one event to add.".to_string()),
            success = Option::<String>::None,
            tab = Some("events".to_string())
        ))));
    }

    let existing = settings_service::list(state, tournament.id, SettingsEntity::Event);
    let mut existing_names: Vec<String> = existing
        .iter()
        .map(|item| item.name.to_lowercase())
        .collect();

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
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, _user.id).ok_or(Status::NotFound)?;
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
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, _user.id).ok_or(Status::NotFound)?;
    match settings_service::delete(state, _user.id, tournament.id, SettingsEntity::Event, id) {
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

#[post("/<slug>/settings/roles", data = "<form>")]
pub fn create_role(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    form: Form<RoleForm>,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, user.id).ok_or(Status::NotFound)?;
    if !access_service::is_owner(state, user.id, tournament.id) {
        return Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some("Only the owner can manage roles.".to_string()),
            success = Option::<String>::None,
            tab = Some("roles".to_string())
        ))));
    }
    match access_service::create_role(state, tournament.id, &form.name) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Option::<String>::None,
            success = Some("Role created.".to_string()),
            tab = Some("roles".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("roles".to_string())
        )))),
    }
}

#[post("/<slug>/settings/roles/<id>/permissions", data = "<form>")]
pub fn update_role_permissions(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    id: i64,
    form: Form<RolePermissionsForm>,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, user.id).ok_or(Status::NotFound)?;
    if !access_service::is_owner(state, user.id, tournament.id) {
        return Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some("Only the owner can manage roles.".to_string()),
            success = Option::<String>::None,
            tab = Some("roles".to_string())
        ))));
    }
    match access_service::update_role_permissions(state, tournament.id, id, &form.permissions) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Option::<String>::None,
            success = Some("Role permissions updated.".to_string()),
            tab = Some("roles".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("roles".to_string())
        )))),
    }
}

#[post("/<slug>/settings/roles/<id>/delete")]
pub fn delete_role(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    id: i64,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, user.id).ok_or(Status::NotFound)?;
    if !access_service::is_owner(state, user.id, tournament.id) {
        return Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some("Only the owner can manage roles.".to_string()),
            success = Option::<String>::None,
            tab = Some("roles".to_string())
        ))));
    }
    match access_service::delete_role(state, tournament.id, id) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Option::<String>::None,
            success = Some("Role deleted.".to_string()),
            tab = Some("roles".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("roles".to_string())
        )))),
    }
}

#[post("/<slug>/settings/roles/users/assign", data = "<form>")]
pub fn assign_user_role(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    form: Form<UserRoleForm>,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, user.id).ok_or(Status::NotFound)?;
    if !access_service::is_owner(state, user.id, tournament.id) {
        return Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some("Only the owner can assign roles.".to_string()),
            success = Option::<String>::None,
            tab = Some("roles".to_string())
        ))));
    }
    match access_service::assign_user_role(state, tournament.id, form.user_id, form.role_id) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Option::<String>::None,
            success = Some("User role updated.".to_string()),
            tab = Some("roles".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("roles".to_string())
        )))),
    }
}

#[post("/<slug>/settings/roles/users/create", data = "<form>")]
pub async fn create_user(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    mut form: Form<CreateUserForm<'_>>,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, user.id).ok_or(Status::NotFound)?;
    if !access_service::is_owner(state, user.id, tournament.id) {
        return Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some("Only the owner can create users.".to_string()),
            success = Option::<String>::None,
            tab = Some("roles".to_string())
        ))));
    }
    let photo_url = match save_user_photo(&mut form.photo_file).await {
        Ok(value) => value,
        Err(err) if err.kind() == std::io::ErrorKind::InvalidInput => {
            return Ok(Redirect::to(uri!(settings_page(
                slug = slug,
                error = Some(format!("Invalid user photo: {}", err)),
                success = Option::<String>::None,
                tab = Some("roles".to_string())
            ))));
        }
        Err(_) => return Err(Status::InternalServerError),
    };

    match access_service::create_user(
        state,
        tournament.id,
        &form.name,
        &form.email,
        &form.password,
        form.role_id,
        photo_url.as_deref(),
    ) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Option::<String>::None,
            success = Some("Tournament user created.".to_string()),
            tab = Some("roles".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("roles".to_string())
        )))),
    }
}

#[post("/<slug>/settings/roles/users/<id>/update", data = "<form>")]
pub async fn update_user(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    id: i64,
    mut form: Form<UpdateUserForm<'_>>,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, user.id).ok_or(Status::NotFound)?;
    if !access_service::is_owner(state, user.id, tournament.id) {
        return Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some("Only the owner can manage users.".to_string()),
            success = Option::<String>::None,
            tab = Some("roles".to_string())
        ))));
    }
    let uploaded_photo = match save_user_photo(&mut form.photo_file).await {
        Ok(value) => value,
        Err(err) if err.kind() == std::io::ErrorKind::InvalidInput => {
            return Ok(Redirect::to(uri!(settings_page(
                slug = slug,
                error = Some(format!("Invalid user photo: {}", err)),
                success = Option::<String>::None,
                tab = Some("roles".to_string())
            ))));
        }
        Err(_) => return Err(Status::InternalServerError),
    };

    let existing_photo = if uploaded_photo.is_none() {
        let mut conn = db::open_conn(&state.pool).map_err(|_| Status::InternalServerError)?;
        conn.exec_first::<Option<String>, _, _>(
            "SELECT photo_url FROM users WHERE id = ? AND tournament_id = ? AND user_type = 'tournament' LIMIT 1",
            (id, tournament.id),
        )
        .map_err(|_| Status::InternalServerError)?
        .flatten()
    } else {
        None
    };
    let photo_url = uploaded_photo.as_deref().or(existing_photo.as_deref());

    let new_password = form
        .password
        .as_deref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    if let Some(password) = new_password {
        let confirm = form
            .password_confirm
            .as_deref()
            .map(|value| value.trim())
            .unwrap_or("");
        if password != confirm {
            return Ok(Redirect::to(uri!(settings_page(
                slug = slug,
                error = Some("Passwords do not match.".to_string()),
                success = Option::<String>::None,
                tab = Some("roles".to_string())
            ))));
        }
    }

    match access_service::update_user(
        state,
        tournament.id,
        id,
        &form.name,
        &form.email,
        photo_url,
        new_password,
    )
    {
        Ok(_) => {
            if let Some(role_id) = form.role_id {
                let _ = access_service::assign_user_role(state, tournament.id, id, role_id);
            }
            Ok(Redirect::to(uri!(settings_page(
                slug = slug,
                error = Option::<String>::None,
                success = Some("User updated.".to_string()),
                tab = Some("roles".to_string())
            ))))
        }
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("roles".to_string())
        )))),
    }
}

async fn save_user_photo(
    file: &mut Option<TempFile<'_>>,
) -> Result<Option<String>, std::io::Error> {
    let Some(upload) = file else {
        return Ok(None);
    };
    if upload.len() == 0 {
        return Ok(None);
    }
    const MAX_UPLOAD_BYTES: u64 = 5 * 1024 * 1024;
    if upload.len() > MAX_UPLOAD_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Photo too large",
        ));
    }
    // Don't trust the uploaded Content-Type; decode is the source of truth.

    let uploads_dir = Path::new("static").join("uploads");
    std::fs::create_dir_all(&uploads_dir)?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let raw_filename = format!("user-photo-raw-{}.bin", timestamp);
    let raw_path = uploads_dir.join(raw_filename);
    upload.persist_to(&raw_path).await?;

    let data = std::fs::read(&raw_path)?;
    if data.len() < 4 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Upload was empty or truncated",
        ));
    }
    let is_png = data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
    let is_jpeg = data.starts_with(&[0xFF, 0xD8, 0xFF]);
    if !(is_png || is_jpeg) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "File is not a PNG or JPEG",
        ));
    }

    let reader = image::ImageReader::new(std::io::Cursor::new(data))
        .with_guessed_format()
        .map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid image format")
        })?;
    let image = reader.decode().map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "Unable to decode image")
    })?;
    let (width, height) = image.dimensions();
    let crop_size = width.min(height);
    let x = (width - crop_size) / 2;
    let y = (height - crop_size) / 2;
    let cropped = image.crop_imm(x, y, crop_size, crop_size);
    let resized = cropped.resize(320, 320, FilterType::CatmullRom);

    let filename = format!("user-photo-{}.png", timestamp);
    let filepath = uploads_dir.join(filename);
    resized
        .save(&filepath)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Unable to save image"))?;
    let _ = std::fs::remove_file(&raw_path);
    let public_path = format!(
        "/static/uploads/{}",
        filepath.file_name().unwrap().to_string_lossy()
    );
    Ok(Some(public_path))
}

#[post("/<slug>/settings/roles/users/<id>/delete")]
pub fn delete_user(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    id: i64,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, user.id).ok_or(Status::NotFound)?;
    if !access_service::is_owner(state, user.id, tournament.id) {
        return Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some("Only the owner can manage users.".to_string()),
            success = Option::<String>::None,
            tab = Some("roles".to_string())
        ))));
    }
    match access_service::remove_user_from_tournament(state, tournament.id, id) {
        Ok(_) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Option::<String>::None,
            success = Some("User removed.".to_string()),
            tab = Some("roles".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(settings_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None,
            tab = Some("roles".to_string())
        )))),
    }
}
