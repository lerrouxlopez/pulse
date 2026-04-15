use crate::models::MatchJudgeScore;
use mysql::prelude::*;
use mysql::{params, PooledConn};

pub fn list_by_match(
    conn: &mut PooledConn,
    tournament_id: i64,
    match_id: i64,
) -> mysql::Result<Vec<MatchJudgeScore>> {
    conn.exec_map(
        "SELECT mj.judge_user_id, u.name, u.photo_url, mj.red_score, mj.blue_score, mj.judge_order
         FROM match_judges mj
         JOIN users u ON u.id = mj.judge_user_id
         WHERE mj.tournament_id = :tournament_id AND mj.match_id = :match_id
         ORDER BY mj.judge_order ASC, u.name ASC",
        params! {
            "tournament_id" => tournament_id,
            "match_id" => match_id,
        },
        |(judge_user_id, judge_name, judge_photo_url, red_score, blue_score, judge_order)| MatchJudgeScore {
            judge_user_id,
            judge_name,
            judge_photo_url,
            red_score,
            blue_score,
            judge_order,
        },
    )
}

pub fn replace_for_match(
    conn: &mut PooledConn,
    tournament_id: i64,
    match_id: i64,
    judges: &[MatchJudgeScore],
) -> mysql::Result<()> {
    conn.exec_drop(
        "DELETE FROM match_judges WHERE tournament_id = :tournament_id AND match_id = :match_id",
        params! {
            "tournament_id" => tournament_id,
            "match_id" => match_id,
        },
    )?;
    for judge in judges {
        conn.exec_drop(
            "INSERT INTO match_judges (tournament_id, match_id, judge_user_id, judge_order, red_score, blue_score)
             VALUES (:tournament_id, :match_id, :judge_user_id, :judge_order, :red_score, :blue_score)",
            params! {
                "tournament_id" => tournament_id,
                "match_id" => match_id,
                "judge_user_id" => judge.judge_user_id,
                "judge_order" => judge.judge_order,
                "red_score" => judge.red_score,
                "blue_score" => judge.blue_score,
            },
        )?;
    }
    Ok(())
}
