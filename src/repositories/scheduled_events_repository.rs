use crate::models::ScheduledEvent;
use rusqlite::{params, Connection};

pub fn list(conn: &Connection, tournament_id: i64) -> rusqlite::Result<Vec<ScheduledEvent>> {
    let mut stmt = conn.prepare(
        "SELECT se.id, se.event_id, e.name, se.contact_type, se.status, se.location, se.event_time
         FROM scheduled_events se
         JOIN events e ON e.id = se.event_id
         WHERE se.tournament_id = ?1
         ORDER BY se.id DESC",
    )?;
    let rows = stmt.query_map(params![tournament_id], |row| {
        Ok(ScheduledEvent {
            id: row.get(0)?,
            event_id: row.get(1)?,
            event_name: row.get(2)?,
            contact_type: row.get(3)?,
            status: row.get(4)?,
            location: row.get(5)?,
            event_time: row.get(6)?,
        })
    })?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok(items)
}

pub fn get_by_id(
    conn: &Connection,
    tournament_id: i64,
    id: i64,
) -> rusqlite::Result<Option<ScheduledEvent>> {
    let mut stmt = conn.prepare(
        "SELECT se.id, se.event_id, e.name, se.contact_type, se.status, se.location, se.event_time
         FROM scheduled_events se
         JOIN events e ON e.id = se.event_id
         WHERE se.tournament_id = ?1 AND se.id = ?2",
    )?;
    let mut rows = stmt.query(params![tournament_id, id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(ScheduledEvent {
            id: row.get(0)?,
            event_id: row.get(1)?,
            event_name: row.get(2)?,
            contact_type: row.get(3)?,
            status: row.get(4)?,
            location: row.get(5)?,
            event_time: row.get(6)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn create(
    conn: &Connection,
    tournament_id: i64,
    event_id: i64,
    contact_type: &str,
    status: &str,
    location: Option<&str>,
    event_time: Option<&str>,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO scheduled_events (tournament_id, event_id, contact_type, status, location, event_time)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![tournament_id, event_id, contact_type, status, location, event_time],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update(
    conn: &Connection,
    tournament_id: i64,
    id: i64,
    event_id: i64,
    contact_type: &str,
    status: &str,
    location: Option<&str>,
    event_time: Option<&str>,
) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE scheduled_events
         SET event_id = ?1, contact_type = ?2, status = ?3, location = ?4, event_time = ?5
         WHERE id = ?6 AND tournament_id = ?7",
        params![
            event_id,
            contact_type,
            status,
            location,
            event_time,
            id,
            tournament_id
        ],
    )
}

pub fn delete(conn: &Connection, tournament_id: i64, id: i64) -> rusqlite::Result<usize> {
    conn.execute(
        "DELETE FROM scheduled_events WHERE id = ?1 AND tournament_id = ?2",
        params![id, tournament_id],
    )
}
