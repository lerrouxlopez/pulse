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
    pub is_setup: bool,
    pub user_id: i64,
    pub started_at: Option<String>,
}
