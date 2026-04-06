use crate::models::ScheduledMatch;
use mysql::prelude::*;
use mysql::{params, PooledConn, Row};

fn row_to_match(row: Row) -> ScheduledMatch {
    let id: i64 = row.get::<Option<i64>, _>(0).unwrap_or(None).unwrap_or_default();
    let scheduled_event_id: i64 = row
        .get::<Option<i64>, _>(1)
        .unwrap_or(None)
        .unwrap_or_default();
    let mat: Option<String> = row.get::<Option<String>, _>(2).unwrap_or(None);
    let category: Option<String> = row.get::<Option<String>, _>(3).unwrap_or(None);
    let red: Option<String> = row.get::<Option<String>, _>(4).unwrap_or(None);
    let blue: Option<String> = row.get::<Option<String>, _>(5).unwrap_or(None);
    let status: String = row
        .get::<Option<String>, _>(6)
        .unwrap_or(None)
        .unwrap_or_default();
    let location: Option<String> = row.get::<Option<String>, _>(7).unwrap_or(None);
    let match_time: Option<String> = row.get::<Option<String>, _>(8).unwrap_or(None);
    let round: Option<i64> = row.get::<Option<i64>, _>(9).unwrap_or(None);
    let slot: Option<i64> = row.get::<Option<i64>, _>(10).unwrap_or(None);
    let red_member_id: Option<i64> = row.get::<Option<i64>, _>(11).unwrap_or(None);
    let blue_member_id: Option<i64> = row.get::<Option<i64>, _>(12).unwrap_or(None);
    let is_bye: i64 = row.get::<Option<i64>, _>(13).unwrap_or(None).unwrap_or_default();
    let winner_side: Option<String> = row.get::<Option<String>, _>(14).unwrap_or(None);

    ScheduledMatch {
        id,
        scheduled_event_id,
        mat,
        category,
        red,
        blue,
        status,
        location,
        match_time,
        round,
        slot,
        red_member_id,
        blue_member_id,
        is_bye: is_bye != 0,
        winner_side,
    }
}

pub fn list(
    conn: &mut PooledConn,
    tournament_id: i64,
    scheduled_event_id: i64,
) -> mysql::Result<Vec<ScheduledMatch>> {
    conn.exec_map(
        "SELECT COALESCE(id, 0), COALESCE(scheduled_event_id, 0), mat, category, red, blue, COALESCE(status, ''), location, match_time, round, slot, red_member_id, blue_member_id, COALESCE(is_bye, 0), winner_side
         FROM matches
         WHERE tournament_id = :tournament_id AND scheduled_event_id = :scheduled_event_id
         ORDER BY id DESC",
        params! {
            "tournament_id" => tournament_id,
            "scheduled_event_id" => scheduled_event_id,
        },
        row_to_match,
    )
}

pub fn create(
    conn: &mut PooledConn,
    tournament_id: i64,
    scheduled_event_id: i64,
    mat: Option<&str>,
    category: Option<&str>,
    red: Option<&str>,
    blue: Option<&str>,
    status: &str,
    location: Option<&str>,
    match_time: Option<&str>,
    round: Option<i64>,
    slot: Option<i64>,
    red_member_id: Option<i64>,
    blue_member_id: Option<i64>,
    is_bye: bool,
) -> mysql::Result<i64> {
    conn.exec_drop(
        "INSERT INTO matches (tournament_id, scheduled_event_id, mat, category, red, blue, status, location, match_time, round, slot, red_member_id, blue_member_id, is_bye, winner_side)
         VALUES (:tournament_id, :scheduled_event_id, :mat, :category, :red, :blue, :status, :location, :match_time, :round, :slot, :red_member_id, :blue_member_id, :is_bye, NULL)",
        params! {
            "tournament_id" => tournament_id,
            "scheduled_event_id" => scheduled_event_id,
            "mat" => mat,
            "category" => category,
            "red" => red,
            "blue" => blue,
            "status" => status,
            "location" => location,
            "match_time" => match_time,
            "round" => round,
            "slot" => slot,
            "red_member_id" => red_member_id,
            "blue_member_id" => blue_member_id,
            "is_bye" => if is_bye { 1 } else { 0 },
        },
    )?;
    Ok(conn.last_insert_id() as i64)
}

pub fn update(
    conn: &mut PooledConn,
    tournament_id: i64,
    id: i64,
    scheduled_event_id: i64,
    mat: Option<&str>,
    category: Option<&str>,
    red: Option<&str>,
    blue: Option<&str>,
    status: &str,
    location: Option<&str>,
    match_time: Option<&str>,
    round: Option<i64>,
    slot: Option<i64>,
    red_member_id: Option<i64>,
    blue_member_id: Option<i64>,
    is_bye: bool,
    winner_side: Option<&str>,
) -> mysql::Result<usize> {
    conn.exec_drop(
        "UPDATE matches
         SET mat = :mat, category = :category, red = :red, blue = :blue, status = :status, location = :location, match_time = :match_time,
             round = :round, slot = :slot, red_member_id = :red_member_id, blue_member_id = :blue_member_id, is_bye = :is_bye, winner_side = :winner_side
         WHERE id = :id AND tournament_id = :tournament_id AND scheduled_event_id = :scheduled_event_id",
        params! {
            "mat" => mat,
            "category" => category,
            "red" => red,
            "blue" => blue,
            "status" => status,
            "location" => location,
            "match_time" => match_time,
            "round" => round,
            "slot" => slot,
            "red_member_id" => red_member_id,
            "blue_member_id" => blue_member_id,
            "is_bye" => if is_bye { 1 } else { 0 },
            "winner_side" => winner_side,
            "id" => id,
            "tournament_id" => tournament_id,
            "scheduled_event_id" => scheduled_event_id,
        },
    )?;
    Ok(conn.affected_rows() as usize)
}

pub fn delete(conn: &mut PooledConn, tournament_id: i64, id: i64) -> mysql::Result<usize> {
    conn.exec_drop(
        "DELETE FROM matches WHERE id = :id AND tournament_id = :tournament_id",
        params! {
            "id" => id,
            "tournament_id" => tournament_id,
        },
    )?;
    Ok(conn.affected_rows() as usize)
}

pub fn delete_by_scheduled_event(
    conn: &mut PooledConn,
    tournament_id: i64,
    scheduled_event_id: i64,
) -> mysql::Result<usize> {
    conn.exec_drop(
        "DELETE FROM matches WHERE tournament_id = :tournament_id AND scheduled_event_id = :scheduled_event_id",
        params! {
            "tournament_id" => tournament_id,
            "scheduled_event_id" => scheduled_event_id,
        },
    )?;
    Ok(conn.affected_rows() as usize)
}

pub fn get_by_id(
    conn: &mut PooledConn,
    tournament_id: i64,
    id: i64,
) -> mysql::Result<Option<ScheduledMatch>> {
    let row: Option<Row> = conn.exec_first(
        "SELECT COALESCE(id, 0), COALESCE(scheduled_event_id, 0), mat, category, red, blue, COALESCE(status, ''), location, match_time, round, slot, red_member_id, blue_member_id, COALESCE(is_bye, 0), winner_side
         FROM matches
         WHERE id = :id AND tournament_id = :tournament_id",
        params! {
            "id" => id,
            "tournament_id" => tournament_id,
        },
    )?;
    Ok(row.map(row_to_match))
}

pub fn get_by_round_slot(
    conn: &mut PooledConn,
    tournament_id: i64,
    scheduled_event_id: i64,
    round: i64,
    slot: i64,
) -> mysql::Result<Option<ScheduledMatch>> {
    let row: Option<Row> = conn.exec_first(
        "SELECT COALESCE(id, 0), COALESCE(scheduled_event_id, 0), mat, category, red, blue, COALESCE(status, ''), location, match_time, round, slot, red_member_id, blue_member_id, COALESCE(is_bye, 0), winner_side
         FROM matches
         WHERE tournament_id = :tournament_id AND scheduled_event_id = :scheduled_event_id AND round = :round AND slot = :slot",
        params! {
            "tournament_id" => tournament_id,
            "scheduled_event_id" => scheduled_event_id,
            "round" => round,
            "slot" => slot,
        },
    )?;
    Ok(row.map(row_to_match))
}
