use crate::models::MatchJudgeScore;
use mysql::prelude::*;
use mysql::{params, PooledConn};

pub fn list_assigned_judges(
    conn: &mut PooledConn,
    tournament_id: i64,
    match_id: i64,
) -> mysql::Result<Vec<(i64, String)>> {
    conn.exec_map(
        "SELECT base.judge_user_id, u.name
         FROM (
           SELECT mj.judge_user_id, MIN(mj.judge_order) AS judge_order
           FROM match_judges mj
           WHERE mj.tournament_id = :tournament_id AND mj.match_id = :match_id
           GROUP BY mj.judge_user_id
         ) base
         JOIN users u ON u.id = base.judge_user_id
         ORDER BY base.judge_order ASC, u.name ASC",
        params! {
            "tournament_id" => tournament_id,
            "match_id" => match_id,
        },
        |(judge_user_id, judge_name)| (judge_user_id, judge_name),
    )
}

pub fn list_by_match(
    conn: &mut PooledConn,
    tournament_id: i64,
    match_id: i64,
    fight_round: i64,
) -> mysql::Result<Vec<MatchJudgeScore>> {
    conn.exec_map(
        // Judges are assigned at the match level, but scores are per-round.
        // If a round has no rows yet, still return the assigned judges with 0/0 scores so UI doesn't "lose" cards.
        "SELECT base.judge_user_id,
                u.name,
                u.photo_url,
                COALESCE(mj.red_score, 0) AS red_score,
                COALESCE(mj.blue_score, 0) AS blue_score,
                base.judge_order
         FROM (
           SELECT mj.judge_user_id, MIN(mj.judge_order) AS judge_order
           FROM match_judges mj
           WHERE mj.tournament_id = :tournament_id AND mj.match_id = :match_id
           GROUP BY mj.judge_user_id
         ) base
         JOIN users u ON u.id = base.judge_user_id
         LEFT JOIN match_judges mj
           ON mj.tournament_id = :tournament_id
          AND mj.match_id = :match_id
          AND mj.judge_user_id = base.judge_user_id
          AND mj.fight_round = :fight_round
         ORDER BY base.judge_order ASC, u.name ASC",
        params! {
            "tournament_id" => tournament_id,
            "match_id" => match_id,
            "fight_round" => fight_round,
        },
        |(judge_user_id, judge_name, judge_photo_url, red_score, blue_score, judge_order)| {
            MatchJudgeScore {
                judge_user_id,
                judge_name,
                judge_photo_url,
                red_score,
                blue_score,
                judge_order,
            }
        },
    )
}

pub fn replace_for_match(
    conn: &mut PooledConn,
    tournament_id: i64,
    match_id: i64,
    fight_round: i64,
    judges: &[MatchJudgeScore],
) -> mysql::Result<()> {
    conn.exec_drop(
        "DELETE FROM match_judges WHERE tournament_id = :tournament_id AND match_id = :match_id AND fight_round = :fight_round",
        params! {
            "tournament_id" => tournament_id,
            "match_id" => match_id,
            "fight_round" => fight_round,
        },
    )?;
    for judge in judges {
        conn.exec_drop(
            "INSERT INTO match_judges (tournament_id, match_id, judge_user_id, fight_round, judge_order, red_score, blue_score)
             VALUES (:tournament_id, :match_id, :judge_user_id, :fight_round, :judge_order, :red_score, :blue_score)",
            params! {
                "tournament_id" => tournament_id,
                "match_id" => match_id,
                "judge_user_id" => judge.judge_user_id,
                "fight_round" => fight_round,
                "judge_order" => judge.judge_order,
                "red_score" => judge.red_score,
                "blue_score" => judge.blue_score,
            },
        )?;
    }
    Ok(())
}

pub fn list_match_ids_for_judge(
    conn: &mut PooledConn,
    tournament_id: i64,
    judge_user_id: i64,
) -> mysql::Result<Vec<i64>> {
    conn.exec_map(
        "SELECT DISTINCT mj.match_id
         FROM match_judges mj
         WHERE mj.tournament_id = :tournament_id AND mj.judge_user_id = :judge_user_id
         ORDER BY mj.match_id DESC",
        params! {
            "tournament_id" => tournament_id,
            "judge_user_id" => judge_user_id,
        },
        |match_id| match_id,
    )
}

pub fn find_judge_order(
    conn: &mut PooledConn,
    tournament_id: i64,
    match_id: i64,
    judge_user_id: i64,
) -> mysql::Result<Option<i32>> {
    conn.exec_first(
        "SELECT mj.judge_order
         FROM match_judges mj
         WHERE mj.tournament_id = :tournament_id AND mj.match_id = :match_id AND mj.judge_user_id = :judge_user_id
         ORDER BY mj.fight_round ASC
         LIMIT 1",
        params! {
            "tournament_id" => tournament_id,
            "match_id" => match_id,
            "judge_user_id" => judge_user_id,
        },
    )
}

pub fn get_score(
    conn: &mut PooledConn,
    tournament_id: i64,
    match_id: i64,
    judge_user_id: i64,
    fight_round: i64,
) -> mysql::Result<Option<(i32, i32)>> {
    conn.exec_first(
        "SELECT mj.red_score, mj.blue_score
         FROM match_judges mj
         WHERE mj.tournament_id = :tournament_id AND mj.match_id = :match_id AND mj.judge_user_id = :judge_user_id AND mj.fight_round = :fight_round
         LIMIT 1",
        params! {
            "tournament_id" => tournament_id,
            "match_id" => match_id,
            "judge_user_id" => judge_user_id,
            "fight_round" => fight_round,
        },
    )
}

pub fn upsert_score(
    conn: &mut PooledConn,
    tournament_id: i64,
    match_id: i64,
    judge_user_id: i64,
    fight_round: i64,
    judge_order: i32,
    red_score: i32,
    blue_score: i32,
) -> mysql::Result<()> {
    conn.exec_drop(
        "INSERT INTO match_judges (tournament_id, match_id, judge_user_id, fight_round, judge_order, red_score, blue_score)
         VALUES (:tournament_id, :match_id, :judge_user_id, :fight_round, :judge_order, :red_score, :blue_score)
         ON DUPLICATE KEY UPDATE red_score = :red_score, blue_score = :blue_score, judge_order = :judge_order",
        params! {
            "tournament_id" => tournament_id,
            "match_id" => match_id,
            "judge_user_id" => judge_user_id,
            "fight_round" => fight_round,
            "judge_order" => judge_order,
            "red_score" => red_score,
            "blue_score" => blue_score,
        },
    )?;
    Ok(())
}

pub fn sum_for_match_round(
    conn: &mut PooledConn,
    tournament_id: i64,
    match_id: i64,
    fight_round: i64,
) -> mysql::Result<(i64, i64)> {
    let row: Option<(i64, i64)> = conn.exec_first(
        "SELECT COALESCE(SUM(mj.red_score), 0), COALESCE(SUM(mj.blue_score), 0)
         FROM match_judges mj
         WHERE mj.tournament_id = :tournament_id AND mj.match_id = :match_id AND mj.fight_round = :fight_round",
        params! {
            "tournament_id" => tournament_id,
            "match_id" => match_id,
            "fight_round" => fight_round,
        },
    )?;
    Ok(row.unwrap_or((0, 0)))
}

pub fn count_distinct_judges_for_match_round(
    conn: &mut PooledConn,
    tournament_id: i64,
    match_id: i64,
    fight_round: i64,
) -> mysql::Result<i64> {
    let row: Option<i64> = conn.exec_first(
        "SELECT COALESCE(COUNT(DISTINCT mj.judge_user_id), 0)
         FROM match_judges mj
         WHERE mj.tournament_id = :tournament_id AND mj.match_id = :match_id AND mj.fight_round = :fight_round",
        params! {
            "tournament_id" => tournament_id,
            "match_id" => match_id,
            "fight_round" => fight_round,
        },
    )?;
    Ok(row.unwrap_or(0))
}

pub fn count_distinct_judges_with_valid_scores_for_match_round(
    conn: &mut PooledConn,
    tournament_id: i64,
    match_id: i64,
    fight_round: i64,
    min_allowed: i32,
    max_allowed: i32,
) -> mysql::Result<i64> {
    // Judges can be "assigned" to a match with placeholder 0/0 rows; treat those as not-yet-scored.
    // For known point systems (5-10, 8/10), min_allowed > 0 cleanly distinguishes scored vs placeholder.
    let row: Option<i64> = conn.exec_first(
        "SELECT COALESCE(COUNT(DISTINCT mj.judge_user_id), 0)
         FROM match_judges mj
         WHERE mj.tournament_id = :tournament_id
           AND mj.match_id = :match_id
           AND mj.fight_round = :fight_round
           AND mj.red_score >= :min_allowed AND mj.red_score <= :max_allowed
           AND mj.blue_score >= :min_allowed AND mj.blue_score <= :max_allowed",
        params! {
            "tournament_id" => tournament_id,
            "match_id" => match_id,
            "fight_round" => fight_round,
            "min_allowed" => min_allowed,
            "max_allowed" => max_allowed,
        },
    )?;
    Ok(row.unwrap_or(0))
}

pub fn count_distinct_judges_with_valid_red_score_for_match_round(
    conn: &mut PooledConn,
    tournament_id: i64,
    match_id: i64,
    fight_round: i64,
    min_allowed: i32,
    max_allowed: i32,
) -> mysql::Result<i64> {
    // Non-contact performances only have a single score column (stored in `red_score`).
    let row: Option<i64> = conn.exec_first(
        "SELECT COALESCE(COUNT(DISTINCT mj.judge_user_id), 0)
         FROM match_judges mj
         WHERE mj.tournament_id = :tournament_id
           AND mj.match_id = :match_id
           AND mj.fight_round = :fight_round
           AND mj.red_score >= :min_allowed AND mj.red_score <= :max_allowed",
        params! {
            "tournament_id" => tournament_id,
            "match_id" => match_id,
            "fight_round" => fight_round,
            "min_allowed" => min_allowed,
            "max_allowed" => max_allowed,
        },
    )?;
    Ok(row.unwrap_or(0))
}

pub fn max_fight_round_for_match(
    conn: &mut PooledConn,
    tournament_id: i64,
    match_id: i64,
) -> mysql::Result<i64> {
    let value: Option<Option<i64>> = conn.exec_first(
        "SELECT MAX(mj.fight_round)
         FROM match_judges mj
         WHERE mj.tournament_id = :tournament_id AND mj.match_id = :match_id",
        params! {
            "tournament_id" => tournament_id,
            "match_id" => match_id,
        },
    )?;
    Ok(value.flatten().unwrap_or(1))
}

pub fn delete_rounds_gt(
    conn: &mut PooledConn,
    tournament_id: i64,
    match_id: i64,
    fight_round: i64,
) -> mysql::Result<usize> {
    conn.exec_drop(
        "DELETE FROM match_judges
         WHERE tournament_id = :tournament_id AND match_id = :match_id AND fight_round > :fight_round",
        params! {
            "tournament_id" => tournament_id,
            "match_id" => match_id,
            "fight_round" => fight_round,
        },
    )?;
    Ok(conn.affected_rows() as usize)
}

pub fn next_judge_order_for_match_round(
    conn: &mut PooledConn,
    tournament_id: i64,
    match_id: i64,
    fight_round: i64,
) -> mysql::Result<i32> {
    let max_value: Option<i32> = conn.exec_first(
        "SELECT COALESCE(MAX(mj.judge_order), 0)
         FROM match_judges mj
         WHERE mj.tournament_id = :tournament_id AND mj.match_id = :match_id AND mj.fight_round = :fight_round",
        params! {
            "tournament_id" => tournament_id,
            "match_id" => match_id,
            "fight_round" => fight_round,
        },
    )?;
    Ok(max_value.unwrap_or(0) + 1)
}

pub fn delete_by_scheduled_event(
    conn: &mut PooledConn,
    tournament_id: i64,
    scheduled_event_id: i64,
) -> mysql::Result<usize> {
    conn.exec_drop(
        "DELETE mj FROM match_judges mj
         JOIN matches m ON m.id = mj.match_id
         WHERE mj.tournament_id = :tournament_id
           AND m.tournament_id = :tournament_id
           AND m.scheduled_event_id = :scheduled_event_id",
        params! {
            "tournament_id" => tournament_id,
            "scheduled_event_id" => scheduled_event_id,
        },
    )?;
    Ok(conn.affected_rows() as usize)
}
