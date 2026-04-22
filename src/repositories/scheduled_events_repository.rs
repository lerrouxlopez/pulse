use crate::models::ScheduledEvent;
use mysql::prelude::*;
use mysql::{params, PooledConn, Row};

fn row_to_event(row: Row) -> ScheduledEvent {
    ScheduledEvent {
        id: row.get(0).unwrap_or_default(),
        event_id: row.get(1).unwrap_or_default(),
        event_name: row
            .get::<Option<String>, _>(2)
            .unwrap_or(None)
            .unwrap_or_default(),
        contact_type: row
            .get::<Option<String>, _>(3)
            .unwrap_or(None)
            .unwrap_or_default(),
        status: row
            .get::<Option<String>, _>(4)
            .unwrap_or(None)
            .unwrap_or_default(),
        location: row.get::<Option<String>, _>(5).unwrap_or(None),
        event_time: row.get::<Option<String>, _>(6).unwrap_or(None),
        point_system: row.get::<Option<String>, _>(7).unwrap_or(None),
        time_rule: row.get::<Option<String>, _>(8).unwrap_or(None),
        draw_system: row.get::<Option<String>, _>(9).unwrap_or(None),
        division_id: row.get::<Option<i64>, _>(10).unwrap_or(None),
        weight_class_id: row.get::<Option<i64>, _>(11).unwrap_or(None),
        winner_member_id: row.get::<Option<i64>, _>(12).unwrap_or(None),
        division_name: row.get::<Option<String>, _>(13).unwrap_or(None),
        weight_class_name: row.get::<Option<String>, _>(14).unwrap_or(None),
        weight_class_label: None,
        winner_name: row.get::<Option<String>, _>(15).unwrap_or(None),
    }
}

pub fn list(conn: &mut PooledConn, tournament_id: i64) -> mysql::Result<Vec<ScheduledEvent>> {
    conn.exec_map(
        "SELECT se.id, se.event_id, e.name, se.contact_type, se.status, se.location, se.event_time,
                se.point_system, se.time_rule, se.draw_system, se.division_id, se.weight_class_id, se.winner_member_id,
                d.name, w.name,
                COALESCE(
                    NULLIF(GROUP_CONCAT(DISTINCT tmw.name ORDER BY tmw.name SEPARATOR ', '), ''),
                    tm.name
                ) AS winner_names
         FROM scheduled_events se
         JOIN events e ON e.id = se.event_id
         LEFT JOIN divisions d ON d.id = se.division_id
         LEFT JOIN weight_classes w ON w.id = se.weight_class_id
         LEFT JOIN scheduled_event_winners sew ON sew.scheduled_event_id = se.id AND sew.tournament_id = se.tournament_id
         LEFT JOIN team_members tmw ON tmw.id = sew.winner_member_id
         LEFT JOIN team_members tm ON tm.id = se.winner_member_id
         WHERE se.tournament_id = :tournament_id
         GROUP BY se.id
         ORDER BY se.id DESC",
        params! {
            "tournament_id" => tournament_id,
        },
        row_to_event,
    )
}

pub fn get_by_id(
    conn: &mut PooledConn,
    tournament_id: i64,
    id: i64,
) -> mysql::Result<Option<ScheduledEvent>> {
    let row: Option<Row> = conn.exec_first(
        "SELECT se.id, se.event_id, e.name, se.contact_type, se.status, se.location, se.event_time,
                se.point_system, se.time_rule, se.draw_system, se.division_id, se.weight_class_id, se.winner_member_id,
                d.name, w.name,
                COALESCE(
                    NULLIF(GROUP_CONCAT(DISTINCT tmw.name ORDER BY tmw.name SEPARATOR ', '), ''),
                    tm.name
                ) AS winner_names
         FROM scheduled_events se
         JOIN events e ON e.id = se.event_id
         LEFT JOIN divisions d ON d.id = se.division_id
         LEFT JOIN weight_classes w ON w.id = se.weight_class_id
         LEFT JOIN scheduled_event_winners sew ON sew.scheduled_event_id = se.id AND sew.tournament_id = se.tournament_id
         LEFT JOIN team_members tmw ON tmw.id = sew.winner_member_id
         LEFT JOIN team_members tm ON tm.id = se.winner_member_id
         WHERE se.tournament_id = :tournament_id AND se.id = :id
         GROUP BY se.id",
        params! {
            "tournament_id" => tournament_id,
            "id" => id,
        },
    )?;
    Ok(row.map(row_to_event))
}

pub fn create(
    conn: &mut PooledConn,
    tournament_id: i64,
    event_id: i64,
    contact_type: &str,
    status: &str,
    location: Option<&str>,
    event_time: Option<&str>,
    point_system: Option<&str>,
    time_rule: Option<&str>,
    draw_system: Option<&str>,
    division_id: Option<i64>,
    weight_class_id: Option<i64>,
) -> mysql::Result<i64> {
    conn.exec_drop(
        "INSERT INTO scheduled_events (tournament_id, event_id, contact_type, status, location, event_time, point_system, time_rule, draw_system, division_id, weight_class_id)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        (
            tournament_id,
            event_id,
            contact_type,
            status,
            location,
            event_time,
            point_system,
            time_rule,
            draw_system,
            division_id,
            weight_class_id,
        ),
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
    point_system: Option<&str>,
    time_rule: Option<&str>,
    draw_system: Option<&str>,
    division_id: Option<i64>,
    weight_class_id: Option<i64>,
) -> mysql::Result<usize> {
    conn.exec_drop(
        "UPDATE scheduled_events
         SET event_id = ?, contact_type = ?, status = ?, location = ?, event_time = ?,
             point_system = ?, time_rule = ?, draw_system = ?, division_id = ?, weight_class_id = ?
         WHERE id = ? AND tournament_id = ?",
        (
            event_id,
            contact_type,
            status,
            location,
            event_time,
            point_system,
            time_rule,
            draw_system,
            division_id,
            weight_class_id,
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

pub fn update_status_and_winner(
    conn: &mut PooledConn,
    tournament_id: i64,
    scheduled_event_id: i64,
    status: &str,
    winner_member_id: Option<i64>,
) -> mysql::Result<usize> {
    conn.exec_drop(
        "UPDATE scheduled_events SET status = ?, winner_member_id = ? WHERE id = ? AND tournament_id = ?",
        (status, winner_member_id, scheduled_event_id, tournament_id),
    )?;
    Ok(conn.affected_rows() as usize)
}
