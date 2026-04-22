use crate::db;
use crate::models::{AccessUser, Role};
use crate::repositories::{
    role_permissions_repository, tournament_roles_repository, tournament_user_roles_repository,
    tournaments_repository, users_repository,
};
use crate::services::auth_service;
use crate::state::AppState;
use mysql::prelude::*;
use rocket::State;
use serde::Serialize;

struct NavPageDefinition {
    label: &'static str,
    active_key: &'static str,
    path: &'static str,
    required_any_permissions: &'static [&'static str],
    visible_before_setup: bool,
}

const NAVIGATION_DEFINITIONS: [NavPageDefinition; 7] = [
    NavPageDefinition {
        label: "Dashboard",
        active_key: "dashboard",
        path: "dashboard",
        required_any_permissions: &["dashboard"],
        visible_before_setup: false,
    },
    NavPageDefinition {
        label: "Results",
        active_key: "results",
        path: "results",
        required_any_permissions: &["dashboard"],
        visible_before_setup: false,
    },
    NavPageDefinition {
        label: "Events",
        active_key: "events",
        path: "events",
        required_any_permissions: &["events"],
        visible_before_setup: false,
    },
    NavPageDefinition {
        label: "Matches",
        active_key: "matches",
        path: "matches",
        required_any_permissions: &["events", "match_timer"],
        visible_before_setup: false,
    },
    NavPageDefinition {
        label: "Scores",
        active_key: "scores",
        path: "scores",
        required_any_permissions: &["scores", "events"],
        visible_before_setup: false,
    },
    NavPageDefinition {
        label: "Teams",
        active_key: "teams",
        path: "teams",
        required_any_permissions: &["teams"],
        visible_before_setup: false,
    },
    NavPageDefinition {
        label: "Settings",
        active_key: "settings",
        path: "settings",
        required_any_permissions: &["settings"],
        visible_before_setup: true,
    },
];

#[derive(Serialize, Clone)]
pub struct SidebarNavItem {
    pub label: String,
    pub active_key: String,
    pub href: String,
}

pub fn permissions() -> Vec<String> {
    let mut items: Vec<String> = Vec::new();
    for page in NAVIGATION_DEFINITIONS {
        for permission in page.required_any_permissions {
            if !items
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(permission))
            {
                items.push((*permission).to_string());
            }
        }
    }
    items
}

pub fn sidebar_nav_items(
    allowed_permissions: &[String],
    is_setup: bool,
    tournament_slug: Option<&str>,
) -> Vec<SidebarNavItem> {
    let mut items: Vec<SidebarNavItem> = Vec::new();
    for page in NAVIGATION_DEFINITIONS {
        if !is_setup && !page.visible_before_setup {
            continue;
        }
        let has_access = page.required_any_permissions.iter().any(|permission| {
            allowed_permissions
                .iter()
                .any(|item| item.eq_ignore_ascii_case(permission))
        });
        if !has_access {
            continue;
        }
        let href = if let Some(slug) = tournament_slug {
            format!("/{}/{}", slug, page.path)
        } else {
            "/dashboard".to_string()
        };
        items.push(SidebarNavItem {
            label: page.label.to_string(),
            active_key: page.active_key.to_string(),
            href,
        });
    }
    items
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
    let owner_id = conn
        .exec_first::<i64, _, _>(
            "SELECT user_id FROM tournaments WHERE id = ?",
            (tournament_id,),
        )
        .unwrap_or(None);
    let mut users: Vec<(i64, String, String)> = conn
        .exec_map(
            "SELECT id, name, email FROM users WHERE tournament_id = ? ORDER BY name",
            (tournament_id,),
            |(id, name, email)| (id, name, email),
        )
        .unwrap_or_default();
    if let Some(owner_id) = owner_id {
        let owner: Option<(i64, String, String)> = conn
            .exec_first(
                "SELECT id, name, email FROM users WHERE id = ?",
                (owner_id,),
            )
            .unwrap_or(None);
        if let Some(owner) = owner {
            if !users.iter().any(|(id, _, _)| *id == owner.0) {
                users.push(owner);
            }
        }
    }
    users.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
    users
        .into_iter()
        .map(|(id, name, email)| {
            let role: Option<(i64, String)> = conn
                .exec_first(
                    "SELECT tr.id, tr.name
                     FROM tournament_user_roles tur
                     JOIN tournament_roles tr ON tr.id = tur.role_id
                     WHERE tur.tournament_id = ? AND tur.user_id = ?
                     LIMIT 1",
                    (tournament_id, id),
                )
                .unwrap_or(None);
            let (role_id, role_name) = role
                .map(|(role_id, role_name)| (Some(role_id), Some(role_name)))
                .unwrap_or((None, None));
            let photo_url = conn
                .exec_first::<Option<String>, _, _>(
                    "SELECT photo_url FROM users WHERE id = ?",
                    (id,),
                )
                .unwrap_or(None)
                .flatten();
            AccessUser {
                id,
                name,
                email,
                role_id,
                role_name,
                photo_url,
            }
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
    let role_id =
        tournament_roles_repository::create(&mut conn, tournament_id, "Owner", true).ok()?;
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

pub fn create_role(state: &State<AppState>, tournament_id: i64, name: &str) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Role name is required.".to_string());
    }
    let mut conn = db::open_conn(&state.pool).map_err(|err| format!("Storage error: {err}"))?;
    let _ = tournament_roles_repository::create(&mut conn, tournament_id, trimmed, false)
        .map_err(|err| format!("Storage error: {err}"))?;
    Ok(())
}

pub fn delete_role(
    state: &State<AppState>,
    tournament_id: i64,
    role_id: i64,
) -> Result<(), String> {
    let mut conn = db::open_conn(&state.pool).map_err(|err| format!("Storage error: {err}"))?;
    let changed = tournament_roles_repository::delete(&mut conn, tournament_id, role_id)
        .map_err(|err| format!("Storage error: {err}"))?;
    if changed == 0 {
        return Err("Role not found or cannot delete Owner.".to_string());
    }
    Ok(())
}

pub fn update_role_permissions(
    state: &State<AppState>,
    tournament_id: i64,
    role_id: i64,
    requested_permissions: &[String],
) -> Result<(), String> {
    let mut conn = db::open_conn(&state.pool).map_err(|err| format!("Storage error: {err}"))?;
    let is_owner = conn
        .exec_first::<i64, _, _>(
            "SELECT COUNT(*) FROM tournament_roles WHERE id = ? AND tournament_id = ? AND is_owner = 1",
            (role_id, tournament_id),
        )
        .map_err(|err| format!("Storage error: {err}"))?
        .unwrap_or(0)
        > 0;
    if is_owner {
        return Err("Owner permissions cannot be changed.".to_string());
    }
    let valid_permissions = permissions();
    let filtered: Vec<String> = requested_permissions
        .iter()
        .filter(|item| {
            valid_permissions
                .iter()
                .any(|perm| perm.eq_ignore_ascii_case(item))
        })
        .map(|item| item.to_string())
        .collect();
    role_permissions_repository::replace_for_role(&mut conn, role_id, &filtered)
        .map_err(|err| format!("Storage error: {err}"))?;
    Ok(())
}

pub fn assign_user_role(
    state: &State<AppState>,
    tournament_id: i64,
    user_id: i64,
    role_id: i64,
) -> Result<(), String> {
    let mut conn = db::open_conn(&state.pool).map_err(|err| format!("Storage error: {err}"))?;
    let has_access = tournaments_repository::user_has_access(&mut conn, tournament_id, user_id)
        .map_err(|err| format!("Storage error: {err}"))?;
    if !has_access {
        return Err("User does not have tournament access.".to_string());
    }
    if let Ok(Some(tournament)) = tournaments_repository::get_by_id(&mut conn, tournament_id) {
        if tournament.user_id == user_id {
            let owner_role_id =
                tournament_roles_repository::get_owner_role_id(&mut conn, tournament_id)
                    .map_err(|err| format!("Storage error: {err}"))?
                    .ok_or_else(|| "Owner role missing.".to_string())?;
            if owner_role_id != role_id {
                return Err("Tournament owner must keep the Owner role.".to_string());
            }
        }
    }
    tournament_user_roles_repository::set_user_role(&mut conn, tournament_id, user_id, role_id)
        .map_err(|err| format!("Storage error: {err}"))?;
    Ok(())
}

pub fn create_user(
    state: &State<AppState>,
    tournament_id: i64,
    name: &str,
    email: &str,
    password: &str,
    role_id: Option<i64>,
    photo_url: Option<&str>,
) -> Result<(), String> {
    let trimmed_name = name.trim();
    let trimmed_email = email.trim().to_lowercase();
    if trimmed_name.is_empty() || trimmed_email.is_empty() {
        return Err("Name and email are required.".to_string());
    }
    if password.len() < 6 {
        return Err("Password must be at least 6 characters.".to_string());
    }
    let user_id = auth_service::create_tournament_user(
        state,
        tournament_id,
        trimmed_name,
        &trimmed_email,
        password,
        photo_url,
    )
    .map_err(|err| match err {
        auth_service::AuthError::EmailTaken => {
            "Email already used for this tournament.".to_string()
        }
        auth_service::AuthError::Validation(message) => message,
        auth_service::AuthError::Storage(message) => message,
        _ => "Unexpected error creating user.".to_string(),
    })?;
    let mut conn = db::open_conn(&state.pool).map_err(|err| format!("Storage error: {err}"))?;
    conn.exec_drop(
        "UPDATE users SET user_type = 'tournament', tournament_id = ? WHERE id = ?",
        (tournament_id, user_id),
    )
    .map_err(|err| format!("Storage error: {err}"))?;
    if let Some(role_id) = role_id {
        let _ = tournament_user_roles_repository::set_user_role(
            &mut conn,
            tournament_id,
            user_id,
            role_id,
        );
    }
    Ok(())
}

pub fn update_user(
    state: &State<AppState>,
    tournament_id: i64,
    user_id: i64,
    name: &str,
    email: &str,
    photo_url: Option<&str>,
    new_password: Option<&str>,
) -> Result<(), String> {
    let trimmed_name = name.trim();
    let trimmed_email = email.trim().to_lowercase();
    if trimmed_name.is_empty() || trimmed_email.is_empty() {
        return Err("Name and email are required.".to_string());
    }
    let mut conn = db::open_conn(&state.pool).map_err(|err| format!("Storage error: {err}"))?;
    let matches: Option<i64> = conn
        .exec_first(
            "SELECT id FROM users WHERE id = ? AND user_type = 'tournament' AND tournament_id = ?",
            (user_id, tournament_id),
        )
        .map_err(|err| format!("Storage error: {err}"))?;
    if matches.is_none() {
        return Err("User not found for this tournament.".to_string());
    }
    users_repository::update_user(&mut conn, user_id, trimmed_name, &trimmed_email, photo_url)
        .map_err(|err| format!("Storage error: {err}"))?;

    if let Some(password) = new_password {
        let password = password.trim();
        if !password.is_empty() {
            if password.len() < 6 {
                return Err("Password must be at least 6 characters.".to_string());
            }
            let password_hash = auth_service::hash_password(password).map_err(|err| match err {
                auth_service::AuthError::Validation(message) => message,
                auth_service::AuthError::Storage(message) => message,
                _ => "Unexpected error creating password hash.".to_string(),
            })?;
            users_repository::update_password_hash(&mut conn, user_id, &password_hash)
                .map_err(|err| format!("Storage error: {err}"))?;
        }
    }
    Ok(())
}

pub fn remove_user_from_tournament(
    state: &State<AppState>,
    tournament_id: i64,
    user_id: i64,
) -> Result<(), String> {
    let mut conn = db::open_conn(&state.pool).map_err(|err| format!("Storage error: {err}"))?;
    let tournament = tournaments_repository::get_by_id(&mut conn, tournament_id)
        .map_err(|err| format!("Storage error: {err}"))?
        .ok_or_else(|| "Tournament not found.".to_string())?;
    if tournament.user_id == user_id {
        return Err("Cannot remove the owner.".to_string());
    }
    let matches: Option<i64> = conn
        .exec_first(
            "SELECT id FROM users WHERE id = ? AND user_type = 'tournament' AND tournament_id = ?",
            (user_id, tournament_id),
        )
        .map_err(|err| format!("Storage error: {err}"))?;
    if matches.is_none() {
        return Err("User not found for this tournament.".to_string());
    }
    tournament_user_roles_repository::remove_user(&mut conn, tournament_id, user_id)
        .map_err(|err| format!("Storage error: {err}"))?;
    conn.exec_drop(
        "DELETE FROM users WHERE id = ? AND user_type = 'tournament' AND tournament_id = ?",
        (user_id, tournament_id),
    )
    .map_err(|err| format!("Storage error: {err}"))?;
    Ok(())
}

pub fn user_permissions(state: &State<AppState>, user_id: i64, tournament_id: i64) -> Vec<String> {
    if is_owner(state, user_id, tournament_id) {
        return permissions();
    }
    let mut conn = match db::open_conn(&state.pool) {
        Ok(conn) => conn,
        Err(_) => return Vec::new(),
    };
    let role_id =
        match tournament_user_roles_repository::get_user_role(&mut conn, tournament_id, user_id) {
            Ok(Some(role_id)) => role_id,
            _ => {
                // Dashboard is always accessible, even if no explicit role is assigned.
                return vec!["dashboard".to_string()];
            }
        };
    let mut perms =
        role_permissions_repository::list_by_role(&mut conn, role_id).unwrap_or_default();

    // Ensure dashboard is always accessible for all users.
    if !perms.iter().any(|p| p.eq_ignore_ascii_case("dashboard")) {
        perms.push("dashboard".to_string());
    }

    perms
}

pub fn user_has_permission(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    permission: &str,
) -> bool {
    let perms = user_permissions(state, user_id, tournament_id);
    perms
        .iter()
        .any(|item| item.eq_ignore_ascii_case(permission))
}
