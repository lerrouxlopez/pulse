use mysql::prelude::*;
use mysql::PooledConn;
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct SystemCounts {
    pub tournaments: u64,
    pub teams: u64,
    pub members: u64,
    pub users: u64,
    pub scheduled_events: u64,
    pub matches: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TournamentSummary {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub owner_user_id: i64,
    pub owner_name: Option<String>,
    pub owner_email: Option<String>,
    pub teams: u64,
    pub members: u64,
    pub scheduled_events: u64,
    pub matches: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminTeamRow {
    pub id: i64,
    pub tournament_id: i64,
    pub tournament_name: String,
    pub tournament_slug: String,
    pub name: String,
    pub logo_url: Option<String>,
    pub members: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminMemberRow {
    pub id: i64,
    pub tournament_id: i64,
    pub team_id: i64,
    pub name: String,
    pub notes: Option<String>,
    pub weight_class: Option<String>,
    pub division_id: Option<i64>,
    pub photo_url: Option<String>,
}

fn count(conn: &mut PooledConn, sql: &str) -> mysql::Result<u64> {
    let row: Option<u64> = conn.exec_first(sql, ())?;
    Ok(row.unwrap_or(0))
}

pub fn system_counts(conn: &mut PooledConn) -> mysql::Result<SystemCounts> {
    Ok(SystemCounts {
        tournaments: count(conn, "SELECT COUNT(*) FROM tournaments")?,
        teams: count(conn, "SELECT COUNT(*) FROM teams")?,
        members: count(conn, "SELECT COUNT(*) FROM team_members")?,
        users: count(conn, "SELECT COUNT(*) FROM users")?,
        scheduled_events: count(conn, "SELECT COUNT(*) FROM scheduled_events")?,
        matches: count(conn, "SELECT COUNT(*) FROM matches")?,
    })
}

pub fn tournament_summaries(conn: &mut PooledConn) -> mysql::Result<Vec<TournamentSummary>> {
    conn.exec_map(
        "SELECT
           t.id,
           t.name,
           COALESCE(t.slug, ''),
           t.user_id,
           u.name,
           u.email,
           COALESCE(tc.teams, 0),
           COALESCE(mc.members, 0),
           COALESCE(sc.scheduled_events, 0),
           COALESCE(xc.matches, 0)
         FROM tournaments t
         LEFT JOIN users u ON u.id = t.user_id
         LEFT JOIN (
           SELECT tournament_id, COUNT(*) AS teams
           FROM teams
           GROUP BY tournament_id
         ) tc ON tc.tournament_id = t.id
         LEFT JOIN (
           SELECT tournament_id, COUNT(*) AS members
           FROM team_members
           GROUP BY tournament_id
         ) mc ON mc.tournament_id = t.id
         LEFT JOIN (
           SELECT tournament_id, COUNT(*) AS scheduled_events
           FROM scheduled_events
           GROUP BY tournament_id
         ) sc ON sc.tournament_id = t.id
         LEFT JOIN (
           SELECT tournament_id, COUNT(*) AS matches
           FROM matches
           GROUP BY tournament_id
         ) xc ON xc.tournament_id = t.id
         ORDER BY t.id DESC",
        (),
        |(
            id,
            name,
            slug,
            owner_user_id,
            owner_name,
            owner_email,
            teams,
            members,
            scheduled_events,
            matches,
        ): (
            i64,
            String,
            String,
            i64,
            Option<String>,
            Option<String>,
            u64,
            u64,
            u64,
            u64,
        )| TournamentSummary {
            id,
            name,
            slug,
            owner_user_id,
            owner_name,
            owner_email,
            teams,
            members,
            scheduled_events,
            matches,
        },
    )
}

pub fn delete_tournament_cascade(conn: &mut PooledConn, tournament_id: i64) -> mysql::Result<()> {
    // Best-effort: there are no FK constraints here, but order still matters for logical integrity.
    // Matches & scoring tables.
    let _ = conn.exec_drop(
        "DELETE FROM match_pause_votes WHERE tournament_id = ?",
        (tournament_id,),
    );
    let _ = conn.exec_drop(
        "DELETE FROM match_pause_vote_events WHERE tournament_id = ?",
        (tournament_id,),
    );
    let _ = conn.exec_drop("DELETE FROM match_judges WHERE tournament_id = ?", (tournament_id,));
    let _ = conn.exec_drop("DELETE FROM matches WHERE tournament_id = ?", (tournament_id,));

    // Scheduled events.
    let _ = conn.exec_drop(
        "DELETE FROM scheduled_event_judges WHERE tournament_id = ?",
        (tournament_id,),
    );
    let _ = conn.exec_drop(
        "DELETE FROM scheduled_event_winners WHERE tournament_id = ?",
        (tournament_id,),
    );
    let _ = conn.exec_drop(
        "DELETE FROM scheduled_events WHERE tournament_id = ?",
        (tournament_id,),
    );

    // Teams & members.
    let _ = conn.exec_drop(
        "DELETE FROM team_member_categories WHERE tournament_id = ?",
        (tournament_id,),
    );
    let _ = conn.exec_drop(
        "DELETE FROM team_member_events WHERE tournament_id = ?",
        (tournament_id,),
    );
    let _ = conn.exec_drop("DELETE FROM team_members WHERE tournament_id = ?", (tournament_id,));
    let _ = conn.exec_drop("DELETE FROM team_divisions WHERE tournament_id = ?", (tournament_id,));
    let _ = conn.exec_drop("DELETE FROM team_categories WHERE tournament_id = ?", (tournament_id,));
    let _ = conn.exec_drop("DELETE FROM team_events WHERE tournament_id = ?", (tournament_id,));
    let _ = conn.exec_drop("DELETE FROM teams WHERE tournament_id = ?", (tournament_id,));

    // Settings tables.
    let _ = conn.exec_drop("DELETE FROM divisions WHERE tournament_id = ?", (tournament_id,));
    let _ = conn.exec_drop("DELETE FROM categories WHERE tournament_id = ?", (tournament_id,));
    let _ = conn.exec_drop(
        "DELETE FROM weight_classes WHERE tournament_id = ?",
        (tournament_id,),
    );
    let _ = conn.exec_drop("DELETE FROM events WHERE tournament_id = ?", (tournament_id,));

    // Roles.
    let _ = conn.exec_drop(
        "DELETE rp FROM role_permissions rp JOIN tournament_roles r ON r.id = rp.role_id WHERE r.tournament_id = ?",
        (tournament_id,),
    );
    let _ = conn.exec_drop(
        "DELETE FROM tournament_user_roles WHERE tournament_id = ?",
        (tournament_id,),
    );
    let _ = conn.exec_drop("DELETE FROM tournament_roles WHERE tournament_id = ?", (tournament_id,));

    // Tournament users (keep system users).
    let _ = conn.exec_drop(
        "DELETE FROM users WHERE user_type = 'tournament' AND tournament_id = ?",
        (tournament_id,),
    );

    // Slug aliases and tournament itself.
    let _ = conn.exec_drop(
        "DELETE FROM tournament_slug_aliases WHERE tournament_id = ?",
        (tournament_id,),
    );
    conn.exec_drop("DELETE FROM tournaments WHERE id = ?", (tournament_id,))?;
    Ok(())
}

pub fn list_all_teams(conn: &mut PooledConn) -> mysql::Result<Vec<AdminTeamRow>> {
    conn.exec_map(
        "SELECT
           tm.id,
           tm.tournament_id,
           COALESCE(t.name, ''),
           COALESCE(t.slug, ''),
           tm.name,
           tm.logo_url,
           COALESCE(mc.members, 0)
         FROM teams tm
         JOIN tournaments t ON t.id = tm.tournament_id
         LEFT JOIN (
           SELECT tournament_id, team_id, COUNT(*) AS members
           FROM team_members
           GROUP BY tournament_id, team_id
         ) mc ON mc.tournament_id = tm.tournament_id AND mc.team_id = tm.id
         ORDER BY tm.id DESC",
        (),
        |(id, tournament_id, tournament_name, tournament_slug, name, logo_url, members): (
            i64,
            i64,
            String,
            String,
            String,
            Option<String>,
            u64,
        )| AdminTeamRow {
            id,
            tournament_id,
            tournament_name,
            tournament_slug,
            name,
            logo_url,
            members,
        },
    )
}

pub fn list_team_members(
    conn: &mut PooledConn,
    tournament_id: i64,
    team_id: i64,
) -> mysql::Result<Vec<AdminMemberRow>> {
    conn.exec_map(
        "SELECT id, tournament_id, team_id, name, notes, weight_class, division_id, photo_url
         FROM team_members
         WHERE tournament_id = ? AND team_id = ?
         ORDER BY id DESC",
        (tournament_id, team_id),
        |(id, tournament_id, team_id, name, notes, weight_class, division_id, photo_url): (
            i64,
            i64,
            i64,
            String,
            Option<String>,
            Option<String>,
            Option<i64>,
            Option<String>,
        )| AdminMemberRow {
            id,
            tournament_id,
            team_id,
            name,
            notes,
            weight_class,
            division_id,
            photo_url,
        },
    )
}

pub fn delete_team_cascade(
    conn: &mut PooledConn,
    tournament_id: i64,
    team_id: i64,
) -> mysql::Result<()> {
    let _ = conn.exec_drop(
        "DELETE FROM team_member_categories WHERE tournament_id = ? AND team_id = ?",
        (tournament_id, team_id),
    );
    let _ = conn.exec_drop(
        "DELETE FROM team_member_events WHERE tournament_id = ? AND team_id = ?",
        (tournament_id, team_id),
    );
    let _ = conn.exec_drop(
        "DELETE FROM team_members WHERE tournament_id = ? AND team_id = ?",
        (tournament_id, team_id),
    );
    let _ = conn.exec_drop(
        "DELETE FROM team_divisions WHERE tournament_id = ? AND team_id = ?",
        (tournament_id, team_id),
    );
    let _ = conn.exec_drop(
        "DELETE FROM team_categories WHERE tournament_id = ? AND team_id = ?",
        (tournament_id, team_id),
    );
    let _ = conn.exec_drop(
        "DELETE FROM team_events WHERE tournament_id = ? AND team_id = ?",
        (tournament_id, team_id),
    );
    conn.exec_drop(
        "DELETE FROM teams WHERE tournament_id = ? AND id = ?",
        (tournament_id, team_id),
    )?;
    Ok(())
}
