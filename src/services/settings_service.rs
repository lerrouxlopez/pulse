use crate::db;
use crate::models::NamedItem;
use crate::repositories::{
    categories_repository, divisions_repository, events_repository, weight_classes_repository,
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
    tournament_id: i64,
    entity: SettingsEntity,
    name: &str,
) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Name is required.".to_string());
    }
    let conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
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
    entity: SettingsEntity,
    id: i64,
    name: &str,
) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Name is required.".to_string());
    }
    let conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
    match entity {
        SettingsEntity::Division => divisions_repository::update(&conn, id, trimmed),
        SettingsEntity::Category => categories_repository::update(&conn, id, trimmed),
        SettingsEntity::WeightClass => weight_classes_repository::update(&conn, id, trimmed),
        SettingsEntity::Event => events_repository::update(&conn, id, trimmed),
    }
    .map_err(|_| "Storage error.".to_string())
}

pub fn delete(state: &State<AppState>, entity: SettingsEntity, id: i64) -> Result<(), String> {
    let conn = db::open_conn(&state.db_path).map_err(|_| "Storage error.")?;
    match entity {
        SettingsEntity::Division => divisions_repository::delete(&conn, id),
        SettingsEntity::Category => categories_repository::delete(&conn, id),
        SettingsEntity::WeightClass => weight_classes_repository::delete(&conn, id),
        SettingsEntity::Event => events_repository::delete(&conn, id),
    }
    .map_err(|_| "Storage error.".to_string())
}
