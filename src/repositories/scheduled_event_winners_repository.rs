use mysql::prelude::*;
use mysql::{params, PooledConn};

pub fn replace_winners(
    conn: &mut PooledConn,
    tournament_id: i64,
    scheduled_event_id: i64,
    winner_member_ids: &[i64],
) -> mysql::Result<()> {
    conn.exec_drop(
        "DELETE FROM scheduled_event_winners WHERE tournament_id = :tournament_id AND scheduled_event_id = :scheduled_event_id",
        params! {
            "tournament_id" => tournament_id,
            "scheduled_event_id" => scheduled_event_id,
        },
    )?;
    for member_id in winner_member_ids {
        conn.exec_drop(
            "INSERT INTO scheduled_event_winners (tournament_id, scheduled_event_id, winner_member_id)
             VALUES (:tournament_id, :scheduled_event_id, :winner_member_id)",
            params! {
                "tournament_id" => tournament_id,
                "scheduled_event_id" => scheduled_event_id,
                "winner_member_id" => member_id,
            },
        )?;
    }
    Ok(())
}

pub fn list_winner_member_ids(
    conn: &mut PooledConn,
    tournament_id: i64,
    scheduled_event_id: i64,
) -> mysql::Result<Vec<i64>> {
    conn.exec_map(
        "SELECT winner_member_id
         FROM scheduled_event_winners
         WHERE tournament_id = :tournament_id AND scheduled_event_id = :scheduled_event_id
         ORDER BY winner_member_id ASC",
        params! {
            "tournament_id" => tournament_id,
            "scheduled_event_id" => scheduled_event_id,
        },
        |id| id,
    )
}

pub fn list_all_winners_for_tournament(
    conn: &mut PooledConn,
    tournament_id: i64,
) -> mysql::Result<Vec<(i64, i64)>> {
    conn.exec_map(
        "SELECT scheduled_event_id, winner_member_id
         FROM scheduled_event_winners
         WHERE tournament_id = :tournament_id",
        params! {
            "tournament_id" => tournament_id,
        },
        |(scheduled_event_id, winner_member_id)| (scheduled_event_id, winner_member_id),
    )
}
