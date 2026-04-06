use rocket::form::FromForm;
use rocket::serde::Serialize;

#[derive(FromForm)]
pub struct RegisterForm {
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(FromForm)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct MatchRow {
    pub mat: String,
    pub category: String,
    pub red: String,
    pub blue: String,
    pub status: String,
    pub status_class: String,
}

#[derive(Serialize)]
pub struct UserSummary {
    pub id: i64,
    pub name: String,
}

pub struct UserAuth {
    pub id: i64,
    pub password_hash: String,
}

#[derive(Serialize)]
pub struct NamedItem {
    pub id: i64,
    pub name: String,
}

#[derive(Serialize)]
pub struct Tournament {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub is_setup: bool,
    pub user_id: i64,
    pub started_at: Option<String>,
}

#[derive(Serialize)]
pub struct Team {
    pub id: i64,
    pub name: String,
    pub logo_url: Option<String>,
    pub members: Vec<TeamMember>,
    pub divisions: Vec<NamedItem>,
    pub categories: Vec<NamedItem>,
    pub events: Vec<NamedItem>,
    pub division_ids: Vec<i64>,
    pub category_ids: Vec<i64>,
    pub event_ids: Vec<i64>,
}

#[derive(Serialize)]
pub struct TeamMember {
    pub id: i64,
    pub name: String,
    pub team_id: i64,
    pub notes: Option<String>,
    pub weight_class: Option<String>,
    pub weight_class_id: Option<i64>,
    pub division_id: Option<i64>,
    pub division_name: Option<String>,
    pub category_ids: Vec<i64>,
    pub event_ids: Vec<i64>,
    pub photo_url: Option<String>,
}

#[derive(Serialize)]
pub struct ScheduledEvent {
    pub id: i64,
    pub event_id: i64,
    pub event_name: String,
    pub contact_type: String,
    pub status: String,
    pub location: Option<String>,
    pub event_time: Option<String>,
    pub point_system: Option<String>,
    pub time_rule: Option<String>,
    pub division_id: Option<i64>,
    pub weight_class_id: Option<i64>,
    pub division_name: Option<String>,
    pub weight_class_name: Option<String>,
    pub weight_class_label: Option<String>,
}

#[derive(Serialize)]
pub struct ScheduledMatch {
    pub id: i64,
    pub scheduled_event_id: i64,
    pub mat: Option<String>,
    pub category: Option<String>,
    pub red: Option<String>,
    pub blue: Option<String>,
    pub status: String,
    pub location: Option<String>,
    pub match_time: Option<String>,
    pub round: Option<i64>,
    pub slot: Option<i64>,
    pub red_member_id: Option<i64>,
    pub blue_member_id: Option<i64>,
    pub is_bye: bool,
    pub winner_side: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct EventCompetitor {
    pub member_id: i64,
    pub team_id: i64,
    pub name: String,
    pub photo_url: Option<String>,
}
