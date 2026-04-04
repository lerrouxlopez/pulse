use crate::models::ScheduledMatch;
use rusqlite::{params, Connection};

pub fn list(conn: &Connection, tournament_id: i64, scheduled_event_id: i64) -> rusqlite::Result<Vec<ScheduledMatch>> {
    let mut stmt = conn.prepare(
        "SELECT id, scheduled_event_id, mat, category, red, blue, status, location, match_time, round, slot, red_member_id, blue_member_id, is_bye, winner_side
         FROM matches
         WHERE tournament_id = ?1 AND scheduled_event_id = ?2
         ORDER BY id DESC",
    )?;
    let rows = stmt.query_map(params![tournament_id, scheduled_event_id], |row| {
        Ok(ScheduledMatch {
            id: row.get(0)?,
            scheduled_event_id: row.get(1)?,
            mat: row.get(2)?,
            category: row.get(3)?,
            red: row.get(4)?,
            blue: row.get(5)?,
            status: row.get(6)?,
            location: row.get(7)?,
            match_time: row.get(8)?,
            round: row.get(9)?,
            slot: row.get(10)?,
            red_member_id: row.get(11)?,
            blue_member_id: row.get(12)?,
            is_bye: row.get::<_, i64>(13)? != 0,
            winner_side: row.get(14)?,
        })
    })?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok(items)
}

pub fn create(
    conn: &Connection,
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
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO matches (tournament_id, scheduled_event_id, mat, category, red, blue, status, location, match_time, round, slot, red_member_id, blue_member_id, is_bye, winner_side)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, NULL)",
        params![
            tournament_id,
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
            if is_bye { 1 } else { 0 }
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update(
    conn: &Connection,
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
) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE matches
         SET mat = ?1, category = ?2, red = ?3, blue = ?4, status = ?5, location = ?6, match_time = ?7,
             round = ?8, slot = ?9, red_member_id = ?10, blue_member_id = ?11, is_bye = ?12, winner_side = ?13
         WHERE id = ?14 AND tournament_id = ?15 AND scheduled_event_id = ?16",
        params![
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
            if is_bye { 1 } else { 0 },
            winner_side,
            id,
            tournament_id,
            scheduled_event_id
        ],
    )
}

pub fn delete(conn: &Connection, tournament_id: i64, id: i64) -> rusqlite::Result<usize> {
    conn.execute(
        "DELETE FROM matches WHERE id = ?1 AND tournament_id = ?2",
        params![id, tournament_id],
    )
}

pub fn get_by_id(
    conn: &Connection,
    tournament_id: i64,
    id: i64,
) -> rusqlite::Result<Option<ScheduledMatch>> {
    let mut stmt = conn.prepare(
        "SELECT id, scheduled_event_id, mat, category, red, blue, status, location, match_time, round, slot, red_member_id, blue_member_id, is_bye, winner_side
         FROM matches
         WHERE id = ?1 AND tournament_id = ?2",
    )?;
    let mut rows = stmt.query(params![id, tournament_id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(ScheduledMatch {
            id: row.get(0)?,
            scheduled_event_id: row.get(1)?,
            mat: row.get(2)?,
            category: row.get(3)?,
            red: row.get(4)?,
            blue: row.get(5)?,
            status: row.get(6)?,
            location: row.get(7)?,
            match_time: row.get(8)?,
            round: row.get(9)?,
            slot: row.get(10)?,
            red_member_id: row.get(11)?,
            blue_member_id: row.get(12)?,
            is_bye: row.get::<_, i64>(13)? != 0,
            winner_side: row.get(14)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn get_by_round_slot(
    conn: &Connection,
    tournament_id: i64,
    scheduled_event_id: i64,
    round: i64,
    slot: i64,
) -> rusqlite::Result<Option<ScheduledMatch>> {
    let mut stmt = conn.prepare(
        "SELECT id, scheduled_event_id, mat, category, red, blue, status, location, match_time, round, slot, red_member_id, blue_member_id, is_bye, winner_side
         FROM matches
         WHERE tournament_id = ?1 AND scheduled_event_id = ?2 AND round = ?3 AND slot = ?4",
    )?;
    let mut rows = stmt.query(params![tournament_id, scheduled_event_id, round, slot])?;
    if let Some(row) = rows.next()? {
        Ok(Some(ScheduledMatch {
            id: row.get(0)?,
            scheduled_event_id: row.get(1)?,
            mat: row.get(2)?,
            category: row.get(3)?,
            red: row.get(4)?,
            blue: row.get(5)?,
            status: row.get(6)?,
            location: row.get(7)?,
            match_time: row.get(8)?,
            round: row.get(9)?,
            slot: row.get(10)?,
            red_member_id: row.get(11)?,
            blue_member_id: row.get(12)?,
            is_bye: row.get::<_, i64>(13)? != 0,
            winner_side: row.get(14)?,
        }))
    } else {
        Ok(None)
    }
}
