use crate::db;
use crate::models::{CurrentUser, LoginForm, RegisterForm};
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

fn hash_password(password: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| AuthError::Storage)
        .map(|hash| hash.to_string())
}

pub fn register_system_user(state: &State<AppState>, form: RegisterForm) -> Result<i64, AuthError> {
    if form.name.trim().is_empty() || form.email.trim().is_empty() || form.password.len() < 6 {
        return Err(AuthError::Validation(
            "Please fill all fields. Password must be 6+ chars.".to_string(),
        ));
    }

    let password_hash = hash_password(&form.password)?;

    let mut conn = db::open_conn(&state.pool).map_err(|_| AuthError::Storage)?;
    let result = users_repository::create_system_user(
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

pub fn create_tournament_user(
    state: &State<AppState>,
    tournament_id: i64,
    name: &str,
    email: &str,
    password: &str,
) -> Result<i64, AuthError> {
    let trimmed_name = name.trim();
    let trimmed_email = email.trim().to_lowercase();
    if trimmed_name.is_empty() || trimmed_email.is_empty() || password.len() < 6 {
        return Err(AuthError::Validation(
            "Name, email, and password are required. Password must be 6+ chars.".to_string(),
        ));
    }
    let password_hash = hash_password(password)?;
    let mut conn = db::open_conn(&state.pool).map_err(|_| AuthError::Storage)?;
    let result = users_repository::create_tournament_user(
        &mut conn,
        tournament_id,
        trimmed_name,
        &trimmed_email,
        &password_hash,
    );
    match result {
        Ok(user_id) => Ok(user_id),
        Err(Error::MySqlError(ref err)) if err.code == 1062 => Err(AuthError::EmailTaken),
        Err(_) => Err(AuthError::Storage),
    }
}

pub fn login_system_user(state: &State<AppState>, form: LoginForm) -> Result<i64, AuthError> {
    let mut conn = db::open_conn(&state.pool).map_err(|_| AuthError::Storage)?;
    let user = users_repository::find_system_user_by_email(&mut conn, &form.email.trim().to_lowercase())
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

pub fn login_tournament_user(
    state: &State<AppState>,
    tournament_id: i64,
    form: LoginForm,
) -> Result<i64, AuthError> {
    let mut conn = db::open_conn(&state.pool).map_err(|_| AuthError::Storage)?;
    let user = users_repository::find_tournament_user_by_email(
        &mut conn,
        tournament_id,
        &form.email.trim().to_lowercase(),
    )
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

pub fn current_user(state: &State<AppState>, jar: &CookieJar<'_>) -> Option<CurrentUser> {
    let user_id = jar.get("user_id")?.value().parse::<i64>().ok()?;
    let mut conn = db::open_conn(&state.pool).ok()?;
    users_repository::find_user_profile_by_id(&mut conn, user_id).ok()?
}
