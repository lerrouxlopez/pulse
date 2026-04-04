use crate::services::{auth_service, settings_service, teams_service, tournament_service};
use crate::services::settings_service::SettingsEntity;
use crate::state::AppState;
use rocket::form::{Form, FromForm};
use rocket::fs::TempFile;
use rocket::http::{Cookie, CookieJar, Status};
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};
use image::{imageops::FilterType, GenericImageView};
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

    jar.add(Cookie::new("last_tournament_slug", tournament.slug.clone()));

    let teams = teams_service::list(state, user.id, tournament.id).unwrap_or_default();
    let divisions = settings_service::list(state, tournament.id, SettingsEntity::Division);
    let categories = settings_service::list(state, tournament.id, SettingsEntity::Category);
    let events = settings_service::list(state, tournament.id, SettingsEntity::Event);
    let weight_classes = settings_service::list(state, tournament.id, SettingsEntity::WeightClass);

    Ok(Template::render(
        "teams",
        context! {
            name: user.name,
            tournament_name: tournament.name,
            tournament_slug: tournament.slug,
            teams: teams,
            divisions: divisions,
            categories: categories,
            events: events,
            weight_classes: weight_classes,
            error: error,
            success: success,
            active: "teams",
            is_setup: tournament.is_setup,
        },
    ))
}

#[get("/<slug>/teams/<id>?<q>&<sort>&<dir>")]
pub fn team_profile(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    id: i64,
    q: Option<String>,
    sort: Option<String>,
    dir: Option<String>,
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
    let members_with_division = team.members.iter().filter(|m| m.division_id.is_some()).count();
    let members_with_category = team.members.iter().filter(|m| !m.category_ids.is_empty()).count();
    let members_with_event = team.members.iter().filter(|m| !m.event_ids.is_empty()).count();
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

        let sort_by = sort
            .as_deref()
            .unwrap_or("name")
            .to_lowercase();
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
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, user.id)
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
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, user.id)
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
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, user.id)
        .ok_or(Status::NotFound)?;
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
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, user.id)
        .ok_or(Status::NotFound)?;
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
                    dir = Option::<String>::None
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
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, user.id)
        .ok_or(Status::NotFound)?;
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
                        dir = Option::<String>::None
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
    let tournament = tournament_service::get_by_slug_for_user(state, &slug, user.id)
        .ok_or(Status::NotFound)?;
    let team_id = teams_service::get_member_team_id(state, user.id, tournament.id, member_id);
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
    match teams_service::update_member(
        state,
        user.id,
        tournament.id,
        member_id,
        form.name.as_deref(),
        form.notes.as_deref(),
        form.weight_class.as_deref(),
        form.division_id,
        form.category_ids.clone(),
        form.event_ids.clone(),
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
                        dir = Option::<String>::None
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

async fn save_player_photo(file: &mut Option<TempFile<'_>>) -> Result<Option<String>, std::io::Error> {
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
    if let Some(content_type) = upload.content_type() {
        let is_supported = content_type
            .extension()
            .map(|ext| {
                let ext = ext.as_str();
                ext.eq_ignore_ascii_case("png")
                    || ext.eq_ignore_ascii_case("jpg")
                    || ext.eq_ignore_ascii_case("jpeg")
            })
            .unwrap_or(false);
        if !is_supported {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Unsupported file type",
            ));
        }
    }

    let uploads_dir = Path::new("static").join("uploads");
    std::fs::create_dir_all(&uploads_dir)?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let raw_filename = format!("player-photo-raw-{}.bin", timestamp);
    let raw_path = uploads_dir.join(raw_filename);
    upload.persist_to(&raw_path).await?;

    let image = image::open(&raw_path)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid image"))?;
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
    let public_path = format!("/static/uploads/{}", filepath.file_name().unwrap().to_string_lossy());
    Ok(Some(public_path))
}
