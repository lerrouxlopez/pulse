use crate::db;
use crate::models::{AccessUser, Role};
use crate::repositories::{
    role_permissions_repository, tournament_roles_repository, tournament_user_roles_repository,
    tournaments_repository, users_repository, tournament_users_repository,
};
use crate::state::AppState;
use mysql::prelude::*;
use rocket::State;

const PERMISSIONS: [&str; 4] = ["dashboard", "events", "teams", "settings"];

pub fn permissions() -> Vec<String> {
    PERMISSIONS.iter().map(|item| item.to_string()).collect()
}

pub fn is_owner(state: &State<AppState>, user_id: i64, tournament_id: i64) -> bool {
    let mut conn = match db::open_conn(&state.pool) {
        Ok(conn) => conn,
        Err(_) => return false,
    };
    let tournament = match tournaments_repository::get_by_id(&mut conn, tournament_id) {
        Ok(Some(tournament)) => tournament,
        _ => return false,
    };
    tournament.user_id == user_id
}

pub fn list_roles(state: &State<AppState>, tournament_id: i64) -> Vec<Role> {
    let mut conn = match db::open_conn(&state.pool) {
        Ok(conn) => conn,
        Err(_) => return Vec::new(),
    };
    let mut roles = tournament_roles_repository::list(&mut conn, tournament_id).unwrap_or_default();
    for role in roles.iter_mut() {
        role.permissions =
            role_permissions_repository::list_by_role(&mut conn, role.id).unwrap_or_default();
    }
    roles
}

pub fn list_access_users(state: &State<AppState>, tournament_id: i64) -> Vec<AccessUser> {
    let mut conn = match db::open_conn(&state.pool) {
        Ok(conn) => conn,
        Err(_) => return Vec::new(),
    };
    let rows: Vec<(i64, String, String, Option<i64>, Option<String>)> = conn
        .exec_map(
            "SELECT u.id, u.name, u.email, tur.role_id, tr.name
             FROM users u
             LEFT JOIN tournament_user_roles tur ON tur.user_id = u.id AND tur.tournament_id = ?1
             LEFT JOIN tournament_roles tr ON tr.id = tur.role_id
             WHERE u.id = (SELECT user_id FROM tournaments WHERE id = ?1)
                OR EXISTS (SELECT 1 FROM tournament_users WHERE tournament_id = ?1 AND user_id = u.id)
             ORDER BY u.name",
            (tournament_id,),
            |(id, name, email, role_id, role_name)| (id, name, email, role_id, role_name),
        )
        .unwrap_or_default();
    rows.into_iter()
        .map(|(id, name, email, role_id, role_name)| AccessUser {
            id,
            name,
            email,
            role_id,
            role_name,
        })
        .collect()
}

pub fn ensure_owner_role(state: &State<AppState>, tournament_id: i64) -> Option<i64> {
    let mut conn = db::open_conn(&state.pool).ok()?;
    if let Some(role_id) = tournament_roles_repository::get_owner_role_id(&mut conn, tournament_id)
        .ok()
        .flatten()
    {
        return Some(role_id);
    }
    let role_id = tournament_roles_repository::create(&mut conn, tournament_id, "Owner", true).ok()?;
    let perms: Vec<String> = permissions();
    let _ = role_permissions_repository::replace_for_role(&mut conn, role_id, &perms);
    Some(role_id)
}

pub fn assign_owner(state: &State<AppState>, tournament_id: i64, user_id: i64) -> bool {
    let mut conn = match db::open_conn(&state.pool) {
        Ok(conn) => conn,
        Err(_) => return false,
    };
    let role_id = match tournament_roles_repository::get_owner_role_id(&mut conn, tournament_id) {
        Ok(Some(role_id)) => role_id,
        _ => return false,
    };
    tournament_user_roles_repository::set_user_role(&mut conn, tournament_id, user_id, role_id)
        .is_ok()
}

pub fn create_role(
    state: &State<AppState>,
    tournament_id: i64,
    name: &str,
) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Role name is required.".to_string());
    }
    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let _ = tournament_roles_repository::create(&mut conn, tournament_id, trimmed, false)
        .map_err(|_| "Storage error.".to_string())?;
    Ok(())
}

pub fn delete_role(
    state: &State<AppState>,
    tournament_id: i64,
    role_id: i64,
) -> Result<(), String> {
    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let changed =
        tournament_roles_repository::delete(&mut conn, tournament_id, role_id).map_err(|_| "Storage error.".to_string())?;
    if changed == 0 {
        return Err("Role not found or cannot delete Owner.".to_string());
    }
    Ok(())
}

pub fn update_role_permissions(
    state: &State<AppState>,
    tournament_id: i64,
    role_id: i64,
    permissions: &[String],
) -> Result<(), String> {
    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let is_owner = conn
        .exec_first::<i64, _, _>(
            "SELECT COUNT(*) FROM tournament_roles WHERE id = ? AND tournament_id = ? AND is_owner = 1",
            (role_id, tournament_id),
        )
        .map_err(|_| "Storage error.".to_string())?
        .unwrap_or(0)
        > 0;
    if is_owner {
        return Err("Owner permissions cannot be changed.".to_string());
    }
    let filtered: Vec<String> = permissions
        .iter()
        .filter(|item| PERMISSIONS.iter().any(|perm| perm.eq_ignore_ascii_case(item)))
        .map(|item| item.to_string())
        .collect();
    role_permissions_repository::replace_for_role(&mut conn, role_id, &filtered)
        .map_err(|_| "Storage error.".to_string())?;
    Ok(())
}

pub fn assign_user_role(
    state: &State<AppState>,
    tournament_id: i64,
    user_id: i64,
    role_id: i64,
) -> Result<(), String> {
    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("User does not have tournament access.".to_string());
    }
    if let Ok(Some(tournament)) = tournaments_repository::get_by_id(&mut conn, tournament_id) {
        if tournament.user_id == user_id {
            let owner_role_id = tournament_roles_repository::get_owner_role_id(&mut conn, tournament_id)
                .map_err(|_| "Storage error.".to_string())?
                .ok_or_else(|| "Owner role missing.".to_string())?;
            if owner_role_id != role_id {
                return Err("Tournament owner must keep the Owner role.".to_string());
            }
        }
    }
    tournament_user_roles_repository::set_user_role(&mut conn, tournament_id, user_id, role_id)
        .map_err(|_| "Storage error.".to_string())?;
    Ok(())
}

pub fn create_user(
    state: &State<AppState>,
    tournament_id: i64,
    name: &str,
    email: &str,
    password_hash: &str,
    role_id: Option<i64>,
) -> Result<(), String> {
    let trimmed_name = name.trim();
    let trimmed_email = email.trim().to_lowercase();
    if trimmed_name.is_empty() || trimmed_email.is_empty() || password_hash.is_empty() {
        return Err("Name, email, and password are required.".to_string());
    }
    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let user_id = users_repository::create_user(&mut conn, trimmed_name, &trimmed_email, password_hash)
        .map_err(|_| "Storage error.".to_string())?;
    conn.exec_drop(
        "INSERT IGNORE INTO tournament_users (tournament_id, user_id) VALUES (?, ?)",
        (tournament_id, user_id),
    )
    .map_err(|_| "Storage error.".to_string())?;
    if let Some(role_id) = role_id {
        let _ = tournament_user_roles_repository::set_user_role(&mut conn, tournament_id, user_id, role_id);
    }
    Ok(())
}

pub fn list_users_not_in_tournament(
    state: &State<AppState>,
    tournament_id: i64,
) -> Vec<AccessUser> {
    let mut conn = match db::open_conn(&state.pool) {
        Ok(conn) => conn,
        Err(_) => return Vec::new(),
    };
    let all_users = users_repository::list_all(&mut conn).unwrap_or_default();
    let existing: Vec<i64> = conn
        .exec_map(
            "SELECT user_id FROM tournament_users WHERE tournament_id = ?",
            (tournament_id,),
            |user_id| user_id,
        )
        .unwrap_or_default();
    let owner_id = tournaments_repository::get_by_id(&mut conn, tournament_id)
        .ok()
        .flatten()
        .map(|t| t.user_id);
    let mut excluded = existing;
    if let Some(owner_id) = owner_id {
        if !excluded.contains(&owner_id) {
            excluded.push(owner_id);
        }
    }
    all_users
        .into_iter()
        .filter(|(id, _, _)| !excluded.contains(id))
        .map(|(id, name, email)| AccessUser {
            id,
            name,
            email,
            role_id: None,
            role_name: None,
        })
        .collect()
}

pub fn add_existing_user(
    state: &State<AppState>,
    tournament_id: i64,
    user_id: i64,
    role_id: Option<i64>,
) -> Result<(), String> {
    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    tournament_users_repository::add_user(&mut conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if let Some(role_id) = role_id {
        tournament_user_roles_repository::set_user_role(&mut conn, tournament_id, user_id, role_id)
            .map_err(|_| "Storage error.".to_string())?;
    }
    Ok(())
}

pub fn update_user(
    state: &State<AppState>,
    tournament_id: i64,
    user_id: i64,
    name: &str,
    email: &str,
) -> Result<(), String> {
    let trimmed_name = name.trim();
    let trimmed_email = email.trim().to_lowercase();
    if trimmed_name.is_empty() || trimmed_email.is_empty() {
        return Err("Name and email are required.".to_string());
    }
    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("User does not have tournament access.".to_string());
    }
    users_repository::update_user(&mut conn, user_id, trimmed_name, &trimmed_email)
        .map_err(|_| "Storage error.".to_string())?;
    Ok(())
}

pub fn remove_user_from_tournament(
    state: &State<AppState>,
    tournament_id: i64,
    user_id: i64,
) -> Result<(), String> {
    let mut conn = db::open_conn(&state.pool).map_err(|_| "Storage error.")?;
    let tournament = tournaments_repository::get_by_id(&mut conn, tournament_id)
        .map_err(|_| "Storage error.".to_string())?
        .ok_or_else(|| "Tournament not found.".to_string())?;
    if tournament.user_id == user_id {
        return Err("Cannot remove the owner.".to_string());
    }
    tournament_user_roles_repository::remove_user(&mut conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    crate::repositories::tournament_users_repository::remove_user(&mut conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    Ok(())
}

pub fn user_permissions(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
) -> Vec<String> {
    if is_owner(state, user_id, tournament_id) {
        return permissions();
    }
    let mut conn = match db::open_conn(&state.pool) {
        Ok(conn) => conn,
        Err(_) => return Vec::new(),
    };
    let role_id = match tournament_user_roles_repository::get_user_role(&mut conn, tournament_id, user_id) {
        Ok(Some(role_id)) => role_id,
        _ => return Vec::new(),
    };
    role_permissions_repository::list_by_role(&mut conn, role_id).unwrap_or_default()
}

pub fn user_has_permission(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    permission: &str,
) -> bool {
    let perms = user_permissions(state, user_id, tournament_id);
    perms.iter().any(|item| item.eq_ignore_ascii_case(permission))
}
