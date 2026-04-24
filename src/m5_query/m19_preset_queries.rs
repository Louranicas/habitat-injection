//! `m19_preset_queries` — Named query presets for the `habitat-query` tool.
//!
//! Each preset function queries the injection database and returns a formatted
//! `String` ready for terminal display — aligned columns, headers, and Unicode
//! separator lines. All functions require the `sqlite` feature.
//!
//! # Presets
//!
//! | Name | Function | Description |
//! |------|----------|-------------|
//! | `trajectory`  | [`query_trajectory`]  | Last N sessions from `session_trajectory` |
//! | `chains`      | [`query_chains`]      | Unresolved causal chains by frequency |
//! | `workstreams` | [`query_workstreams`] | Active + blocked workstreams |
//! | `patterns`    | [`query_patterns`]    | Top patterns by Hebbian weight |
//! | `summary`     | [`query_summary`]     | One-line count of all tables |
//!
//! # Layer
//!
//! `m5_query`
//!
//! # Dependencies
//!
//! `m02_errors`, `m07_causal_chain`, `m08_trajectory`, `m09_workstream`,
//! `m10_pattern`

use crate::m1_foundation::m02_errors::QueryError;

#[cfg(feature = "sqlite")]
use rusqlite::Connection;

#[cfg(feature = "sqlite")]
use crate::m2_schema::{
    m07_causal_chain::{count_unresolved, find_unresolved},
    m08_trajectory::{count as trajectory_count, get_recent},
    m09_workstream::{count_by_status, get_active, get_blocked},
    m10_pattern::{count as pattern_count, get_top_by_weight},
};

// ---------------------------------------------------------------------------
// Module-level column-width constants (sqlite-gated)
// ---------------------------------------------------------------------------

/// Column widths for the trajectory table.
#[cfg(feature = "sqlite")]
mod traj_cols {
    pub const SESSION: usize = 7;
    pub const FITNESS: usize = 7;
    pub const FIELD: usize = 7;
    pub const THERMAL: usize = 7;
    pub const LTP: usize = 7;
    pub const SVC: usize = 8;
    pub const SUMMARY: usize = 30;
}

/// Column widths for the chains table.
#[cfg(feature = "sqlite")]
mod chain_cols {
    pub const LABEL: usize = 30;
    pub const COUNT: usize = 5;
    pub const TYPE: usize = 8;
    pub const DESC: usize = 40;
    pub const SEP_DESC: usize = 11;
}

/// Column widths for the workstreams table.
#[cfg(feature = "sqlite")]
mod ws_cols {
    pub const ID: usize = 15;
    pub const STATUS: usize = 8;
    pub const PRIO: usize = 8;
    pub const PROG: usize = 8;
    pub const TITLE: usize = 30;
}

/// Column widths for the patterns table.
#[cfg(feature = "sqlite")]
mod pat_cols {
    pub const PATTERN: usize = 28;
    pub const WEIGHT: usize = 6;
    pub const HITS: usize = 4;
    pub const CAT: usize = 11;
    pub const DESC: usize = 35;
    pub const SEP_DESC: usize = 11;
}

// ---------------------------------------------------------------------------
// Internal formatting helpers
// ---------------------------------------------------------------------------

/// Produce a separator string of `n` Unicode box-drawing dashes.
#[cfg(feature = "sqlite")]
fn sep(n: usize) -> String {
    "─".repeat(n)
}

/// Pad `s` to exactly `width` characters on the right, or truncate to
/// `width` if longer. The result is always ASCII-safe for printable columns.
#[cfg(feature = "sqlite")]
fn cell(s: &str, width: usize) -> String {
    if s.len() >= width {
        // Find the last valid char boundary ≤ `width` bytes.
        let mut boundary = width;
        while !s.is_char_boundary(boundary) {
            boundary -= 1;
        }
        s[..boundary].to_owned()
    } else {
        format!("{s:<width$}")
    }
}

/// Map a schema error to [`QueryError::ExecutionFailed`].
#[cfg(feature = "sqlite")]
use super::query_err;

// ---------------------------------------------------------------------------
// query_trajectory
// ---------------------------------------------------------------------------

/// Return the last `limit` sessions from `session_trajectory`, formatted as
/// an aligned table.
///
/// Default limit when `limit == 0`: 10.
///
/// # Errors
///
/// Returns [`QueryError::ExecutionFailed`] on any database error.
#[cfg(feature = "sqlite")]
pub fn query_trajectory(conn: &Connection, limit: usize) -> Result<String, QueryError> {
    use traj_cols::{FIELD, FITNESS, LTP, SESSION, SUMMARY, SVC, THERMAL};

    let effective_limit = if limit == 0 { 10 } else { limit };
    let rows = get_recent(conn, effective_limit).map_err(|ref e| query_err(e))?;

    let header = format!(
        "{:<SESSION$}  {:<FITNESS$}  {:<FIELD$}  {:<THERMAL$}  {:<LTP$}  {:<SVC$}  {}",
        "SESSION", "FITNESS", "FIELD_R", "THERMAL", "LTP/LTD", "SERVICES", "SUMMARY"
    );
    let separator = format!(
        "{}  {}  {}  {}  {}  {}  {}",
        sep(SESSION),
        sep(FITNESS),
        sep(FIELD),
        sep(THERMAL),
        sep(LTP),
        sep(SVC),
        sep(7),
    );

    let mut out = String::with_capacity(256 + rows.len() * 120);
    out.push_str(&header);
    out.push('\n');
    out.push_str(&separator);
    out.push('\n');

    if rows.is_empty() {
        out.push_str("(no sessions recorded)\n");
        return Ok(out);
    }

    for r in &rows {
        let session_label = format!("S{}", r.session_id);
        let fitness_str = format!("{:.3}", r.ralph_fitness);
        let field_str = format!("{:.3}", r.field_r);
        let thermal_str = format!("{:.3}", r.thermal_t);
        let ltp_str = format!("{:.2}", r.ltp_ltd_ratio);
        let svc_str = r.services_healthy.to_string();

        let line = format!(
            "{}  {}  {}  {}  {}  {}  {}",
            cell(&session_label, SESSION),
            cell(&fitness_str, FITNESS),
            cell(&field_str, FIELD),
            cell(&thermal_str, THERMAL),
            cell(&ltp_str, LTP),
            cell(&svc_str, SVC),
            cell(&r.delta_summary, SUMMARY),
        );
        out.push_str(&line);
        out.push('\n');
    }

    Ok(out)
}

// ---------------------------------------------------------------------------
// query_chains
// ---------------------------------------------------------------------------

/// Return up to `limit` unresolved causal chains ordered by reinforcement
/// frequency, formatted as an aligned table.
///
/// Default limit when `limit == 0`: 20.
///
/// # Errors
///
/// Returns [`QueryError::ExecutionFailed`] on any database error.
#[cfg(feature = "sqlite")]
pub fn query_chains(conn: &Connection, limit: usize) -> Result<String, QueryError> {
    use chain_cols::{COUNT, DESC, LABEL, SEP_DESC, TYPE};

    let effective_limit = if limit == 0 { 20 } else { limit };
    let rows = find_unresolved(conn, effective_limit).map_err(|ref e| query_err(e))?;

    let header = format!(
        "{:<LABEL$}  {:<COUNT$}  {:<TYPE$}  {}",
        "LABEL", "COUNT", "TYPE", "DESCRIPTION"
    );
    let separator = format!(
        "{}  {}  {}  {}",
        sep(LABEL),
        sep(COUNT),
        sep(TYPE),
        sep(SEP_DESC),
    );

    let mut out = String::with_capacity(256 + rows.len() * 100);
    out.push_str(&header);
    out.push('\n');
    out.push_str(&separator);
    out.push('\n');

    if rows.is_empty() {
        out.push_str("(no unresolved chains)\n");
        return Ok(out);
    }

    for r in &rows {
        let count_str = r.reinforcement_count.to_string();
        let line = format!(
            "{}  {}  {}  {}",
            cell(&r.label, LABEL),
            cell(&count_str, COUNT),
            cell(&r.chain_type, TYPE),
            cell(&r.description, DESC),
        );
        out.push_str(&line);
        out.push('\n');
    }

    Ok(out)
}

// ---------------------------------------------------------------------------
// query_workstreams
// ---------------------------------------------------------------------------

/// Return active and blocked workstreams, formatted as an aligned table.
///
/// Active rows appear first (ordered by priority), followed by blocked rows.
///
/// # Errors
///
/// Returns [`QueryError::ExecutionFailed`] on any database error.
#[cfg(feature = "sqlite")]
pub fn query_workstreams(conn: &Connection) -> Result<String, QueryError> {
    use ws_cols::{ID, PRIO, PROG, STATUS, TITLE};

    let active = get_active(conn).map_err(|ref e| query_err(e))?;
    let blocked = get_blocked(conn).map_err(|ref e| query_err(e))?;

    let header = format!(
        "{:<ID$}  {:<STATUS$}  {:<PRIO$}  {:<PROG$}  {}",
        "ID", "STATUS", "PRIORITY", "PROGRESS", "TITLE"
    );
    let separator = format!(
        "{}  {}  {}  {}  {}",
        sep(ID),
        sep(STATUS),
        sep(PRIO),
        sep(PROG),
        sep(5),
    );

    let total = active.len() + blocked.len();
    let mut out = String::with_capacity(256 + total * 100);
    out.push_str(&header);
    out.push('\n');
    out.push_str(&separator);
    out.push('\n');

    if total == 0 {
        out.push_str("(no active or blocked workstreams)\n");
        return Ok(out);
    }

    for r in active.iter().chain(blocked.iter()) {
        let prio_str = r.priority.to_string();
        let prog_str = match (r.items_done, r.items_total) {
            (Some(done), Some(tot)) => format!("{done}/{tot}"),
            _ => "-".to_string(),
        };
        let line = format!(
            "{}  {}  {}  {}  {}",
            cell(&r.ws_id, ID),
            cell(&r.status, STATUS),
            cell(&prio_str, PRIO),
            cell(&prog_str, PROG),
            cell(&r.title, TITLE),
        );
        out.push_str(&line);
        out.push('\n');
    }

    Ok(out)
}

// ---------------------------------------------------------------------------
// query_patterns
// ---------------------------------------------------------------------------

/// Return up to `limit` patterns ordered by Hebbian weight descending,
/// formatted as an aligned table.
///
/// Default limit when `limit == 0`: 20.
///
/// # Errors
///
/// Returns [`QueryError::ExecutionFailed`] on any database error.
#[cfg(feature = "sqlite")]
pub fn query_patterns(conn: &Connection, limit: usize) -> Result<String, QueryError> {
    use pat_cols::{CAT, DESC, HITS, PATTERN, SEP_DESC, WEIGHT};

    let effective_limit = if limit == 0 { 20 } else { limit };
    let rows = get_top_by_weight(conn, effective_limit).map_err(|ref e| query_err(e))?;

    let header = format!(
        "{:<PATTERN$}  {:<WEIGHT$}  {:<HITS$}  {:<CAT$}  {}",
        "PATTERN", "WEIGHT", "HITS", "CATEGORY", "DESCRIPTION"
    );
    let separator = format!(
        "{}  {}  {}  {}  {}",
        sep(PATTERN),
        sep(WEIGHT),
        sep(HITS),
        sep(CAT),
        sep(SEP_DESC),
    );

    let mut out = String::with_capacity(256 + rows.len() * 110);
    out.push_str(&header);
    out.push('\n');
    out.push_str(&separator);
    out.push('\n');

    if rows.is_empty() {
        out.push_str("(no patterns recorded)\n");
        return Ok(out);
    }

    for r in &rows {
        let weight_str = format!("{:.4}", r.weight);
        let hits_str = r.hit_count.to_string();
        let line = format!(
            "{}  {}  {}  {}  {}",
            cell(&r.pattern_id, PATTERN),
            cell(&weight_str, WEIGHT),
            cell(&hits_str, HITS),
            cell(&r.category, CAT),
            cell(&r.description, DESC),
        );
        out.push_str(&line);
        out.push('\n');
    }

    Ok(out)
}

// ---------------------------------------------------------------------------
// query_summary
// ---------------------------------------------------------------------------

/// Return a single-line summary of all table counts.
///
/// Format:
/// ```text
/// Chains: 15 (5 unresolved) | Sessions: 109 | Workstreams: 6 (2 active, 1 blocked) | Patterns: 45
/// ```
///
/// # Errors
///
/// Returns [`QueryError::ExecutionFailed`] on any database error.
#[cfg(feature = "sqlite")]
pub fn query_summary(conn: &Connection) -> Result<String, QueryError> {
    let chain_unresolved = count_unresolved(conn).map_err(|ref e| query_err(e))?;
    let chain_total: u64 = conn
        .query_row(
            "SELECT COUNT(*) FROM causal_chain",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(i64::cast_unsigned)
        .map_err(|e| query_err(&e))?;

    let session_total = trajectory_count(conn).map_err(|ref e| query_err(e))?;

    let ws_active = count_by_status(conn, "active").map_err(|ref e| query_err(e))?;
    let ws_blocked = count_by_status(conn, "blocked").map_err(|ref e| query_err(e))?;
    let ws_deferred = count_by_status(conn, "deferred").map_err(|ref e| query_err(e))?;
    let ws_complete = count_by_status(conn, "complete").map_err(|ref e| query_err(e))?;
    let ws_total = ws_active + ws_blocked + ws_deferred + ws_complete;

    let pattern_total = pattern_count(conn).map_err(|ref e| query_err(e))?;

    Ok(format!(
        "Chains: {chain_total} ({chain_unresolved} unresolved) | \
         Sessions: {session_total} | \
         Workstreams: {ws_total} ({ws_active} active, {ws_blocked} blocked) | \
         Patterns: {pattern_total}"
    ))
}

// ---------------------------------------------------------------------------
// query_preset (dispatcher)
// ---------------------------------------------------------------------------

/// Dispatch a named preset to its corresponding query function.
///
/// Valid `name` values: `"trajectory"`, `"chains"`, `"workstreams"`,
/// `"patterns"`, `"summary"`.
///
/// # Errors
///
/// - [`QueryError::ExecutionFailed`] for unknown preset names, propagated
///   from the dispatched function on database failure.
#[cfg(feature = "sqlite")]
pub fn query_preset(conn: &Connection, name: &str) -> Result<String, QueryError> {
    match name {
        "trajectory" => query_trajectory(conn, 10),
        "chains" => query_chains(conn, 20),
        "workstreams" => query_workstreams(conn),
        "patterns" => query_patterns(conn, 20),
        "summary" => query_summary(conn),
        other => Err(QueryError::ExecutionFailed(format!(
            "unknown preset '{other}'; valid: trajectory, chains, workstreams, patterns, summary"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use crate::m2_schema::{
        m06_schema::open_memory,
        m07_causal_chain::insert_chain,
        m08_trajectory::insert_point,
        m09_workstream::{insert_workstream, set_blocker, update_progress},
        m10_pattern::{insert_pattern, reinforce},
    };

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn mem() -> Connection {
        open_memory().expect("open_memory must succeed in tests")
    }

    fn seed_trajectories(conn: &Connection, n: u32) {
        for i in 1..=n {
            insert_point(
                conn,
                i,
                0.5 + f64::from(i) * 0.01,
                0.8,
                0.5,
                f64::from(i) * 0.5,
                11,
                &format!("session {i} summary"),
                None,
            )
            .expect("insert_point must succeed");
        }
    }

    fn seed_chains(conn: &Connection, n: u32) {
        for i in 1..=n {
            insert_chain(
                conn,
                100 + i,
                "bug",
                &format!("BUG-{i:03}"),
                &format!("description for BUG-{i}"),
            )
            .expect("insert_chain must succeed");
        }
    }

    fn seed_workstreams(conn: &Connection) {
        insert_workstream(conn, "ws-active-1", "Active One", "active", 109, "resume active-1")
            .expect("insert_workstream must succeed");
        insert_workstream(conn, "ws-active-2", "Active Two", "active", 108, "resume active-2")
            .expect("insert_workstream must succeed");
        insert_workstream(
            conn,
            "ws-blocked-1",
            "Blocked One",
            "blocked",
            107,
            "resume blocked-1",
        )
        .expect("insert_workstream must succeed");
    }

    fn seed_patterns(conn: &Connection, n: usize) {
        let categories = ["procedural", "semantic", "trap", "feedback"];
        for i in 1..=n {
            let cat = categories[(i - 1) % 4];
            insert_pattern(
                conn,
                &format!("pattern-{i:03}"),
                cat,
                &format!("description of pattern {i}"),
                None,
            )
            .expect("insert_pattern must succeed");
        }
    }

    // -----------------------------------------------------------------------
    // query_trajectory — empty DB
    // -----------------------------------------------------------------------

    #[test]
    fn trajectory_empty_db_returns_header_and_empty_message() {
        let conn = mem();
        let result = query_trajectory(&conn, 10).expect("query must succeed");
        assert!(result.contains("SESSION"));
        assert!(result.contains("FITNESS"));
        assert!(result.contains("no sessions recorded"));
    }

    #[test]
    fn trajectory_empty_db_has_separator() {
        let conn = mem();
        let result = query_trajectory(&conn, 10).expect("query must succeed");
        assert!(result.contains('─'));
    }

    // -----------------------------------------------------------------------
    // query_trajectory — seeded data
    // -----------------------------------------------------------------------

    #[test]
    fn trajectory_seeded_contains_session_labels() {
        let conn = mem();
        seed_trajectories(&conn, 3);
        let result = query_trajectory(&conn, 10).expect("query must succeed");
        assert!(result.contains("S3"));
        assert!(result.contains("S2"));
        assert!(result.contains("S1"));
    }

    #[test]
    fn trajectory_limit_respected() {
        let conn = mem();
        seed_trajectories(&conn, 15);
        let result = query_trajectory(&conn, 5).expect("query must succeed");
        // Latest 5 sessions appear; oldest session label "S1 " truncated
        assert!(result.contains("S15"));
        // "S10" and "S1" share a prefix — check that S1 (alone) is absent
        // by verifying S10 is there but S9 is not (we have S15..S11)
        assert!(!result.contains("S9"));
    }

    #[test]
    fn trajectory_zero_limit_uses_default_10() {
        let conn = mem();
        seed_trajectories(&conn, 12);
        let result = query_trajectory(&conn, 0).expect("query must succeed");
        // Default 10: session 12 must appear
        assert!(result.contains("S12"));
    }

    #[test]
    fn trajectory_has_all_column_headers() {
        let conn = mem();
        let result = query_trajectory(&conn, 1).expect("query must succeed");
        assert!(result.contains("SESSION"));
        assert!(result.contains("FITNESS"));
        assert!(result.contains("FIELD_R"));
        assert!(result.contains("THERMAL"));
        assert!(result.contains("LTP/LTD"));
        assert!(result.contains("SERVICES"));
        assert!(result.contains("SUMMARY"));
    }

    #[test]
    fn trajectory_separator_line_present() {
        let conn = mem();
        seed_trajectories(&conn, 2);
        let result = query_trajectory(&conn, 5).expect("query must succeed");
        assert!(result.contains('─'));
    }

    #[test]
    fn trajectory_delta_summary_appears_in_output() {
        let conn = mem();
        insert_point(&conn, 1, 0.5, 0.0, 0.5, 1.0, 11, "fitness stable", None)
            .expect("insert must succeed");
        let result = query_trajectory(&conn, 5).expect("query must succeed");
        assert!(result.contains("fitness stable"));
    }

    #[test]
    fn trajectory_fitness_formatted_as_decimal() {
        let conn = mem();
        insert_point(&conn, 1, 0.664, 0.876, 0.515, 4.2, 11, "ok", None)
            .expect("insert must succeed");
        let result = query_trajectory(&conn, 5).expect("query must succeed");
        assert!(result.contains("0.664"));
    }

    #[test]
    fn trajectory_ordered_newest_first() {
        let conn = mem();
        seed_trajectories(&conn, 5);
        let result = query_trajectory(&conn, 5).expect("query must succeed");
        let s5_pos = result.find("S5").expect("S5 must appear");
        let s1_pos = result.find("S1").expect("S1 must appear");
        assert!(s5_pos < s1_pos, "newest session must appear before oldest");
    }

    #[test]
    fn trajectory_services_healthy_appears() {
        let conn = mem();
        insert_point(&conn, 1, 0.5, 0.0, 0.5, 1.0, 10, "ok", None)
            .expect("insert must succeed");
        let result = query_trajectory(&conn, 5).expect("query must succeed");
        assert!(result.contains("10"));
    }

    #[test]
    fn trajectory_ltp_ltd_formatted() {
        let conn = mem();
        insert_point(&conn, 1, 0.5, 0.0, 0.5, 9.88, 11, "ok", None)
            .expect("insert must succeed");
        let result = query_trajectory(&conn, 5).expect("query must succeed");
        assert!(result.contains("9.88"));
    }

    #[test]
    fn trajectory_single_row_no_crash() {
        let conn = mem();
        insert_point(&conn, 109, 0.669, 0.876, 0.515, 4.2, 11, "fitness stable", None)
            .expect("insert must succeed");
        let result = query_trajectory(&conn, 10).expect("query must succeed");
        assert!(result.contains("S109"));
    }

    // -----------------------------------------------------------------------
    // query_chains — empty DB
    // -----------------------------------------------------------------------

    #[test]
    fn chains_empty_db_returns_header_and_empty_message() {
        let conn = mem();
        let result = query_chains(&conn, 20).expect("query must succeed");
        assert!(result.contains("LABEL"));
        assert!(result.contains("COUNT"));
        assert!(result.contains("no unresolved chains"));
    }

    #[test]
    fn chains_empty_db_has_separator() {
        let conn = mem();
        let result = query_chains(&conn, 20).expect("query must succeed");
        assert!(result.contains('─'));
    }

    // -----------------------------------------------------------------------
    // query_chains — seeded data
    // -----------------------------------------------------------------------

    #[test]
    fn chains_seeded_shows_labels() {
        let conn = mem();
        seed_chains(&conn, 3);
        let result = query_chains(&conn, 10).expect("query must succeed");
        assert!(result.contains("BUG-001"));
    }

    #[test]
    fn chains_shows_chain_type() {
        let conn = mem();
        insert_chain(&conn, 109, "trap", "TRAP-1", "a trap").expect("insert must succeed");
        let result = query_chains(&conn, 10).expect("query must succeed");
        assert!(result.contains("trap"));
    }

    #[test]
    fn chains_has_all_headers() {
        let conn = mem();
        let result = query_chains(&conn, 1).expect("query must succeed");
        assert!(result.contains("LABEL"));
        assert!(result.contains("COUNT"));
        assert!(result.contains("TYPE"));
        assert!(result.contains("DESCRIPTION"));
    }

    #[test]
    fn chains_separator_present() {
        let conn = mem();
        seed_chains(&conn, 1);
        let result = query_chains(&conn, 5).expect("query must succeed");
        assert!(result.contains('─'));
    }

    #[test]
    fn chains_limit_respected() {
        let conn = mem();
        // Insert 10 chains; with limit=3 only 3 rows appear.
        // Count lines that start with "BUG-" by checking row count:
        // header + separator + 3 data rows + final newline = at least 5 lines.
        // After the separator line, there should be exactly 3 data rows.
        seed_chains(&conn, 10);
        let result = query_chains(&conn, 3).expect("query must succeed");
        let data_lines: Vec<&str> = result
            .lines()
            .skip(2) // skip header + separator
            .filter(|l| !l.is_empty())
            .collect();
        assert_eq!(data_lines.len(), 3);
    }

    #[test]
    fn chains_zero_limit_uses_default_20() {
        let conn = mem();
        seed_chains(&conn, 5);
        let result = query_chains(&conn, 0).expect("query must succeed");
        assert!(result.contains("BUG-001"));
    }

    #[test]
    fn chains_excludes_resolved_chains() {
        let conn = mem();
        let id =
            insert_chain(&conn, 109, "bug", "RESOLVED", "resolved one").expect("insert must succeed");
        crate::m2_schema::m07_causal_chain::resolve_chain(&conn, id, 110)
            .expect("resolve must succeed");
        insert_chain(&conn, 109, "bug", "OPEN", "open one").expect("insert must succeed");
        let result = query_chains(&conn, 10).expect("query must succeed");
        assert!(result.contains("OPEN"));
        assert!(!result.contains("RESOLVED"));
    }

    #[test]
    fn chains_description_appears() {
        let conn = mem();
        insert_chain(&conn, 109, "bug", "DESC-TEST", "very specific description text")
            .expect("insert must succeed");
        let result = query_chains(&conn, 5).expect("query must succeed");
        assert!(result.contains("very specific description text"));
    }

    #[test]
    fn chains_reinforcement_count_appears() {
        let conn = mem();
        insert_chain(&conn, 109, "bug", "REINF-COUNT", "desc").expect("insert must succeed");
        crate::m2_schema::m07_causal_chain::reinforce_chain(&conn, "REINF-COUNT", 110)
            .expect("reinforce must succeed");
        crate::m2_schema::m07_causal_chain::reinforce_chain(&conn, "REINF-COUNT", 111)
            .expect("reinforce must succeed");
        // reinforcement_count should now be 3 (initial 1 + 2 reinforcements)
        let result = query_chains(&conn, 10).expect("query must succeed");
        assert!(result.contains('3'));
    }

    // -----------------------------------------------------------------------
    // query_workstreams — empty DB
    // -----------------------------------------------------------------------

    #[test]
    fn workstreams_empty_db_returns_headers_and_empty_message() {
        let conn = mem();
        let result = query_workstreams(&conn).expect("query must succeed");
        assert!(result.contains("ID"));
        assert!(result.contains("STATUS"));
        assert!(result.contains("no active or blocked workstreams"));
    }

    #[test]
    fn workstreams_empty_db_has_separator() {
        let conn = mem();
        let result = query_workstreams(&conn).expect("query must succeed");
        assert!(result.contains('─'));
    }

    // -----------------------------------------------------------------------
    // query_workstreams — seeded data
    // -----------------------------------------------------------------------

    #[test]
    fn workstreams_seeded_shows_ids() {
        let conn = mem();
        seed_workstreams(&conn);
        let result = query_workstreams(&conn).expect("query must succeed");
        assert!(result.contains("ws-active-1"));
        assert!(result.contains("ws-active-2"));
        assert!(result.contains("ws-blocked-1"));
    }

    #[test]
    fn workstreams_has_all_headers() {
        let conn = mem();
        let result = query_workstreams(&conn).expect("query must succeed");
        assert!(result.contains("ID"));
        assert!(result.contains("STATUS"));
        assert!(result.contains("PRIORITY"));
        assert!(result.contains("PROGRESS"));
        assert!(result.contains("TITLE"));
    }

    #[test]
    fn workstreams_separator_present() {
        let conn = mem();
        seed_workstreams(&conn);
        let result = query_workstreams(&conn).expect("query must succeed");
        assert!(result.contains('─'));
    }

    #[test]
    fn workstreams_shows_active_status() {
        let conn = mem();
        insert_workstream(&conn, "ws-a", "An active ws", "active", 109, "ctx")
            .expect("insert must succeed");
        let result = query_workstreams(&conn).expect("query must succeed");
        assert!(result.contains("active"));
    }

    #[test]
    fn workstreams_shows_blocked_status() {
        let conn = mem();
        insert_workstream(&conn, "ws-b", "A blocked ws", "blocked", 109, "ctx")
            .expect("insert must succeed");
        let result = query_workstreams(&conn).expect("query must succeed");
        assert!(result.contains("blocked"));
    }

    #[test]
    fn workstreams_excludes_deferred_and_complete() {
        let conn = mem();
        insert_workstream(&conn, "ws-def", "Deferred", "deferred", 109, "ctx")
            .expect("insert must succeed");
        insert_workstream(&conn, "ws-cmp", "Complete", "complete", 109, "ctx")
            .expect("insert must succeed");
        let result = query_workstreams(&conn).expect("query must succeed");
        assert!(result.contains("no active or blocked workstreams"));
    }

    #[test]
    fn workstreams_progress_shows_when_set() {
        let conn = mem();
        insert_workstream(&conn, "ws-prog", "Progress ws", "active", 109, "ctx")
            .expect("insert must succeed");
        update_progress(&conn, "ws-prog", 10, 16, 109).expect("update must succeed");
        let result = query_workstreams(&conn).expect("query must succeed");
        assert!(result.contains("10/16"));
    }

    #[test]
    fn workstreams_progress_shows_dash_when_null() {
        let conn = mem();
        insert_workstream(&conn, "ws-noprog", "No progress ws", "active", 109, "ctx")
            .expect("insert must succeed");
        let result = query_workstreams(&conn).expect("query must succeed");
        assert!(result.contains('-'));
    }

    #[test]
    fn workstreams_title_appears() {
        let conn = mem();
        insert_workstream(&conn, "ws-title", "My Specific Title", "active", 109, "ctx")
            .expect("insert must succeed");
        let result = query_workstreams(&conn).expect("query must succeed");
        assert!(result.contains("My Specific Title"));
    }

    #[test]
    fn blocked_workstream_appears_in_workstreams_preset() {
        let conn = mem();
        insert_workstream(&conn, "ws-blk", "Blocked WS", "active", 109, "ctx")
            .expect("insert must succeed");
        set_blocker(&conn, "ws-blk", "waiting on apt", 110).expect("set_blocker must succeed");
        let result = query_workstreams(&conn).expect("query must succeed");
        assert!(result.contains("ws-blk"));
        assert!(result.contains("blocked"));
    }

    // -----------------------------------------------------------------------
    // query_patterns — empty DB
    // -----------------------------------------------------------------------

    #[test]
    fn patterns_empty_db_returns_header_and_empty_message() {
        let conn = mem();
        let result = query_patterns(&conn, 20).expect("query must succeed");
        assert!(result.contains("PATTERN"));
        assert!(result.contains("WEIGHT"));
        assert!(result.contains("no patterns recorded"));
    }

    #[test]
    fn patterns_empty_db_has_separator() {
        let conn = mem();
        let result = query_patterns(&conn, 20).expect("query must succeed");
        assert!(result.contains('─'));
    }

    // -----------------------------------------------------------------------
    // query_patterns — seeded data
    // -----------------------------------------------------------------------

    #[test]
    fn patterns_seeded_shows_pattern_ids() {
        let conn = mem();
        seed_patterns(&conn, 3);
        let result = query_patterns(&conn, 10).expect("query must succeed");
        assert!(result.contains("pattern-001"));
    }

    #[test]
    fn patterns_has_all_headers() {
        let conn = mem();
        let result = query_patterns(&conn, 1).expect("query must succeed");
        assert!(result.contains("PATTERN"));
        assert!(result.contains("WEIGHT"));
        assert!(result.contains("HITS"));
        assert!(result.contains("CATEGORY"));
        assert!(result.contains("DESCRIPTION"));
    }

    #[test]
    fn patterns_separator_present() {
        let conn = mem();
        seed_patterns(&conn, 1);
        let result = query_patterns(&conn, 5).expect("query must succeed");
        assert!(result.contains('─'));
    }

    #[test]
    fn patterns_limit_respected() {
        let conn = mem();
        seed_patterns(&conn, 10);
        let result = query_patterns(&conn, 3).expect("query must succeed");
        let matches = result.matches("pattern-").count();
        assert_eq!(matches, 3);
    }

    #[test]
    fn patterns_zero_limit_uses_default_20() {
        let conn = mem();
        seed_patterns(&conn, 5);
        let result = query_patterns(&conn, 0).expect("query must succeed");
        assert!(result.contains("pattern-001"));
    }

    #[test]
    fn patterns_ordered_by_weight_desc() {
        let conn = mem();
        insert_pattern(&conn, "low-weight", "trap", "low weight pattern", None)
            .expect("insert must succeed");
        insert_pattern(&conn, "high-weight", "procedural", "high weight pattern", None)
            .expect("insert must succeed");
        for s in 0_u32..20 {
            reinforce(&conn, "high-weight", s).expect("reinforce must succeed");
        }
        let result = query_patterns(&conn, 10).expect("query must succeed");
        let high_pos = result.find("high-weight").expect("high-weight must appear");
        let low_pos = result.find("low-weight").expect("low-weight must appear");
        assert!(high_pos < low_pos, "higher weight pattern must appear first");
    }

    #[test]
    fn patterns_weight_formatted_as_decimal() {
        let conn = mem();
        insert_pattern(&conn, "p1", "trap", "desc", None).expect("insert must succeed");
        let result = query_patterns(&conn, 5).expect("query must succeed");
        // Default weight is 0.5; should appear as "0.5000"
        assert!(result.contains("0.5000"));
    }

    #[test]
    fn patterns_category_appears() {
        let conn = mem();
        insert_pattern(&conn, "p1", "semantic", "desc", None).expect("insert must succeed");
        let result = query_patterns(&conn, 5).expect("query must succeed");
        assert!(result.contains("semantic"));
    }

    #[test]
    fn patterns_hit_count_appears() {
        let conn = mem();
        insert_pattern(&conn, "p1", "procedural", "desc", None).expect("insert must succeed");
        reinforce(&conn, "p1", 109).expect("reinforce must succeed");
        // hit_count starts at 1 and is incremented by reinforce → 2
        let result = query_patterns(&conn, 5).expect("query must succeed");
        assert!(result.contains('2'));
    }

    #[test]
    fn patterns_description_appears() {
        let conn = mem();
        insert_pattern(&conn, "p1", "feedback", "unique description text here", None)
            .expect("insert must succeed");
        let result = query_patterns(&conn, 5).expect("query must succeed");
        assert!(result.contains("unique description text here"));
    }

    // -----------------------------------------------------------------------
    // query_summary
    // -----------------------------------------------------------------------

    #[test]
    fn summary_empty_db_all_zeros() {
        let conn = mem();
        let result = query_summary(&conn).expect("query must succeed");
        assert!(result.contains("Chains: 0"));
        assert!(result.contains("Sessions: 0"));
        assert!(result.contains("Workstreams: 0"));
        assert!(result.contains("Patterns: 0"));
    }

    #[test]
    fn summary_chain_count_accurate() {
        let conn = mem();
        seed_chains(&conn, 5);
        let result = query_summary(&conn).expect("query must succeed");
        assert!(result.contains("Chains: 5"));
    }

    #[test]
    fn summary_unresolved_count_accurate() {
        let conn = mem();
        let id1 = insert_chain(&conn, 109, "bug", "OPEN-1", "open").expect("insert must succeed");
        insert_chain(&conn, 109, "bug", "OPEN-2", "open 2").expect("insert must succeed");
        crate::m2_schema::m07_causal_chain::resolve_chain(&conn, id1, 110)
            .expect("resolve must succeed");
        let result = query_summary(&conn).expect("query must succeed");
        assert!(result.contains("Chains: 2 (1 unresolved)"));
    }

    #[test]
    fn summary_session_count_accurate() {
        let conn = mem();
        seed_trajectories(&conn, 7);
        let result = query_summary(&conn).expect("query must succeed");
        assert!(result.contains("Sessions: 7"));
    }

    #[test]
    fn summary_workstream_active_count_accurate() {
        let conn = mem();
        insert_workstream(&conn, "w1", "T", "active", 109, "ctx").expect("insert must succeed");
        insert_workstream(&conn, "w2", "T", "active", 109, "ctx").expect("insert must succeed");
        insert_workstream(&conn, "w3", "T", "blocked", 109, "ctx").expect("insert must succeed");
        let result = query_summary(&conn).expect("query must succeed");
        assert!(result.contains("2 active"));
        assert!(result.contains("1 blocked"));
    }

    #[test]
    fn summary_workstream_total_includes_all_statuses() {
        let conn = mem();
        for (i, status) in ["active", "blocked", "deferred", "complete"].iter().enumerate() {
            insert_workstream(&conn, &format!("ws-sum-{i}"), "T", status, 109, "ctx")
                .expect("insert must succeed");
        }
        let result = query_summary(&conn).expect("query must succeed");
        assert!(result.contains("Workstreams: 4"));
    }

    #[test]
    fn summary_pattern_count_accurate() {
        let conn = mem();
        seed_patterns(&conn, 9);
        let result = query_summary(&conn).expect("query must succeed");
        assert!(result.contains("Patterns: 9"));
    }

    #[test]
    fn summary_has_pipe_separators() {
        let conn = mem();
        let result = query_summary(&conn).expect("query must succeed");
        assert!(result.contains('|'));
    }

    #[test]
    fn summary_is_single_line() {
        let conn = mem();
        let result = query_summary(&conn).expect("query must succeed");
        assert_eq!(result.lines().count(), 1);
    }

    #[test]
    fn summary_zero_unresolved_when_all_resolved() {
        let conn = mem();
        let id =
            insert_chain(&conn, 109, "bug", "CLOSED", "desc").expect("insert must succeed");
        crate::m2_schema::m07_causal_chain::resolve_chain(&conn, id, 110)
            .expect("resolve must succeed");
        let result = query_summary(&conn).expect("query must succeed");
        assert!(result.contains("(0 unresolved)"));
    }

    // -----------------------------------------------------------------------
    // query_preset — dispatcher
    // -----------------------------------------------------------------------

    #[test]
    fn preset_trajectory_dispatches() {
        let conn = mem();
        let result = query_preset(&conn, "trajectory").expect("must succeed");
        assert!(result.contains("SESSION"));
    }

    #[test]
    fn preset_chains_dispatches() {
        let conn = mem();
        let result = query_preset(&conn, "chains").expect("must succeed");
        assert!(result.contains("LABEL"));
    }

    #[test]
    fn preset_workstreams_dispatches() {
        let conn = mem();
        let result = query_preset(&conn, "workstreams").expect("must succeed");
        assert!(result.contains("ID"));
        assert!(result.contains("STATUS"));
    }

    #[test]
    fn preset_patterns_dispatches() {
        let conn = mem();
        let result = query_preset(&conn, "patterns").expect("must succeed");
        assert!(result.contains("PATTERN"));
    }

    #[test]
    fn preset_summary_dispatches() {
        let conn = mem();
        let result = query_preset(&conn, "summary").expect("must succeed");
        assert!(result.contains("Chains:"));
    }

    #[test]
    fn preset_unknown_name_returns_error() {
        let conn = mem();
        let result = query_preset(&conn, "unknown-preset");
        assert!(result.is_err());
        if let Err(QueryError::ExecutionFailed(msg)) = result {
            assert!(msg.contains("unknown-preset"));
        } else {
            panic!("expected ExecutionFailed error");
        }
    }

    #[test]
    fn preset_empty_name_returns_error() {
        let conn = mem();
        let result = query_preset(&conn, "");
        assert!(result.is_err());
    }

    #[test]
    fn preset_case_sensitive_capitals_error() {
        let conn = mem();
        let result = query_preset(&conn, "Trajectory");
        assert!(result.is_err());
    }

    #[test]
    fn preset_error_message_lists_valid_presets() {
        let conn = mem();
        let err = query_preset(&conn, "bogus").expect_err("must fail");
        let msg = err.to_string();
        assert!(msg.contains("trajectory"));
        assert!(msg.contains("chains"));
        assert!(msg.contains("workstreams"));
        assert!(msg.contains("patterns"));
        assert!(msg.contains("summary"));
    }

    // -----------------------------------------------------------------------
    // cell helper
    // -----------------------------------------------------------------------

    #[test]
    fn cell_pads_short_string() {
        let result = cell("hi", 6);
        assert_eq!(result.len(), 6);
        assert_eq!(&result, "hi    ");
    }

    #[test]
    fn cell_truncates_long_string() {
        let result = cell("hello world", 5);
        assert_eq!(result.len(), 5);
        assert_eq!(&result, "hello");
    }

    #[test]
    fn cell_exact_length_unchanged() {
        let result = cell("exact", 5);
        assert_eq!(result, "exact");
    }

    #[test]
    fn cell_empty_string_pads_to_width() {
        let result = cell("", 4);
        assert_eq!(result, "    ");
        assert_eq!(result.len(), 4);
    }

    // -----------------------------------------------------------------------
    // sep helper
    // -----------------------------------------------------------------------

    #[test]
    fn sep_produces_correct_char_count() {
        let r = sep(10);
        assert_eq!(r.chars().count(), 10);
    }

    #[test]
    fn sep_zero_produces_empty() {
        let r = sep(0);
        assert!(r.is_empty());
    }

    // -----------------------------------------------------------------------
    // Integration
    // -----------------------------------------------------------------------

    #[test]
    fn all_presets_succeed_on_mixed_db() {
        let conn = mem();
        seed_trajectories(&conn, 5);
        seed_chains(&conn, 4);
        seed_workstreams(&conn);
        seed_patterns(&conn, 6);

        query_preset(&conn, "trajectory").expect("trajectory must succeed");
        query_preset(&conn, "chains").expect("chains must succeed");
        query_preset(&conn, "workstreams").expect("workstreams must succeed");
        query_preset(&conn, "patterns").expect("patterns must succeed");
        query_preset(&conn, "summary").expect("summary must succeed");
    }

    #[test]
    fn summary_counts_agree_with_seeded_data() {
        let conn = mem();
        seed_trajectories(&conn, 3);
        seed_chains(&conn, 2);
        seed_patterns(&conn, 4);

        let summary = query_summary(&conn).expect("summary must succeed");
        assert!(summary.contains("Sessions: 3"));
        assert!(summary.contains("Patterns: 4"));
    }

    #[test]
    fn all_presets_ok_on_empty_db() {
        let conn = mem();
        query_preset(&conn, "trajectory").expect("empty trajectory must not error");
        query_preset(&conn, "chains").expect("empty chains must not error");
        query_preset(&conn, "workstreams").expect("empty workstreams must not error");
        query_preset(&conn, "patterns").expect("empty patterns must not error");
        query_preset(&conn, "summary").expect("empty summary must not error");
    }
}
