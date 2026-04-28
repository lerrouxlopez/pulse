use mysql::prelude::*;
use mysql::Pool;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root@127.0.0.1:3306/pulse-db".to_string());
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        eprintln!("Usage: db_check_scores <tournament_id> <event_id> <division_id> <weight_class_id>");
        std::process::exit(2);
    }
    let mut tournament_id: i64 = args[1].parse()?;
    let event_id: i64 = args[2].parse()?;
    let division_id: i64 = args[3].parse()?;
    let weight_class_id: i64 = args[4].parse()?;

    let pool = Pool::new(db_url.as_str())?;
    let mut conn = pool.get_conn()?;

    if tournament_id == 0 {
        tournament_id = conn
            .exec_first::<(i64,), _, _>(
                "SELECT id FROM tournaments ORDER BY id DESC LIMIT 1",
                (),
            )?
            .map(|row| row.0)
            .unwrap_or(0);
        println!("tournament_id resolved to {}", tournament_id);
    }

    let scheduled: Vec<(i64, String)> = conn.exec_map(
        "SELECT id, COALESCE(contact_type,'')
         FROM scheduled_events
         WHERE tournament_id = ? AND event_id = ? AND division_id = ? AND weight_class_id = ?
         ORDER BY id",
        (tournament_id, event_id, division_id, weight_class_id),
        |(id, contact_type)| (id, contact_type),
    )?;

    println!("scheduled_events found: {}", scheduled.len());
    for (se_id, contact_type) in &scheduled {
        let matches_total: i64 = conn
            .exec_first(
                "SELECT COALESCE(COUNT(*),0) FROM matches WHERE tournament_id=? AND scheduled_event_id=?",
                (tournament_id, se_id),
            )?
            .unwrap_or(0);
        let matches_open: i64 = conn
            .exec_first(
                "SELECT COALESCE(COUNT(*),0)
                 FROM matches
                 WHERE tournament_id=? AND scheduled_event_id=?
                   AND COALESCE(is_bye,0)=0
                   AND NOT (LOWER(COALESCE(status,''))='finished' OR LOWER(COALESCE(status,''))='forfeit')",
                (tournament_id, se_id),
            )?
            .unwrap_or(0);
        let sej: i64 = conn
            .exec_first(
                "SELECT COALESCE(COUNT(DISTINCT judge_user_id),0)
                 FROM scheduled_event_judges
                 WHERE tournament_id=? AND scheduled_event_id=?",
                (tournament_id, se_id),
            )?
            .unwrap_or(0);
        let mj: i64 = conn
            .exec_first(
                "SELECT COALESCE(COUNT(DISTINCT judge_user_id),0)
                 FROM match_judges mj
                 JOIN matches m ON m.id=mj.match_id AND m.tournament_id=mj.tournament_id
                 WHERE mj.tournament_id=? AND m.scheduled_event_id=?",
                (tournament_id, se_id),
            )?
            .unwrap_or(0);

        println!(
            "scheduled_event_id={} contact_type='{}' matches_total={} matches_open={} scheduled_event_judges={} match_judges={}",
            se_id, contact_type, matches_total, matches_open, sej, mj
        );
    }

    Ok(())
}
