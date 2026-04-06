use mysql::prelude::*;
use mysql::Pool;
use std::collections::HashSet;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "mysql://root@127.0.0.1:3306/pulse-db".to_string());
    let pool = Pool::new(db_url.as_str())?;
    let mut conn = pool.get_conn()?;

    let tournament: (i64, String) = conn
        .exec_first(
            "SELECT id, name FROM tournaments ORDER BY id DESC LIMIT 1",
            (),
        )?
        .ok_or("No tournaments found. Create one first.")?;
    let tournament_id = tournament.0;

    let categories: Vec<(i64, String)> = conn.exec_map(
        "SELECT id, name FROM categories WHERE tournament_id = ?",
        (tournament_id,),
        |(id, name)| (id, name),
    )?;
    let divisions: Vec<(i64, String)> = conn.exec_map(
        "SELECT id, name FROM divisions WHERE tournament_id = ?",
        (tournament_id,),
        |(id, name)| (id, name),
    )?;
    let events: Vec<(i64, String)> = conn.exec_map(
        "SELECT id, name FROM events WHERE tournament_id = ?",
        (tournament_id,),
        |(id, name)| (id, name),
    )?;
    let weight_classes: Vec<(i64, String)> = conn.exec_map(
        "SELECT id, name FROM weight_classes WHERE tournament_id = ?",
        (tournament_id,),
        |(id, name)| (id, name),
    )?;

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

    let existing_team_ids: Vec<i64> = conn.exec_map(
        "SELECT id FROM teams WHERE tournament_id = ? ORDER BY id",
        (tournament_id,),
        |id| id,
    )?;

    let mut team_ids = Vec::new();
    for (idx, name) in team_name_pool.iter().enumerate() {
        let team_id = if let Some(existing_id) = existing_team_ids.get(idx) {
            conn.exec_drop(
                "UPDATE teams SET name = ? WHERE id = ? AND tournament_id = ?",
                (name, existing_id, tournament_id),
            )?;
            *existing_id
        } else {
            conn.exec_drop(
                "INSERT INTO teams (tournament_id, name) VALUES (?, ?)",
                (tournament_id, name),
            )?;
            conn.last_insert_id() as i64
        };
        team_ids.push(team_id);

        let team_division_set: HashSet<i64> = conn
            .exec_map(
            "SELECT division_id FROM team_divisions WHERE tournament_id = ? AND team_id = ?",
            (tournament_id, team_id),
            |division_id| division_id,
        )?
            .into_iter()
            .collect();
        for (division_id, _) in &divisions {
            if !team_division_set.contains(division_id) {
                conn.exec_drop(
                    "INSERT INTO team_divisions (tournament_id, team_id, division_id) VALUES (?, ?, ?)",
                    (tournament_id, team_id, division_id),
                )?;
            }
        }

        let team_category_set: HashSet<i64> = conn
            .exec_map(
            "SELECT category_id FROM team_categories WHERE tournament_id = ? AND team_id = ?",
            (tournament_id, team_id),
            |category_id| category_id,
        )?
            .into_iter()
            .collect();
        for (cat_id, _) in &categories {
            if !team_category_set.contains(cat_id) {
                conn.exec_drop(
                    "INSERT INTO team_categories (tournament_id, team_id, category_id) VALUES (?, ?, ?)",
                    (tournament_id, team_id, cat_id),
                )?;
            }
        }

        let team_event_set: HashSet<i64> = conn
            .exec_map(
            "SELECT event_id FROM team_events WHERE tournament_id = ? AND team_id = ?",
            (tournament_id, team_id),
            |event_id| event_id,
        )?
            .into_iter()
            .collect();
        for (event_id, _) in &events {
            if !team_event_set.contains(event_id) {
                conn.exec_drop(
                    "INSERT INTO team_events (tournament_id, team_id, event_id) VALUES (?, ?, ?)",
                    (tournament_id, team_id, event_id),
                )?;
            }
        }
    }

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
        let mut member_ids: Vec<i64> = conn.exec_map(
            "SELECT id FROM team_members WHERE tournament_id = ? AND team_id = ? ORDER BY id",
            (tournament_id, team_id),
            |id| id,
        )?;

        let extra = (team_idx % 7) as i64;
        let target_players = 10 + extra;

        while member_ids.len() < target_players as usize {
            conn.exec_drop(
                "INSERT INTO team_members (tournament_id, team_id, name, weight_class, weight_class_id) VALUES (?, ?, ?, ?, ?)",
                (
                    tournament_id,
                    team_id,
                    "Temp Player",
                    &weight_classes[0].1,
                    weight_classes[0].0,
                ),
            )?;
            member_ids.push(conn.last_insert_id() as i64);
        }

        for member_id in member_ids.iter() {
            player_index += 1;
            let first = &first_names[(player_index - 1) % first_names.len()];
            let last = &last_names[(player_index - 1 + team_idx) % last_names.len()];
            let name = format!("{} {}", first, last);
            let (weight_id, weight_name) = &weight_classes[(player_index - 1) % weight_classes.len()];
            let division_id = divisions[(player_index - 1) % divisions.len()].0;
            conn.exec_drop(
                "UPDATE team_members SET name = ?, weight_class = ?, weight_class_id = ?, division_id = ? WHERE id = ? AND tournament_id = ?",
                (name, weight_name, weight_id, division_id, member_id, tournament_id),
            )?;

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

            conn.exec_drop(
                "DELETE FROM team_member_categories WHERE tournament_id = ? AND member_id = ?",
                (tournament_id, member_id),
            )?;
            conn.exec_drop(
                "DELETE FROM team_member_events WHERE tournament_id = ? AND member_id = ?",
                (tournament_id, member_id),
            )?;
            for cat_id in cat_ids.into_iter().collect::<HashSet<_>>() {
                conn.exec_drop(
                    "INSERT INTO team_member_categories (tournament_id, team_id, member_id, category_id) VALUES (?, ?, ?, ?)",
                    (tournament_id, team_id, member_id, cat_id),
                )?;
            }
            for event_id in event_ids.into_iter().collect::<HashSet<_>>() {
                conn.exec_drop(
                    "INSERT INTO team_member_events (tournament_id, team_id, member_id, event_id) VALUES (?, ?, ?, ?)",
                    (tournament_id, team_id, member_id, event_id),
                )?;
            }
        }
    }

    println!("Seeded tournament {} with realistic team and player names.", tournament_id);
    Ok(())
}
