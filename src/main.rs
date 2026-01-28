#[macro_use]
extern crate rocket;

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand_core::OsRng;
use rocket::form::Form;
use rocket::fs::{relative, FileServer};
use rocket::http::{Cookie, CookieJar, Status};
use rocket::response::Redirect;
use rocket::serde::Serialize;
use rocket::State;
use rocket_dyn_templates::{context, Template};
use rusqlite::{params, Connection};
use std::fs;
use std::path::PathBuf;

#[derive(Clone)]
struct AppState {
    db_path: PathBuf,
}

#[derive(FromForm)]
struct RegisterForm {
    name: String,
    email: String,
    password: String,
}

#[derive(FromForm)]
struct LoginForm {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct MatchRow {
    mat: String,
    category: String,
    red: String,
    blue: String,
    status: String,
    status_class: String,
}

fn init_db(db_path: &PathBuf) -> rusqlite::Result<()> {
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).ok();
    }
    let conn = Connection::open(db_path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;
    Ok(())
}

fn open_conn(state: &State<AppState>) -> rusqlite::Result<Connection> {
    Connection::open(&state.db_path)
}

fn current_user(state: &State<AppState>, jar: &CookieJar<'_>) -> Option<(i64, String)> {
    let user_id = jar.get("user_id")?.value().parse::<i64>().ok()?;
    let conn = open_conn(state).ok()?;
    let mut stmt = conn
        .prepare("SELECT id, name FROM users WHERE id = ?1")
        .ok()?;
    let mut rows = stmt.query(params![user_id]).ok()?;
    if let Some(row) = rows.next().ok()? {
        let id: i64 = row.get(0).ok()?;
        let name: String = row.get(1).ok()?;
        Some((id, name))
    } else {
        None
    }
}

#[get("/")]
fn index() -> Template {
    Template::render("index", context! {})
}

#[get("/auth?<error>&<success>")]
fn auth_page(error: Option<&str>, success: Option<&str>) -> Template {
    Template::render(
        "auth",
        context! {
            error: error.map(|v| v.to_string()),
            success: success.map(|v| v.to_string()),
        },
    )
}

#[post("/register", data = "<form>")]
fn register(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    form: Form<RegisterForm>,
) -> Result<Redirect, Status> {
    let form = form.into_inner();
    if form.name.trim().is_empty() || form.email.trim().is_empty() || form.password.len() < 6 {
        return Ok(Redirect::to(uri!(auth_page(
            error = Some("Please fill all fields. Password must be 6+ chars."),
            success = Option::<&str>::None
        ))));
    }

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(form.password.as_bytes(), &salt)
        .map_err(|_| Status::InternalServerError)?
        .to_string();

    let conn = open_conn(state).map_err(|_| Status::InternalServerError)?;
    let result = conn.execute(
        "INSERT INTO users (name, email, password_hash) VALUES (?1, ?2, ?3)",
        params![form.name.trim(), form.email.trim().to_lowercase(), password_hash],
    );

    match result {
        Ok(_) => {
            let user_id = conn.last_insert_rowid();
            jar.add(Cookie::new("user_id", user_id.to_string()));
            Ok(Redirect::to(uri!(dashboard)))
        }
        Err(_) => Ok(Redirect::to(uri!(auth_page(
            error = Some("Email already registered."),
            success = Option::<&str>::None
        )))),
    }
}

#[post("/login", data = "<form>")]
fn login(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    form: Form<LoginForm>,
) -> Result<Redirect, Status> {
    let form = form.into_inner();
    let conn = open_conn(state).map_err(|_| Status::InternalServerError)?;
    let mut stmt = conn
        .prepare("SELECT id, password_hash FROM users WHERE email = ?1")
        .map_err(|_| Status::InternalServerError)?;
    let mut rows = stmt
        .query(params![form.email.trim().to_lowercase()])
        .map_err(|_| Status::InternalServerError)?;
    if let Some(row) = rows.next().map_err(|_| Status::InternalServerError)? {
        let user_id: i64 = row.get(0).map_err(|_| Status::InternalServerError)?;
        let stored_hash: String = row.get(1).map_err(|_| Status::InternalServerError)?;
        let parsed_hash = PasswordHash::new(&stored_hash).map_err(|_| Status::InternalServerError)?;
        if Argon2::default()
            .verify_password(form.password.as_bytes(), &parsed_hash)
            .is_ok()
        {
            jar.add(Cookie::new("user_id", user_id.to_string()));
            return Ok(Redirect::to(uri!(dashboard)));
        }
    }

    Ok(Redirect::to(uri!(auth_page(
        error = Some("Invalid email or password."),
        success = Option::<&str>::None
    ))))
}

#[get("/dashboard")]
fn dashboard(state: &State<AppState>, jar: &CookieJar<'_>) -> Result<Template, Redirect> {
    let (_user_id, name) = match current_user(state, jar) {
        Some(user) => user,
        None => {
            return Err(Redirect::to(uri!(auth_page(
                error = Option::<&str>::None,
                success = Option::<&str>::None
            ))))
        }
    };

    let matches = vec![
        MatchRow {
            mat: "1".into(),
            category: "Senior Men - Single Stick".into(),
            red: "Fighter A".into(),
            blue: "Fighter B".into(),
            status: "LIVE".into(),
            status_class: "status-live".into(),
        },
        MatchRow {
            mat: "2".into(),
            category: "Junior - Double Stick".into(),
            red: "Fighter C".into(),
            blue: "Fighter D".into(),
            status: "READY".into(),
            status_class: "status-ready".into(),
        },
    ];

    Ok(Template::render(
        "dashboard",
        context! {
            name: name,
            matches: matches,
        },
    ))
}

#[post("/logout")]
fn logout(jar: &CookieJar<'_>) -> Redirect {
    jar.remove(Cookie::from("user_id"));
    Redirect::to(uri!(index))
}

#[launch]
fn rocket() -> _ {
    let db_path = PathBuf::from("data/pulse.db");
    let _ = init_db(&db_path);
    rocket::build()
        .manage(AppState { db_path })
        .mount(
            "/",
            routes![index, auth_page, register, login, dashboard, logout],
        )
        .mount("/static", FileServer::from(relative!("static")))
        .attach(Template::fairing())
}
