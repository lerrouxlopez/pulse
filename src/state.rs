use mysql::Pool;

#[derive(Clone)]
pub struct AppState {
    pub pool: Pool,
}
