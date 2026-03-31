use crate::models::{LoginForm, RegisterForm};
use crate::services::auth_service::{self, AuthError};
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
    match auth_service::register(state, form) {
        Ok(user_id) => {
            jar.add(Cookie::new("user_id", user_id.to_string()));
            Ok(Redirect::to(uri!(crate::controllers::dashboard_controller::dashboard)))
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
    match auth_service::login(state, form) {
        Ok(user_id) => {
            jar.add(Cookie::new("user_id", user_id.to_string()));
            Ok(Redirect::to(uri!(crate::controllers::dashboard_controller::dashboard)))
        }
        Err(AuthError::InvalidCredentials) => Ok(Redirect::to(uri!(auth_page(
            error = Some("Invalid email or password.".to_string()),
            success = Option::<String>::None
        )))),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[post("/logout")]
pub fn logout(jar: &CookieJar<'_>) -> Redirect {
    jar.remove(Cookie::from("user_id"));
    Redirect::to(uri!(crate::controllers::index_controller::index))
}
