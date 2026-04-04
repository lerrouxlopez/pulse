use rusqlite::{params, Connection};
use std::collections::HashSet;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = PathBuf::from("data/pulse.db");
    let conn = Connection::open(db_path)?;
    let _ = conn.execute("ALTER TABLE team_members ADD COLUMN division_id INTEGER", []);

    let tournament = conn
        .query_row(
            "SELECT id, name FROM tournaments ORDER BY id DESC LIMIT 1",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .map_err(|_| "No tournaments found. Create one first.")?;
    let tournament_id = tournament.0;

    let mut stmt = conn.prepare("SELECT id, name FROM categories WHERE tournament_id = ?1")?;
    let categories: Vec<(i64, String)> = stmt
        .query_map(params![tournament_id], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let mut stmt = conn.prepare("SELECT id, name FROM divisions WHERE tournament_id = ?1")?;
    let divisions: Vec<(i64, String)> = stmt
        .query_map(params![tournament_id], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let mut stmt = conn.prepare("SELECT id, name FROM events WHERE tournament_id = ?1")?;
    let events: Vec<(i64, String)> = stmt
        .query_map(params![tournament_id], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let mut stmt = conn.prepare("SELECT name FROM weight_classes WHERE tournament_id = ?1")?;
    let weight_classes: Vec<String> = stmt
        .query_map(params![tournament_id], |row| Ok(row.get(0)?))?
        .collect::<Result<_, _>>()?;

    if weight_classes.is_empty() {
        return Err("No weight classes found. Add weight classes first.".into());
    }
    if divisions.is_empty() {
        return Err("No divisions found. Add divisions first.".into());
    }

    let team_name_pool = vec![
        "North Harbor Martial Arts",
        "Iron Lotus Dojo",
        "Sakura Guard Academy",
        "Atlas Combat Club",
        "Riverstone Kendo",
        "Golden Crane School",
        "Eastwind Arnis",
        "Summit Blade Collective",
        "Blue Lantern FMA",
        "Redwood Warriors",
    ];

    let mut stmt = conn.prepare("SELECT id FROM teams WHERE tournament_id = ?1 ORDER BY id")?;
    let existing_team_ids: Vec<i64> = stmt
        .query_map(params![tournament_id], |row| row.get(0))?
        .collect::<Result<_, _>>()?;

    let mut team_ids = Vec::new();
    for (idx, name) in team_name_pool.iter().enumerate() {
        let team_id = if let Some(existing_id) = existing_team_ids.get(idx) {
            conn.execute(
                "UPDATE teams SET name = ?1 WHERE id = ?2 AND tournament_id = ?3",
                params![name, existing_id, tournament_id],
            )?;
            *existing_id
        } else {
            conn.execute(
                "INSERT INTO teams (tournament_id, name) VALUES (?1, ?2)",
                params![tournament_id, name],
            )?;
            conn.last_insert_rowid()
        };
        team_ids.push(team_id);

        // Ensure all categories/events are assigned to the team
        let mut team_division_set = HashSet::new();
        let mut stmt = conn.prepare(
            "SELECT division_id FROM team_divisions WHERE tournament_id = ?1 AND team_id = ?2",
        )?;
        for row in stmt.query_map(params![tournament_id, team_id], |row| row.get::<_, i64>(0))? {
            team_division_set.insert(row?);
        }
        for (division_id, _) in &divisions {
            if !team_division_set.contains(division_id) {
                conn.execute(
                    "INSERT INTO team_divisions (tournament_id, team_id, division_id) VALUES (?1, ?2, ?3)",
                    params![tournament_id, team_id, division_id],
                )?;
            }
        }
        let mut team_category_set = HashSet::new();
        let mut stmt = conn.prepare(
            "SELECT category_id FROM team_categories WHERE tournament_id = ?1 AND team_id = ?2",
        )?;
        for row in stmt.query_map(params![tournament_id, team_id], |row| row.get::<_, i64>(0))? {
            team_category_set.insert(row?);
        }
        for (cat_id, _) in &categories {
            if !team_category_set.contains(cat_id) {
                conn.execute(
                    "INSERT INTO team_categories (tournament_id, team_id, category_id) VALUES (?1, ?2, ?3)",
                    params![tournament_id, team_id, cat_id],
                )?;
            }
        }

        let mut team_event_set = HashSet::new();
        let mut stmt =
            conn.prepare("SELECT event_id FROM team_events WHERE tournament_id = ?1 AND team_id = ?2")?;
        for row in stmt.query_map(params![tournament_id, team_id], |row| row.get::<_, i64>(0))? {
            team_event_set.insert(row?);
        }
        for (event_id, _) in &events {
            if !team_event_set.contains(event_id) {
                conn.execute(
                    "INSERT INTO team_events (tournament_id, team_id, event_id) VALUES (?1, ?2, ?3)",
                    params![tournament_id, team_id, event_id],
                )?;
            }
        }
    }

    // Create or update players (>=10 per team) and ensure coverage of weight/category/event options.
    let mut player_index: usize = 0;
    let first_names = vec![
        "Aiden", "Bella", "Carlos", "Diana", "Eli", "Farah", "Gabe", "Hana", "Ivan", "Jade",
        "Kai", "Luna", "Milo", "Nina", "Owen", "Pia", "Quinn", "Ravi", "Sora", "Tara", "Uma",
        "Vince", "Wren", "Xavi", "Yara", "Zane", "Maya", "Jules", "Rina", "Theo",
    ];
    let last_names = vec![
        "Lopez", "Santos", "Cruz", "Reyes", "Garcia", "Nguyen", "Kim", "Patel", "Torres", "Diaz",
        "Lee", "Wong", "Khan", "Tan", "Lim", "Singh", "Park", "Choi", "Ahmed", "Ali", "Bautista",
        "Castro", "Delos Reyes", "Flores", "Hernandez", "Ibrahim", "Jensen", "King",
    ];

    for (team_idx, team_id) in team_ids.iter().enumerate() {
        let mut stmt = conn.prepare(
            "SELECT id FROM team_members WHERE tournament_id = ?1 AND team_id = ?2 ORDER BY id",
        )?;
        let mut member_ids: Vec<i64> = stmt
            .query_map(params![tournament_id, team_id], |row| row.get(0))?
            .collect::<Result<_, _>>()?;

        let extra = (team_idx % 7) as i64; // 0..6
        let target_players = 10 + extra;

        while member_ids.len() < target_players as usize {
            conn.execute(
                "INSERT INTO team_members (tournament_id, team_id, name, weight_class) VALUES (?1, ?2, ?3, ?4)",
                params![tournament_id, team_id, "Temp Player", weight_classes[0]],
            )?;
            member_ids.push(conn.last_insert_rowid());
        }

        for member_id in member_ids.iter() {
            player_index += 1;
            let first = &first_names[(player_index - 1) % first_names.len()];
            let last = &last_names[(player_index - 1 + team_idx) % last_names.len()];
            let name = format!("{} {}", first, last);
            let weight = &weight_classes[(player_index - 1) % weight_classes.len()];
            let division_id = divisions[(player_index - 1) % divisions.len()].0;
            conn.execute(
                "UPDATE team_members SET name = ?1, weight_class = ?2, division_id = ?3 WHERE id = ?4 AND tournament_id = ?5",
                params![name, weight, division_id, member_id, tournament_id],
            )?;

            // Assign categories/events (cycle to guarantee coverage)
            let mut cat_ids = Vec::new();
            if !categories.is_empty() {
                cat_ids.push(categories[(player_index - 1) % categories.len()].0);
                if categories.len() > 1 {
                    cat_ids.push(categories[player_index % categories.len()].0);
                }
            }
            let mut event_ids = Vec::new();
            if !events.is_empty() {
                event_ids.push(events[(player_index - 1) % events.len()].0);
                if events.len() > 1 {
                    event_ids.push(events[player_index % events.len()].0);
                }
            }

            conn.execute(
                "DELETE FROM team_member_categories WHERE tournament_id = ?1 AND member_id = ?2",
                params![tournament_id, member_id],
            )?;
            conn.execute(
                "DELETE FROM team_member_events WHERE tournament_id = ?1 AND member_id = ?2",
                params![tournament_id, member_id],
            )?;
            for cat_id in cat_ids.into_iter().collect::<HashSet<_>>() {
                conn.execute(
                    "INSERT INTO team_member_categories (tournament_id, team_id, member_id, category_id) VALUES (?1, ?2, ?3, ?4)",
                    params![tournament_id, team_id, member_id, cat_id],
                )?;
            }
            for event_id in event_ids.into_iter().collect::<HashSet<_>>() {
                conn.execute(
                    "INSERT INTO team_member_events (tournament_id, team_id, member_id, event_id) VALUES (?1, ?2, ?3, ?4)",
                    params![tournament_id, team_id, member_id, event_id],
                )?;
            }
        }
    }

    println!("Seeded tournament {} with realistic team and player names.", tournament_id);
    Ok(())
}
