#[macro_use]
extern crate rocket;

mod controllers;
mod db;
mod models;
mod repositories;
mod services;
mod slug;
mod state;

use rocket::fs::{relative, FileServer};
use rocket_dyn_templates::Template;
use state::AppState;
use std::env;

#[launch]
fn rocket() -> _ {
    let _ = dotenvy::dotenv();
    let db_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root@127.0.0.1:3306/pulse-db".to_string());
    let pool = mysql::Pool::new(db_url.as_str()).expect("failed to create MySQL pool");
    let _ = db::init_db(&pool);
    let _ = services::tournament_service::ensure_slugs(&pool);
    rocket::build()
        .manage(AppState { pool })
        .mount(
            "/",
            routes![
                controllers::index_controller::index,
                controllers::auth_controller::auth_page,
                controllers::auth_controller::register,
                controllers::auth_controller::login,
                controllers::auth_controller::tournament_login_page,
                controllers::auth_controller::tournament_login,
                controllers::dashboard_controller::dashboard,
                controllers::dashboard_controller::tournament_dashboard,
                controllers::slug_controller::slug_root,
                controllers::auth_controller::logout,
                controllers::settings_controller::settings_page,
                controllers::settings_controller::complete_setup,
                controllers::settings_controller::create_division,
                controllers::settings_controller::update_division,
                controllers::settings_controller::delete_division,
                controllers::settings_controller::create_category,
                controllers::settings_controller::create_category_options,
                controllers::settings_controller::update_category,
                controllers::settings_controller::delete_category,
                controllers::settings_controller::create_weight_class,
                controllers::settings_controller::create_weight_options,
                controllers::settings_controller::update_weight_class,
                controllers::settings_controller::delete_weight_class,
                controllers::settings_controller::create_event,
                controllers::settings_controller::create_event_options,
                controllers::settings_controller::update_event,
                controllers::settings_controller::delete_event,
                controllers::settings_controller::create_role,
                controllers::settings_controller::update_role_permissions,
                controllers::settings_controller::delete_role,
                controllers::settings_controller::assign_user_role,
                controllers::settings_controller::create_user,
                controllers::settings_controller::update_user,
                controllers::settings_controller::delete_user,
                controllers::events_controller::events_page,
                controllers::events_controller::event_profile,
                controllers::events_controller::create_event,
                controllers::events_controller::update_event,
                controllers::events_controller::delete_event,
                controllers::events_controller::create_match,
                controllers::events_controller::update_match,
                controllers::events_controller::toggle_match_timer,
                controllers::events_controller::delete_match,
                controllers::events_controller::reset_matchmaking,
                controllers::matches_controller::matches_page,
                controllers::matches_controller::match_page,
                controllers::matches_controller::match_live,
                controllers::scores_controller::scores_page,
                controllers::scores_controller::adjust_score,
                controllers::results_controller::results_page,
                controllers::results_controller::match_result_detail,
                controllers::tournaments_controller::create_tournament,
                controllers::tournaments_controller::select_tournament,
                controllers::teams_controller::teams_page,
                controllers::teams_controller::team_profile,
                controllers::teams_controller::create_team,
                controllers::teams_controller::update_team,
                controllers::teams_controller::delete_team,
                controllers::teams_controller::add_member,
                controllers::teams_controller::delete_member,
                controllers::teams_controller::update_member
            ],
        )
        .mount("/static", FileServer::from(relative!("static")))
        .attach(Template::fairing())
}
