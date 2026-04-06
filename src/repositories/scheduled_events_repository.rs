use crate::models::ScheduledEvent;
use mysql::prelude::*;
use mysql::PooledConn;

pub fn list(conn: &mut PooledConn, tournament_id: i64) -> mysql::Result<Vec<ScheduledEvent>> {
    conn.exec_map(
        "SELECT se.id, se.event_id, e.name, se.contact_type, se.status, se.location, se.event_time
         FROM scheduled_events se
         JOIN events e ON e.id = se.event_id
         WHERE se.tournament_id = ?
         ORDER BY se.id DESC",
        (tournament_id,),
        |(id, event_id, event_name, contact_type, status, location, event_time)| ScheduledEvent {
            id,
            event_id,
            event_name,
            contact_type,
            status,
            location,
            event_time,
        },
    )
}

pub fn get_by_id(
    conn: &mut PooledConn,
    tournament_id: i64,
    id: i64,
) -> mysql::Result<Option<ScheduledEvent>> {
    let row: Option<(i64, i64, String, String, String, Option<String>, Option<String>)> =
        conn.exec_first(
        "SELECT se.id, se.event_id, e.name, se.contact_type, se.status, se.location, se.event_time
         FROM scheduled_events se
         JOIN events e ON e.id = se.event_id
         WHERE se.tournament_id = ? AND se.id = ?",
            (tournament_id, id),
        )?;
    Ok(row.map(
        |(id, event_id, event_name, contact_type, status, location, event_time)| ScheduledEvent {
            id,
            event_id,
            event_name,
            contact_type,
            status,
            location,
            event_time,
        },
    ))
}

pub fn create(
    conn: &mut PooledConn,
    tournament_id: i64,
    event_id: i64,
    contact_type: &str,
    status: &str,
    location: Option<&str>,
    event_time: Option<&str>,
) -> mysql::Result<i64> {
    conn.exec_drop(
        "INSERT INTO scheduled_events (tournament_id, event_id, contact_type, status, location, event_time)
         VALUES (?, ?, ?, ?, ?, ?)",
        (tournament_id, event_id, contact_type, status, location, event_time),
    )?;
    Ok(conn.last_insert_id() as i64)
}

pub fn update(
    conn: &mut PooledConn,
    tournament_id: i64,
    id: i64,
    event_id: i64,
    contact_type: &str,
    status: &str,
    location: Option<&str>,
    event_time: Option<&str>,
) -> mysql::Result<usize> {
    conn.exec_drop(
        "UPDATE scheduled_events
         SET event_id = ?, contact_type = ?, status = ?, location = ?, event_time = ?
         WHERE id = ? AND tournament_id = ?",
        (
            event_id,
            contact_type,
            status,
            location,
            event_time,
            id,
            tournament_id,
        ),
    )?;
    Ok(conn.affected_rows() as usize)
}

pub fn delete(conn: &mut PooledConn, tournament_id: i64, id: i64) -> mysql::Result<usize> {
    conn.exec_drop(
        "DELETE FROM scheduled_events WHERE id = ? AND tournament_id = ?",
        (id, tournament_id),
    )?;
    Ok(conn.affected_rows() as usize)
}
