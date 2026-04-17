use crate::models::{LoginForm, RegisterForm};
use crate::services::auth_service::{self, AuthError};
use crate::services::tournament_service;
use crate::state::AppState;
use rocket::form::Form;
use rocket::http::{Cookie, CookieJar, Status};
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};

#[get("/auth?<error>&<success>")]
pub fn auth_page(error: Option<String>, success: Option<String>) -> Template {
    Template::render(
        "auth",
        context! {
            error: error,
            success: success,
        },
    )
}

#[post("/register", data = "<form>")]
pub fn register(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    form: Form<RegisterForm>,
) -> Result<Redirect, Status> {
    let form = form.into_inner();
    match auth_service::register_system_user(state, form) {
        Ok(user_id) => {
            jar.add(Cookie::new("user_id", user_id.to_string()));
            let tournament = tournament_service::create(state, user_id, "New Tournament")
                .ok_or(Status::InternalServerError)?;
            Ok(Redirect::to(uri!(
                crate::controllers::settings_controller::settings_page(
                    slug = tournament.slug,
                    error = Option::<String>::None,
                    success = Option::<String>::None,
                    tab = Option::<String>::None
                )
            )))
        }
        Err(AuthError::Validation(message)) => Ok(Redirect::to(uri!(auth_page(
            error = Some(message),
            success = Option::<String>::None
        )))),
        Err(AuthError::EmailTaken) => Ok(Redirect::to(uri!(auth_page(
            error = Some("Email already registered.".to_string()),
            success = Option::<String>::None
        )))),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[post("/login", data = "<form>")]
pub fn login(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    form: Form<LoginForm>,
) -> Result<Redirect, Status> {
    let form = form.into_inner();
    match auth_service::login_system_user(state, form) {
        Ok(user_id) => {
            jar.add(Cookie::new("user_id", user_id.to_string()));
            jar.remove(Cookie::from("tournament_id"));
            Ok(Redirect::to(uri!(
                crate::controllers::dashboard_controller::dashboard
            )))
        }
        Err(AuthError::InvalidCredentials) => Ok(Redirect::to(uri!(auth_page(
            error = Some("Invalid email or password.".to_string()),
            success = Option::<String>::None
        )))),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[get("/t/<slug>/login?<error>", rank = 1)]
pub fn tournament_login_page(
    state: &State<AppState>,
    slug: String,
    error: Option<String>,
) -> Result<Template, Redirect> {
    let tournament = match tournament_service::get_by_slug(state, &slug) {
        Some(tournament) => tournament,
        None => {
            return Err(Redirect::to(uri!(
                crate::controllers::auth_controller::auth_page(
                    error = Some("Tournament not found.".to_string()),
                    success = Option::<String>::None
                )
            )))
        }
    };
    Ok(Template::render(
        "tournament_login",
        context! {
            error: error,
            tournament_name: tournament.name,
            tournament_slug: tournament.slug,
        },
    ))
}

#[post("/t/<slug>/login", data = "<form>", rank = 1)]
pub fn tournament_login(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    slug: String,
    form: Form<LoginForm>,
) -> Result<Redirect, Status> {
    let tournament = tournament_service::get_by_slug(state, &slug).ok_or(Status::NotFound)?;
    let form = form.into_inner();
    match auth_service::login_tournament_user(state, tournament.id, form) {
        Ok(user_id) => {
            jar.add(Cookie::new("user_id", user_id.to_string()));
            jar.add(Cookie::new("last_tournament_slug", tournament.slug.clone()));
            Ok(Redirect::to(uri!(
                crate::controllers::dashboard_controller::tournament_dashboard(
                    slug = tournament.slug
                )
            )))
        }
        Err(AuthError::InvalidCredentials) => Ok(Redirect::to(uri!(tournament_login_page(
            slug = slug,
            error = Some("Invalid email or password.".to_string())
        )))),
        Err(AuthError::EmailTaken) => Ok(Redirect::to(uri!(tournament_login_page(
            slug = slug,
            error = Some("Invalid email or password.".to_string())
        )))),
        Err(AuthError::Validation(message)) => Ok(Redirect::to(uri!(tournament_login_page(
            slug = slug,
            error = Some(message)
        )))),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[post("/logout")]
pub fn logout(jar: &CookieJar<'_>) -> Redirect {
    jar.remove(Cookie::from("user_id"));
    jar.remove(Cookie::from("tournament_id"));
    jar.remove(Cookie::from("last_tournament_slug"));
    Redirect::to(uri!(crate::controllers::index_controller::index))
}
