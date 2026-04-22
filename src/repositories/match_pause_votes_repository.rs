use mysql::prelude::*;
use mysql::{params, PooledConn};

#[derive(Debug, Clone)]
pub struct PauseVoteEvent {
    pub fight_round: i64,
    pub pause_seq: i64,
    pub winner_side: Option<String>,
    pub applied_at: Option<String>,
}

pub fn create_vote_event(
    conn: &mut PooledConn,
    tournament_id: i64,
    match_id: i64,
    fight_round: i64,
    pause_seq: i64,
) -> mysql::Result<()> {
    conn.exec_drop(
        "INSERT INTO match_pause_vote_events (tournament_id, match_id, fight_round, pause_seq, winner_side, applied_at)
         VALUES (:tournament_id, :match_id, :fight_round, :pause_seq, NULL, NULL)",
        params! {
            "tournament_id" => tournament_id,
            "match_id" => match_id,
            "fight_round" => fight_round,
            "pause_seq" => pause_seq,
        },
    )?;
    Ok(())
}

pub fn next_pause_seq(
    conn: &mut PooledConn,
    tournament_id: i64,
    match_id: i64,
    fight_round: i64,
) -> mysql::Result<i64> {
    let value: Option<i64> = conn.exec_first(
        "SELECT COALESCE(MAX(e.pause_seq), 0)
         FROM match_pause_vote_events e
         WHERE e.tournament_id = :tournament_id AND e.match_id = :match_id AND e.fight_round = :fight_round",
        params! {
            "tournament_id" => tournament_id,
            "match_id" => match_id,
            "fight_round" => fight_round,
        },
    )?;
    Ok(value.unwrap_or(0) + 1)
}

pub fn latest_pending_vote_event(
    conn: &mut PooledConn,
    tournament_id: i64,
    match_id: i64,
    fight_round: i64,
) -> mysql::Result<Option<PauseVoteEvent>> {
    conn.exec_first(
        "SELECT e.fight_round, e.pause_seq, e.winner_side, e.applied_at
         FROM match_pause_vote_events e
         WHERE e.tournament_id = :tournament_id
           AND e.match_id = :match_id
           AND e.fight_round = :fight_round
           AND (e.applied_at IS NULL OR e.applied_at = '')
         ORDER BY e.pause_seq DESC
         LIMIT 1",
        params! {
            "tournament_id" => tournament_id,
            "match_id" => match_id,
            "fight_round" => fight_round,
        },
    )
    .map(|row: Option<(i64, i64, Option<String>, Option<String>)>| {
        row.map(|(fight_round, pause_seq, winner_side, applied_at)| PauseVoteEvent {
            fight_round,
            pause_seq,
            winner_side,
            applied_at,
        })
    })
}

pub fn count_votes(
    conn: &mut PooledConn,
    tournament_id: i64,
    match_id: i64,
    fight_round: i64,
    pause_seq: i64,
) -> mysql::Result<i64> {
    let value: Option<i64> = conn.exec_first(
        "SELECT COALESCE(COUNT(DISTINCT v.judge_user_id), 0)
         FROM match_pause_votes v
         WHERE v.tournament_id = :tournament_id
           AND v.match_id = :match_id
           AND v.fight_round = :fight_round
           AND v.pause_seq = :pause_seq",
        params! {
            "tournament_id" => tournament_id,
            "match_id" => match_id,
            "fight_round" => fight_round,
            "pause_seq" => pause_seq,
        },
    )?;
    Ok(value.unwrap_or(0))
}

pub fn get_vote_for_judge(
    conn: &mut PooledConn,
    tournament_id: i64,
    match_id: i64,
    fight_round: i64,
    pause_seq: i64,
    judge_user_id: i64,
) -> mysql::Result<Option<String>> {
    conn.exec_first(
        "SELECT v.side
         FROM match_pause_votes v
         WHERE v.tournament_id = :tournament_id
           AND v.match_id = :match_id
           AND v.fight_round = :fight_round
           AND v.pause_seq = :pause_seq
           AND v.judge_user_id = :judge_user_id
         LIMIT 1",
        params! {
            "tournament_id" => tournament_id,
            "match_id" => match_id,
            "fight_round" => fight_round,
            "pause_seq" => pause_seq,
            "judge_user_id" => judge_user_id,
        },
    )
}

pub fn upsert_vote(
    conn: &mut PooledConn,
    tournament_id: i64,
    match_id: i64,
    fight_round: i64,
    pause_seq: i64,
    judge_user_id: i64,
    side: &str,
) -> mysql::Result<()> {
    conn.exec_drop(
        "INSERT INTO match_pause_votes (tournament_id, match_id, fight_round, pause_seq, judge_user_id, side)
         VALUES (:tournament_id, :match_id, :fight_round, :pause_seq, :judge_user_id, :side)
         ON DUPLICATE KEY UPDATE side = :side",
        params! {
            "tournament_id" => tournament_id,
            "match_id" => match_id,
            "fight_round" => fight_round,
            "pause_seq" => pause_seq,
            "judge_user_id" => judge_user_id,
            "side" => side,
        },
    )?;
    Ok(())
}

pub fn tally_votes(
    conn: &mut PooledConn,
    tournament_id: i64,
    match_id: i64,
    fight_round: i64,
    pause_seq: i64,
) -> mysql::Result<(i64, i64)> {
    let row: Option<(i64, i64)> = conn.exec_first(
        "SELECT
            COALESCE(SUM(CASE WHEN v.side = 'red' THEN 1 ELSE 0 END), 0) AS red_votes,
            COALESCE(SUM(CASE WHEN v.side = 'blue' THEN 1 ELSE 0 END), 0) AS blue_votes
         FROM match_pause_votes v
         WHERE v.tournament_id = :tournament_id
           AND v.match_id = :match_id
           AND v.fight_round = :fight_round
           AND v.pause_seq = :pause_seq",
        params! {
            "tournament_id" => tournament_id,
            "match_id" => match_id,
            "fight_round" => fight_round,
            "pause_seq" => pause_seq,
        },
    )?;
    Ok(row.unwrap_or((0, 0)))
}

pub fn mark_applied(
    conn: &mut PooledConn,
    tournament_id: i64,
    match_id: i64,
    fight_round: i64,
    pause_seq: i64,
    winner_side: &str,
) -> mysql::Result<usize> {
    conn.exec_drop(
        "UPDATE match_pause_vote_events e
         SET e.winner_side = :winner_side,
             e.applied_at = CURRENT_TIMESTAMP
         WHERE e.tournament_id = :tournament_id
           AND e.match_id = :match_id
           AND e.fight_round = :fight_round
           AND e.pause_seq = :pause_seq
           AND (e.applied_at IS NULL OR e.applied_at = '')",
        params! {
            "winner_side" => winner_side,
            "tournament_id" => tournament_id,
            "match_id" => match_id,
            "fight_round" => fight_round,
            "pause_seq" => pause_seq,
        },
    )?;
    Ok(conn.affected_rows() as usize)
}

pub fn first_applied_point_side(
    conn: &mut PooledConn,
    tournament_id: i64,
    match_id: i64,
) -> mysql::Result<Option<String>> {
    conn.exec_first(
        "SELECT e.winner_side
         FROM match_pause_vote_events e
         WHERE e.tournament_id = :tournament_id
           AND e.match_id = :match_id
           AND e.winner_side IS NOT NULL
           AND e.winner_side <> ''
           AND e.applied_at IS NOT NULL
         ORDER BY e.applied_at ASC, e.fight_round ASC, e.pause_seq ASC
         LIMIT 1",
        params! {
            "tournament_id" => tournament_id,
            "match_id" => match_id,
        },
    )
}

