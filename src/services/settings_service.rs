use crate::db;
use crate::models::NamedItem;
use crate::repositories::{
    categories_repository, divisions_repository, events_repository, weight_classes_repository,
    tournaments_repository,
};
use crate::state::AppState;
use rocket::State;

pub enum SettingsEntity {
    Division,
    Category,
    WeightClass,
    Event,
}

pub fn list(
    state: &State<AppState>,
    tournament_id: i64,
    entity: SettingsEntity,
) -> Vec<NamedItem> {
    let conn = match db::open_conn(&state.db_path) {
        Ok(conn) => conn,
        Err(_) => return Vec::new(),
    };
    match entity {
        SettingsEntity::Division => divisions_repository::list(&conn, tournament_id).unwrap_or_default(),
        SettingsEntity::Category => categories_repository::list(&conn, tournament_id).unwrap_or_default(),
        SettingsEntity::WeightClass => weight_classes_repository::list(&conn, tournament_id).unwrap_or_default(),
        SettingsEntity::Event => events_repository::list(&conn, tournament_id).unwrap_or_default(),
    }
}

pub fn create(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    entity: SettingsEntity,
    name: &str,
) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Name is required.".to_string());
    }
    let conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }
    match entity {
        SettingsEntity::Division => divisions_repository::create(&conn, tournament_id, trimmed),
        SettingsEntity::Category => categories_repository::create(&conn, tournament_id, trimmed),
        SettingsEntity::WeightClass => weight_classes_repository::create(&conn, tournament_id, trimmed),
        SettingsEntity::Event => events_repository::create(&conn, tournament_id, trimmed),
    }
    .map(|_| ())
    .map_err(|_| "Storage error.".to_string())
}

pub fn update(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    entity: SettingsEntity,
    id: i64,
    name: &str,
) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Name is required.".to_string());
    }
    let conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }

    let changed = match entity {
        SettingsEntity::Division => divisions_repository::update(&conn, tournament_id, id, trimmed),
        SettingsEntity::Category => categories_repository::update(&conn, tournament_id, id, trimmed),
        SettingsEntity::WeightClass => {
            weight_classes_repository::update(&conn, tournament_id, id, trimmed)
        }
        SettingsEntity::Event => events_repository::update(&conn, tournament_id, id, trimmed),
    }
    .map_err(|_| "Storage error.".to_string())?;

    if changed == 0 {
        return Err("Item not found for this tournament.".to_string());
    }
    Ok(())
}

pub fn delete(
    state: &State<AppState>,
    user_id: i64,
    tournament_id: i64,
    entity: SettingsEntity,
    id: i64,
) -> Result<(), String> {
    let mut conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
    let has_access = tournaments_repository::user_has_access(&conn, tournament_id, user_id)
        .map_err(|_| "Storage error.".to_string())?;
    if !has_access {
        return Err("Tournament not found.".to_string());
    }

    let changed = match entity {
        SettingsEntity::Division => divisions_repository::delete(&conn, tournament_id, id),
        SettingsEntity::Category => categories_repository::delete(&mut conn, tournament_id, id),
        SettingsEntity::WeightClass => weight_classes_repository::delete(&conn, tournament_id, id),
        SettingsEntity::Event => events_repository::delete(&mut conn, tournament_id, id),
    }
    .map_err(|_| "Unable to delete item. It may be in use.".to_string())?;

    if changed == 0 {
        return Err("Item not found for this tournament.".to_string());
    }
    Ok(())
}
