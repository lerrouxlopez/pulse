use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;

pub fn init_db(db_path: &PathBuf) -> rusqlite::Result<()> {
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
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tournaments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            slug TEXT,
            is_setup INTEGER NOT NULL DEFAULT 0,
            started_at TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;
    let _ = conn.execute("ALTER TABLE tournaments ADD COLUMN user_id INTEGER", []);
    let _ = conn.execute("ALTER TABLE tournaments ADD COLUMN slug TEXT", []);
    let _ = conn.execute("ALTER TABLE tournaments ADD COLUMN started_at TEXT", []);
    let _ = conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_tournaments_slug ON tournaments(slug)",
        [],
    );
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tournament_users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tournament_id INTEGER NOT NULL,
            user_id INTEGER NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(tournament_id, user_id),
            FOREIGN KEY (tournament_id) REFERENCES tournaments(id),
            FOREIGN KEY (user_id) REFERENCES users(id)
        )",
        [],
    )?;
    let _ = conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_tournament_users_tournament_id ON tournament_users(tournament_id)",
        [],
    );
    let _ = conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_tournament_users_user_id ON tournament_users(user_id)",
        [],
    );
    conn.execute(
        "CREATE TABLE IF NOT EXISTS divisions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tournament_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (tournament_id) REFERENCES tournaments(id)
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS categories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tournament_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (tournament_id) REFERENCES tournaments(id)
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS weight_classes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tournament_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (tournament_id) REFERENCES tournaments(id)
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tournament_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (tournament_id) REFERENCES tournaments(id)
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS teams (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tournament_id INTEGER NOT NULL,
            logo_url TEXT,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (tournament_id) REFERENCES tournaments(id)
        )",
        [],
    )?;
    let _ = conn.execute("ALTER TABLE teams ADD COLUMN logo_url TEXT", []);
    conn.execute(
        "CREATE TABLE IF NOT EXISTS team_divisions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tournament_id INTEGER NOT NULL,
            team_id INTEGER NOT NULL,
            division_id INTEGER NOT NULL,
            FOREIGN KEY (tournament_id) REFERENCES tournaments(id),
            FOREIGN KEY (team_id) REFERENCES teams(id),
            FOREIGN KEY (division_id) REFERENCES divisions(id)
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS team_categories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tournament_id INTEGER NOT NULL,
            team_id INTEGER NOT NULL,
            category_id INTEGER NOT NULL,
            FOREIGN KEY (tournament_id) REFERENCES tournaments(id),
            FOREIGN KEY (team_id) REFERENCES teams(id),
            FOREIGN KEY (category_id) REFERENCES categories(id)
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS team_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tournament_id INTEGER NOT NULL,
            team_id INTEGER NOT NULL,
            event_id INTEGER NOT NULL,
            FOREIGN KEY (tournament_id) REFERENCES tournaments(id),
            FOREIGN KEY (team_id) REFERENCES teams(id),
            FOREIGN KEY (event_id) REFERENCES events(id)
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS team_members (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tournament_id INTEGER NOT NULL,
            team_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            notes TEXT,
            weight_class TEXT,
            division_id INTEGER,
            photo_url TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (tournament_id) REFERENCES tournaments(id),
            FOREIGN KEY (team_id) REFERENCES teams(id)
        )",
        [],
    )?;
    let _ = conn.execute("ALTER TABLE team_members ADD COLUMN notes TEXT", []);
    let _ = conn.execute("ALTER TABLE team_members ADD COLUMN weight_class TEXT", []);
    let _ = conn.execute("ALTER TABLE team_members ADD COLUMN division_id INTEGER", []);
    let _ = conn.execute("ALTER TABLE team_members ADD COLUMN photo_url TEXT", []);
    conn.execute(
        "CREATE TABLE IF NOT EXISTS scheduled_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tournament_id INTEGER NOT NULL,
            event_id INTEGER NOT NULL,
            contact_type TEXT NOT NULL,
            status TEXT NOT NULL,
            location TEXT,
            event_time TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (tournament_id) REFERENCES tournaments(id),
            FOREIGN KEY (event_id) REFERENCES events(id)
        )",
        [],
    )?;
    let _ = conn.execute("ALTER TABLE scheduled_events ADD COLUMN event_time TEXT", []);
    let _ = conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_scheduled_events_unique ON scheduled_events(tournament_id, event_id)",
        [],
    );
    conn.execute(
        "CREATE TABLE IF NOT EXISTS matches (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tournament_id INTEGER NOT NULL,
            scheduled_event_id INTEGER NOT NULL,
            mat TEXT,
            category TEXT,
            red TEXT,
            blue TEXT,
            status TEXT NOT NULL,
            location TEXT,
            match_time TEXT,
            round INTEGER,
            slot INTEGER,
            red_member_id INTEGER,
            blue_member_id INTEGER,
            is_bye INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (tournament_id) REFERENCES tournaments(id),
            FOREIGN KEY (scheduled_event_id) REFERENCES scheduled_events(id)
        )",
        [],
    )?;
    let _ = conn.execute("ALTER TABLE matches ADD COLUMN location TEXT", []);
    let _ = conn.execute("ALTER TABLE matches ADD COLUMN match_time TEXT", []);
    let _ = conn.execute("ALTER TABLE matches ADD COLUMN round INTEGER", []);
    let _ = conn.execute("ALTER TABLE matches ADD COLUMN slot INTEGER", []);
    let _ = conn.execute("ALTER TABLE matches ADD COLUMN red_member_id INTEGER", []);
    let _ = conn.execute("ALTER TABLE matches ADD COLUMN blue_member_id INTEGER", []);
    let _ = conn.execute("ALTER TABLE matches ADD COLUMN is_bye INTEGER", []);
    let _ = conn.execute("ALTER TABLE matches ADD COLUMN winner_side TEXT", []);
    conn.execute(
        "CREATE TABLE IF NOT EXISTS team_member_categories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tournament_id INTEGER NOT NULL,
            team_id INTEGER NOT NULL,
            member_id INTEGER NOT NULL,
            category_id INTEGER NOT NULL,
            FOREIGN KEY (tournament_id) REFERENCES tournaments(id),
            FOREIGN KEY (team_id) REFERENCES teams(id),
            FOREIGN KEY (member_id) REFERENCES team_members(id),
            FOREIGN KEY (category_id) REFERENCES categories(id)
        )",
        [],
    )?;
    let _ = conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_member_categories_unique ON team_member_categories(tournament_id, member_id, category_id)",
        [],
    );
    conn.execute(
        "CREATE TABLE IF NOT EXISTS team_member_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tournament_id INTEGER NOT NULL,
            team_id INTEGER NOT NULL,
            member_id INTEGER NOT NULL,
            event_id INTEGER NOT NULL,
            FOREIGN KEY (tournament_id) REFERENCES tournaments(id),
            FOREIGN KEY (team_id) REFERENCES teams(id),
            FOREIGN KEY (member_id) REFERENCES team_members(id),
            FOREIGN KEY (event_id) REFERENCES events(id)
        )",
        [],
    )?;
    let _ = conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_member_events_unique ON team_member_events(tournament_id, member_id, event_id)",
        [],
    );
    let _ = conn.execute(
        "INSERT OR IGNORE INTO team_member_categories (tournament_id, team_id, member_id, category_id)
         SELECT tournament_id, team_id, id, category_id
         FROM team_members
         WHERE category_id IS NOT NULL",
        [],
    );
    let _ = conn.execute(
        "INSERT OR IGNORE INTO team_member_events (tournament_id, team_id, member_id, event_id)
         SELECT tournament_id, team_id, id, event_id
         FROM team_members
         WHERE event_id IS NOT NULL",
        [],
    );
    Ok(())
}

pub fn open_conn(db_path: &PathBuf) -> rusqlite::Result<Connection> {
    Connection::open(db_path)
}
