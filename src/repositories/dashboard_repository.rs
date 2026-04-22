use mysql::prelude::*;
use mysql::{params, PooledConn};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DashboardCounts {
    pub teams: u64,
    pub members: u64,
    pub scheduled_events: u64,
    pub matches: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpcomingScheduledEventRow {
    pub scheduled_event_id: i64,
    pub event_name: String,
    pub contact_type: String,
    pub status: String,
    pub event_time: Option<String>,
    pub location: Option<String>,
    pub division_name: Option<String>,
    pub weight_class_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecentMatchRow {
    pub match_id: i64,
    pub scheduled_event_id: i64,
    pub event_name: String,
    pub status: String,
    pub mat: Option<String>,
    pub created_at: Option<String>,
}

pub fn counts(conn: &mut PooledConn, tournament_id: i64) -> mysql::Result<DashboardCounts> {
    let (teams, members, scheduled_events, matches): (u64, u64, u64, u64) = conn.exec_first(
        "SELECT
            (SELECT COUNT(*) FROM teams WHERE tournament_id = :tournament_id) AS teams,
            (SELECT COUNT(*) FROM team_members WHERE tournament_id = :tournament_id) AS members,
            (SELECT COUNT(*) FROM scheduled_events WHERE tournament_id = :tournament_id) AS scheduled_events,
            (SELECT COUNT(*) FROM matches WHERE tournament_id = :tournament_id) AS matches",
        params! {
            "tournament_id" => tournament_id,
        },
    )?
    .unwrap_or((0, 0, 0, 0));
    Ok(DashboardCounts {
        teams,
        members,
        scheduled_events,
        matches,
    })
}

pub fn scheduled_events_by_status(
    conn: &mut PooledConn,
    tournament_id: i64,
) -> mysql::Result<Vec<(String, u64)>> {
    conn.exec_map(
        "SELECT status, COUNT(*)
         FROM scheduled_events
         WHERE tournament_id = :tournament_id
         GROUP BY status
         ORDER BY COUNT(*) DESC",
        params! {
            "tournament_id" => tournament_id,
        },
        |(status, count): (String, u64)| (status, count),
    )
}

pub fn matches_by_status(
    conn: &mut PooledConn,
    tournament_id: i64,
) -> mysql::Result<Vec<(String, u64)>> {
    conn.exec_map(
        "SELECT status, COUNT(*)
         FROM matches
         WHERE tournament_id = :tournament_id
         GROUP BY status
         ORDER BY COUNT(*) DESC",
        params! {
            "tournament_id" => tournament_id,
        },
        |(status, count): (String, u64)| (status, count),
    )
}

pub fn scheduled_events_by_contact_type(
    conn: &mut PooledConn,
    tournament_id: i64,
) -> mysql::Result<Vec<(String, u64)>> {
    conn.exec_map(
        "SELECT contact_type, COUNT(*)
         FROM scheduled_events
         WHERE tournament_id = :tournament_id
         GROUP BY contact_type
         ORDER BY COUNT(*) DESC",
        params! {
            "tournament_id" => tournament_id,
        },
        |(contact_type, count): (String, u64)| (contact_type, count),
    )
}

pub fn scheduled_events_timeseries(
    conn: &mut PooledConn,
    tournament_id: i64,
    days: u64,
) -> mysql::Result<Vec<(String, u64)>> {
    conn.exec_map(
        "SELECT DATE_FORMAT(created_at, '%Y-%m-%d') AS day, COUNT(*)
         FROM scheduled_events
         WHERE tournament_id = :tournament_id
           AND created_at >= DATE_SUB(CURDATE(), INTERVAL :days DAY)
         GROUP BY day
         ORDER BY day",
        params! {
            "tournament_id" => tournament_id,
            "days" => days,
        },
        |(day, count): (String, u64)| (day, count),
    )
}

pub fn matches_timeseries(
    conn: &mut PooledConn,
    tournament_id: i64,
    days: u64,
) -> mysql::Result<Vec<(String, u64)>> {
    conn.exec_map(
        "SELECT DATE_FORMAT(created_at, '%Y-%m-%d') AS day, COUNT(*)
         FROM matches
         WHERE tournament_id = :tournament_id
           AND created_at >= DATE_SUB(CURDATE(), INTERVAL :days DAY)
         GROUP BY day
         ORDER BY day",
        params! {
            "tournament_id" => tournament_id,
            "days" => days,
        },
        |(day, count): (String, u64)| (day, count),
    )
}

pub fn upcoming_scheduled_events(
    conn: &mut PooledConn,
    tournament_id: i64,
    limit: u64,
) -> mysql::Result<Vec<UpcomingScheduledEventRow>> {
    conn.exec_map(
        "SELECT
            se.id,
            e.name,
            se.contact_type,
            se.status,
            se.event_time,
            se.location,
            d.name,
            w.name
         FROM scheduled_events se
         JOIN events e ON e.id = se.event_id
         LEFT JOIN divisions d ON d.id = se.division_id
         LEFT JOIN weight_classes w ON w.id = se.weight_class_id
         WHERE se.tournament_id = :tournament_id
           AND se.event_time IS NOT NULL
           AND se.event_time <> ''
           AND STR_TO_DATE(REPLACE(se.event_time, 'T', ' '), '%Y-%m-%d %H:%i') >= NOW()
         ORDER BY STR_TO_DATE(REPLACE(se.event_time, 'T', ' '), '%Y-%m-%d %H:%i') ASC
         LIMIT :limit",
        params! {
            "tournament_id" => tournament_id,
            "limit" => limit,
        },
        |(
            scheduled_event_id,
            event_name,
            contact_type,
            status,
            event_time,
            location,
            division_name,
            weight_class_name,
        ): (
            i64,
            String,
            String,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        )| UpcomingScheduledEventRow {
            scheduled_event_id,
            event_name,
            contact_type,
            status,
            event_time,
            location,
            division_name,
            weight_class_name,
        },
    )
}

pub fn recent_matches(
    conn: &mut PooledConn,
    tournament_id: i64,
    limit: u64,
) -> mysql::Result<Vec<RecentMatchRow>> {
    conn.exec_map(
        "SELECT
            m.id,
            m.scheduled_event_id,
            e.name,
            m.status,
            m.mat,
            m.created_at
         FROM matches m
         JOIN scheduled_events se ON se.id = m.scheduled_event_id AND se.tournament_id = m.tournament_id
         JOIN events e ON e.id = se.event_id
         WHERE m.tournament_id = :tournament_id
         ORDER BY m.id DESC
         LIMIT :limit",
        params! {
            "tournament_id" => tournament_id,
            "limit" => limit,
        },
        |(match_id, scheduled_event_id, event_name, status, mat, created_at): (
            i64,
            i64,
            String,
            String,
            Option<String>,
            Option<String>,
        )| RecentMatchRow {
            match_id,
            scheduled_event_id,
            event_name,
            status,
            mat,
            created_at,
        },
    )
}
