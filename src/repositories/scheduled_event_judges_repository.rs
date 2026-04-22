use mysql::prelude::*;
use mysql::{params, PooledConn};

pub fn list_assigned_judges(
    conn: &mut PooledConn,
    tournament_id: i64,
    scheduled_event_id: i64,
) -> mysql::Result<Vec<i64>> {
    conn.exec_map(
        "SELECT sej.judge_user_id
         FROM scheduled_event_judges sej
         WHERE sej.tournament_id = :tournament_id AND sej.scheduled_event_id = :scheduled_event_id
         ORDER BY sej.judge_order ASC, sej.id ASC",
        params! {
            "tournament_id" => tournament_id,
            "scheduled_event_id" => scheduled_event_id,
        },
        |judge_user_id| judge_user_id,
    )
}

pub fn replace_for_event(
    conn: &mut PooledConn,
    tournament_id: i64,
    scheduled_event_id: i64,
    judge_user_ids: &[i64],
) -> mysql::Result<()> {
    conn.exec_drop(
        "DELETE FROM scheduled_event_judges WHERE tournament_id = :tournament_id AND scheduled_event_id = :scheduled_event_id",
        params! {
            "tournament_id" => tournament_id,
            "scheduled_event_id" => scheduled_event_id,
        },
    )?;
    for (idx, judge_user_id) in judge_user_ids.iter().copied().enumerate() {
        conn.exec_drop(
            "INSERT INTO scheduled_event_judges (tournament_id, scheduled_event_id, judge_user_id, judge_order)
             VALUES (:tournament_id, :scheduled_event_id, :judge_user_id, :judge_order)",
            params! {
                "tournament_id" => tournament_id,
                "scheduled_event_id" => scheduled_event_id,
                "judge_user_id" => judge_user_id,
                "judge_order" => (idx as i32) + 1,
            },
        )?;
    }
    Ok(())
}

