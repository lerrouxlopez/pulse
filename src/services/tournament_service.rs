use crate::db;
use crate::models::Tournament;
use crate::repositories::{tournament_users_repository, tournaments_repository, users_repository};
use crate::services::access_service;
use crate::slug::slugify;
use crate::state::AppState;
use rocket::State;
use mysql::Pool;

pub fn get_by_id(state: &State<AppState>, tournament_id: i64) -> Option<Tournament> {
    let mut conn = db::open_conn(&state.pool).ok()?;
    tournaments_repository::get_by_id(&mut conn, tournament_id).ok()?
}

pub fn get_by_id_for_user(
    state: &State<AppState>,
    tournament_id: i64,
    user_id: i64,
) -> Option<Tournament> {
    let mut conn = db::open_conn(&state.pool).ok()?;
    tournaments_repository::get_by_id_for_user(&mut conn, tournament_id, user_id).ok()?
}

pub fn list_by_user(state: &State<AppState>, user_id: i64) -> Vec<Tournament> {
    let mut conn = match db::open_conn(&state.pool) {
        Ok(conn) => conn,
        Err(_) => return Vec::new(),
    };
    tournaments_repository::list_by_user(&mut conn, user_id).unwrap_or_default()
}

pub fn create(state: &State<AppState>, user_id: i64, name: &str) -> Option<Tournament> {
    let mut conn = db::open_conn(&state.pool).ok()?;
    let base_slug = slugify(name);
    let slug = unique_slug(&mut conn, &base_slug).ok()?;
    let tournament_id = tournaments_repository::create(&mut conn, user_id, name, &slug).ok()?;
    let _ = access_service::ensure_owner_role(state, tournament_id);
    let _ = access_service::assign_owner(state, tournament_id, user_id);
    tournaments_repository::get_by_id(&mut conn, tournament_id).ok()?
}

pub fn mark_setup_complete(state: &State<AppState>, tournament_id: i64) -> bool {
    let mut conn = match db::open_conn(&state.pool) {
        Ok(conn) => conn,
        Err(_) => return false,
    };
    tournaments_repository::set_setup(&mut conn, tournament_id, true).is_ok()
}

pub fn get_by_slug(state: &State<AppState>, slug: &str) -> Option<Tournament> {
    let mut conn = db::open_conn(&state.pool).ok()?;
    tournaments_repository::get_by_slug(&mut conn, slug).ok()?
}

pub fn get_by_slug_for_user(
    state: &State<AppState>,
    slug: &str,
    user_id: i64,
) -> Option<Tournament> {
    let mut conn = db::open_conn(&state.pool).ok()?;
    tournaments_repository::get_by_slug_for_user(&mut conn, slug, user_id).ok()?
}

pub fn list_access_users(state: &State<AppState>, tournament_id: i64) -> Vec<crate::models::UserSummary> {
    let mut conn = match db::open_conn(&state.pool) {
        Ok(conn) => conn,
        Err(_) => return Vec::new(),
    };
    tournaments_repository::list_access_users(&mut conn, tournament_id).unwrap_or_default()
}

pub fn invite_user_by_email(
    state: &State<AppState>,
    inviter_id: i64,
    tournament_id: i64,
    email: &str,
) -> Result<(), String> {
    let trimmed = email.trim().to_lowercase();
    if trimmed.is_empty() {
        return Err("Email is required.".to_string());
    }
    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, inviter_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    let user_id = users_repository::find_user_id_by_email(&mut conn, &trimmed)
        .map_err(|_| "Storage error.".to_string())?
        .ok_or_else(|| "No user found with that email.".to_string())?;
    let already = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if already {
        return Err("User already has access.".to_string());
    }
    tournament_users_repository::add_user(&mut conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    Ok(())
}

pub fn ensure_slugs(pool: &Pool) -> bool {
    let mut conn = match db::open_conn(pool) {
        Ok(conn) => conn,
        Err(_) => return false,
    };
    let missing = match tournaments_repository::list_missing_slugs(&mut conn) {
        Ok(items) => items,
        Err(_) => return false,
    };
    for (tournament_id, name) in missing {
        let base_slug = slugify(&name);
        let slug = match unique_slug(&mut conn, &base_slug) {
            Ok(slug) => slug,
            Err(_) => return false,
        };
        if tournaments_repository::update_slug(&mut conn, tournament_id, &slug).is_err() {
            return false;
        }
    }
    true
}

fn unique_slug(conn: &mut mysql::PooledConn, base_slug: &str) -> mysql::Result<String> {
    let mut slug = base_slug.to_string();
    let mut counter = 2;
    while tournaments_repository::slug_exists(conn, &slug)? {
        slug = format!("{}-{}", base_slug, counter);
        counter += 1;
    }
    Ok(slug)
}
