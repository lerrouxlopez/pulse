use mysql::prelude::*;
use mysql::{Pool, PooledConn};

pub fn init_db(pool: &Pool) -> mysql::Result<()> {
    let mut conn = pool.get_conn()?;

    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS users (
            id BIGINT PRIMARY KEY AUTO_INCREMENT,
            name TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
        )",
    )?;

    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS tournaments (
            id BIGINT PRIMARY KEY AUTO_INCREMENT,
            user_id BIGINT NOT NULL,
            name TEXT NOT NULL,
            slug TEXT,
            is_setup TINYINT(1) NOT NULL DEFAULT 0,
            started_at TEXT,
            created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
            UNIQUE KEY idx_tournaments_slug (slug)
        )",
    )?;

    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS tournament_users (
            id BIGINT PRIMARY KEY AUTO_INCREMENT,
            tournament_id BIGINT NOT NULL,
            user_id BIGINT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
            UNIQUE KEY idx_tournament_users_unique (tournament_id, user_id),
            KEY idx_tournament_users_tournament_id (tournament_id),
            KEY idx_tournament_users_user_id (user_id)
        )",
    )?;

    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS divisions (
            id BIGINT PRIMARY KEY AUTO_INCREMENT,
            tournament_id BIGINT NOT NULL,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
        )",
    )?;

    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS categories (
            id BIGINT PRIMARY KEY AUTO_INCREMENT,
            tournament_id BIGINT NOT NULL,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
        )",
    )?;

    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS weight_classes (
            id BIGINT PRIMARY KEY AUTO_INCREMENT,
            tournament_id BIGINT NOT NULL,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
        )",
    )?;

    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS events (
            id BIGINT PRIMARY KEY AUTO_INCREMENT,
            tournament_id BIGINT NOT NULL,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
        )",
    )?;

    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS teams (
            id BIGINT PRIMARY KEY AUTO_INCREMENT,
            tournament_id BIGINT NOT NULL,
            logo_url TEXT,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
        )",
    )?;

    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS team_divisions (
            id BIGINT PRIMARY KEY AUTO_INCREMENT,
            tournament_id BIGINT NOT NULL,
            team_id BIGINT NOT NULL,
            division_id BIGINT NOT NULL
        )",
    )?;

    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS team_categories (
            id BIGINT PRIMARY KEY AUTO_INCREMENT,
            tournament_id BIGINT NOT NULL,
            team_id BIGINT NOT NULL,
            category_id BIGINT NOT NULL
        )",
    )?;

    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS team_events (
            id BIGINT PRIMARY KEY AUTO_INCREMENT,
            tournament_id BIGINT NOT NULL,
            team_id BIGINT NOT NULL,
            event_id BIGINT NOT NULL
        )",
    )?;

    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS team_members (
            id BIGINT PRIMARY KEY AUTO_INCREMENT,
            tournament_id BIGINT NOT NULL,
            team_id BIGINT NOT NULL,
            name TEXT NOT NULL,
            notes TEXT,
            weight_class TEXT,
            division_id BIGINT,
            photo_url TEXT,
            created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
        )",
    )?;

    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS scheduled_events (
            id BIGINT PRIMARY KEY AUTO_INCREMENT,
            tournament_id BIGINT NOT NULL,
            event_id BIGINT NOT NULL,
            contact_type TEXT NOT NULL,
            status TEXT NOT NULL,
            location TEXT,
            event_time TEXT,
            created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
            UNIQUE KEY idx_scheduled_events_unique (tournament_id, event_id)
        )",
    )?;

    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS matches (
            id BIGINT PRIMARY KEY AUTO_INCREMENT,
            tournament_id BIGINT NOT NULL,
            scheduled_event_id BIGINT NOT NULL,
            mat TEXT,
            category TEXT,
            red TEXT,
            blue TEXT,
            status TEXT NOT NULL,
            location TEXT,
            match_time TEXT,
            round BIGINT,
            slot BIGINT,
            red_member_id BIGINT,
            blue_member_id BIGINT,
            is_bye TINYINT(1) NOT NULL DEFAULT 0,
            winner_side TEXT,
            created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
        )",
    )?;

    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS team_member_categories (
            id BIGINT PRIMARY KEY AUTO_INCREMENT,
            tournament_id BIGINT NOT NULL,
            team_id BIGINT NOT NULL,
            member_id BIGINT NOT NULL,
            category_id BIGINT NOT NULL,
            UNIQUE KEY idx_member_categories_unique (tournament_id, member_id, category_id)
        )",
    )?;

    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS team_member_events (
            id BIGINT PRIMARY KEY AUTO_INCREMENT,
            tournament_id BIGINT NOT NULL,
            team_id BIGINT NOT NULL,
            member_id BIGINT NOT NULL,
            event_id BIGINT NOT NULL,
            UNIQUE KEY idx_member_events_unique (tournament_id, member_id, event_id)
        )",
    )?;

    Ok(())
}

pub fn open_conn(pool: &Pool) -> mysql::Result<PooledConn> {
    pool.get_conn()
}
