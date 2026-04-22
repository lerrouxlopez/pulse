use mysql::prelude::*;
use mysql::{Pool, PooledConn};

pub fn init_db(pool: &Pool) -> mysql::Result<()> {
    let mut conn = pool.get_conn()?;

    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            id VARCHAR(255) PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
        )",
    )?;

    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS users (
            id BIGINT PRIMARY KEY AUTO_INCREMENT,
            name TEXT NOT NULL,
            email TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            user_type TEXT NOT NULL DEFAULT 'system',
            tournament_id BIGINT NOT NULL DEFAULT 0,
            photo_url TEXT,
            created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
            UNIQUE KEY idx_users_unique (tournament_id, email(191)),
            KEY idx_users_type (user_type),
            KEY idx_users_tournament_id (tournament_id)
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
        "CREATE TABLE IF NOT EXISTS tournament_roles (
            id BIGINT PRIMARY KEY AUTO_INCREMENT,
            tournament_id BIGINT NOT NULL,
            name TEXT NOT NULL,
            is_owner TINYINT(1) NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
        )",
    )?;

    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS role_permissions (
            id BIGINT PRIMARY KEY AUTO_INCREMENT,
            role_id BIGINT NOT NULL,
            permission_key TEXT NOT NULL
        )",
    )?;

    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS tournament_user_roles (
            id BIGINT PRIMARY KEY AUTO_INCREMENT,
            tournament_id BIGINT NOT NULL,
            user_id BIGINT NOT NULL,
            role_id BIGINT NOT NULL,
            UNIQUE KEY idx_tournament_user_roles_unique (tournament_id, user_id)
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
            weight_class_id BIGINT,
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
            point_system TEXT,
            time_rule TEXT,
            draw_system TEXT,
            division_id BIGINT,
            weight_class_id BIGINT,
            winner_member_id BIGINT,
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
            red_total_score INT NOT NULL DEFAULT 0,
            blue_total_score INT NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
        )",
    )?;

    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS match_judges (
            id BIGINT PRIMARY KEY AUTO_INCREMENT,
            tournament_id BIGINT NOT NULL,
            match_id BIGINT NOT NULL,
            judge_user_id BIGINT NOT NULL,
            fight_round BIGINT NOT NULL DEFAULT 1,
            judge_order INT NOT NULL DEFAULT 0,
            red_score INT NOT NULL DEFAULT 0,
            blue_score INT NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
            UNIQUE KEY idx_match_judges_match_judge_round (match_id, judge_user_id, fight_round),
            UNIQUE KEY idx_match_judges_match_order_round (match_id, judge_order, fight_round)
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

    apply_user_roles_redo_migration(&mut conn)?;
    apply_match_scoring_migration(&mut conn)?;
    apply_match_timer_migration(&mut conn)?;
    apply_match_timer_round_lock_migration(&mut conn)?;
    apply_match_judge_round_scores_migration(&mut conn)?;
    apply_scores_permission_migration(&mut conn)?;
    apply_draw_system_migration(&mut conn)?;
    apply_contact_pause_vote_scoring_migration(&mut conn)?;
    apply_non_contact_performances_migration(&mut conn)?;

    Ok(())
}

pub fn open_conn(pool: &Pool) -> mysql::Result<PooledConn> {
    pool.get_conn()
}

fn apply_user_roles_redo_migration(conn: &mut PooledConn) -> mysql::Result<()> {
    let migration_id = "20260407_user_roles_redo";
    if migration_applied(conn, migration_id)? {
        return Ok(());
    }

    if column_exists(conn, "users", "user_type")? == false {
        conn.query_drop("ALTER TABLE users ADD COLUMN user_type TEXT NOT NULL DEFAULT 'system'")?;
    }
    if column_exists(conn, "users", "tournament_id")? == false {
        conn.query_drop("ALTER TABLE users ADD COLUMN tournament_id BIGINT NOT NULL DEFAULT 0")?;
    }

    drop_unique_index_on_email(conn)?;

    if !index_exists(conn, "users", "idx_users_unique")? {
        conn.query_drop(
            "CREATE UNIQUE INDEX idx_users_unique ON users (tournament_id, email(191))",
        )?;
    }
    if !index_exists(conn, "users", "idx_users_type")? {
        conn.query_drop("CREATE INDEX idx_users_type ON users (user_type)")?;
    }
    if !index_exists(conn, "users", "idx_users_tournament_id")? {
        conn.query_drop("CREATE INDEX idx_users_tournament_id ON users (tournament_id)")?;
    }

    if table_exists(conn, "tournament_users")? {
        conn.query_drop(
            "CREATE TEMPORARY TABLE tmp_tournament_users
             SELECT user_id, COUNT(*) AS cnt, MIN(tournament_id) AS tournament_id
             FROM tournament_users
             GROUP BY user_id",
        )?;
        conn.query_drop(
            "UPDATE users u
             LEFT JOIN tournaments t ON t.user_id = u.id
             LEFT JOIN tmp_tournament_users tu ON tu.user_id = u.id
             SET u.user_type = CASE
                 WHEN t.id IS NOT NULL THEN 'system'
                 WHEN tu.cnt = 1 THEN 'tournament'
                 ELSE 'system'
             END,
             u.tournament_id = CASE
                 WHEN t.id IS NOT NULL THEN 0
                 WHEN tu.cnt = 1 THEN tu.tournament_id
                 ELSE 0
             END",
        )?;
        conn.query_drop("DROP TABLE IF EXISTS tournament_users")?;
    } else {
        conn.query_drop(
            "UPDATE users SET user_type = 'system' WHERE user_type IS NULL OR user_type = ''",
        )?;
        conn.query_drop("UPDATE users SET tournament_id = 0 WHERE tournament_id IS NULL")?;
    }

    conn.exec_drop(
        "INSERT INTO schema_migrations (id) VALUES (?)",
        (migration_id,),
    )?;
    Ok(())
}

fn migration_applied(conn: &mut PooledConn, migration_id: &str) -> mysql::Result<bool> {
    let row: Option<String> = conn.exec_first(
        "SELECT id FROM schema_migrations WHERE id = ?",
        (migration_id,),
    )?;
    Ok(row.is_some())
}

fn table_exists(conn: &mut PooledConn, table: &str) -> mysql::Result<bool> {
    let row: Option<String> = conn.exec_first(
        "SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ?",
        (table,),
    )?;
    Ok(row.is_some())
}

fn column_exists(conn: &mut PooledConn, table: &str, column: &str) -> mysql::Result<bool> {
    let row: Option<String> = conn.exec_first(
        "SELECT COLUMN_NAME FROM INFORMATION_SCHEMA.COLUMNS WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? AND COLUMN_NAME = ?",
        (table, column),
    )?;
    Ok(row.is_some())
}

fn index_exists(conn: &mut PooledConn, table: &str, index: &str) -> mysql::Result<bool> {
    let row: Option<String> = conn.exec_first(
        "SELECT INDEX_NAME FROM INFORMATION_SCHEMA.STATISTICS WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? AND INDEX_NAME = ? LIMIT 1",
        (table, index),
    )?;
    Ok(row.is_some())
}

fn drop_unique_index_on_email(conn: &mut PooledConn) -> mysql::Result<()> {
    let row: Option<String> = conn.exec_first(
        "SELECT INDEX_NAME FROM INFORMATION_SCHEMA.STATISTICS
         WHERE TABLE_SCHEMA = DATABASE()
           AND TABLE_NAME = 'users'
           AND COLUMN_NAME = 'email'
           AND NON_UNIQUE = 0
         LIMIT 1",
        (),
    )?;
    if let Some(index_name) = row {
        if index_name != "PRIMARY" && index_name != "idx_users_unique" {
            conn.query_drop(format!("ALTER TABLE users DROP INDEX {}", index_name))?;
        }
    }
    Ok(())
}

fn apply_match_scoring_migration(conn: &mut PooledConn) -> mysql::Result<()> {
    let migration_id = "20260414_match_scoring_and_user_photos";
    if migration_applied(conn, migration_id)? {
        return Ok(());
    }

    if !column_exists(conn, "users", "photo_url")? {
        conn.query_drop("ALTER TABLE users ADD COLUMN photo_url TEXT")?;
    }
    if !column_exists(conn, "matches", "red_total_score")? {
        conn.query_drop("ALTER TABLE matches ADD COLUMN red_total_score INT NOT NULL DEFAULT 0")?;
    }
    if !column_exists(conn, "matches", "blue_total_score")? {
        conn.query_drop("ALTER TABLE matches ADD COLUMN blue_total_score INT NOT NULL DEFAULT 0")?;
    }
    if !table_exists(conn, "match_judges")? {
        conn.query_drop(
            "CREATE TABLE match_judges (
                id BIGINT PRIMARY KEY AUTO_INCREMENT,
                tournament_id BIGINT NOT NULL,
                match_id BIGINT NOT NULL,
                judge_user_id BIGINT NOT NULL,
                fight_round BIGINT NOT NULL DEFAULT 1,
                judge_order INT NOT NULL DEFAULT 0,
                red_score INT NOT NULL DEFAULT 0,
                blue_score INT NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
                UNIQUE KEY idx_match_judges_match_judge_round (match_id, judge_user_id, fight_round),
                UNIQUE KEY idx_match_judges_match_order_round (match_id, judge_order, fight_round)
            )",
        )?;
    }

    conn.exec_drop(
        "INSERT INTO schema_migrations (id) VALUES (?)",
        (migration_id,),
    )?;
    Ok(())
}

fn apply_match_judge_round_scores_migration(conn: &mut PooledConn) -> mysql::Result<()> {
    let migration_id = "20260417_match_judge_round_scores";
    if migration_applied(conn, migration_id)? {
        return Ok(());
    }

    if !column_exists(conn, "match_judges", "fight_round")? {
        conn.query_drop(
            "ALTER TABLE match_judges ADD COLUMN fight_round BIGINT NOT NULL DEFAULT 1",
        )?;
    }

    // Replace unique keys so judges can have one score per round.
    if index_exists(conn, "match_judges", "idx_match_judges_match_judge")? {
        conn.query_drop("DROP INDEX idx_match_judges_match_judge ON match_judges")?;
    }
    if index_exists(conn, "match_judges", "idx_match_judges_match_order")? {
        conn.query_drop("DROP INDEX idx_match_judges_match_order ON match_judges")?;
    }
    if !index_exists(conn, "match_judges", "idx_match_judges_match_judge_round")? {
        conn.query_drop(
            "CREATE UNIQUE INDEX idx_match_judges_match_judge_round ON match_judges (match_id, judge_user_id, fight_round)",
        )?;
    }
    if !index_exists(conn, "match_judges", "idx_match_judges_match_order_round")? {
        conn.query_drop(
            "CREATE UNIQUE INDEX idx_match_judges_match_order_round ON match_judges (match_id, judge_order, fight_round)",
        )?;
    }

    conn.exec_drop(
        "INSERT INTO schema_migrations (id) VALUES (?)",
        (migration_id,),
    )?;
    Ok(())
}

fn apply_scores_permission_migration(conn: &mut PooledConn) -> mysql::Result<()> {
    let migration_id = "20260417_scores_permission";
    if migration_applied(conn, migration_id)? {
        return Ok(());
    }

    // Ensure all Owner roles can access the Scores page.
    conn.query_drop(
        "INSERT INTO role_permissions (role_id, permission_key)
         SELECT tr.id, 'scores'
         FROM tournament_roles tr
         LEFT JOIN role_permissions rp
           ON rp.role_id = tr.id AND LOWER(rp.permission_key) = 'scores'
         WHERE tr.is_owner = 1 AND rp.id IS NULL",
    )?;

    conn.exec_drop(
        "INSERT INTO schema_migrations (id) VALUES (?)",
        (migration_id,),
    )?;
    Ok(())
}

fn apply_match_timer_migration(conn: &mut PooledConn) -> mysql::Result<()> {
    let migration_id = "20260417_match_timer_and_round";
    if migration_applied(conn, migration_id)? {
        return Ok(());
    }

    if !column_exists(conn, "matches", "fight_round")? {
        conn.query_drop("ALTER TABLE matches ADD COLUMN fight_round BIGINT")?;
    }
    if !column_exists(conn, "matches", "timer_started_at")? {
        conn.query_drop("ALTER TABLE matches ADD COLUMN timer_started_at BIGINT")?;
    }
    if !column_exists(conn, "matches", "timer_duration_seconds")? {
        conn.query_drop("ALTER TABLE matches ADD COLUMN timer_duration_seconds INT")?;
    }
    if !column_exists(conn, "matches", "timer_is_running")? {
        conn.query_drop(
            "ALTER TABLE matches ADD COLUMN timer_is_running TINYINT(1) NOT NULL DEFAULT 0",
        )?;
    }

    conn.exec_drop(
        "INSERT INTO schema_migrations (id) VALUES (?)",
        (migration_id,),
    )?;
    Ok(())
}

fn apply_match_timer_round_lock_migration(conn: &mut PooledConn) -> mysql::Result<()> {
    let migration_id = "20260417_match_timer_round_lock";
    if migration_applied(conn, migration_id)? {
        return Ok(());
    }

    if !column_exists(conn, "matches", "timer_last_completed_round")? {
        conn.query_drop("ALTER TABLE matches ADD COLUMN timer_last_completed_round BIGINT")?;
    }

    conn.exec_drop(
        "INSERT INTO schema_migrations (id) VALUES (?)",
        (migration_id,),
    )?;
    Ok(())
}

fn apply_draw_system_migration(conn: &mut PooledConn) -> mysql::Result<()> {
    let migration_id = "20260418_draw_system";
    if migration_applied(conn, migration_id)? {
        return Ok(());
    }

    if !column_exists(conn, "scheduled_events", "draw_system")? {
        conn.query_drop("ALTER TABLE scheduled_events ADD COLUMN draw_system TEXT")?;
    }

    conn.exec_drop(
        "INSERT INTO schema_migrations (id) VALUES (?)",
        (migration_id,),
    )?;
    Ok(())
}

fn apply_contact_pause_vote_scoring_migration(conn: &mut PooledConn) -> mysql::Result<()> {
    let migration_id = "20260422_contact_pause_vote_scoring";
    if migration_applied(conn, migration_id)? {
        return Ok(());
    }

    if !table_exists(conn, "match_pause_vote_events")? {
        conn.query_drop(
            "CREATE TABLE match_pause_vote_events (
                id BIGINT PRIMARY KEY AUTO_INCREMENT,
                tournament_id BIGINT NOT NULL,
                match_id BIGINT NOT NULL,
                fight_round BIGINT NOT NULL DEFAULT 1,
                pause_seq BIGINT NOT NULL DEFAULT 1,
                winner_side TEXT,
                applied_at TEXT,
                created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
                UNIQUE KEY idx_pause_vote_events_unique (match_id, fight_round, pause_seq),
                KEY idx_pause_vote_events_match (match_id),
                KEY idx_pause_vote_events_tournament (tournament_id)
            )",
        )?;
    }

    if !table_exists(conn, "match_pause_votes")? {
        conn.query_drop(
            "CREATE TABLE match_pause_votes (
                id BIGINT PRIMARY KEY AUTO_INCREMENT,
                tournament_id BIGINT NOT NULL,
                match_id BIGINT NOT NULL,
                fight_round BIGINT NOT NULL DEFAULT 1,
                pause_seq BIGINT NOT NULL DEFAULT 1,
                judge_user_id BIGINT NOT NULL,
                side TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
                UNIQUE KEY idx_pause_votes_unique (match_id, fight_round, pause_seq, judge_user_id),
                KEY idx_pause_votes_match (match_id),
                KEY idx_pause_votes_tournament (tournament_id)
            )",
        )?;
    }

    conn.exec_drop(
        "INSERT INTO schema_migrations (id) VALUES (?)",
        (migration_id,),
    )?;
    Ok(())
}

fn apply_non_contact_performances_migration(conn: &mut PooledConn) -> mysql::Result<()> {
    let migration_id = "20260422_non_contact_performances";
    if migration_applied(conn, migration_id)? {
        return Ok(());
    }

    if !table_exists(conn, "scheduled_event_judges")? {
        conn.query_drop(
            "CREATE TABLE scheduled_event_judges (
                id BIGINT PRIMARY KEY AUTO_INCREMENT,
                tournament_id BIGINT NOT NULL,
                scheduled_event_id BIGINT NOT NULL,
                judge_user_id BIGINT NOT NULL,
                judge_order INT NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
                UNIQUE KEY idx_scheduled_event_judges_unique (scheduled_event_id, judge_user_id),
                UNIQUE KEY idx_scheduled_event_judges_order (scheduled_event_id, judge_order),
                KEY idx_scheduled_event_judges_event (scheduled_event_id),
                KEY idx_scheduled_event_judges_tournament (tournament_id)
            )",
        )?;
    }

    if !table_exists(conn, "scheduled_event_winners")? {
        conn.query_drop(
            "CREATE TABLE scheduled_event_winners (
                id BIGINT PRIMARY KEY AUTO_INCREMENT,
                tournament_id BIGINT NOT NULL,
                scheduled_event_id BIGINT NOT NULL,
                winner_member_id BIGINT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
                UNIQUE KEY idx_scheduled_event_winners_unique (scheduled_event_id, winner_member_id),
                KEY idx_scheduled_event_winners_event (scheduled_event_id),
                KEY idx_scheduled_event_winners_tournament (tournament_id)
            )",
        )?;
    }

    conn.exec_drop(
        "INSERT INTO schema_migrations (id) VALUES (?)",
        (migration_id,),
    )?;
    Ok(())
}
