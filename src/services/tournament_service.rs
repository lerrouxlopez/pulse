use crate::db;
use crate::models::Tournament;
use crate::repositories::tournaments_repository;
use crate::state::AppState;
use rocket::State;

pub fn get_by_id(state: &State<AppState>, tournament_id: i64) -> Option<Tournament> {
    let conn = db::open_conn(&state.db_path).ok()?;
    tournaments_repository::get_by_id(&conn, tournament_id).ok()?
}

pub fn get_by_id_for_user(
    state: &State<AppState>,
    tournament_id: i64,
    user_id: i64,
) -> Option<Tournament> {
    let conn = db::open_conn(&state.db_path).ok()?;
    tournaments_repository::get_by_id_for_user(&conn, tournament_id, user_id).ok()?
}

pub fn list_by_user(state: &State<AppState>, user_id: i64) -> Vec<Tournament> {
    let conn = match db::open_conn(&state.db_path) {
        Ok(conn) => conn,
        Err(_) => return Vec::new(),
    };
    tournaments_repository::list_by_user(&conn, user_id).unwrap_or_default()
}

pub fn create(state: &State<AppState>, user_id: i64, name: &str) -> Option<Tournament> {
    let conn = db::open_conn(&state.db_path).ok()?;
    let tournament_id = tournaments_repository::create(&conn, user_id, name).ok()?;
    tournaments_repository::get_by_id(&conn, tournament_id).ok()?
}

pub fn mark_setup_complete(state: &State<AppState>, tournament_id: i64) -> bool {
    let conn = match db::open_conn(&state.db_path) {
        Ok(conn) => conn,
        Err(_) => return false,
    };
    tournaments_repository::set_setup(&conn, tournament_id, true).is_ok()
}
