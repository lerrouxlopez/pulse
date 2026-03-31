use crate::db;
use crate::models::Tournament;
use crate::repositories::tournaments_repository;
use crate::state::AppState;
use rocket::State;

pub fn get_or_create(state: &State<AppState>) -> Option<Tournament> {
    let conn = db::open_conn(&state.db_path).ok()?;
    if let Some(tournament) = tournaments_repository::get_first(&conn).ok()? {
        return Some(tournament);
    }
    // If no tournament exists, create a placeholder to keep the flow moving.
    let _ = tournaments_repository::create(&conn, "New Tournament");
    tournaments_repository::get_first(&conn).ok()?
}

pub fn mark_setup_complete(state: &State<AppState>, tournament_id: i64) -> bool {
    let conn = match db::open_conn(&state.db_path) {
        Ok(conn) => conn,
        Err(_) => return false,
    };
    tournaments_repository::set_setup(&conn, tournament_id, true).is_ok()
}
