use crate::db;
use crate::models::{LoginForm, RegisterForm, UserSummary};
use crate::repositories::users_repository;
use crate::state::AppState;
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand_core::OsRng;
use rocket::http::CookieJar;
use rocket::State;
use mysql::Error;

pub enum AuthError {
    Validation(String),
    EmailTaken,
    InvalidCredentials,
    Storage,
}

pub fn register(state: &State<AppState>, form: RegisterForm) -> Result<i64, AuthError> {
    if form.name.trim().is_empty() || form.email.trim().is_empty() || form.password.len() < 6 {
        return Err(AuthError::Validation(
            "Please fill all fields. Password must be 6+ chars.".to_string(),
        ));
    }

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(form.password.as_bytes(), &salt)
        .map_err(|_| AuthError::Storage)?
        .to_string();

    let mut conn = db::open_conn(&state.pool).map_err(|_| AuthError::Storage)?;
    let result = users_repository::create_user(
        &mut conn,
        form.name.trim(),
        &form.email.trim().to_lowercase(),
        &password_hash,
    );

    match result {
        Ok(user_id) => Ok(user_id),
        Err(Error::MySqlError(ref err)) if err.code == 1062 => Err(AuthError::EmailTaken),
        Err(_) => Err(AuthError::Storage),
    }
}

pub fn login(state: &State<AppState>, form: LoginForm) -> Result<i64, AuthError> {
    let mut conn = db::open_conn(&state.pool).map_err(|_| AuthError::Storage)?;
    let user = users_repository::find_user_by_email(&mut conn, &form.email.trim().to_lowercase())
        .map_err(|_| AuthError::Storage)?;
    if let Some(user) = user {
        let parsed_hash = PasswordHash::new(&user.password_hash).map_err(|_| AuthError::Storage)?;
        if Argon2::default()
            .verify_password(form.password.as_bytes(), &parsed_hash)
            .is_ok()
        {
            return Ok(user.id);
        }
    }

    Err(AuthError::InvalidCredentials)
}

pub fn current_user(state: &State<AppState>, jar: &CookieJar<'_>) -> Option<UserSummary> {
    let user_id = jar.get("user_id")?.value().parse::<i64>().ok()?;
    let mut conn = db::open_conn(&state.pool).ok()?;
    users_repository::find_user_by_id(&mut conn, user_id).ok()?
}
