//! `m08_trajectory` — CRUD for the `session_trajectory` table.
//!
//! Provides [`insert_point`], [`get_recent`], [`compute_delta`], [`get_trend`],
//! [`get_by_session`], and [`count`] — the practitioner's universal table for
//! tracking multi-session fitness arcs.
//!
//! Layer: `m2_schema`
//! Dependencies: `m01_types`, `m02_errors`, `m06_schema`

use rusqlite::{Connection, OptionalExtension as _};
use serde::{Deserialize, Serialize};

use crate::m1_foundation::m02_errors::SchemaError;

use super::sqlite_err;

// ---------------------------------------------------------------------------
// TrajectoryRow
// ---------------------------------------------------------------------------

/// A single row from the `session_trajectory` table.
///
/// All fields correspond directly to table columns. `key_achievement` is
/// nullable; `consent` defaults to `"Emit"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryRow {
    /// Primary key — the habitat session number.
    pub session_id: u32,
    /// RALPH fitness score at session close.
    pub ralph_fitness: f64,
    /// Kuramoto field coherence `r` at session close.
    pub field_r: f64,
    /// SYNTHEX thermal temperature at session close.
    pub thermal_t: f64,
    /// LTP/LTD ratio at session close.
    pub ltp_ltd_ratio: f64,
    /// Number of healthy services at session close.
    pub services_healthy: u32,
    /// Short prose summary of the fitness delta this session.
    pub delta_summary: String,
    /// Optional headline achievement for the session.
    pub key_achievement: Option<String>,
    /// Consent level — `"Emit"`, `"Store"`, or `"Forget"`.
    pub consent: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Insert a trajectory data point for `session_id`.
///
/// Fails with [`SchemaError::Sqlite`] if `session_id` already exists
/// (primary-key constraint) or if `consent` is not one of the accepted values.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on any `rusqlite` error.
// The function signature is dictated by the table schema and the public API
// contract specified in the module doc. A builder struct would add complexity
// without reducing the call sites; the allow is intentional.
#[allow(clippy::too_many_arguments)]
pub fn insert_point(
    conn: &Connection,
    session_id: u32,
    ralph_fitness: f64,
    field_r: f64,
    thermal_t: f64,
    ltp_ltd_ratio: f64,
    services_healthy: u32,
    delta_summary: &str,
    key_achievement: Option<&str>,
) -> Result<(), SchemaError> {
    conn.execute(
        "INSERT INTO session_trajectory
             (session_id, ralph_fitness, field_r, thermal_t, ltp_ltd_ratio,
              services_healthy, delta_summary, key_achievement)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            session_id,
            ralph_fitness,
            field_r,
            thermal_t,
            ltp_ltd_ratio,
            services_healthy,
            delta_summary,
            key_achievement,
        ],
    )
    .map_err(|e| sqlite_err(&e))?;
    Ok(())
}

/// Return the `n` most-recent trajectory rows, ordered by `session_id DESC`.
///
/// Returns an empty `Vec` when the table is empty.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on any `rusqlite` error.
pub fn get_recent(conn: &Connection, n: usize) -> Result<Vec<TrajectoryRow>, SchemaError> {
    let n_i64 = i64::try_from(n).unwrap_or(i64::MAX);
    let mut stmt = conn
        .prepare(
            "SELECT session_id, ralph_fitness, field_r, thermal_t, ltp_ltd_ratio,
                    services_healthy, delta_summary, key_achievement, consent
             FROM session_trajectory
             ORDER BY session_id DESC
             LIMIT ?1",
        )
        .map_err(|e| sqlite_err(&e))?;

    let rows = stmt
        .query_map(rusqlite::params![n_i64], map_row)
        .map_err(|e| sqlite_err(&e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| sqlite_err(&e))?;

    Ok(rows)
}

/// Compute the fitness delta between `session_id` and the immediately
/// preceding session.
///
/// Returns `None` when there is no previous session row in the table.
/// Returns `Some(current_fitness - previous_fitness)`.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] if the target `session_id` does not exist or
/// on any `rusqlite` error.
pub fn compute_delta(conn: &Connection, session_id: u32) -> Result<Option<f64>, SchemaError> {
    // Fetch the fitness of the target session. Return Ok(None) when the session
    // does not exist — consistent with the documented contract.
    let current: Option<f64> = conn
        .query_row(
            "SELECT ralph_fitness FROM session_trajectory WHERE session_id = ?1",
            rusqlite::params![session_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| sqlite_err(&e))?;

    let Some(current) = current else {
        return Ok(None);
    };

    // Fetch the fitness of the previous session (the one with the highest
    // session_id that is strictly less than session_id).
    let prev: Option<f64> = conn
        .query_row(
            "SELECT ralph_fitness FROM session_trajectory
             WHERE session_id < ?1
             ORDER BY session_id DESC
             LIMIT 1",
            rusqlite::params![session_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| sqlite_err(&e))?;

    Ok(prev.map(|p| current - p))
}

/// Compute the linear-regression slope of `ralph_fitness` across the last `n`
/// sessions (ordered by `session_id DESC`).
///
/// Returns `None` when fewer than 2 data points are available.
///
/// The slope is computed via ordinary least squares with `session_id` as the
/// x-axis and `ralph_fitness` as the y-axis. A positive value means fitness
/// is increasing over time.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on any `rusqlite` error.
pub fn get_trend(conn: &Connection, n: usize) -> Result<Option<f64>, SchemaError> {
    let rows = get_recent(conn, n)?;
    Ok(compute_ols_slope(&rows))
}

/// Fetch a single [`TrajectoryRow`] by `session_id`, returning `None` when not
/// found.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on any `rusqlite` error.
pub fn get_by_session(
    conn: &Connection,
    session_id: u32,
) -> Result<Option<TrajectoryRow>, SchemaError> {
    conn.query_row(
        "SELECT session_id, ralph_fitness, field_r, thermal_t, ltp_ltd_ratio,
                services_healthy, delta_summary, key_achievement, consent
         FROM session_trajectory
         WHERE session_id = ?1",
        rusqlite::params![session_id],
        map_row,
    )
    .optional()
    .map_err(|e| sqlite_err(&e))
}

/// Return the total number of rows in `session_trajectory`.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on any `rusqlite` error.
pub fn count(conn: &Connection) -> Result<u64, SchemaError> {
    conn.query_row(
        "SELECT COUNT(*) FROM session_trajectory",
        [],
        |r| r.get::<_, i64>(0),
    )
    .map(i64::cast_unsigned)
    .map_err(|e| sqlite_err(&e))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Map a `rusqlite` row to [`TrajectoryRow`].
fn map_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<TrajectoryRow> {
    Ok(TrajectoryRow {
        session_id: r.get::<_, u32>(0)?,
        ralph_fitness: r.get::<_, f64>(1)?,
        field_r: r.get::<_, f64>(2)?,
        thermal_t: r.get::<_, f64>(3)?,
        ltp_ltd_ratio: r.get::<_, f64>(4)?,
        services_healthy: r.get::<_, u32>(5)?,
        delta_summary: r.get::<_, String>(6)?,
        key_achievement: r.get::<_, Option<String>>(7)?,
        consent: r.get::<_, String>(8)?,
    })
}

/// Compute ordinary-least-squares slope of `ralph_fitness` vs `session_id`
/// for the provided rows.
///
/// Returns `None` if fewer than 2 rows are supplied.
fn compute_ols_slope(rows: &[TrajectoryRow]) -> Option<f64> {
    if rows.len() < 2 {
        return None;
    }

    // Safe: trajectory tables realistically hold O(100s) of rows — well below
    // the 2^52 mantissa limit of f64.
    #[allow(clippy::cast_precision_loss)]
    let n = rows.len() as f64;
    let sum_x: f64 = rows.iter().map(|r| f64::from(r.session_id)).sum();
    let sum_y: f64 = rows.iter().map(|r| r.ralph_fitness).sum();
    // Cross-sum Σ(x·y); named distinctly to avoid clippy::similar_names.
    let cross_sum: f64 = rows
        .iter()
        .map(|r| f64::from(r.session_id) * r.ralph_fitness)
        .sum();
    let sum_x2: f64 = rows.iter().map(|r| f64::from(r.session_id).powi(2)).sum();

    let denom = n * sum_x2 - sum_x * sum_x;
    if denom.abs() < f64::EPSILON {
        // All x-values identical — slope is undefined; return 0.
        return Some(0.0);
    }

    let slope = (n * cross_sum - sum_x * sum_y) / denom;
    Some(slope)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::m2_schema::m06_schema::open_memory;

    // -- helpers --

    fn mem_db() -> Connection {
        open_memory().unwrap()
    }

    /// Insert a row with default field values; only `session_id` and
    /// `ralph_fitness` vary.
    fn insert_simple(conn: &Connection, session_id: u32, ralph_fitness: f64) {
        insert_point(conn, session_id, ralph_fitness, 0.5, 0.5, 2.0, 11, "ok", None).unwrap();
    }

    // -----------------------------------------------------------------------
    // insert_point
    // -----------------------------------------------------------------------

    #[test]
    fn insert_single_row_succeeds() {
        let conn = mem_db();
        assert!(
            insert_point(&conn, 109, 0.664, 0.876, 0.515, 4.2, 11, "stable", None).is_ok()
        );
    }

    #[test]
    fn insert_sets_correct_fitness() {
        let conn = mem_db();
        insert_simple(&conn, 109, 0.75);
        let row = get_by_session(&conn, 109).unwrap().unwrap();
        assert!((row.ralph_fitness - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn insert_sets_correct_field_r() {
        let conn = mem_db();
        insert_point(&conn, 1, 0.5, 0.888, 0.5, 1.0, 11, "x", None).unwrap();
        let row = get_by_session(&conn, 1).unwrap().unwrap();
        assert!((row.field_r - 0.888).abs() < f64::EPSILON);
    }

    #[test]
    fn insert_sets_correct_thermal_t() {
        let conn = mem_db();
        insert_point(&conn, 1, 0.5, 0.0, 0.333, 1.0, 11, "x", None).unwrap();
        let row = get_by_session(&conn, 1).unwrap().unwrap();
        assert!((row.thermal_t - 0.333).abs() < f64::EPSILON);
    }

    #[test]
    fn insert_sets_correct_ltp_ltd() {
        let conn = mem_db();
        insert_point(&conn, 1, 0.5, 0.0, 0.5, 9.88, 11, "x", None).unwrap();
        let row = get_by_session(&conn, 1).unwrap().unwrap();
        assert!((row.ltp_ltd_ratio - 9.88).abs() < 1e-10);
    }

    #[test]
    fn insert_sets_correct_services_healthy() {
        let conn = mem_db();
        insert_point(&conn, 1, 0.5, 0.0, 0.5, 1.0, 10, "x", None).unwrap();
        let row = get_by_session(&conn, 1).unwrap().unwrap();
        assert_eq!(row.services_healthy, 10);
    }

    #[test]
    fn insert_with_key_achievement_some() {
        let conn = mem_db();
        insert_point(&conn, 1, 0.5, 0.0, 0.5, 1.0, 11, "x", Some("L8 sealed")).unwrap();
        let row = get_by_session(&conn, 1).unwrap().unwrap();
        assert_eq!(row.key_achievement, Some("L8 sealed".to_string()));
    }

    #[test]
    fn insert_with_key_achievement_none() {
        let conn = mem_db();
        insert_point(&conn, 1, 0.5, 0.0, 0.5, 1.0, 11, "x", None).unwrap();
        let row = get_by_session(&conn, 1).unwrap().unwrap();
        assert!(row.key_achievement.is_none());
    }

    #[test]
    fn insert_duplicate_session_id_fails() {
        let conn = mem_db();
        insert_simple(&conn, 1, 0.5);
        assert!(insert_simple_result(&conn, 1, 0.6).is_err());
    }

    fn insert_simple_result(conn: &Connection, session_id: u32, ralph_fitness: f64) -> Result<(), SchemaError> {
        insert_point(conn, session_id, ralph_fitness, 0.5, 0.5, 2.0, 11, "ok", None)
    }

    #[test]
    fn insert_default_consent_is_emit() {
        let conn = mem_db();
        insert_simple(&conn, 1, 0.5);
        let row = get_by_session(&conn, 1).unwrap().unwrap();
        assert_eq!(row.consent, "Emit");
    }

    #[test]
    fn insert_multiple_rows_succeeds() {
        let conn = mem_db();
        for s in 100..=110 {
            insert_simple(&conn, s, 0.5 + f64::from(s - 100) * 0.01);
        }
        assert_eq!(count(&conn).unwrap(), 11);
    }

    #[test]
    fn insert_delta_summary_stored_correctly() {
        let conn = mem_db();
        insert_point(&conn, 1, 0.5, 0.0, 0.5, 1.0, 11, "fitness stable", None).unwrap();
        let row = get_by_session(&conn, 1).unwrap().unwrap();
        assert_eq!(row.delta_summary, "fitness stable");
    }

    // -----------------------------------------------------------------------
    // get_by_session
    // -----------------------------------------------------------------------

    #[test]
    fn get_by_session_returns_some() {
        let conn = mem_db();
        insert_simple(&conn, 109, 0.664);
        let result = get_by_session(&conn, 109).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn get_by_session_returns_none_when_missing() {
        let conn = mem_db();
        assert!(get_by_session(&conn, 999).unwrap().is_none());
    }

    #[test]
    fn get_by_session_correct_session_id() {
        let conn = mem_db();
        insert_simple(&conn, 42, 0.5);
        let row = get_by_session(&conn, 42).unwrap().unwrap();
        assert_eq!(row.session_id, 42);
    }

    #[test]
    fn get_by_session_does_not_return_neighbour() {
        let conn = mem_db();
        insert_simple(&conn, 1, 0.5);
        insert_simple(&conn, 2, 0.6);
        let row = get_by_session(&conn, 1).unwrap().unwrap();
        assert!((row.ralph_fitness - 0.5).abs() < f64::EPSILON);
    }

    // -----------------------------------------------------------------------
    // get_recent
    // -----------------------------------------------------------------------

    #[test]
    fn get_recent_empty_table_returns_empty_vec() {
        let conn = mem_db();
        assert_eq!(get_recent(&conn, 5).unwrap().len(), 0);
    }

    #[test]
    fn get_recent_returns_correct_count() {
        let conn = mem_db();
        for s in 100..=109 {
            insert_simple(&conn, s, 0.5);
        }
        assert_eq!(get_recent(&conn, 5).unwrap().len(), 5);
    }

    #[test]
    fn get_recent_ordered_desc() {
        let conn = mem_db();
        for s in [105_u32, 108, 106, 107] {
            insert_simple(&conn, s, 0.5);
        }
        let rows = get_recent(&conn, 4).unwrap();
        let ids: Vec<u32> = rows.iter().map(|r| r.session_id).collect();
        assert_eq!(ids, vec![108, 107, 106, 105]);
    }

    #[test]
    fn get_recent_with_n_larger_than_table_returns_all() {
        let conn = mem_db();
        insert_simple(&conn, 1, 0.5);
        insert_simple(&conn, 2, 0.6);
        let rows = get_recent(&conn, 100).unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn get_recent_n_zero_returns_empty() {
        let conn = mem_db();
        insert_simple(&conn, 1, 0.5);
        let rows = get_recent(&conn, 0).unwrap();
        assert_eq!(rows.len(), 0);
    }

    #[test]
    fn get_recent_first_row_is_latest() {
        let conn = mem_db();
        for s in 100_u32..=109 {
            insert_simple(&conn, s, f64::from(s) * 0.01);
        }
        let rows = get_recent(&conn, 3).unwrap();
        assert_eq!(rows[0].session_id, 109);
    }

    #[test]
    fn get_recent_all_fields_populated() {
        let conn = mem_db();
        insert_point(&conn, 50, 0.77, 0.88, 0.55, 3.14, 10, "delta ok", Some("achievement")).unwrap();
        let rows = get_recent(&conn, 1).unwrap();
        let r = &rows[0];
        assert_eq!(r.session_id, 50);
        assert!((r.ralph_fitness - 0.77).abs() < f64::EPSILON);
        assert!((r.field_r - 0.88).abs() < f64::EPSILON);
        assert!((r.thermal_t - 0.55).abs() < f64::EPSILON);
        assert!((r.ltp_ltd_ratio - 3.14).abs() < 1e-10);
        assert_eq!(r.services_healthy, 10);
        assert_eq!(r.delta_summary, "delta ok");
        assert_eq!(r.key_achievement.as_deref(), Some("achievement"));
    }

    // -----------------------------------------------------------------------
    // compute_delta
    // -----------------------------------------------------------------------

    #[test]
    fn delta_none_when_no_previous() {
        let conn = mem_db();
        insert_simple(&conn, 1, 0.5);
        assert!(compute_delta(&conn, 1).unwrap().is_none());
    }

    #[test]
    fn delta_positive_when_fitness_increased() {
        let conn = mem_db();
        insert_simple(&conn, 1, 0.5);
        insert_simple(&conn, 2, 0.7);
        let d = compute_delta(&conn, 2).unwrap().unwrap();
        assert!((d - 0.2).abs() < 1e-10);
    }

    #[test]
    fn delta_negative_when_fitness_decreased() {
        let conn = mem_db();
        insert_simple(&conn, 1, 0.8);
        insert_simple(&conn, 2, 0.6);
        let d = compute_delta(&conn, 2).unwrap().unwrap();
        assert!((d - (-0.2)).abs() < 1e-10);
    }

    #[test]
    fn delta_zero_when_fitness_unchanged() {
        let conn = mem_db();
        insert_simple(&conn, 1, 0.664);
        insert_simple(&conn, 2, 0.664);
        let d = compute_delta(&conn, 2).unwrap().unwrap();
        assert!(d.abs() < f64::EPSILON);
    }

    #[test]
    fn delta_uses_immediately_preceding_session() {
        let conn = mem_db();
        insert_simple(&conn, 100, 0.3);
        insert_simple(&conn, 105, 0.5);
        insert_simple(&conn, 109, 0.8);
        // Delta for 109 should be vs 105 (0.3 gap in ids but nearest lesser)
        let d = compute_delta(&conn, 109).unwrap().unwrap();
        assert!((d - 0.3).abs() < 1e-10);
    }

    #[test]
    fn delta_none_when_session_not_found() {
        let conn = mem_db();
        // Session 999 doesn't exist — should return Ok(None) per the documented contract
        let result = compute_delta(&conn, 999);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn delta_with_many_sessions_uses_direct_predecessor() {
        let conn = mem_db();
        for s in 1_u32..=5 {
            insert_simple(&conn, s, f64::from(s) * 0.1);
        }
        // Session 5 has fitness 0.5; session 4 has fitness 0.4
        let d = compute_delta(&conn, 5).unwrap().unwrap();
        assert!((d - 0.1).abs() < 1e-10);
    }

    // -----------------------------------------------------------------------
    // get_trend
    // -----------------------------------------------------------------------

    #[test]
    fn trend_none_when_table_empty() {
        let conn = mem_db();
        assert!(get_trend(&conn, 5).unwrap().is_none());
    }

    #[test]
    fn trend_none_when_only_one_row() {
        let conn = mem_db();
        insert_simple(&conn, 1, 0.5);
        assert!(get_trend(&conn, 5).unwrap().is_none());
    }

    #[test]
    fn trend_positive_for_upward_fitness() {
        let conn = mem_db();
        // Fitness strictly increasing with session_id
        for s in 1_u32..=5 {
            insert_simple(&conn, s, f64::from(s) * 0.1);
        }
        let slope = get_trend(&conn, 5).unwrap().unwrap();
        assert!(slope > 0.0, "expected positive slope, got {slope}");
    }

    #[test]
    fn trend_negative_for_downward_fitness() {
        let conn = mem_db();
        // Fitness strictly decreasing with session_id
        for s in 1_u32..=5 {
            insert_simple(&conn, s, 1.0 - f64::from(s) * 0.1);
        }
        let slope = get_trend(&conn, 5).unwrap().unwrap();
        assert!(slope < 0.0, "expected negative slope, got {slope}");
    }

    #[test]
    fn trend_near_zero_for_flat_fitness() {
        let conn = mem_db();
        for s in 1_u32..=5 {
            insert_simple(&conn, s, 0.664);
        }
        let slope = get_trend(&conn, 5).unwrap().unwrap();
        assert!(slope.abs() < 1e-10, "expected ~0 slope, got {slope}");
    }

    #[test]
    fn trend_with_two_points_exact_slope() {
        let conn = mem_db();
        // session 1 → 0.4, session 2 → 0.6 → slope = 0.2 per session
        insert_simple(&conn, 1, 0.4);
        insert_simple(&conn, 2, 0.6);
        let slope = get_trend(&conn, 10).unwrap().unwrap();
        assert!((slope - 0.2).abs() < 1e-10, "expected 0.2, got {slope}");
    }

    #[test]
    fn trend_n_larger_than_table_uses_all_rows() {
        let conn = mem_db();
        for s in 1_u32..=3 {
            insert_simple(&conn, s, f64::from(s) * 0.1);
        }
        // n=100 but table has 3 rows — should still compute
        let slope = get_trend(&conn, 100).unwrap();
        assert!(slope.is_some());
        assert!(slope.unwrap() > 0.0);
    }

    #[test]
    fn trend_n_limits_to_recent_sessions() {
        let conn = mem_db();
        // Early sessions: flat at 0.2
        for s in 1_u32..=3 {
            insert_simple(&conn, s, 0.2);
        }
        // Recent sessions: strongly increasing
        for s in 100_u32..=105 {
            insert_simple(&conn, s, f64::from(s - 99) * 0.1);
        }
        // With n=3, only recent sessions factor in → positive slope
        let slope_recent = get_trend(&conn, 3).unwrap().unwrap();
        // With n=100, earlier flat sessions are included
        let slope_all = get_trend(&conn, 100).unwrap().unwrap();
        // Both positive, but recent-only slope should be distinctly non-zero
        assert!(slope_recent > 0.0);
        assert!(slope_all > 0.0);
    }

    #[test]
    fn trend_with_noncontiguous_sessions() {
        let conn = mem_db();
        // Gaps in session IDs are fine; OLS uses actual values
        insert_simple(&conn, 100, 0.5);
        insert_simple(&conn, 200, 0.7);
        let slope = get_trend(&conn, 5).unwrap().unwrap();
        // slope = (0.7 - 0.5) / (200 - 100) = 0.002
        assert!((slope - 0.002).abs() < 1e-10, "expected 0.002, got {slope}");
    }

    // -----------------------------------------------------------------------
    // count
    // -----------------------------------------------------------------------

    #[test]
    fn count_zero_when_empty() {
        let conn = mem_db();
        assert_eq!(count(&conn).unwrap(), 0);
    }

    #[test]
    fn count_increments_on_insert() {
        let conn = mem_db();
        for s in 1_u32..=7 {
            insert_simple(&conn, s, 0.5);
            assert_eq!(count(&conn).unwrap(), u64::from(s));
        }
    }

    #[test]
    fn count_ten_rows() {
        let conn = mem_db();
        for s in 1_u32..=10 {
            insert_simple(&conn, s, 0.5);
        }
        assert_eq!(count(&conn).unwrap(), 10);
    }

    // -----------------------------------------------------------------------
    // error handling
    // -----------------------------------------------------------------------

    #[test]
    fn get_by_session_on_empty_db_returns_none() {
        let conn = mem_db();
        let result = get_by_session(&conn, 1);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn get_recent_on_empty_db_returns_ok_empty() {
        let conn = mem_db();
        let result = get_recent(&conn, 10);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn compute_delta_none_on_nonexistent_session() {
        let conn = mem_db();
        // Inserting session 1 but querying delta for 2 (which doesn't exist)
        insert_simple(&conn, 1, 0.5);
        let result = compute_delta(&conn, 2);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    // -----------------------------------------------------------------------
    // consent field
    // -----------------------------------------------------------------------

    #[test]
    fn row_consent_defaults_to_emit() {
        let conn = mem_db();
        insert_simple(&conn, 1, 0.5);
        let row = get_by_session(&conn, 1).unwrap().unwrap();
        assert_eq!(row.consent, "Emit");
    }

    #[test]
    fn raw_insert_with_store_consent_round_trips() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO session_trajectory
                 (session_id, ralph_fitness, field_r, thermal_t, ltp_ltd_ratio,
                  services_healthy, delta_summary, consent)
             VALUES (1, 0.5, 0.0, 0.5, 1.0, 11, 'ok', 'Store')",
            [],
        )
        .unwrap();
        let row = get_by_session(&conn, 1).unwrap().unwrap();
        assert_eq!(row.consent, "Store");
    }

    #[test]
    fn raw_insert_with_forget_consent_round_trips() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO session_trajectory
                 (session_id, ralph_fitness, field_r, thermal_t, ltp_ltd_ratio,
                  services_healthy, delta_summary, consent)
             VALUES (1, 0.5, 0.0, 0.5, 1.0, 11, 'ok', 'Forget')",
            [],
        )
        .unwrap();
        let row = get_by_session(&conn, 1).unwrap().unwrap();
        assert_eq!(row.consent, "Forget");
    }

    #[test]
    fn raw_insert_invalid_consent_fails() {
        let conn = mem_db();
        let result = conn.execute(
            "INSERT INTO session_trajectory
                 (session_id, ralph_fitness, field_r, thermal_t, ltp_ltd_ratio,
                  services_healthy, delta_summary, consent)
             VALUES (1, 0.5, 0.0, 0.5, 1.0, 11, 'ok', 'INVALID')",
            [],
        );
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // edge-case: identical x-values for OLS (degenerate regression)
    // -----------------------------------------------------------------------

    #[test]
    fn trend_identical_session_ids_returns_zero_slope() {
        // This is only possible by bypassing the PK — use two rows from a
        // manually constructed Vec to test the internal OLS helper directly.
        let rows = vec![
            TrajectoryRow {
                session_id: 5,
                ralph_fitness: 0.4,
                field_r: 0.0,
                thermal_t: 0.0,
                ltp_ltd_ratio: 1.0,
                services_healthy: 11,
                delta_summary: "a".into(),
                key_achievement: None,
                consent: "Emit".into(),
            },
            TrajectoryRow {
                session_id: 5, // same x → degenerate
                ralph_fitness: 0.8,
                field_r: 0.0,
                thermal_t: 0.0,
                ltp_ltd_ratio: 1.0,
                services_healthy: 11,
                delta_summary: "b".into(),
                key_achievement: None,
                consent: "Emit".into(),
            },
        ];
        let slope = compute_ols_slope(&rows).unwrap();
        assert!(slope.abs() < f64::EPSILON);
    }

    // -----------------------------------------------------------------------
    // serde round-trip
    // -----------------------------------------------------------------------

    #[test]
    fn trajectory_row_serde_round_trip() {
        let row = TrajectoryRow {
            session_id: 109,
            ralph_fitness: 0.664,
            field_r: 0.876,
            thermal_t: 0.515,
            ltp_ltd_ratio: 4.2,
            services_healthy: 11,
            delta_summary: "fitness stable".into(),
            key_achievement: Some("L8 sealed".into()),
            consent: "Emit".into(),
        };
        let json = serde_json::to_string(&row).unwrap();
        let back: TrajectoryRow = serde_json::from_str(&json).unwrap();
        assert_eq!(back.session_id, row.session_id);
        assert!((back.ralph_fitness - row.ralph_fitness).abs() < f64::EPSILON);
        assert_eq!(back.key_achievement, row.key_achievement);
        assert_eq!(back.consent, row.consent);
    }

    #[test]
    fn trajectory_row_serde_none_key_achievement() {
        let row = TrajectoryRow {
            session_id: 1,
            ralph_fitness: 0.5,
            field_r: 0.0,
            thermal_t: 0.0,
            ltp_ltd_ratio: 1.0,
            services_healthy: 11,
            delta_summary: "x".into(),
            key_achievement: None,
            consent: "Emit".into(),
        };
        let json = serde_json::to_string(&row).unwrap();
        let back: TrajectoryRow = serde_json::from_str(&json).unwrap();
        assert!(back.key_achievement.is_none());
    }

    // -----------------------------------------------------------------------
    // clone / debug derives
    // -----------------------------------------------------------------------

    #[test]
    fn trajectory_row_clone() {
        let row = TrajectoryRow {
            session_id: 1,
            ralph_fitness: 0.5,
            field_r: 0.0,
            thermal_t: 0.0,
            ltp_ltd_ratio: 1.0,
            services_healthy: 11,
            delta_summary: "x".into(),
            key_achievement: None,
            consent: "Emit".into(),
        };
        let cloned = row.clone();
        assert_eq!(cloned.session_id, row.session_id);
    }

    #[test]
    fn trajectory_row_debug_not_empty() {
        let row = TrajectoryRow {
            session_id: 1,
            ralph_fitness: 0.5,
            field_r: 0.0,
            thermal_t: 0.0,
            ltp_ltd_ratio: 1.0,
            services_healthy: 11,
            delta_summary: "x".into(),
            key_achievement: None,
            consent: "Emit".into(),
        };
        assert!(!format!("{row:?}").is_empty());
    }
}
