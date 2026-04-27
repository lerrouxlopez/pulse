use crate::db;
use crate::repositories::admin_repository;
use crate::repositories::{
    categories_repository, divisions_repository, events_repository, matches_repository,
    scheduled_event_winners_repository, scheduled_events_repository, teams_repository,
    weight_classes_repository,
};
use crate::repositories::tournaments_repository;
use crate::repositories::users_repository;
use crate::services::auth_service;
use crate::slug::slugify;
use crate::state::AppState;
use mysql::prelude::Queryable;
use rocket::form::Form;
use rocket::http::{Cookie, CookieJar, SameSite, Status};
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};
use serde::Serialize;
use std::env;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const ADMIN_SESSION_COOKIE: &str = "admin_session";

#[derive(Debug, Clone, Serialize)]
struct AdminDashboardCharts {
    labels: Vec<String>,
    teams: Vec<u64>,
    members: Vec<u64>,
    scheduled_events: Vec<u64>,
    matches: Vec<u64>,
}

fn admin_credentials() -> (String, String) {
    let email = env::var("ADMIN_EMAIL").unwrap_or_else(|_| "foundation.nirvana@gmail.com".to_string());
    let password = env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "Angelus69@@@".to_string());
    (email.trim().to_string(), password)
}

fn is_admin(jar: &CookieJar<'_>) -> bool {
    jar.get_private(ADMIN_SESSION_COOKIE)
        .and_then(|c| c.value().parse::<u64>().ok())
        .is_some()
}

fn require_admin(jar: &CookieJar<'_>) -> Result<(), Redirect> {
    if is_admin(jar) {
        Ok(())
    } else {
        Err(Redirect::to(uri!(admin_login_page(
            error = Option::<String>::None
        ))))
    }
}

#[derive(FromForm)]
pub struct AdminLoginForm {
    pub email: String,
    pub password: String,
}

#[get("/admin", rank = 0)]
pub fn admin_root(jar: &CookieJar<'_>) -> Redirect {
    if is_admin(jar) {
        Redirect::to(uri!(admin_dashboard))
    } else {
        Redirect::to(uri!(admin_login_page(
            error = Option::<String>::None
        )))
    }
}

// Rocket doesn't allow a literal "/admin/" route, so we use a catch-all to handle "/admin/"
// (and any other unknown admin path) and redirect to the proper admin entrypoint.
#[get("/admin/<_path..>", rank = 20)]
pub fn admin_root_catchall(jar: &CookieJar<'_>, _path: std::path::PathBuf) -> Redirect {
    admin_root(jar)
}

#[get("/admin/dashboard", rank = 0)]
pub fn admin_dashboard(state: &State<AppState>, jar: &CookieJar<'_>) -> Result<Template, Redirect> {
    require_admin(jar)?;

    let mut conn = match db::open_conn(&state.pool) {
        Ok(conn) => conn,
        Err(err) => {
            let empty_charts = serde_json::to_string(&AdminDashboardCharts {
                labels: Vec::new(),
                teams: Vec::new(),
                members: Vec::new(),
                scheduled_events: Vec::new(),
                matches: Vec::new(),
            })
            .unwrap_or_else(|_| "{}".to_string());
            return Ok(Template::render(
                "admin_dashboard",
                context! {
                    active: "admin_dashboard",
                    error: format!("Storage error: {err}"),
                    counts: Option::<admin_repository::SystemCounts>::None,
                    summaries: Vec::<admin_repository::TournamentSummary>::new(),
                    tournament_charts_json: empty_charts,
                },
            ));
        }
    };

    let counts = admin_repository::system_counts(&mut conn).ok();
    let summaries = admin_repository::tournament_summaries(&mut conn).unwrap_or_default();

    // Charts: prefer chronological order for readability (oldest -> newest).
    let mut ordered = summaries.clone();
    ordered.reverse();
    let charts = AdminDashboardCharts {
        labels: ordered
            .iter()
            .map(|t| format!("#{} {}", t.id, t.name))
            .collect(),
        teams: ordered.iter().map(|t| t.teams).collect(),
        members: ordered.iter().map(|t| t.members).collect(),
        scheduled_events: ordered.iter().map(|t| t.scheduled_events).collect(),
        matches: ordered.iter().map(|t| t.matches).collect(),
    };
    let tournament_charts_json =
        serde_json::to_string(&charts).unwrap_or_else(|_| "{}".to_string());

    Ok(Template::render(
        "admin_dashboard",
        context! {
            active: "admin_dashboard",
            error: Option::<String>::None,
            counts: counts,
            summaries: summaries,
            tournament_charts_json: tournament_charts_json,
        },
    ))
}

#[get("/admin/login?<error>", rank = 0)]
pub fn admin_login_page(error: Option<String>) -> Template {
    Template::render(
        "admin_login",
        context! {
            error: error,
        },
    )
}

#[post("/admin/login", data = "<form>", rank = 0)]
pub fn admin_login(jar: &CookieJar<'_>, form: Form<AdminLoginForm>) -> Result<Redirect, Status> {
    let submitted = form.into_inner();
    let (admin_email, admin_password) = admin_credentials();

    if submitted.email.trim().eq_ignore_ascii_case(admin_email.trim())
        && submitted.password == admin_password
    {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
        let mut cookie = Cookie::new(ADMIN_SESSION_COOKIE, now.to_string());
        cookie.set_same_site(SameSite::Lax);
        cookie.set_http_only(true);
        jar.add_private(cookie);
        return Ok(Redirect::to(uri!(admin_dashboard)));
    }

    Ok(Redirect::to(uri!(admin_login_page(
        error = Some("Invalid admin credentials.".to_string())
    ))))
}

#[post("/admin/logout", rank = 0)]
pub fn admin_logout(jar: &CookieJar<'_>) -> Redirect {
    jar.remove_private(Cookie::from(ADMIN_SESSION_COOKIE));
    Redirect::to(uri!(admin_login_page(
        error = Option::<String>::None
    )))
}

#[derive(FromForm)]
pub struct AdminUserUpsertForm {
    pub name: String,
    pub email: String,
    pub password: Option<String>,
    pub user_type: String,
    pub tournament_id: Option<i64>,
}

#[derive(FromForm)]
pub struct AdminTournamentUpsertForm {
    pub name: String,
    pub slug: Option<String>,
    pub owner_user_id: i64,
}

#[derive(FromForm)]
pub struct AdminTeamUpsertForm {
    pub tournament_id: i64,
    pub name: String,
}

#[derive(FromForm)]
pub struct AdminTeamRenameForm {
    pub name: String,
}

#[derive(FromForm)]
pub struct AdminMemberUpsertForm {
    pub name: String,
    pub notes: Option<String>,
    pub weight_class: Option<String>,
}

#[derive(FromForm)]
pub struct AdminScheduledEventUpsertForm {
    pub tournament_id: i64,
    pub event_id: i64,
    pub contact_type: String,
    pub status: String,
    pub location: Option<String>,
    pub event_time: Option<String>,
    pub point_system: Option<String>,
    pub time_rule: Option<String>,
    pub draw_system: Option<String>,
    pub division_id: Option<i64>,
    pub weight_class_id: Option<i64>,
}

#[derive(FromForm)]
pub struct AdminSettingCreateForm {
    pub tournament_id: i64,
    pub name: String,
}

#[derive(FromForm)]
pub struct AdminSettingUpdateForm {
    pub tournament_id: i64,
    pub name: String,
}

#[derive(FromForm)]
pub struct AdminSettingDeleteForm {
    pub tournament_id: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminTournamentRowView {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub is_setup: bool,
    pub user_id: i64,
    pub owner_label: Option<String>,
}

fn normalize_user_type(raw: &str) -> Option<&'static str> {
    let value = raw.trim().to_lowercase();
    if value == "system" {
        Some("system")
    } else if value == "tournament" {
        Some("tournament")
    } else {
        None
    }
}

#[get("/admin/users?<error>&<success>", rank = 0)]
pub fn admin_users_page(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    error: Option<String>,
    success: Option<String>,
) -> Result<Template, Redirect> {
    require_admin(jar)?;
    let mut conn = db::open_conn(&state.pool).map_err(|_| Redirect::to(uri!(admin_dashboard)))?;
    let users = users_repository::list_all_users(&mut conn).unwrap_or_default();
    let tournaments = tournaments_repository::list_all(&mut conn).unwrap_or_default();
    Ok(Template::render(
        "admin_users",
        context! {
            active: "admin_users",
            users: users,
            tournaments: tournaments,
            error: error,
            success: success,
        },
    ))
}

#[post("/admin/users/create", data = "<form>", rank = 0)]
pub fn admin_users_create(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    form: Form<AdminUserUpsertForm>,
) -> Result<Redirect, Status> {
    require_admin(jar).map_err(|_| Status::Unauthorized)?;

    let input = form.into_inner();
    let user_type = normalize_user_type(&input.user_type)
        .ok_or(Status::BadRequest)?;

    let name = input.name.trim();
    let email = input.email.trim().to_lowercase();
    let password = input.password.unwrap_or_default();
    if name.is_empty() || email.is_empty() || password.trim().len() < 6 {
        return Ok(Redirect::to(uri!(admin_users_page(
            error = Some("Name, email, and password (6+ chars) are required.".to_string()),
            success = Option::<String>::None
        ))));
    }

    let mut conn = db::open_conn(&state.pool).map_err(|_| Status::InternalServerError)?;
    let password_hash = auth_service::hash_password(password.trim())
        .map_err(|_| Status::InternalServerError)?;

    let tournament_id = if user_type == "system" {
        0
    } else {
        input.tournament_id.unwrap_or(0)
    };
    if user_type == "tournament" && tournament_id <= 0 {
        return Ok(Redirect::to(uri!(admin_users_page(
            error = Some("Tournament is required for tournament users.".to_string()),
            success = Option::<String>::None
        ))));
    }
    if user_type == "tournament" {
        let exists: Option<i64> = conn
            .exec_first("SELECT id FROM tournaments WHERE id = ? LIMIT 1", (tournament_id,))
            .map_err(|_| Status::InternalServerError)?;
        if exists.is_none() {
            return Ok(Redirect::to(uri!(admin_users_page(
                error = Some("Tournament not found.".to_string()),
                success = Option::<String>::None
            ))));
        }
    }

    let result = if user_type == "system" {
        users_repository::create_system_user(&mut conn, name, &email, &password_hash)
    } else {
        users_repository::create_tournament_user(&mut conn, tournament_id, name, &email, &password_hash, None)
    };

    match result {
        Ok(_) => Ok(Redirect::to(uri!(admin_users_page(
            error = Option::<String>::None,
            success = Some("User created.".to_string())
        )))),
        Err(mysql::Error::MySqlError(ref err)) if err.code == 1062 => Ok(Redirect::to(uri!(admin_users_page(
            error = Some("Email already exists for that tournament.".to_string()),
            success = Option::<String>::None
        )))),
        Err(err) => Ok(Redirect::to(uri!(admin_users_page(
            error = Some(format!("Storage error: {err}")),
            success = Option::<String>::None
        )))),
    }
}

#[post("/admin/users/<id>/update", data = "<form>", rank = 0)]
pub fn admin_users_update(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    id: i64,
    form: Form<AdminUserUpsertForm>,
) -> Result<Redirect, Status> {
    require_admin(jar).map_err(|_| Status::Unauthorized)?;

    let input = form.into_inner();
    let user_type = normalize_user_type(&input.user_type)
        .ok_or(Status::BadRequest)?;
    let name = input.name.trim();
    let email = input.email.trim().to_lowercase();
    if name.is_empty() || email.is_empty() {
        return Ok(Redirect::to(uri!(admin_users_page(
            error = Some("Name and email are required.".to_string()),
            success = Option::<String>::None
        ))));
    }

    let mut conn = db::open_conn(&state.pool).map_err(|_| Status::InternalServerError)?;

    let tournament_id = if user_type == "system" {
        0
    } else {
        input.tournament_id.unwrap_or(0)
    };
    if user_type == "tournament" && tournament_id <= 0 {
        return Ok(Redirect::to(uri!(admin_users_page(
            error = Some("Tournament is required for tournament users.".to_string()),
            success = Option::<String>::None
        ))));
    }
    if user_type == "tournament" {
        let exists: Option<i64> = conn
            .exec_first("SELECT id FROM tournaments WHERE id = ? LIMIT 1", (tournament_id,))
            .map_err(|_| Status::InternalServerError)?;
        if exists.is_none() {
            return Ok(Redirect::to(uri!(admin_users_page(
                error = Some("Tournament not found.".to_string()),
                success = Option::<String>::None
            ))));
        }
    }

    // Prevent deleting/retyping an owner away from system if they own tournaments.
    if user_type != "system" {
        let owned: Option<u64> = conn
            .exec_first("SELECT COUNT(*) FROM tournaments WHERE user_id = ?", (id,))
            .map_err(|_| Status::InternalServerError)?;
        if owned.unwrap_or(0) > 0 {
            return Ok(Redirect::to(uri!(admin_users_page(
                error = Some("User owns tournament(s). Reassign ownership first.".to_string()),
                success = Option::<String>::None
            ))));
        }
    }

    let changed = users_repository::admin_update_user_fields(&mut conn, id, name, &email, user_type, tournament_id);
    match changed {
        Ok(0) => Ok(Redirect::to(uri!(admin_users_page(
            error = Some("User not found.".to_string()),
            success = Option::<String>::None
        )))),
        Ok(_) => {
            if let Some(password) = input.password.as_deref().map(|p| p.trim()).filter(|p| !p.is_empty()) {
                if password.len() < 6 {
                    return Ok(Redirect::to(uri!(admin_users_page(
                        error = Some("Password must be 6+ characters.".to_string()),
                        success = Option::<String>::None
                    ))));
                }
                let password_hash = auth_service::hash_password(password).map_err(|_| Status::InternalServerError)?;
                let _ = users_repository::update_password_hash(&mut conn, id, &password_hash);
            }
            Ok(Redirect::to(uri!(admin_users_page(
                error = Option::<String>::None,
                success = Some("User updated.".to_string())
            ))))
        }
        Err(mysql::Error::MySqlError(ref err)) if err.code == 1062 => Ok(Redirect::to(uri!(admin_users_page(
            error = Some("Email already exists for that tournament.".to_string()),
            success = Option::<String>::None
        )))),
        Err(err) => Ok(Redirect::to(uri!(admin_users_page(
            error = Some(format!("Storage error: {err}")),
            success = Option::<String>::None
        )))),
    }
}

#[post("/admin/users/<id>/delete", rank = 0)]
pub fn admin_users_delete(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    id: i64,
) -> Result<Redirect, Status> {
    require_admin(jar).map_err(|_| Status::Unauthorized)?;
    let mut conn = db::open_conn(&state.pool).map_err(|_| Status::InternalServerError)?;

    let owned: Option<u64> = conn
        .exec_first("SELECT COUNT(*) FROM tournaments WHERE user_id = ?", (id,))
        .map_err(|_| Status::InternalServerError)?;
    if owned.unwrap_or(0) > 0 {
        return Ok(Redirect::to(uri!(admin_users_page(
            error = Some("User owns tournament(s). Reassign ownership first.".to_string()),
            success = Option::<String>::None
        ))));
    }

    match users_repository::admin_delete_user(&mut conn, id) {
        Ok(0) => Ok(Redirect::to(uri!(admin_users_page(
            error = Some("User not found.".to_string()),
            success = Option::<String>::None
        )))),
        Ok(_) => Ok(Redirect::to(uri!(admin_users_page(
            error = Option::<String>::None,
            success = Some("User deleted.".to_string())
        )))),
        Err(err) => Ok(Redirect::to(uri!(admin_users_page(
            error = Some(format!("Storage error: {err}")),
            success = Option::<String>::None
        )))),
    }
}

#[get("/admin/tournaments?<error>&<success>", rank = 0)]
pub fn admin_tournaments_page(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    error: Option<String>,
    success: Option<String>,
) -> Result<Template, Redirect> {
    require_admin(jar)?;
    let mut conn = db::open_conn(&state.pool).map_err(|_| Redirect::to(uri!(admin_dashboard)))?;

    let tournaments = tournaments_repository::list_all(&mut conn).unwrap_or_default();
    let users = users_repository::list_all_users(&mut conn).unwrap_or_default();
    let owners: Vec<users_repository::AdminUserRow> = users
        .iter()
        .filter(|u| u.user_type.eq_ignore_ascii_case("system"))
        .cloned()
        .collect();

    let mut owner_label_by_id: std::collections::HashMap<i64, String> = std::collections::HashMap::new();
    for u in users.iter().filter(|u| u.user_type.eq_ignore_ascii_case("system")) {
        owner_label_by_id.insert(u.id, format!("{} ({})", u.email, u.name));
    }

    let tournament_rows: Vec<AdminTournamentRowView> = tournaments
        .iter()
        .map(|t| AdminTournamentRowView {
            id: t.id,
            name: t.name.clone(),
            slug: t.slug.clone(),
            is_setup: t.is_setup,
            user_id: t.user_id,
            owner_label: owner_label_by_id.get(&t.user_id).cloned(),
        })
        .collect();

    Ok(Template::render(
        "admin_tournaments",
        context! {
            active: "admin_tournaments",
            tournaments: tournament_rows,
            owners: owners,
            error: error,
            success: success,
        },
    ))
}

fn unique_slug(conn: &mut mysql::PooledConn, base: &str) -> Result<String, mysql::Error> {
    let mut slug = base.to_string();
    let mut counter = 2;
    while tournaments_repository::slug_exists(conn, &slug)? {
        slug = format!("{}-{}", base, counter);
        counter += 1;
    }
    Ok(slug)
}

#[post("/admin/tournaments/create", data = "<form>", rank = 0)]
pub fn admin_tournaments_create(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    form: Form<AdminTournamentUpsertForm>,
) -> Result<Redirect, Status> {
    require_admin(jar).map_err(|_| Status::Unauthorized)?;
    let input = form.into_inner();
    let name = input.name.trim();
    if name.is_empty() {
        return Ok(Redirect::to(uri!(admin_tournaments_page(
            error = Some("Tournament name is required.".to_string()),
            success = Option::<String>::None
        ))));
    }

    let mut conn = db::open_conn(&state.pool).map_err(|_| Status::InternalServerError)?;
    let owner_exists: Option<i64> = conn
        .exec_first(
            "SELECT id FROM users WHERE id = ? AND user_type = 'system' LIMIT 1",
            (input.owner_user_id,),
        )
        .map_err(|_| Status::InternalServerError)?;
    if owner_exists.is_none() {
        return Ok(Redirect::to(uri!(admin_tournaments_page(
            error = Some("Owner user not found (must be a system user).".to_string()),
            success = Option::<String>::None
        ))));
    }

    let requested_slug = input.slug.unwrap_or_default();
    let base = if requested_slug.trim().is_empty() {
        slugify(name)
    } else {
        slugify(requested_slug.trim())
    };
    let slug = unique_slug(&mut conn, &base).map_err(|_| Status::InternalServerError)?;
    match tournaments_repository::create(&mut conn, input.owner_user_id, name, &slug) {
        Ok(_) => Ok(Redirect::to(uri!(admin_tournaments_page(
            error = Option::<String>::None,
            success = Some("Tournament created.".to_string())
        )))),
        Err(err) => Ok(Redirect::to(uri!(admin_tournaments_page(
            error = Some(format!("Storage error: {err}")),
            success = Option::<String>::None
        )))),
    }
}

#[post("/admin/tournaments/<id>/update", data = "<form>", rank = 0)]
pub fn admin_tournaments_update(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    id: i64,
    form: Form<AdminTournamentUpsertForm>,
) -> Result<Redirect, Status> {
    require_admin(jar).map_err(|_| Status::Unauthorized)?;
    let input = form.into_inner();
    let name = input.name.trim();
    let slug_in = input.slug.unwrap_or_default();
    let slug_trimmed = slug_in.trim();
    if name.is_empty() || slug_trimmed.is_empty() {
        return Ok(Redirect::to(uri!(admin_tournaments_page(
            error = Some("Name and slug are required.".to_string()),
            success = Option::<String>::None
        ))));
    }

    let mut conn = db::open_conn(&state.pool).map_err(|_| Status::InternalServerError)?;
    let existing = tournaments_repository::get_by_id(&mut conn, id)
        .map_err(|_| Status::InternalServerError)?;
    let Some(existing) = existing else {
        return Ok(Redirect::to(uri!(admin_tournaments_page(
            error = Some("Tournament not found.".to_string()),
            success = Option::<String>::None
        ))));
    };

    let owner_exists: Option<i64> = conn
        .exec_first(
            "SELECT id FROM users WHERE id = ? AND user_type = 'system' LIMIT 1",
            (input.owner_user_id,),
        )
        .map_err(|_| Status::InternalServerError)?;
    if owner_exists.is_none() {
        return Ok(Redirect::to(uri!(admin_tournaments_page(
            error = Some("Owner user not found (must be a system user).".to_string()),
            success = Option::<String>::None
        ))));
    }

    let next_slug = slugify(slug_trimmed);
    if next_slug.is_empty() {
        return Ok(Redirect::to(uri!(admin_tournaments_page(
            error = Some("Invalid slug.".to_string()),
            success = Option::<String>::None
        ))));
    }
    let unique = if next_slug == existing.slug {
        next_slug
    } else {
        unique_slug(&mut conn, &next_slug).map_err(|_| Status::InternalServerError)?
    };

    if name != existing.name {
        let _ = tournaments_repository::update_name(&mut conn, id, name);
    }
    if input.owner_user_id != existing.user_id {
        let _ = tournaments_repository::update_owner(&mut conn, id, input.owner_user_id);
    }
    if unique != existing.slug {
        let _ = tournaments_repository::create_slug_alias(&mut conn, id, &existing.slug);
        let _ = tournaments_repository::update_slug(&mut conn, id, &unique);
    }

    Ok(Redirect::to(uri!(admin_tournaments_page(
        error = Option::<String>::None,
        success = Some("Tournament updated.".to_string())
    ))))
}

#[post("/admin/tournaments/<id>/delete", rank = 0)]
pub fn admin_tournaments_delete(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    id: i64,
) -> Result<Redirect, Status> {
    require_admin(jar).map_err(|_| Status::Unauthorized)?;
    let mut conn = db::open_conn(&state.pool).map_err(|_| Status::InternalServerError)?;
    match admin_repository::delete_tournament_cascade(&mut conn, id) {
        Ok(_) => Ok(Redirect::to(uri!(admin_tournaments_page(
            error = Option::<String>::None,
            success = Some("Tournament deleted.".to_string())
        )))),
        Err(err) => Ok(Redirect::to(uri!(admin_tournaments_page(
            error = Some(format!("Storage error: {err}")),
            success = Option::<String>::None
        )))),
    }
}

#[get("/admin/teams?<error>&<success>", rank = 0)]
pub fn admin_teams_page(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    error: Option<String>,
    success: Option<String>,
) -> Result<Template, Redirect> {
    require_admin(jar)?;
    let mut conn = db::open_conn(&state.pool).map_err(|_| Redirect::to(uri!(admin_dashboard)))?;

    let teams = admin_repository::list_all_teams(&mut conn).unwrap_or_default();
    let tournaments = tournaments_repository::list_all(&mut conn).unwrap_or_default();

    Ok(Template::render(
        "admin_teams",
        context! {
            active: "admin_teams",
            teams: teams,
            tournaments: tournaments,
            error: error,
            success: success,
        },
    ))
}

#[post("/admin/teams/create", data = "<form>", rank = 0)]
pub fn admin_teams_create(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    form: Form<AdminTeamUpsertForm>,
) -> Result<Redirect, Status> {
    require_admin(jar).map_err(|_| Status::Unauthorized)?;
    let input = form.into_inner();
    let name = input.name.trim();
    if name.is_empty() {
        return Ok(Redirect::to(uri!(admin_teams_page(
            error = Some("Team name is required.".to_string()),
            success = Option::<String>::None
        ))));
    }
    let mut conn = db::open_conn(&state.pool).map_err(|_| Status::InternalServerError)?;
    let exists: Option<i64> = conn
        .exec_first("SELECT id FROM tournaments WHERE id = ? LIMIT 1", (input.tournament_id,))
        .map_err(|_| Status::InternalServerError)?;
    if exists.is_none() {
        return Ok(Redirect::to(uri!(admin_teams_page(
            error = Some("Tournament not found.".to_string()),
            success = Option::<String>::None
        ))));
    }
    conn.exec_drop(
        "INSERT INTO teams (tournament_id, name, logo_url) VALUES (?, ?, NULL)",
        (input.tournament_id, name),
    )
    .map_err(|_| Status::InternalServerError)?;
    Ok(Redirect::to(uri!(admin_teams_page(
        error = Option::<String>::None,
        success = Some("Team created.".to_string())
    ))))
}

#[post(
    "/admin/teams/<tournament_id>/<team_id>/update",
    data = "<form>",
    rank = 0
)]
pub fn admin_teams_update(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    tournament_id: i64,
    team_id: i64,
    form: Form<AdminTeamRenameForm>,
) -> Result<Redirect, Status> {
    require_admin(jar).map_err(|_| Status::Unauthorized)?;
    let name = form.name.trim();
    if name.is_empty() {
        return Ok(Redirect::to(uri!(admin_teams_page(
            error = Some("Team name is required.".to_string()),
            success = Option::<String>::None
        ))));
    }
    let mut conn = db::open_conn(&state.pool).map_err(|_| Status::InternalServerError)?;
    conn.exec_drop(
        "UPDATE teams SET name = ? WHERE id = ? AND tournament_id = ?",
        (name, team_id, tournament_id),
    )
    .map_err(|_| Status::InternalServerError)?;
    Ok(Redirect::to(uri!(admin_teams_page(
        error = Option::<String>::None,
        success = Some("Team updated.".to_string())
    ))))
}

#[post("/admin/teams/<tournament_id>/<team_id>/delete", rank = 0)]
pub fn admin_teams_delete(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    tournament_id: i64,
    team_id: i64,
) -> Result<Redirect, Status> {
    require_admin(jar).map_err(|_| Status::Unauthorized)?;
    let mut conn = db::open_conn(&state.pool).map_err(|_| Status::InternalServerError)?;
    match admin_repository::delete_team_cascade(&mut conn, tournament_id, team_id) {
        Ok(_) => Ok(Redirect::to(uri!(admin_teams_page(
            error = Option::<String>::None,
            success = Some("Team deleted.".to_string())
        )))),
        Err(err) => Ok(Redirect::to(uri!(admin_teams_page(
            error = Some(format!("Storage error: {err}")),
            success = Option::<String>::None
        )))),
    }
}

#[get("/admin/teams/<tournament_id>/<team_id>?<error>&<success>", rank = 0)]
pub fn admin_team_detail(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    tournament_id: i64,
    team_id: i64,
    error: Option<String>,
    success: Option<String>,
) -> Result<Template, Redirect> {
    require_admin(jar)?;
    let mut conn = db::open_conn(&state.pool).map_err(|_| Redirect::to(uri!(admin_dashboard)))?;
    let team: Option<(String, Option<String>, String, String)> = conn
        .exec_first(
            "SELECT tm.name, tm.logo_url, t.name, COALESCE(t.slug, '')
             FROM teams tm
             JOIN tournaments t ON t.id = tm.tournament_id
             WHERE tm.tournament_id = ? AND tm.id = ?",
            (tournament_id, team_id),
        )
        .ok()
        .flatten();
    let Some((team_name, _logo_url, tournament_name, tournament_slug)) = team else {
        return Err(Redirect::to(uri!(admin_teams_page(
            error = Some("Team not found.".to_string()),
            success = Option::<String>::None
        ))));
    };
    let team_view = admin_repository::AdminTeamRow {
        id: team_id,
        tournament_id,
        tournament_name,
        tournament_slug,
        name: team_name,
        logo_url: None,
        members: 0,
    };
    let members = admin_repository::list_team_members(&mut conn, tournament_id, team_id).unwrap_or_default();
    Ok(Template::render(
        "admin_team_detail",
        context! {
            active: "admin_teams",
            team: team_view,
            members: members,
            error: error,
            success: success,
        },
    ))
}

#[post(
    "/admin/teams/<tournament_id>/<team_id>/members/create",
    data = "<form>",
    rank = 0
)]
pub fn admin_team_members_create(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    tournament_id: i64,
    team_id: i64,
    form: Form<AdminMemberUpsertForm>,
) -> Result<Redirect, Status> {
    require_admin(jar).map_err(|_| Status::Unauthorized)?;
    let input = form.into_inner();
    let name = input.name.trim();
    if name.is_empty() {
        return Ok(Redirect::to(uri!(admin_team_detail(
            tournament_id = tournament_id,
            team_id = team_id,
            error = Some("Member name is required.".to_string()),
            success = Option::<String>::None
        ))));
    }
    let mut conn = db::open_conn(&state.pool).map_err(|_| Status::InternalServerError)?;
    conn.exec_drop(
        "INSERT INTO team_members (tournament_id, team_id, name, notes, weight_class, weight_class_id, division_id, photo_url)
         VALUES (?, ?, ?, ?, ?, NULL, NULL, NULL)",
        (
            tournament_id,
            team_id,
            name,
            input.notes.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()),
            input.weight_class.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()),
        ),
    )
    .map_err(|_| Status::InternalServerError)?;
    Ok(Redirect::to(uri!(admin_team_detail(
        tournament_id = tournament_id,
        team_id = team_id,
        error = Option::<String>::None,
        success = Some("Member added.".to_string())
    ))))
}

#[post(
    "/admin/teams/<tournament_id>/<team_id>/members/<member_id>/update",
    data = "<form>",
    rank = 0
)]
pub fn admin_team_members_update(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    tournament_id: i64,
    team_id: i64,
    member_id: i64,
    form: Form<AdminMemberUpsertForm>,
) -> Result<Redirect, Status> {
    require_admin(jar).map_err(|_| Status::Unauthorized)?;
    let input = form.into_inner();
    let name = input.name.trim();
    if name.is_empty() {
        return Ok(Redirect::to(uri!(admin_team_detail(
            tournament_id = tournament_id,
            team_id = team_id,
            error = Some("Member name is required.".to_string()),
            success = Option::<String>::None
        ))));
    }
    let mut conn = db::open_conn(&state.pool).map_err(|_| Status::InternalServerError)?;
    conn.exec_drop(
        "UPDATE team_members
         SET name = ?, notes = ?, weight_class = ?
         WHERE id = ? AND team_id = ? AND tournament_id = ?",
        (
            name,
            input.notes.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()),
            input.weight_class.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()),
            member_id,
            team_id,
            tournament_id,
        ),
    )
    .map_err(|_| Status::InternalServerError)?;
    Ok(Redirect::to(uri!(admin_team_detail(
        tournament_id = tournament_id,
        team_id = team_id,
        error = Option::<String>::None,
        success = Some("Member updated.".to_string())
    ))))
}

#[post("/admin/teams/<tournament_id>/<team_id>/members/<member_id>/delete", rank = 0)]
pub fn admin_team_members_delete(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    tournament_id: i64,
    team_id: i64,
    member_id: i64,
) -> Result<Redirect, Status> {
    require_admin(jar).map_err(|_| Status::Unauthorized)?;
    let mut conn = db::open_conn(&state.pool).map_err(|_| Status::InternalServerError)?;
    let _ = conn.exec_drop(
        "DELETE FROM team_member_categories WHERE tournament_id = ? AND member_id = ?",
        (tournament_id, member_id),
    );
    let _ = conn.exec_drop(
        "DELETE FROM team_member_events WHERE tournament_id = ? AND member_id = ?",
        (tournament_id, member_id),
    );
    conn.exec_drop(
        "DELETE FROM team_members WHERE tournament_id = ? AND team_id = ? AND id = ?",
        (tournament_id, team_id, member_id),
    )
    .map_err(|_| Status::InternalServerError)?;
    Ok(Redirect::to(uri!(admin_team_detail(
        tournament_id = tournament_id,
        team_id = team_id,
        error = Option::<String>::None,
        success = Some("Member deleted.".to_string())
    ))))
}

fn resolve_selected_tournament_id(
    tournaments: &[crate::models::Tournament],
    requested: Option<i64>,
) -> Option<i64> {
    if tournaments.is_empty() {
        return None;
    }
    if let Some(id) = requested.filter(|value| *value > 0) {
        if tournaments.iter().any(|t| t.id == id) {
            return Some(id);
        }
    }
    Some(tournaments[0].id)
}

fn normalize_opt(value: &Option<String>) -> Option<&str> {
    value
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
}

fn canonicalize_scheduled_event_input(
    is_contact: bool,
    input: &AdminScheduledEventUpsertForm,
) -> Result<
    (
        i64,
        &'static str,
        &'static str,
        Option<&str>,
        Option<&str>,
        Option<&str>,
        Option<&str>,
        Option<&str>,
        Option<i64>,
        Option<i64>,
    ),
    String,
> {
    let contact_type_trim = input.contact_type.trim();
    let contact_type = if contact_type_trim.eq_ignore_ascii_case("Contact") {
        "Contact"
    } else if contact_type_trim.eq_ignore_ascii_case("Non-Contact")
        || contact_type_trim.eq_ignore_ascii_case("Non Contact")
    {
        "Non-Contact"
    } else {
        return Err("Invalid contact type.".to_string());
    };

    let status_trim = input.status.trim();
    let status = if status_trim.eq_ignore_ascii_case("Scheduled") {
        "Scheduled"
    } else if status_trim.eq_ignore_ascii_case("Ongoing") {
        "Ongoing"
    } else if status_trim.eq_ignore_ascii_case("Finished") {
        "Finished"
    } else if status_trim.eq_ignore_ascii_case("Cancelled") {
        "Cancelled"
    } else {
        return Err("Invalid status.".to_string());
    };

    let location = normalize_opt(&input.location);
    let event_time = normalize_opt(&input.event_time);

    if is_contact {
        let point_system = normalize_opt(&input.point_system).ok_or_else(|| "Point system is required.".to_string())?;
        let time_rule = normalize_opt(&input.time_rule).ok_or_else(|| "Time rule is required.".to_string())?;
        if time_rule.eq_ignore_ascii_case(crate::services::scheduled_events_service::NO_TIME_LIMIT_RULE) {
            return Err("No time limit is only supported for Non-Contact events.".to_string());
        }
        let draw_system = normalize_opt(&input.draw_system).ok_or_else(|| "Draw system is required.".to_string())?;
        let division_id = input.division_id.filter(|v| *v > 0).ok_or_else(|| "Division is required.".to_string())?;
        let weight_class_id = input.weight_class_id.filter(|v| *v > 0).ok_or_else(|| "Weight class is required.".to_string())?;
        Ok((
            input.event_id,
            contact_type,
            status,
            location,
            event_time,
            Some(point_system),
            Some(time_rule),
            Some(draw_system),
            Some(division_id),
            Some(weight_class_id),
        ))
    } else {
        // Non-contact performances always use the simple 5-10 scale, with either:
        // - a configurable 1-2 minute timer, or
        // - an unlimited timer ("No time limit").
        let rule = normalize_opt(&input.time_rule).unwrap_or("1 round | 2 minutes");
        let canonical_rule = if rule.eq_ignore_ascii_case(crate::services::scheduled_events_service::NO_TIME_LIMIT_RULE)
        {
            crate::services::scheduled_events_service::NO_TIME_LIMIT_RULE
        } else if rule.eq_ignore_ascii_case("1 round | 1 minute") {
            "1 round | 1 minute"
        } else {
            "1 round | 2 minutes"
        };
        let division_id = input
            .division_id
            .filter(|v| *v > 0)
            .ok_or_else(|| "Division is required.".to_string())?;
        let weight_class_id = input
            .weight_class_id
            .filter(|v| *v > 0)
            .ok_or_else(|| "Weight class is required.".to_string())?;
        Ok((
            input.event_id,
            "Non-Contact",
            status,
            location,
            event_time,
            Some("5-10 points"),
            Some(canonical_rule),
            None,
            Some(division_id),
            Some(weight_class_id),
        ))
    }
}

#[get("/admin/events?<tournament_id>&<error>&<success>", rank = 0)]
pub fn admin_events_page(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    tournament_id: Option<i64>,
    error: Option<String>,
    success: Option<String>,
) -> Result<Template, Redirect> {
    require_admin(jar)?;
    let mut conn = db::open_conn(&state.pool).map_err(|_| Redirect::to(uri!(admin_dashboard)))?;
    let tournaments = tournaments_repository::list_all(&mut conn).unwrap_or_default();
    let selected_id = resolve_selected_tournament_id(&tournaments, tournament_id);

    let (scheduled_events, events, divisions, weight_classes) = if let Some(tid) = selected_id {
        (
            scheduled_events_repository::list(&mut conn, tid).unwrap_or_default(),
            events_repository::list(&mut conn, tid).unwrap_or_default(),
            divisions_repository::list(&mut conn, tid).unwrap_or_default(),
            weight_classes_repository::list(&mut conn, tid).unwrap_or_default(),
        )
    } else {
        (Vec::new(), Vec::new(), Vec::new(), Vec::new())
    };

    Ok(Template::render(
        "admin_events",
        context! {
            active: "admin_events",
            tournaments: tournaments,
            selected_tournament_id: selected_id,
            scheduled_events: scheduled_events,
            events: events,
            divisions: divisions,
            weight_classes: weight_classes,
            error: error,
            success: success,
        },
    ))
}

#[post("/admin/events/create", data = "<form>", rank = 0)]
pub fn admin_events_create(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    form: Form<AdminScheduledEventUpsertForm>,
) -> Result<Redirect, Status> {
    require_admin(jar).map_err(|_| Status::Unauthorized)?;
    let input = form.into_inner();
    if input.tournament_id <= 0 {
        return Ok(Redirect::to(uri!(admin_events_page(
            tournament_id = Option::<i64>::None,
            error = Some("Tournament is required.".to_string()),
            success = Option::<String>::None
        ))));
    }

    let mut conn = db::open_conn(&state.pool).map_err(|_| Status::InternalServerError)?;
    let tournaments = tournaments_repository::list_all(&mut conn).unwrap_or_default();
    if !tournaments.iter().any(|t| t.id == input.tournament_id) {
        return Ok(Redirect::to(uri!(admin_events_page(
            tournament_id = Option::<i64>::None,
            error = Some("Tournament not found.".to_string()),
            success = Option::<String>::None
        ))));
    }

    let is_contact = input.contact_type.trim().eq_ignore_ascii_case("Contact");
    let (event_id, contact_type, status, location, event_time, point_system, time_rule, draw_system, division_id, weight_class_id) =
        match canonicalize_scheduled_event_input(is_contact, &input) {
            Ok(value) => value,
            Err(message) => {
                return Ok(Redirect::to(uri!(admin_events_page(
                    tournament_id = Some(input.tournament_id),
                    error = Some(message),
                    success = Option::<String>::None
                ))))
            }
        };

    let event_ids = events_repository::list(&mut conn, input.tournament_id)
        .map_err(|_| Status::InternalServerError)?
        .into_iter()
        .map(|e| e.id)
        .collect::<Vec<_>>();
    if !event_ids.contains(&event_id) {
        return Ok(Redirect::to(uri!(admin_events_page(
            tournament_id = Some(input.tournament_id),
            error = Some("Event is not included in this tournament.".to_string()),
            success = Option::<String>::None
        ))));
    }

    let division_id = match division_id {
        Some(value) => value,
        None => {
            return Ok(Redirect::to(uri!(admin_events_page(
                tournament_id = Some(input.tournament_id),
                error = Some("Division is required.".to_string()),
                success = Option::<String>::None
            ))));
        }
    };
    let weight_class_id = match weight_class_id {
        Some(value) => value,
        None => {
            return Ok(Redirect::to(uri!(admin_events_page(
                tournament_id = Some(input.tournament_id),
                error = Some("Weight class is required.".to_string()),
                success = Option::<String>::None
            ))));
        }
    };
    if divisions_repository::get_by_id(&mut conn, input.tournament_id, division_id)
        .unwrap_or(None)
        .is_none()
    {
        return Ok(Redirect::to(uri!(admin_events_page(
            tournament_id = Some(input.tournament_id),
            error = Some("Division not found.".to_string()),
            success = Option::<String>::None
        ))));
    }
    if weight_classes_repository::get_by_id(&mut conn, input.tournament_id, weight_class_id)
        .unwrap_or(None)
        .is_none()
    {
        return Ok(Redirect::to(uri!(admin_events_page(
            tournament_id = Some(input.tournament_id),
            error = Some("Weight class not found.".to_string()),
            success = Option::<String>::None
        ))));
    }

    // Prevent duplicates following the same rules as the tournament UI.
    let existing = scheduled_events_repository::list(&mut conn, input.tournament_id).unwrap_or_default();
    let dupe_division = Some(division_id);
    let dupe_weight = Some(weight_class_id);
    if existing.iter().any(|item| {
        item.event_id == event_id
            && item.contact_type.eq_ignore_ascii_case(contact_type)
            && item.division_id == dupe_division
            && item.weight_class_id == dupe_weight
    }) {
        return Ok(Redirect::to(uri!(admin_events_page(
            tournament_id = Some(input.tournament_id),
            error = Some("Event is already scheduled for this tournament.".to_string()),
            success = Option::<String>::None
        ))));
    }

    match scheduled_events_repository::create(
        &mut conn,
        input.tournament_id,
        event_id,
        contact_type,
        status,
        location,
        event_time,
        point_system,
        time_rule,
        draw_system,
        Some(division_id),
        Some(weight_class_id),
    ) {
        Ok(_) => Ok(Redirect::to(uri!(admin_events_page(
            tournament_id = Some(input.tournament_id),
            error = Option::<String>::None,
            success = Some("Scheduled event created.".to_string())
        )))),
        Err(err) => Ok(Redirect::to(uri!(admin_events_page(
            tournament_id = Some(input.tournament_id),
            error = Some(format!("Storage error: {err}")),
            success = Option::<String>::None
        )))),
    }
}

#[post("/admin/events/<tournament_id>/<id>/update", data = "<form>", rank = 0)]
pub fn admin_events_update(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    tournament_id: i64,
    id: i64,
    form: Form<AdminScheduledEventUpsertForm>,
) -> Result<Redirect, Status> {
    require_admin(jar).map_err(|_| Status::Unauthorized)?;
    if tournament_id <= 0 || id <= 0 {
        return Err(Status::BadRequest);
    }

    let input = form.into_inner();
    let mut conn = db::open_conn(&state.pool).map_err(|_| Status::InternalServerError)?;
    let existing_row = scheduled_events_repository::get_by_id(&mut conn, tournament_id, id)
        .map_err(|_| Status::InternalServerError)?;
    if existing_row.is_none() {
        return Ok(Redirect::to(uri!(admin_events_page(
            tournament_id = Some(tournament_id),
            error = Some("Scheduled event not found.".to_string()),
            success = Option::<String>::None
        ))));
    }

    let is_contact = input.contact_type.trim().eq_ignore_ascii_case("Contact");
    let (event_id, contact_type, status, location, event_time, point_system, time_rule, draw_system, division_id, weight_class_id) =
        match canonicalize_scheduled_event_input(is_contact, &input) {
            Ok(value) => value,
            Err(message) => {
                return Ok(Redirect::to(uri!(admin_events_page(
                    tournament_id = Some(tournament_id),
                    error = Some(message),
                    success = Option::<String>::None
                ))))
            }
        };

    let event_ids = events_repository::list(&mut conn, tournament_id)
        .map_err(|_| Status::InternalServerError)?
        .into_iter()
        .map(|e| e.id)
        .collect::<Vec<_>>();
    if !event_ids.contains(&event_id) {
        return Ok(Redirect::to(uri!(admin_events_page(
            tournament_id = Some(tournament_id),
            error = Some("Event is not included in this tournament.".to_string()),
            success = Option::<String>::None
        ))));
    }

    let division_id = match division_id {
        Some(value) => value,
        None => {
            return Ok(Redirect::to(uri!(admin_events_page(
                tournament_id = Some(tournament_id),
                error = Some("Division is required.".to_string()),
                success = Option::<String>::None
            ))));
        }
    };
    let weight_class_id = match weight_class_id {
        Some(value) => value,
        None => {
            return Ok(Redirect::to(uri!(admin_events_page(
                tournament_id = Some(tournament_id),
                error = Some("Weight class is required.".to_string()),
                success = Option::<String>::None
            ))));
        }
    };
    if divisions_repository::get_by_id(&mut conn, tournament_id, division_id)
        .unwrap_or(None)
        .is_none()
    {
        return Ok(Redirect::to(uri!(admin_events_page(
            tournament_id = Some(tournament_id),
            error = Some("Division not found.".to_string()),
            success = Option::<String>::None
        ))));
    }
    if weight_classes_repository::get_by_id(&mut conn, tournament_id, weight_class_id)
        .unwrap_or(None)
        .is_none()
    {
        return Ok(Redirect::to(uri!(admin_events_page(
            tournament_id = Some(tournament_id),
            error = Some("Weight class not found.".to_string()),
            success = Option::<String>::None
        ))));
    }

    // Prevent duplicates (excluding the row being updated).
    let existing = scheduled_events_repository::list(&mut conn, tournament_id).unwrap_or_default();
    let dupe_division = Some(division_id);
    let dupe_weight = Some(weight_class_id);
    if existing.iter().any(|item| {
        item.id != id
            && item.event_id == event_id
            && item.contact_type.eq_ignore_ascii_case(contact_type)
            && item.division_id == dupe_division
            && item.weight_class_id == dupe_weight
    }) {
        return Ok(Redirect::to(uri!(admin_events_page(
            tournament_id = Some(tournament_id),
            error = Some("Event is already scheduled for this tournament.".to_string()),
            success = Option::<String>::None
        ))));
    }

    match scheduled_events_repository::update(
        &mut conn,
        tournament_id,
        id,
        event_id,
        contact_type,
        status,
        location,
        event_time,
        point_system,
        time_rule,
        draw_system,
        Some(division_id),
        Some(weight_class_id),
    ) {
        Ok(_) => Ok(Redirect::to(uri!(admin_events_page(
            tournament_id = Some(tournament_id),
            error = Option::<String>::None,
            success = Some("Scheduled event updated.".to_string())
        )))),
        Err(err) => Ok(Redirect::to(uri!(admin_events_page(
            tournament_id = Some(tournament_id),
            error = Some(format!("Storage error: {err}")),
            success = Option::<String>::None
        )))),
    }
}

#[post("/admin/events/<tournament_id>/<id>/delete", rank = 0)]
pub fn admin_events_delete(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    tournament_id: i64,
    id: i64,
) -> Result<Redirect, Status> {
    require_admin(jar).map_err(|_| Status::Unauthorized)?;
    let mut conn = db::open_conn(&state.pool).map_err(|_| Status::InternalServerError)?;
    let _ = conn.exec_drop(
        "DELETE FROM scheduled_event_winners WHERE tournament_id = ? AND scheduled_event_id = ?",
        (tournament_id, id),
    );
    let _ = conn.exec_drop(
        "DELETE FROM scheduled_event_judges WHERE tournament_id = ? AND scheduled_event_id = ?",
        (tournament_id, id),
    );
    let _ = conn.exec_drop(
        "DELETE FROM matches WHERE tournament_id = ? AND scheduled_event_id = ?",
        (tournament_id, id),
    );
    match scheduled_events_repository::delete(&mut conn, tournament_id, id) {
        Ok(_) => Ok(Redirect::to(uri!(admin_events_page(
            tournament_id = Some(tournament_id),
            error = Option::<String>::None,
            success = Some("Scheduled event deleted.".to_string())
        )))),
        Err(err) => Ok(Redirect::to(uri!(admin_events_page(
            tournament_id = Some(tournament_id),
            error = Some(format!("Storage error: {err}")),
            success = Option::<String>::None
        )))),
    }
}

#[get("/admin/settings?<tournament_id>&<error>&<success>", rank = 0)]
pub fn admin_settings_page(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    tournament_id: Option<i64>,
    error: Option<String>,
    success: Option<String>,
) -> Result<Template, Redirect> {
    require_admin(jar)?;
    let mut conn = db::open_conn(&state.pool).map_err(|_| Redirect::to(uri!(admin_dashboard)))?;
    let tournaments = tournaments_repository::list_all(&mut conn).unwrap_or_default();
    let selected_id = resolve_selected_tournament_id(&tournaments, tournament_id);

    let (divisions, categories, weight_classes, events) = if let Some(tid) = selected_id {
        (
            divisions_repository::list(&mut conn, tid).unwrap_or_default(),
            categories_repository::list(&mut conn, tid).unwrap_or_default(),
            weight_classes_repository::list(&mut conn, tid).unwrap_or_default(),
            events_repository::list(&mut conn, tid).unwrap_or_default(),
        )
    } else {
        (Vec::new(), Vec::new(), Vec::new(), Vec::new())
    };

    Ok(Template::render(
        "admin_settings",
        context! {
            active: "admin_settings",
            tournaments: tournaments,
            selected_tournament_id: selected_id,
            divisions: divisions,
            categories: categories,
            weight_classes: weight_classes,
            events: events,
            error: error,
            success: success,
        },
    ))
}

fn normalize_settings_entity(raw: &str) -> Option<&'static str> {
    let value = raw.trim().to_lowercase();
    match value.as_str() {
        "division" | "divisions" => Some("divisions"),
        "category" | "categories" => Some("categories"),
        "weight" | "weights" | "weight_class" | "weight_classes" => Some("weight_classes"),
        "event" | "events" => Some("events"),
        _ => None,
    }
}

#[post("/admin/settings/<entity>/create", data = "<form>", rank = 0)]
pub fn admin_settings_create(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    entity: String,
    form: Form<AdminSettingCreateForm>,
) -> Result<Redirect, Status> {
    require_admin(jar).map_err(|_| Status::Unauthorized)?;
    let kind = normalize_settings_entity(&entity).ok_or(Status::BadRequest)?;
    let input = form.into_inner();
    let name = input.name.trim();
    if input.tournament_id <= 0 || name.is_empty() {
        return Ok(Redirect::to(uri!(admin_settings_page(
            tournament_id = Some(input.tournament_id),
            error = Some("Tournament and name are required.".to_string()),
            success = Option::<String>::None
        ))));
    }
    let mut conn = db::open_conn(&state.pool).map_err(|_| Status::InternalServerError)?;
    let result = match kind {
        "divisions" => divisions_repository::create(&mut conn, input.tournament_id, name).map(|_| ()),
        "categories" => categories_repository::create(&mut conn, input.tournament_id, name).map(|_| ()),
        "weight_classes" => weight_classes_repository::create(&mut conn, input.tournament_id, name).map(|_| ()),
        "events" => events_repository::create(&mut conn, input.tournament_id, name).map(|_| ()),
        _ => unreachable!(),
    };
    match result {
        Ok(_) => Ok(Redirect::to(uri!(admin_settings_page(
            tournament_id = Some(input.tournament_id),
            error = Option::<String>::None,
            success = Some("Setting created.".to_string())
        )))),
        Err(err) => Ok(Redirect::to(uri!(admin_settings_page(
            tournament_id = Some(input.tournament_id),
            error = Some(format!("Storage error: {err}")),
            success = Option::<String>::None
        )))),
    }
}

#[post("/admin/settings/<entity>/<id>/update", data = "<form>", rank = 0)]
pub fn admin_settings_update(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    entity: String,
    id: i64,
    form: Form<AdminSettingUpdateForm>,
) -> Result<Redirect, Status> {
    require_admin(jar).map_err(|_| Status::Unauthorized)?;
    let kind = normalize_settings_entity(&entity).ok_or(Status::BadRequest)?;
    let input = form.into_inner();
    let name = input.name.trim();
    if input.tournament_id <= 0 || id <= 0 || name.is_empty() {
        return Ok(Redirect::to(uri!(admin_settings_page(
            tournament_id = Some(input.tournament_id),
            error = Some("Tournament and name are required.".to_string()),
            success = Option::<String>::None
        ))));
    }
    let mut conn = db::open_conn(&state.pool).map_err(|_| Status::InternalServerError)?;
    let result = match kind {
        "divisions" => divisions_repository::update(&mut conn, input.tournament_id, id, name).map(|_| ()),
        "categories" => categories_repository::update(&mut conn, input.tournament_id, id, name).map(|_| ()),
        "weight_classes" => weight_classes_repository::update(&mut conn, input.tournament_id, id, name).map(|_| ()),
        "events" => events_repository::update(&mut conn, input.tournament_id, id, name).map(|_| ()),
        _ => unreachable!(),
    };
    match result {
        Ok(_) => Ok(Redirect::to(uri!(admin_settings_page(
            tournament_id = Some(input.tournament_id),
            error = Option::<String>::None,
            success = Some("Setting updated.".to_string())
        )))),
        Err(err) => Ok(Redirect::to(uri!(admin_settings_page(
            tournament_id = Some(input.tournament_id),
            error = Some(format!("Storage error: {err}")),
            success = Option::<String>::None
        )))),
    }
}

#[post("/admin/settings/<entity>/<id>/delete", data = "<form>", rank = 0)]
pub fn admin_settings_delete(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    entity: String,
    id: i64,
    form: Form<AdminSettingDeleteForm>,
) -> Result<Redirect, Status> {
    require_admin(jar).map_err(|_| Status::Unauthorized)?;
    let kind = normalize_settings_entity(&entity).ok_or(Status::BadRequest)?;
    let input = form.into_inner();
    if input.tournament_id <= 0 || id <= 0 {
        return Err(Status::BadRequest);
    }
    let mut conn = db::open_conn(&state.pool).map_err(|_| Status::InternalServerError)?;
    let result = match kind {
        "divisions" => divisions_repository::delete(&mut conn, input.tournament_id, id).map(|_| ()),
        "categories" => categories_repository::delete(&mut conn, input.tournament_id, id).map(|_| ()),
        "weight_classes" => weight_classes_repository::delete(&mut conn, input.tournament_id, id).map(|_| ()),
        "events" => events_repository::delete(&mut conn, input.tournament_id, id).map(|_| ()),
        _ => unreachable!(),
    };
    match result {
        Ok(_) => Ok(Redirect::to(uri!(admin_settings_page(
            tournament_id = Some(input.tournament_id),
            error = Option::<String>::None,
            success = Some("Setting deleted.".to_string())
        )))),
        Err(err) => Ok(Redirect::to(uri!(admin_settings_page(
            tournament_id = Some(input.tournament_id),
            error = Some(format!("Storage error: {err}")),
            success = Option::<String>::None
        )))),
    }
}

#[derive(Serialize)]
struct AdminEventWinnerRow {
    event_name: String,
    division_name: Option<String>,
    weight_class_label: Option<String>,
    winner_name: String,
    winner_team: Option<String>,
}

#[derive(Serialize, Clone)]
struct AdminMatchResultRow {
    match_id: i64,
    event_name: String,
    label: String,
    status: String,
    winner: Option<String>,
    red_total_score: i32,
    blue_total_score: i32,
}

#[derive(Serialize, Clone)]
struct AdminTeamChampionRow {
    team_id: i64,
    team_name: String,
    wins: i64,
}

#[get("/admin/results?<tournament_id>&<error>", rank = 0)]
pub fn admin_results_page(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    tournament_id: Option<i64>,
    error: Option<String>,
) -> Result<Template, Redirect> {
    require_admin(jar)?;
    let mut conn = db::open_conn(&state.pool).map_err(|_| Redirect::to(uri!(admin_dashboard)))?;
    let tournaments = tournaments_repository::list_all(&mut conn).unwrap_or_default();
    let selected_id = resolve_selected_tournament_id(&tournaments, tournament_id);

    let (event_winners, champion_teams, team_leaderboard, match_results, err) = if let Some(tid) = selected_id {
        let mut outcomes = scheduled_events_repository::list(&mut conn, tid).unwrap_or_default();
        outcomes.retain(|item| {
            item.status.eq_ignore_ascii_case("Finished")
                && item
                    .winner_name
                    .as_ref()
                    .map(|name| !name.trim().is_empty())
                    .unwrap_or(false)
        });

        let mut team_name_by_id: std::collections::HashMap<i64, String> =
            std::collections::HashMap::new();
        let mut member_team_by_id: std::collections::HashMap<i64, i64> =
            std::collections::HashMap::new();

        let teams = teams_repository::list_teams(&mut conn, tid).unwrap_or_default();
        for team in teams {
            team_name_by_id.insert(team.id, team.name);
        }
        let members = teams_repository::list_members(&mut conn, tid).unwrap_or_default();
        for member in members {
            member_team_by_id.insert(member.id, member.team_id);
        }

        let mut winners_by_event_id: std::collections::HashMap<i64, Vec<i64>> =
            std::collections::HashMap::new();
        if let Ok(rows) = scheduled_event_winners_repository::list_all_winners_for_tournament(&mut conn, tid) {
            for (scheduled_event_id, winner_member_id) in rows {
                winners_by_event_id
                    .entry(scheduled_event_id)
                    .or_default()
                    .push(winner_member_id);
            }
        }

        let mut wins_by_team: std::collections::HashMap<i64, i64> = std::collections::HashMap::new();
        for outcome in &outcomes {
            let winner_member_ids: Vec<i64> = if let Some(list) = winners_by_event_id.get(&outcome.id) {
                list.clone()
            } else if let Some(member_id) = outcome.winner_member_id {
                vec![member_id]
            } else {
                Vec::new()
            };
            for member_id in winner_member_ids {
                let Some(team_id) = member_team_by_id.get(&member_id).copied() else {
                    continue;
                };
                *wins_by_team.entry(team_id).or_insert(0) += 1;
            }
        }

        let mut leaderboard: Vec<AdminTeamChampionRow> = wins_by_team
            .iter()
            .filter_map(|(team_id, wins)| {
                let name = team_name_by_id.get(team_id)?.clone();
                Some(AdminTeamChampionRow {
                    team_id: *team_id,
                    team_name: name,
                    wins: *wins,
                })
            })
            .collect();
        leaderboard.sort_by(|a, b| {
            b.wins
                .cmp(&a.wins)
                .then_with(|| a.team_name.cmp(&b.team_name))
        });
        let top_wins = leaderboard.first().map(|row| row.wins).unwrap_or(0);
        let champs: Vec<AdminTeamChampionRow> = if top_wins > 0 {
            leaderboard
                .iter()
                .cloned()
                .filter(|row| row.wins == top_wins)
                .collect()
        } else {
            Vec::new()
        };

        let winners = outcomes
            .iter()
            .map(|item| {
                let winner_team = item
                    .winner_member_id
                    .and_then(|member_id| member_team_by_id.get(&member_id).copied())
                    .and_then(|team_id| team_name_by_id.get(&team_id).cloned());
                AdminEventWinnerRow {
                    event_name: item.event_name.clone(),
                    division_name: item.division_name.clone(),
                    weight_class_label: item
                        .weight_class_label
                        .clone()
                        .or(item.weight_class_name.clone()),
                    winner_name: item.winner_name.clone().unwrap_or_default(),
                    winner_team,
                }
            })
            .collect::<Vec<_>>();

        let scheduled_events = scheduled_events_repository::list(&mut conn, tid).unwrap_or_default();
        let mut event_name_by_scheduled_id: std::collections::HashMap<i64, String> = std::collections::HashMap::new();
        for item in scheduled_events {
            event_name_by_scheduled_id.insert(item.id, item.event_name);
        }

        let mut matches = matches_repository::list_by_tournament(&mut conn, tid).unwrap_or_default();
        matches.retain(|m| {
            m.status.eq_ignore_ascii_case("Finished")
                || m.status.eq_ignore_ascii_case("Forfeit")
                || m.winner_side.is_some()
        });

        let match_rows = matches
            .into_iter()
            .filter_map(|m| {
                let winner = match m.winner_side.as_deref().map(|s| s.to_lowercase()) {
                    Some(side) if side == "red" => m.red.clone().filter(|v| !v.trim().is_empty()),
                    Some(side) if side == "blue" => m.blue.clone().filter(|v| !v.trim().is_empty()),
                    _ => None,
                };
                let red = m.red.clone().unwrap_or_else(|| "TBD".to_string());
                let blue = if m.is_bye {
                    "BYE".to_string()
                } else {
                    m.blue.clone().unwrap_or_else(|| "TBD".to_string())
                };
                let label = format!("{} vs {}", red, blue);
                let event_name = event_name_by_scheduled_id
                    .get(&m.scheduled_event_id)
                    .cloned()
                    .unwrap_or_else(|| "Event".to_string());

                Some(AdminMatchResultRow {
                    match_id: m.id,
                    event_name,
                    label,
                    status: m.status,
                    winner,
                    red_total_score: m.red_total_score,
                    blue_total_score: m.blue_total_score,
                })
            })
            .collect::<Vec<_>>();

        (winners, champs, leaderboard, match_rows, Option::<String>::None)
    } else {
        (
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Some("No tournaments available.".to_string()),
        )
    };

    Ok(Template::render(
        "admin_results",
        context! {
            active: "admin_results",
            tournaments: tournaments,
            selected_tournament_id: selected_id,
            event_winners: event_winners,
            champion_teams: champion_teams,
            team_leaderboard: team_leaderboard,
            match_results: match_results,
            error: error.or(err),
        },
    ))
}
