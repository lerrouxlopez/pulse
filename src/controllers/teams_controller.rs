use crate::services::settings_service::SettingsEntity;
use crate::services::{
    access_service, auth_service, settings_service, teams_service, tournament_service,
};
use crate::state::AppState;
use calamine::{open_workbook_auto, Data, Reader};
use image::{imageops::FilterType, GenericImageView};
use rocket::form::{Form, FromForm};
use rocket::fs::TempFile;
use rocket::http::{ContentType, Cookie, CookieJar, Status};
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs;
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
pub struct MemberForm<'r> {
    pub name: String,
    pub notes: Option<String>,
    pub weight_class: Option<String>,
    pub division_id: Option<i64>,
    pub category_ids: Option<Vec<i64>>,
    pub event_ids: Option<Vec<i64>>,
    pub photo_file: Option<TempFile<'r>>,
    pub return_to: Option<String>,
}

#[derive(FromForm)]
pub struct UpdateMemberForm<'r> {
    pub name: Option<String>,
    pub notes: Option<String>,
    pub weight_class: Option<String>,
    pub division_id: Option<i64>,
    pub clear_notes: Option<String>,
    pub clear_weight_class: Option<String>,
    pub clear_division: Option<String>,
    pub category_ids: Option<Vec<i64>>,
    pub event_ids: Option<Vec<i64>>,
    pub clear_categories: Option<String>,
    pub clear_events: Option<String>,
    pub photo_file: Option<TempFile<'r>>,
    pub clear_photo: Option<String>,
    pub return_to: Option<String>,
}

#[derive(FromForm)]
pub struct ReturnToForm {
    pub return_to: Option<String>,
}

#[derive(FromForm)]
pub struct ImportTeamsForm<'r> {
    pub import_type: String,
    pub import_file: TempFile<'r>,
}

#[derive(FromForm)]
pub struct BulkAssignMembersForm {
    pub member_ids: Vec<i64>,
    pub apply_division: Option<String>,
    pub division_id: Option<i64>,
    pub apply_categories: Option<String>,
    pub category_ids: Option<Vec<i64>>,
    pub apply_events: Option<String>,
    pub event_ids: Option<Vec<i64>>,
}

#[derive(Serialize, Clone)]
struct ImportFailureView {
    row_number: usize,
    row_data: String,
    error: String,
}

#[derive(Clone)]
struct ParsedImportRow {
    row_number: usize,
    raw: Vec<String>,
    columns: HashMap<String, String>,
}

fn render_teams_template(
    state: &State<AppState>,
    user: &crate::models::CurrentUser,
    tournament: &crate::models::Tournament,
    error: Option<String>,
    success: Option<String>,
    import_failures: Vec<ImportFailureView>,
) -> Template {
    let teams = teams_service::list(state, user.id, tournament.id).unwrap_or_default();
    let divisions = settings_service::list(state, tournament.id, SettingsEntity::Division);
    let categories = settings_service::list(state, tournament.id, SettingsEntity::Category);
    let events = settings_service::list(state, tournament.id, SettingsEntity::Event);
    let weight_classes = settings_service::list(state, tournament.id, SettingsEntity::WeightClass);
    let allowed_pages = access_service::user_permissions(state, user.id, tournament.id);
    let sidebar_nav_items =
        access_service::sidebar_nav_items(&allowed_pages, tournament.is_setup, Some(&tournament.slug));

    Template::render(
        "teams",
        context! {
            name: user.name.clone(),
            tournament_name: tournament.name.clone(),
            tournament_slug: tournament.slug.clone(),
            teams: teams,
            divisions: divisions,
            categories: categories,
            events: events,
            weight_classes: weight_classes,
            error: error,
            success: success,
            import_failures: import_failures,
            active: "teams",
            is_setup: tournament.is_setup,
            allowed_pages: allowed_pages,
            sidebar_nav_items: sidebar_nav_items,
        },
    )
}

#[get("/<slug>/teams?<error>&<success>")]
pub fn teams_page(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
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

    let tournament = match tournament_service::get_by_slug_for_user(state, &slug, user.id) {
        Some(tournament) => tournament,
        None => {
            return Err(Redirect::to(uri!(
                crate::controllers::dashboard_controller::dashboard
            )))
        }
    };
    if !access_service::user_has_permission(state, user.id, tournament.id, "teams") {
        return Err(Redirect::to(uri!(
            crate::controllers::dashboard_controller::tournament_dashboard(slug = tournament.slug)
        )));
    }

    jar.add(Cookie::new("last_tournament_slug", tournament.slug.clone()));

    Ok(render_teams_template(
        state,
        &user,
        &tournament,
        error,
        success,
        Vec::new(),
    ))
}

#[get("/<slug>/teams/import/template/<kind>")]
pub fn download_import_template(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    kind: String,
) -> Result<(ContentType, String), Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, user.id).ok_or(Status::NotFound)?;
    if !access_service::user_has_permission(state, user.id, tournament.id, "teams") {
        return Err(Status::Forbidden);
    }

    let body = match kind.as_str() {
        "teams" => {
            "name,divisions,categories,events\nAlpha Team,\"Male Division|Female Division\",\"6-8 years\",\"Single Live Stick|Double Live Stick\"\n"
        }
        "members" => {
            "team_name,name,weight_class,division,categories,events,notes\nAlpha Team,Juan Dela Cruz,\"Light Weight: -66 kg (Men), -57 kg (Women)\",Male Division,\"6-8 years\",\"Single Live Stick\",\"Sample note\"\n"
        }
        _ => return Err(Status::NotFound),
    };
    Ok((ContentType::CSV, body.to_string()))
}

#[post("/<slug>/teams/import", data = "<form>")]
pub async fn import_teams_data(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    mut form: Form<ImportTeamsForm<'_>>,
) -> Result<Template, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, user.id).ok_or(Status::NotFound)?;
    if !access_service::user_has_permission(state, user.id, tournament.id, "teams") {
        return Err(Status::Forbidden);
    }

    let import_type = form.import_type.trim().to_lowercase();
    if import_type != "teams" && import_type != "members" {
        return Ok(render_teams_template(
            state,
            &user,
            &tournament,
            Some("Invalid import type.".to_string()),
            None,
            Vec::new(),
        ));
    }

    let original_name = form
        .import_file
        .raw_name()
        .map(|name| name.dangerous_unsafe_unsanitized_raw().to_string())
        .unwrap_or_default();
    let extension = Path::new(&original_name)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_lowercase();

    let temp_dir = std::env::temp_dir().join("pulse-imports");
    fs::create_dir_all(&temp_dir).map_err(|_| Status::InternalServerError)?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let filepath = temp_dir.join(format!(
        "import-{}-{}.{}",
        tournament.id,
        timestamp,
        if extension.is_empty() { "csv" } else { &extension }
    ));
    form.import_file
        .persist_to(&filepath)
        .await
        .map_err(|_| Status::InternalServerError)?;

    let parsed_rows = if extension == "xlsx" {
        parse_xlsx_rows(&filepath)
    } else {
        parse_csv_rows(&filepath)
    };
    let _ = fs::remove_file(&filepath);

    let rows = match parsed_rows {
        Ok(rows) => rows,
        Err(message) => {
            return Ok(render_teams_template(
                state,
                &user,
                &tournament,
                Some(message),
                None,
                Vec::new(),
            ))
        }
    };

    if rows.is_empty() {
        return Ok(render_teams_template(
            state,
            &user,
            &tournament,
            Some("Import file has no data rows.".to_string()),
            None,
            Vec::new(),
        ));
    }

    let (imported, skipped, failures) = if import_type == "teams" {
        process_team_import_rows(state, user.id, tournament.id, rows)
    } else {
        process_member_import_rows(state, user.id, tournament.id, rows)
    };

    let success = Some(format!(
        "Imported {} row(s). Skipped {} duplicate row(s). {} row(s) failed.",
        imported,
        skipped,
        failures.len()
    ));
    let error = if failures.is_empty() {
        None
    } else {
        Some("Some rows failed. Review and correct the rows below.".to_string())
    };
    Ok(render_teams_template(
        state,
        &user,
        &tournament,
        error,
        success,
        failures,
    ))
}

#[get("/<slug>/teams/<id>?<q>&<sort>&<dir>&<error>&<success>")]
pub fn team_profile(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    id: i64,
    q: Option<String>,
    sort: Option<String>,
    dir: Option<String>,
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

    let tournament = match tournament_service::get_by_slug_for_user(state, &slug, user.id) {
        Some(tournament) => tournament,
        None => {
            return Err(Redirect::to(uri!(
                crate::controllers::dashboard_controller::dashboard
            )))
        }
    };
    if !access_service::user_has_permission(state, user.id, tournament.id, "teams") {
        return Err(Redirect::to(uri!(
            crate::controllers::dashboard_controller::tournament_dashboard(slug = tournament.slug)
        )));
    }

    jar.add(Cookie::new("last_tournament_slug", tournament.slug.clone()));

    let team = match teams_service::get_team(state, user.id, tournament.id, id) {
        Ok(Some(team)) => team,
        _ => {
            return Err(Redirect::to(uri!(
                crate::controllers::teams_controller::teams_page(
                    slug = slug,
                    error = Some("Team not found.".to_string()),
                    success = Option::<String>::None
                )
            )))
        }
    };
    let weight_classes = settings_service::list(state, tournament.id, SettingsEntity::WeightClass);

    let total_members = team.members.len();
    let divisions_count = team.divisions.len();
    let categories_count = team.categories.len();
    let events_count = team.events.len();
    let members_with_division = team
        .members
        .iter()
        .filter(|m| m.division_id.is_some())
        .count();
    let members_with_category = team
        .members
        .iter()
        .filter(|m| !m.category_ids.is_empty())
        .count();
    let members_with_event = team
        .members
        .iter()
        .filter(|m| !m.event_ids.is_empty())
        .count();
    let coverage_division = if total_members == 0 {
        0
    } else {
        (members_with_division * 100 / total_members) as i64
    };
    let coverage_category = if total_members == 0 {
        0
    } else {
        (members_with_category * 100 / total_members) as i64
    };
    let coverage_event = if total_members == 0 {
        0
    } else {
        (members_with_event * 100 / total_members) as i64
    };

    let filtered_team = {
        let mut filtered = team;
        if let Some(ref query) = q {
            let needle = query.trim().to_lowercase();
            if !needle.is_empty() {
                filtered.members = filtered
                    .members
                    .into_iter()
                    .filter(|member| {
                        let name = member.name.to_lowercase();
                        let weight = member.weight_class.as_deref().unwrap_or("").to_lowercase();
                        let notes = member.notes.as_deref().unwrap_or("").to_lowercase();
                        name.contains(&needle)
                            || weight.contains(&needle)
                            || notes.contains(&needle)
                    })
                    .collect();
            }
        }

        let sort_by = sort.as_deref().unwrap_or("name").to_lowercase();
        let sort_dir = dir.as_deref().unwrap_or("asc").to_lowercase();
        filtered.members.sort_by(|a, b| {
            let key_a = match sort_by.as_str() {
                "weight" => a.weight_class.as_deref().unwrap_or(""),
                _ => a.name.as_str(),
            }
            .to_lowercase();
            let key_b = match sort_by.as_str() {
                "weight" => b.weight_class.as_deref().unwrap_or(""),
                _ => b.name.as_str(),
            }
            .to_lowercase();
            key_a.cmp(&key_b)
        });
        if sort_dir == "desc" {
            filtered.members.reverse();
        }

        filtered
    };
    let allowed_pages = access_service::user_permissions(state, user.id, tournament.id);
    let sidebar_nav_items =
        access_service::sidebar_nav_items(&allowed_pages, tournament.is_setup, Some(&tournament.slug));

    Ok(Template::render(
        "team_profile",
        context! {
            name: user.name,
            tournament_name: tournament.name,
            tournament_slug: tournament.slug,
            team: filtered_team,
            active: "teams",
            is_setup: tournament.is_setup,
            search_query: q,
            sort_by: sort.unwrap_or_else(|| "name".to_string()),
            sort_dir: dir.unwrap_or_else(|| "asc".to_string()),
            error: error,
            success: success,
            weight_classes: weight_classes,
            total_members: total_members,
            divisions_count: divisions_count,
            categories_count: categories_count,
            events_count: events_count,
            members_with_division: members_with_division,
            members_with_category: members_with_category,
            members_with_event: members_with_event,
            coverage_division: coverage_division,
            coverage_category: coverage_category,
            coverage_event: coverage_event,
            allowed_pages: allowed_pages,
            sidebar_nav_items: sidebar_nav_items,
        },
    ))
}

#[post("/<slug>/teams", data = "<form>")]
pub async fn create_team(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    mut form: Form<TeamForm<'_>>,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, user.id).ok_or(Status::NotFound)?;
    let logo_url = match save_logo(&mut form.logo_file).await {
        Ok(value) => value,
        Err(err) => {
            return Ok(Redirect::to(uri!(teams_page(
                slug = slug,
                error = Some(format!("Unable to save team logo: {err}")),
                success = Option::<String>::None
            ))))
        }
    };

    let mut division_ids = form.division_ids.clone().unwrap_or_default();
    division_ids.sort_unstable();
    division_ids.dedup();
    let mut category_ids = form.category_ids.clone().unwrap_or_default();
    category_ids.sort_unstable();
    category_ids.dedup();
    let mut event_ids = form.event_ids.clone().unwrap_or_default();
    event_ids.sort_unstable();
    event_ids.dedup();
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
            slug = slug,
            error = Option::<String>::None,
            success = Some("Team added.".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(teams_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}

#[post("/<slug>/teams/<id>/update", data = "<form>")]
pub async fn update_team(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    id: i64,
    mut form: Form<TeamForm<'_>>,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, user.id).ok_or(Status::NotFound)?;
    let should_save_logo = form
        .logo_file
        .as_ref()
        .is_some_and(|upload| upload.len() > 0);
    let uploaded_logo = if should_save_logo {
        match save_logo(&mut form.logo_file).await {
            Ok(value) => value,
            Err(err) => {
                return Ok(Redirect::to(uri!(teams_page(
                    slug = slug,
                    error = Some(format!("Unable to save team logo: {err}")),
                    success = Option::<String>::None
                ))))
            }
        }
    } else {
        None
    };
    let logo_url = if uploaded_logo.is_some() {
        uploaded_logo
    } else {
        match teams_service::get_team_logo(state, user.id, tournament.id, id) {
            Ok(value) => value,
            Err(message) => {
                return Ok(Redirect::to(uri!(teams_page(
                    slug = slug,
                    error = Some(message),
                    success = Option::<String>::None
                ))))
            }
        }
    };
    let mut division_ids = form.division_ids.clone().unwrap_or_default();
    division_ids.sort_unstable();
    division_ids.dedup();
    let mut category_ids = form.category_ids.clone().unwrap_or_default();
    category_ids.sort_unstable();
    category_ids.dedup();
    let mut event_ids = form.event_ids.clone().unwrap_or_default();
    event_ids.sort_unstable();
    event_ids.dedup();
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
            slug = slug,
            error = Option::<String>::None,
            success = Some("Team updated.".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(teams_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}

#[post("/<slug>/teams/<id>/delete")]
pub fn delete_team(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    id: i64,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, user.id).ok_or(Status::NotFound)?;
    match teams_service::delete_team(state, user.id, tournament.id, id) {
        Ok(_) => Ok(Redirect::to(uri!(teams_page(
            slug = slug,
            error = Option::<String>::None,
            success = Some("Team deleted.".to_string())
        )))),
        Err(message) => Ok(Redirect::to(uri!(teams_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}

#[post("/<slug>/teams/<team_id>/members", data = "<form>")]
pub async fn add_member(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    team_id: i64,
    mut form: Form<MemberForm<'_>>,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, user.id).ok_or(Status::NotFound)?;
    let photo_url = match save_player_photo(&mut form.photo_file).await {
        Ok(value) => value,
        Err(err) if err.kind() == std::io::ErrorKind::InvalidInput => {
            return Ok(Redirect::to(uri!(teams_page(
                slug = slug,
                error = Some("Invalid player photo. Use PNG/JPEG under 5MB.".to_string()),
                success = Option::<String>::None
            ))))
        }
        Err(_) => return Err(Status::InternalServerError),
    };
    match teams_service::add_member(
        state,
        user.id,
        tournament.id,
        team_id,
        &form.name,
        form.notes.as_deref(),
        form.weight_class.as_deref(),
        form.division_id,
        &form.category_ids.clone().unwrap_or_default(),
        &form.event_ids.clone().unwrap_or_default(),
        photo_url.as_deref(),
    ) {
        Ok(_) => {
            if form.return_to.as_deref() == Some("teams") {
                Ok(Redirect::to(uri!(teams_page(
                    slug = slug,
                    error = Option::<String>::None,
                    success = Some("Player added.".to_string())
                ))))
            } else {
                Ok(Redirect::to(uri!(team_profile(
                    slug = slug,
                    id = team_id,
                    q = Option::<String>::None,
                    sort = Option::<String>::None,
                    dir = Option::<String>::None,
                    error = Option::<String>::None,
                    success = Option::<String>::None
                ))))
            }
        }
        Err(message) => Ok(Redirect::to(uri!(teams_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}

#[post("/<slug>/teams/members/<member_id>/delete", data = "<form>")]
pub fn delete_member(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    member_id: i64,
    form: Form<ReturnToForm>,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, user.id).ok_or(Status::NotFound)?;
    let team_id = teams_service::get_member_team_id(state, user.id, tournament.id, member_id);
    match teams_service::delete_member(state, user.id, tournament.id, member_id) {
        Ok(_) => {
            if form.return_to.as_deref() == Some("teams") {
                Ok(Redirect::to(uri!(teams_page(
                    slug = slug,
                    error = Option::<String>::None,
                    success = Some("Player removed.".to_string())
                ))))
            } else {
                match team_id {
                    Ok(team_id) => Ok(Redirect::to(uri!(team_profile(
                        slug = slug,
                        id = team_id,
                        q = Option::<String>::None,
                        sort = Option::<String>::None,
                        dir = Option::<String>::None,
                        error = Option::<String>::None,
                        success = Option::<String>::None
                    )))),
                    Err(_) => Ok(Redirect::to(uri!(teams_page(
                        slug = slug,
                        error = Option::<String>::None,
                        success = Some("Player removed.".to_string())
                    )))),
                }
            }
        }
        Err(message) => Ok(Redirect::to(uri!(teams_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}

#[post("/<slug>/teams/members/<member_id>/update", data = "<form>")]
pub async fn update_member(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    member_id: i64,
    mut form: Form<UpdateMemberForm<'_>>,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, user.id).ok_or(Status::NotFound)?;
    let team_id = teams_service::get_member_team_id(state, user.id, tournament.id, member_id);
    let should_save_photo = form
        .photo_file
        .as_ref()
        .is_some_and(|upload| upload.len() > 0);
    let photo_url = if should_save_photo {
        match save_player_photo(&mut form.photo_file).await {
            Ok(value) => value,
            Err(err) if err.kind() == std::io::ErrorKind::InvalidInput => {
                return Ok(Redirect::to(uri!(teams_page(
                    slug = slug,
                    error = Some("Invalid player photo. Use PNG/JPEG under 5MB.".to_string()),
                    success = Option::<String>::None
                ))))
            }
            Err(err) => {
                return Ok(Redirect::to(uri!(teams_page(
                    slug = slug,
                    error = Some(format!("Unable to save player photo: {err}")),
                    success = Option::<String>::None
                ))))
            }
        }
    } else {
        None
    };

    let next_category_ids = form.category_ids.clone().map(|mut ids| {
        ids.sort_unstable();
        ids.dedup();
        ids
    });
    let next_event_ids = form.event_ids.clone().map(|mut ids| {
        ids.sort_unstable();
        ids.dedup();
        ids
    });
    match teams_service::update_member(
        state,
        user.id,
        tournament.id,
        member_id,
        form.name.as_deref(),
        form.notes.as_deref(),
        form.weight_class.as_deref(),
        form.division_id,
        next_category_ids,
        next_event_ids,
        form.clear_notes.is_some(),
        form.clear_weight_class.is_some(),
        form.clear_division.is_some(),
        form.clear_categories.is_some(),
        form.clear_events.is_some(),
        photo_url.as_deref(),
        form.clear_photo.is_some(),
    ) {
        Ok(_) => {
            if form.return_to.as_deref() == Some("teams") {
                Ok(Redirect::to(uri!(teams_page(
                    slug = slug,
                    error = Option::<String>::None,
                    success = Some("Player updated.".to_string())
                ))))
            } else {
                match team_id {
                    Ok(team_id) => Ok(Redirect::to(uri!(team_profile(
                        slug = slug,
                        id = team_id,
                        q = Option::<String>::None,
                        sort = Option::<String>::None,
                        dir = Option::<String>::None,
                        error = Option::<String>::None,
                        success = Option::<String>::None
                    )))),
                    Err(_) => Ok(Redirect::to(uri!(teams_page(
                        slug = slug,
                        error = Option::<String>::None,
                        success = Some("Player updated.".to_string())
                    )))),
                }
            }
        }
        Err(message) => Ok(Redirect::to(uri!(teams_page(
            slug = slug,
            error = Some(message),
            success = Option::<String>::None
        )))),
    }
}

#[post("/<slug>/teams/<team_id>/members/bulk-assign", data = "<form>")]
pub fn bulk_assign_members(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    team_id: i64,
    form: Form<BulkAssignMembersForm>,
) -> Result<Redirect, Status> {
    let user = auth_service::current_user(state, jar).ok_or(Status::Unauthorized)?;
    let tournament =
        tournament_service::get_by_slug_for_user(state, &slug, user.id).ok_or(Status::NotFound)?;

    let team = teams_service::get_team(state, user.id, tournament.id, team_id)
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound)?;
    let member_set: HashSet<i64> = team.members.iter().map(|member| member.id).collect();

    let selected_ids: Vec<i64> = form
        .member_ids
        .iter()
        .copied()
        .filter(|id| member_set.contains(id))
        .collect();
    if selected_ids.is_empty() {
        return Ok(Redirect::to(uri!(team_profile(
            slug = slug,
            id = team_id,
            q = Option::<String>::None,
            sort = Option::<String>::None,
            dir = Option::<String>::None,
            error = Some("Select at least one player for bulk update.".to_string()),
            success = Option::<String>::None
        ))));
    }

    let apply_division = form.apply_division.is_some();
    let apply_categories = form.apply_categories.is_some();
    let apply_events = form.apply_events.is_some();
    if !apply_division && !apply_categories && !apply_events {
        return Ok(Redirect::to(uri!(team_profile(
            slug = slug,
            id = team_id,
            q = Option::<String>::None,
            sort = Option::<String>::None,
            dir = Option::<String>::None,
            error = Some("Choose at least one field to apply.".to_string()),
            success = Option::<String>::None
        ))));
    }

    let mut updated_count = 0usize;
    let mut failed_count = 0usize;
    for member_id in selected_ids {
        let division_id = if apply_division { form.division_id } else { None };
        let category_ids = if apply_categories {
            Some(form.category_ids.clone().unwrap_or_default())
        } else {
            None
        };
        let event_ids = if apply_events {
            Some(form.event_ids.clone().unwrap_or_default())
        } else {
            None
        };

        match teams_service::update_member(
            state,
            user.id,
            tournament.id,
            member_id,
            None,
            None,
            None,
            division_id,
            category_ids,
            event_ids,
            false,
            false,
            apply_division && form.division_id.is_none(),
            apply_categories && form.category_ids.clone().unwrap_or_default().is_empty(),
            apply_events && form.event_ids.clone().unwrap_or_default().is_empty(),
            None,
            false,
        ) {
            Ok(_) => updated_count += 1,
            Err(_) => failed_count += 1,
        }
    }

    let success = if failed_count == 0 {
        Some(format!("Updated {} player(s).", updated_count))
    } else {
        Some(format!(
            "Updated {} player(s). {} player(s) failed to update.",
            updated_count, failed_count
        ))
    };

    Ok(Redirect::to(uri!(team_profile(
        slug = slug,
        id = team_id,
        q = Option::<String>::None,
        sort = Option::<String>::None,
        dir = Option::<String>::None,
        error = Option::<String>::None,
        success = success
    ))))
}

fn normalize_header(value: &str) -> String {
    value
        .trim()
        // Handle UTF-8 BOM when the CSV is saved with a BOM (common in Excel).
        .trim_start_matches('\u{feff}')
        .to_lowercase()
        .replace([' ', '-'], "_")
        .replace("__", "_")
}

fn split_multi_value(value: &str) -> Vec<String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Vec::new();
    }
    let separator = if normalized.contains('|') { '|' } else { ',' };
    normalized
        .split(separator)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

fn parse_csv_rows(path: &Path) -> Result<Vec<ParsedImportRow>, String> {
    let bytes = std::fs::read(path).map_err(|_| "Unable to read CSV file.".to_string())?;

    // CSV exported by spreadsheet apps is often semicolon-delimited in some locales.
    let mut delimiter = b',';
    if let Some(line_end) = bytes.iter().position(|b| *b == b'\n') {
        let head = &bytes[..line_end];
        let comma_count = head.iter().filter(|b| **b == b',').count();
        let semi_count = head.iter().filter(|b| **b == b';').count();
        if semi_count > comma_count {
            delimiter = b';';
        }
    }

    // Use byte records so we can tolerate non-UTF8 CSV encodings (decode lossily per-field).
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .flexible(true)
        .from_reader(bytes.as_slice());

    let headers = reader
        .byte_headers()
        .map_err(|err| format!("CSV header row is invalid: {err}"))?
        .iter()
        .map(|value| normalize_header(&String::from_utf8_lossy(value)))
        .collect::<Vec<_>>();
    if headers.is_empty() {
        return Err("CSV header row is required.".to_string());
    }

    let mut rows = Vec::new();
    for (index, record_result) in reader.byte_records().enumerate() {
        let record = record_result.map_err(|err| {
            format!(
                "CSV row is invalid (row {}): {err}",
                // +2 for the 1-based header row + 1-based record index.
                index + 2
            )
        })?;
        let raw = record
            .iter()
            .map(|value| String::from_utf8_lossy(value).trim().to_string())
            .collect::<Vec<_>>();
        if raw.iter().all(|value| value.is_empty()) {
            continue;
        }
        let mut columns = HashMap::new();
        for (position, header) in headers.iter().enumerate() {
            let value = record
                .get(position)
                .map(|value| String::from_utf8_lossy(value).trim().to_string())
                .unwrap_or_default();
            columns.insert(header.clone(), value);
        }
        rows.push(ParsedImportRow {
            row_number: index + 2,
            raw,
            columns,
        });
    }
    Ok(rows)
}

fn parse_xlsx_rows(path: &Path) -> Result<Vec<ParsedImportRow>, String> {
    let mut workbook =
        open_workbook_auto(path).map_err(|_| "Unable to read XLSX file.".to_string())?;
    let sheet_name = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| "XLSX file has no sheets.".to_string())?;
    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|_| "XLSX sheet is invalid.".to_string())?;
    let mut row_iter = range.rows();
    let header_cells = row_iter
        .next()
        .ok_or_else(|| "XLSX header row is required.".to_string())?;
    let headers = header_cells
        .iter()
        .map(|cell: &Data| normalize_header(&cell.to_string()))
        .collect::<Vec<_>>();
    if headers.is_empty() {
        return Err("XLSX header row is required.".to_string());
    }

    let mut rows = Vec::new();
    for (index, row) in row_iter.enumerate() {
        let raw = row
            .iter()
            .map(|cell: &Data| cell.to_string().trim().to_string())
            .collect::<Vec<_>>();
        if raw.iter().all(|value| value.is_empty()) {
            continue;
        }
        let mut columns = HashMap::new();
        for (position, header) in headers.iter().enumerate() {
            let value = raw.get(position).cloned().unwrap_or_default();
            columns.insert(header.clone(), value);
        }
        rows.push(ParsedImportRow {
            row_number: index + 2,
            raw,
            columns,
        });
    }
    Ok(rows)
}

fn parse_name_to_id_list(
    field: &str,
    value: &str,
    lookup: &HashMap<String, i64>,
) -> Result<Vec<i64>, String> {
    let mut ids = Vec::new();
    for item in split_multi_value(value) {
        let key = item.to_lowercase();
        match lookup.get(&key) {
            Some(id) => ids.push(*id),
            None => return Err(format!("Unknown {}: {}", field, item)),
        }
    }
    Ok(ids)
}

fn build_row_text(raw: &[String]) -> String {
    raw.join(" | ")
}

fn normalize_import_key(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_lowercase()
}

fn process_team_import_rows(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    rows: Vec<ParsedImportRow>,
) -> (usize, usize, Vec<ImportFailureView>) {
    let division_lookup: HashMap<String, i64> =
        settings_service::list(state, tournament_id, SettingsEntity::Division)
            .into_iter()
            .map(|item| (item.name.to_lowercase(), item.id))
            .collect();
    let category_lookup: HashMap<String, i64> =
        settings_service::list(state, tournament_id, SettingsEntity::Category)
            .into_iter()
            .map(|item| (item.name.to_lowercase(), item.id))
            .collect();
    let event_lookup: HashMap<String, i64> =
        settings_service::list(state, tournament_id, SettingsEntity::Event)
            .into_iter()
            .map(|item| (item.name.to_lowercase(), item.id))
            .collect();
    let mut existing_team_names: HashSet<String> =
        teams_service::list(state, user_id, tournament_id)
            .unwrap_or_default()
            .into_iter()
            .map(|team| normalize_import_key(&team.name))
            .collect();

    let mut imported = 0usize;
    let mut skipped = 0usize;
    let mut failures = Vec::new();
    for row in rows {
        let team_name = row
            .columns
            .get("name")
            .map(|value| value.trim().to_string())
            .unwrap_or_default();
        if team_name.is_empty() {
            failures.push(ImportFailureView {
                row_number: row.row_number,
                row_data: build_row_text(&row.raw),
                error: "Team name is required.".to_string(),
            });
            continue;
        }
        let lower_name = normalize_import_key(&team_name);
        if existing_team_names.contains(&lower_name) {
            skipped += 1;
            continue;
        }

        let divisions = row.columns.get("divisions").cloned().unwrap_or_default();
        let categories = row.columns.get("categories").cloned().unwrap_or_default();
        let events = row.columns.get("events").cloned().unwrap_or_default();

        let division_ids = match parse_name_to_id_list("division", &divisions, &division_lookup) {
            Ok(value) => value,
            Err(error) => {
                failures.push(ImportFailureView {
                    row_number: row.row_number,
                    row_data: build_row_text(&row.raw),
                    error,
                });
                continue;
            }
        };
        let category_ids = match parse_name_to_id_list("category", &categories, &category_lookup) {
            Ok(value) => value,
            Err(error) => {
                failures.push(ImportFailureView {
                    row_number: row.row_number,
                    row_data: build_row_text(&row.raw),
                    error,
                });
                continue;
            }
        };
        let event_ids = match parse_name_to_id_list("event", &events, &event_lookup) {
            Ok(value) => value,
            Err(error) => {
                failures.push(ImportFailureView {
                    row_number: row.row_number,
                    row_data: build_row_text(&row.raw),
                    error,
                });
                continue;
            }
        };

        match teams_service::create_team(
            state,
            user_id,
            tournament_id,
            &team_name,
            None,
            &division_ids,
            &category_ids,
            &event_ids,
        ) {
            Ok(_) => {
                existing_team_names.insert(lower_name);
                imported += 1;
            }
            Err(error) => failures.push(ImportFailureView {
                row_number: row.row_number,
                row_data: build_row_text(&row.raw),
                error,
            }),
        }
    }
    (imported, skipped, failures)
}

fn process_member_import_rows(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    rows: Vec<ParsedImportRow>,
) -> (usize, usize, Vec<ImportFailureView>) {
    let teams = teams_service::list(state, user_id, tournament_id).unwrap_or_default();
    let team_lookup: HashMap<String, i64> = teams
        .iter()
        .map(|team| (normalize_import_key(&team.name), team.id))
        .collect();
    let mut existing_member_keys: HashSet<String> = teams
        .iter()
        .flat_map(|team| {
            team.members.iter().map(move |member| {
                format!("{}|{}", team.id, normalize_import_key(&member.name))
            })
        })
        .collect();
    let division_lookup: HashMap<String, i64> =
        settings_service::list(state, tournament_id, SettingsEntity::Division)
            .into_iter()
            .map(|item| (item.name.to_lowercase(), item.id))
            .collect();
    let category_lookup: HashMap<String, i64> =
        settings_service::list(state, tournament_id, SettingsEntity::Category)
            .into_iter()
            .map(|item| (item.name.to_lowercase(), item.id))
            .collect();
    let event_lookup: HashMap<String, i64> =
        settings_service::list(state, tournament_id, SettingsEntity::Event)
            .into_iter()
            .map(|item| (item.name.to_lowercase(), item.id))
            .collect();

    let mut imported = 0usize;
    let mut skipped = 0usize;
    let mut failures = Vec::new();
    for row in rows {
        let team_name = row
            .columns
            .get("team_name")
            .map(|value| value.trim().to_string())
            .unwrap_or_default();
        let member_name = row
            .columns
            .get("name")
            .map(|value| value.trim().to_string())
            .unwrap_or_default();
        if team_name.is_empty() || member_name.is_empty() {
            failures.push(ImportFailureView {
                row_number: row.row_number,
                row_data: build_row_text(&row.raw),
                error: "team_name and name are required.".to_string(),
            });
            continue;
        }
        let team_id = match team_lookup.get(&normalize_import_key(&team_name)) {
            Some(id) => *id,
            None => {
                failures.push(ImportFailureView {
                    row_number: row.row_number,
                    row_data: build_row_text(&row.raw),
                    error: format!("Unknown team: {}", team_name),
                });
                continue;
            }
        };
        let member_key = format!("{}|{}", team_id, normalize_import_key(&member_name));
        if existing_member_keys.contains(&member_key) {
            skipped += 1;
            continue;
        }

        let division_id = {
            let value = row.columns.get("division").cloned().unwrap_or_default();
            if value.trim().is_empty() {
                None
            } else {
                match division_lookup.get(&value.to_lowercase()) {
                    Some(id) => Some(*id),
                    None => {
                        failures.push(ImportFailureView {
                            row_number: row.row_number,
                            row_data: build_row_text(&row.raw),
                            error: format!("Unknown division: {}", value),
                        });
                        continue;
                    }
                }
            }
        };
        let category_ids = match parse_name_to_id_list(
            "category",
            &row.columns.get("categories").cloned().unwrap_or_default(),
            &category_lookup,
        ) {
            Ok(value) => value,
            Err(error) => {
                failures.push(ImportFailureView {
                    row_number: row.row_number,
                    row_data: build_row_text(&row.raw),
                    error,
                });
                continue;
            }
        };
        let event_ids = match parse_name_to_id_list(
            "event",
            &row.columns.get("events").cloned().unwrap_or_default(),
            &event_lookup,
        ) {
            Ok(value) => value,
            Err(error) => {
                failures.push(ImportFailureView {
                    row_number: row.row_number,
                    row_data: build_row_text(&row.raw),
                    error,
                });
                continue;
            }
        };

        let weight_class = row.columns.get("weight_class").cloned().unwrap_or_default();
        let notes = row.columns.get("notes").cloned().unwrap_or_default();
        let weight_ref = if weight_class.trim().is_empty() {
            None
        } else {
            Some(weight_class.as_str())
        };
        let notes_ref = if notes.trim().is_empty() {
            None
        } else {
            Some(notes.as_str())
        };
        match teams_service::add_member(
            state,
            user_id,
            tournament_id,
            team_id,
            &member_name,
            notes_ref,
            weight_ref,
            division_id,
            &category_ids,
            &event_ids,
            None,
        ) {
            Ok(_) => {
                existing_member_keys.insert(member_key);
                imported += 1;
            }
            Err(error) => failures.push(ImportFailureView {
                row_number: row.row_number,
                row_data: build_row_text(&row.raw),
                error,
            }),
        }
    }
    (imported, skipped, failures)
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
    let public_path = format!(
        "/static/uploads/{}",
        filepath.file_name().unwrap().to_string_lossy()
    );
    Ok(Some(public_path))
}

async fn save_player_photo(
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
    let raw_filename = format!("player-photo-raw-{}.bin", timestamp);
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

    let filename = format!("player-photo-{}.png", timestamp);
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
