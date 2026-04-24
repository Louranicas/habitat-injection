//! `m15b_trajectory_capture` — Post-session: accepts pre-fetched ORAC health data,
//! computes `delta_summary` vs the previous session, and inserts a point into
//! `session_trajectory`.
//!
//! The library layer does **not** perform HTTP calls — callers (binaries / CLI) are
//! responsible for fetching live service data and passing it in as a
//! [`HealthSnapshot`].  This module focuses on the delta computation and DB insert.
//!
//! Layer: `m4_consolidation`
//! Dependencies: `m01_types`, `m02_errors`, `m08_trajectory`

#[cfg(feature = "sqlite")]
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
#[cfg(feature = "sqlite")]
use tracing::{debug, info, instrument};

#[cfg(feature = "sqlite")]
use crate::m1_foundation::m02_errors::ConsolidationError;
use crate::m2_schema::m08_trajectory::TrajectoryRow;
#[cfg(feature = "sqlite")]
use crate::m2_schema::m08_trajectory::{get_by_session, get_recent, insert_point};

// ---------------------------------------------------------------------------
// Epsilon for flat-fitness detection
// ---------------------------------------------------------------------------

/// Minimum absolute fitness delta required to call a change "UP" or "DOWN".
///
/// Changes smaller than this value in absolute terms are classified as `"FLAT"`.
const FITNESS_FLAT_EPSILON: f64 = 0.005;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Health snapshot captured from ORAC and other services.
///
/// This struct is intentionally decoupled from the HTTP layer — it is
/// constructed by the binary after querying service endpoints and passed
/// into [`capture_trajectory`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSnapshot {
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
    /// Optional headline achievement for the session.
    pub key_achievement: Option<String>,
}

/// Result of a trajectory capture.
///
/// Returned by [`capture_trajectory`] regardless of whether an insert occurred.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureResult {
    /// Session identifier that was targeted.
    pub session_id: u32,
    /// One-line English summary of the fitness delta this session.
    pub delta_summary: String,
    /// Signed fitness delta vs the immediately preceding session.
    ///
    /// `None` when this is the first trajectory point in the database.
    pub fitness_delta: Option<f64>,
    /// Whether a new row was actually inserted.
    ///
    /// `false` when the session already had a trajectory point (idempotent).
    pub inserted: bool,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Capture a trajectory point for `session_id`.
///
/// Steps performed:
/// 1. Check whether `session_id` already has a trajectory row — if so, return
///    a [`CaptureResult`] with `inserted: false` without touching the DB.
/// 2. Fetch the most-recent preceding row (if any) to compute the delta.
/// 3. Build the `delta_summary` string via [`compute_delta_summary`].
/// 4. Insert the new trajectory point.
/// 5. Return a [`CaptureResult`] describing what happened.
///
/// # Errors
///
/// Returns [`ConsolidationError::TrajectoryCaptureFailed`] when any schema
/// operation fails.  The `reason` field includes the underlying [`SchemaError`]
/// message.
#[cfg(feature = "sqlite")]
#[instrument(skip(conn, snapshot), fields(session_id))]
pub fn capture_trajectory(
    conn: &Connection,
    session_id: u32,
    snapshot: &HealthSnapshot,
) -> Result<CaptureResult, ConsolidationError> {
    // ---- 1. Idempotency guard -----------------------------------------------
    let existing = get_by_session(conn, session_id).map_err(|e| {
        ConsolidationError::TrajectoryCaptureFailed {
            session: session_id.into(),
            reason: e.to_string(),
        }
    })?;

    if existing.is_some() {
        debug!(session_id, "trajectory point already exists — skipping insert");
        // Re-compute summary from the stored row so the result is consistent.
        let previous = previous_row(conn, session_id)?;
        let delta_summary = compute_delta_summary(snapshot, previous.as_ref());
        let fitness_delta = previous.map(|p| snapshot.ralph_fitness - p.ralph_fitness);

        return Ok(CaptureResult {
            session_id,
            delta_summary,
            fitness_delta,
            inserted: false,
        });
    }

    // ---- 2. Fetch previous row for delta computation ------------------------
    let previous = previous_row(conn, session_id)?;
    let fitness_delta = previous
        .as_ref()
        .map(|p| snapshot.ralph_fitness - p.ralph_fitness);

    // ---- 3. Build delta summary ---------------------------------------------
    let delta_summary = compute_delta_summary(snapshot, previous.as_ref());

    // ---- 4. Insert ----------------------------------------------------------
    insert_point(
        conn,
        session_id,
        snapshot.ralph_fitness,
        snapshot.field_r,
        snapshot.thermal_t,
        snapshot.ltp_ltd_ratio,
        snapshot.services_healthy,
        &delta_summary,
        snapshot.key_achievement.as_deref(),
    )
    .map_err(|e| ConsolidationError::TrajectoryCaptureFailed {
        session: session_id.into(),
        reason: e.to_string(),
    })?;

    info!(
        session_id,
        delta_summary = %delta_summary,
        fitness_delta,
        "trajectory point inserted"
    );

    Ok(CaptureResult {
        session_id,
        delta_summary,
        fitness_delta,
        inserted: true,
    })
}

/// Build the one-line delta summary for a trajectory point.
///
/// Format (with previous row):
/// ```text
/// fitness +0.005 after L8 sealed
/// fitness -0.003 after session
/// fitness stable after session
/// ```
///
/// When no previous row exists:
/// ```text
/// first trajectory point
/// ```
///
/// The signed delta uses a `+` prefix for positive values and an explicit `-`
/// for negative values.  Values within [`FITNESS_FLAT_EPSILON`] are rendered as
/// `"stable"` rather than a numeric delta.
#[must_use]
pub fn compute_delta_summary(
    current: &HealthSnapshot,
    previous: Option<&TrajectoryRow>,
) -> String {
    let Some(prev) = previous else {
        return "first trajectory point".to_string();
    };

    let delta = current.ralph_fitness - prev.ralph_fitness;
    let trend = compute_fitness_trend(current.ralph_fitness, Some(prev.ralph_fitness));

    let delta_str = if trend == "FLAT" {
        "stable".to_string()
    } else {
        // Format to 3 decimal places with explicit sign.
        format!("{delta:+.3}")
    };

    let label = current
        .key_achievement
        .as_deref()
        .unwrap_or("session");

    format!("fitness {delta_str} after {label}")
}

/// Classify a fitness change as `"UP"`, `"DOWN"`, or `"FLAT"`.
///
/// The classification uses an epsilon of [`FITNESS_FLAT_EPSILON`] (0.005):
/// - `|current - previous| < 0.005` → `"FLAT"`
/// - `current - previous >= 0.005`  → `"UP"`
/// - `current - previous <= -0.005` → `"DOWN"`
///
/// When `previous` is `None` there is no change to classify — returns `"FLAT"`.
#[must_use]
pub fn compute_fitness_trend(current: f64, previous: Option<f64>) -> &'static str {
    let Some(prev) = previous else {
        return "FLAT";
    };

    let delta = current - prev;

    if delta >= FITNESS_FLAT_EPSILON {
        "UP"
    } else if delta <= -FITNESS_FLAT_EPSILON {
        "DOWN"
    } else {
        "FLAT"
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Return the most-recent [`TrajectoryRow`] with `session_id` strictly less
/// than the given `session_id`.
///
/// Uses [`get_recent`] with `n = 1` after the caller has confirmed the target
/// session does not yet exist, so the most-recent row is guaranteed to be the
/// predecessor.  When `session_id` is already in the table (idempotency path)
/// we need the row immediately before it — handled by a targeted query.
#[cfg(feature = "sqlite")]
fn previous_row(
    conn: &Connection,
    session_id: u32,
) -> Result<Option<TrajectoryRow>, ConsolidationError> {
    // We want the row with the largest session_id that is < session_id.
    // get_recent returns rows ordered DESC; we query the full recent list and
    // pick the first one that is strictly less than session_id.
    //
    // Using limit=2 is sufficient when the target session is already present
    // (idempotency path); we scan up to a small fixed window to find the
    // predecessor.  For the common case (new session), get_recent(1) returns
    // the only preceding row.
    let rows = get_recent(conn, 2).map_err(|e| {
        ConsolidationError::TrajectoryCaptureFailed {
            session: session_id.into(),
            reason: e.to_string(),
        }
    })?;

    // The rows are ordered DESC by session_id.  Pick the first whose
    // session_id is strictly less than the target.
    Ok(rows.into_iter().find(|r| r.session_id < session_id))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use crate::m2_schema::m06_schema::open_memory;
    use crate::m2_schema::m08_trajectory::{insert_point, TrajectoryRow};

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn mem_db() -> Connection {
        open_memory().unwrap()
    }

    fn snapshot(fitness: f64) -> HealthSnapshot {
        HealthSnapshot {
            ralph_fitness: fitness,
            field_r: 0.876,
            thermal_t: 0.515,
            ltp_ltd_ratio: 4.2,
            services_healthy: 11,
            key_achievement: None,
        }
    }

    fn snapshot_with_achievement(fitness: f64, achievement: &str) -> HealthSnapshot {
        HealthSnapshot {
            ralph_fitness: fitness,
            field_r: 0.876,
            thermal_t: 0.515,
            ltp_ltd_ratio: 4.2,
            services_healthy: 11,
            key_achievement: Some(achievement.to_string()),
        }
    }

    fn make_trajectory_row(session_id: u32, fitness: f64) -> TrajectoryRow {
        TrajectoryRow {
            session_id,
            ralph_fitness: fitness,
            field_r: 0.5,
            thermal_t: 0.5,
            ltp_ltd_ratio: 2.0,
            services_healthy: 11,
            delta_summary: "ok".to_string(),
            key_achievement: None,
            consent: "Emit".to_string(),
        }
    }

    fn seed_previous(conn: &Connection, session_id: u32, fitness: f64) {
        insert_point(conn, session_id, fitness, 0.5, 0.5, 2.0, 11, "seed", None).unwrap();
    }

    // -----------------------------------------------------------------------
    // capture_trajectory — first point (no previous)
    // -----------------------------------------------------------------------

    #[test]
    fn capture_first_point_inserted_true() {
        let conn = mem_db();
        let snap = snapshot(0.664);
        let result = capture_trajectory(&conn, 109, &snap).unwrap();
        assert!(result.inserted);
    }

    #[test]
    fn capture_first_point_session_id_matches() {
        let conn = mem_db();
        let result = capture_trajectory(&conn, 109, &snapshot(0.664)).unwrap();
        assert_eq!(result.session_id, 109);
    }

    #[test]
    fn capture_first_point_fitness_delta_is_none() {
        let conn = mem_db();
        let result = capture_trajectory(&conn, 109, &snapshot(0.664)).unwrap();
        assert!(result.fitness_delta.is_none());
    }

    #[test]
    fn capture_first_point_delta_summary_is_first_trajectory_point() {
        let conn = mem_db();
        let result = capture_trajectory(&conn, 109, &snapshot(0.664)).unwrap();
        assert_eq!(result.delta_summary, "first trajectory point");
    }

    #[test]
    fn capture_first_point_row_exists_in_db() {
        let conn = mem_db();
        capture_trajectory(&conn, 109, &snapshot(0.664)).unwrap();
        let row = get_by_session(&conn, 109).unwrap();
        assert!(row.is_some());
    }

    #[test]
    fn capture_first_point_fitness_stored_correctly() {
        let conn = mem_db();
        capture_trajectory(&conn, 109, &snapshot(0.664)).unwrap();
        let row = get_by_session(&conn, 109).unwrap().unwrap();
        assert!((row.ralph_fitness - 0.664).abs() < f64::EPSILON);
    }

    // -----------------------------------------------------------------------
    // capture_trajectory — with previous (positive delta)
    // -----------------------------------------------------------------------

    #[test]
    fn capture_positive_delta_inserted_true() {
        let conn = mem_db();
        seed_previous(&conn, 108, 0.660);
        let result = capture_trajectory(&conn, 109, &snapshot(0.670)).unwrap();
        assert!(result.inserted);
    }

    #[test]
    fn capture_positive_delta_correct_value() {
        let conn = mem_db();
        seed_previous(&conn, 108, 0.660);
        let result = capture_trajectory(&conn, 109, &snapshot(0.670)).unwrap();
        let delta = result.fitness_delta.unwrap();
        assert!((delta - 0.010).abs() < 1e-10);
    }

    #[test]
    fn capture_positive_delta_summary_contains_plus() {
        let conn = mem_db();
        seed_previous(&conn, 108, 0.660);
        let result = capture_trajectory(&conn, 109, &snapshot(0.670)).unwrap();
        assert!(result.delta_summary.contains('+'), "expected '+' in {:?}", result.delta_summary);
    }

    // -----------------------------------------------------------------------
    // capture_trajectory — negative delta
    // -----------------------------------------------------------------------

    #[test]
    fn capture_negative_delta_inserted_true() {
        let conn = mem_db();
        seed_previous(&conn, 108, 0.700);
        let result = capture_trajectory(&conn, 109, &snapshot(0.690)).unwrap();
        assert!(result.inserted);
    }

    #[test]
    fn capture_negative_delta_correct_value() {
        let conn = mem_db();
        seed_previous(&conn, 108, 0.700);
        let result = capture_trajectory(&conn, 109, &snapshot(0.690)).unwrap();
        let delta = result.fitness_delta.unwrap();
        assert!((delta - (-0.010)).abs() < 1e-10);
    }

    #[test]
    fn capture_negative_delta_summary_contains_minus() {
        let conn = mem_db();
        seed_previous(&conn, 108, 0.700);
        let result = capture_trajectory(&conn, 109, &snapshot(0.690)).unwrap();
        assert!(result.delta_summary.contains('-'), "expected '-' in {:?}", result.delta_summary);
    }

    // -----------------------------------------------------------------------
    // capture_trajectory — flat delta (within epsilon)
    // -----------------------------------------------------------------------

    #[test]
    fn capture_flat_delta_summary_contains_stable() {
        let conn = mem_db();
        seed_previous(&conn, 108, 0.664);
        // Delta of 0.003 < 0.005 epsilon → stable
        let result = capture_trajectory(&conn, 109, &snapshot(0.667)).unwrap();
        assert!(
            result.delta_summary.contains("stable"),
            "expected 'stable' in {:?}",
            result.delta_summary
        );
    }

    #[test]
    fn capture_flat_exact_zero_delta_stable() {
        let conn = mem_db();
        seed_previous(&conn, 108, 0.664);
        let result = capture_trajectory(&conn, 109, &snapshot(0.664)).unwrap();
        assert!(result.delta_summary.contains("stable"));
    }

    // -----------------------------------------------------------------------
    // capture_trajectory — duplicate session_id (idempotency)
    // -----------------------------------------------------------------------

    #[test]
    fn capture_duplicate_returns_inserted_false() {
        let conn = mem_db();
        capture_trajectory(&conn, 109, &snapshot(0.664)).unwrap();
        let result = capture_trajectory(&conn, 109, &snapshot(0.664)).unwrap();
        assert!(!result.inserted);
    }

    #[test]
    fn capture_duplicate_does_not_error() {
        let conn = mem_db();
        capture_trajectory(&conn, 109, &snapshot(0.664)).unwrap();
        assert!(capture_trajectory(&conn, 109, &snapshot(0.700)).is_ok());
    }

    #[test]
    fn capture_duplicate_row_count_stays_at_one() {
        let conn = mem_db();
        capture_trajectory(&conn, 109, &snapshot(0.664)).unwrap();
        capture_trajectory(&conn, 109, &snapshot(0.700)).unwrap();
        let count = crate::m2_schema::m08_trajectory::count(&conn).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn capture_duplicate_returns_correct_session_id() {
        let conn = mem_db();
        capture_trajectory(&conn, 109, &snapshot(0.664)).unwrap();
        let result = capture_trajectory(&conn, 109, &snapshot(0.664)).unwrap();
        assert_eq!(result.session_id, 109);
    }

    // -----------------------------------------------------------------------
    // capture_trajectory — key_achievement in summary
    // -----------------------------------------------------------------------

    #[test]
    fn capture_key_achievement_appears_in_summary() {
        let conn = mem_db();
        seed_previous(&conn, 108, 0.660);
        let snap = snapshot_with_achievement(0.670, "L8 sealed");
        let result = capture_trajectory(&conn, 109, &snap).unwrap();
        assert!(
            result.delta_summary.contains("L8 sealed"),
            "expected achievement in {:?}",
            result.delta_summary
        );
    }

    #[test]
    fn capture_no_key_achievement_uses_session_word() {
        let conn = mem_db();
        seed_previous(&conn, 108, 0.660);
        let result = capture_trajectory(&conn, 109, &snapshot(0.670)).unwrap();
        assert!(
            result.delta_summary.ends_with("session"),
            "expected 'session' suffix in {:?}",
            result.delta_summary
        );
    }

    #[test]
    fn capture_first_point_with_achievement_still_first_trajectory_point() {
        let conn = mem_db();
        let snap = snapshot_with_achievement(0.664, "genesis");
        let result = capture_trajectory(&conn, 109, &snap).unwrap();
        // No previous row → summary is always "first trajectory point"
        assert_eq!(result.delta_summary, "first trajectory point");
    }

    // -----------------------------------------------------------------------
    // compute_delta_summary
    // -----------------------------------------------------------------------

    #[test]
    fn delta_summary_no_previous_returns_first_trajectory_point() {
        let snap = snapshot(0.664);
        assert_eq!(compute_delta_summary(&snap, None), "first trajectory point");
    }

    #[test]
    fn delta_summary_positive_delta_format() {
        let prev = make_trajectory_row(108, 0.660);
        let snap = snapshot(0.670);
        let summary = compute_delta_summary(&snap, Some(&prev));
        // 0.670 - 0.660 = 0.010
        assert!(summary.contains("+0.010"), "got: {summary}");
    }

    #[test]
    fn delta_summary_negative_delta_format() {
        let prev = make_trajectory_row(108, 0.700);
        let snap = snapshot(0.690);
        let summary = compute_delta_summary(&snap, Some(&prev));
        assert!(summary.contains("-0.010"), "got: {summary}");
    }

    #[test]
    fn delta_summary_flat_contains_stable() {
        let prev = make_trajectory_row(108, 0.664);
        let snap = snapshot(0.664);
        let summary = compute_delta_summary(&snap, Some(&prev));
        assert!(summary.contains("stable"), "got: {summary}");
    }

    #[test]
    fn delta_summary_starts_with_fitness() {
        let prev = make_trajectory_row(108, 0.660);
        let snap = snapshot(0.670);
        let summary = compute_delta_summary(&snap, Some(&prev));
        assert!(summary.starts_with("fitness "), "got: {summary}");
    }

    #[test]
    fn delta_summary_contains_after() {
        let prev = make_trajectory_row(108, 0.660);
        let snap = snapshot(0.670);
        let summary = compute_delta_summary(&snap, Some(&prev));
        assert!(summary.contains(" after "), "got: {summary}");
    }

    #[test]
    fn delta_summary_uses_key_achievement_when_present() {
        let prev = make_trajectory_row(108, 0.660);
        let snap = snapshot_with_achievement(0.670, "daemon wired");
        let summary = compute_delta_summary(&snap, Some(&prev));
        assert!(summary.ends_with("daemon wired"), "got: {summary}");
    }

    #[test]
    fn delta_summary_uses_session_when_no_achievement() {
        let prev = make_trajectory_row(108, 0.660);
        let snap = snapshot(0.670);
        let summary = compute_delta_summary(&snap, Some(&prev));
        assert!(summary.ends_with("session"), "got: {summary}");
    }

    #[test]
    fn delta_summary_three_decimal_places() {
        let prev = make_trajectory_row(108, 0.500);
        let snap = snapshot(0.512);
        let summary = compute_delta_summary(&snap, Some(&prev));
        // 0.512 - 0.500 = 0.012 → "+0.012"
        assert!(summary.contains("+0.012"), "got: {summary}");
    }

    // -----------------------------------------------------------------------
    // compute_fitness_trend
    // -----------------------------------------------------------------------

    #[test]
    fn trend_no_previous_returns_flat() {
        assert_eq!(compute_fitness_trend(0.664, None), "FLAT");
    }

    #[test]
    fn trend_exact_zero_delta_is_flat() {
        assert_eq!(compute_fitness_trend(0.664, Some(0.664)), "FLAT");
    }

    #[test]
    fn trend_delta_below_epsilon_is_flat() {
        // 0.004 < 0.005 → FLAT
        assert_eq!(compute_fitness_trend(0.664, Some(0.660)), "FLAT");
    }

    #[test]
    fn trend_delta_at_epsilon_is_up() {
        // exactly 0.005 → UP (>= epsilon)
        assert_eq!(compute_fitness_trend(0.665, Some(0.660)), "UP");
    }

    #[test]
    fn trend_delta_above_epsilon_is_up() {
        // 0.006 > 0.005 → UP
        assert_eq!(compute_fitness_trend(0.666, Some(0.660)), "UP");
    }

    #[test]
    fn trend_negative_below_epsilon_is_flat() {
        // -0.004 > -0.005 → FLAT
        assert_eq!(compute_fitness_trend(0.656, Some(0.660)), "FLAT");
    }

    #[test]
    fn trend_negative_at_epsilon_is_down() {
        // exactly -0.005 → DOWN (<= -epsilon)
        assert_eq!(compute_fitness_trend(0.655, Some(0.660)), "DOWN");
    }

    #[test]
    fn trend_negative_above_epsilon_is_down() {
        // -0.006 → DOWN
        assert_eq!(compute_fitness_trend(0.654, Some(0.660)), "DOWN");
    }

    #[test]
    fn trend_large_positive_is_up() {
        assert_eq!(compute_fitness_trend(0.800, Some(0.500)), "UP");
    }

    #[test]
    fn trend_large_negative_is_down() {
        assert_eq!(compute_fitness_trend(0.400, Some(0.800)), "DOWN");
    }

    #[test]
    fn trend_zero_fitness_no_change_is_flat() {
        assert_eq!(compute_fitness_trend(0.0, Some(0.0)), "FLAT");
    }

    // -----------------------------------------------------------------------
    // Boundary cases at epsilon: 0.004 / 0.005 / 0.006
    // -----------------------------------------------------------------------

    #[test]
    fn trend_boundary_0004_flat() {
        // delta = +0.004 < epsilon → FLAT
        assert_eq!(compute_fitness_trend(0.504, Some(0.500)), "FLAT");
    }

    #[test]
    fn trend_boundary_0005_up() {
        // delta = +0.005 == epsilon → UP
        assert_eq!(compute_fitness_trend(0.505, Some(0.500)), "UP");
    }

    #[test]
    fn trend_boundary_0006_up() {
        // delta = +0.006 > epsilon → UP
        assert_eq!(compute_fitness_trend(0.506, Some(0.500)), "UP");
    }

    #[test]
    fn trend_boundary_neg_0004_flat() {
        // delta = -0.004 → FLAT
        assert_eq!(compute_fitness_trend(0.496, Some(0.500)), "FLAT");
    }

    #[test]
    fn trend_boundary_neg_0005_down() {
        // delta = -0.005 == -epsilon → DOWN
        assert_eq!(compute_fitness_trend(0.495, Some(0.500)), "DOWN");
    }

    #[test]
    fn trend_boundary_neg_0006_down() {
        // delta = -0.006 → DOWN
        assert_eq!(compute_fitness_trend(0.494, Some(0.500)), "DOWN");
    }

    // -----------------------------------------------------------------------
    // HealthSnapshot serde
    // -----------------------------------------------------------------------

    #[test]
    fn health_snapshot_serde_round_trip() {
        let snap = HealthSnapshot {
            ralph_fitness: 0.664,
            field_r: 0.876,
            thermal_t: 0.515,
            ltp_ltd_ratio: 4.2,
            services_healthy: 11,
            key_achievement: Some("L8 sealed".to_string()),
        };
        let json = serde_json::to_string(&snap).unwrap();
        let back: HealthSnapshot = serde_json::from_str(&json).unwrap();
        assert!((back.ralph_fitness - snap.ralph_fitness).abs() < f64::EPSILON);
        assert!((back.field_r - snap.field_r).abs() < f64::EPSILON);
        assert!((back.thermal_t - snap.thermal_t).abs() < f64::EPSILON);
        assert!((back.ltp_ltd_ratio - snap.ltp_ltd_ratio).abs() < 1e-10);
        assert_eq!(back.services_healthy, snap.services_healthy);
        assert_eq!(back.key_achievement, snap.key_achievement);
    }

    #[test]
    fn health_snapshot_serde_none_achievement() {
        let snap = HealthSnapshot {
            ralph_fitness: 0.5,
            field_r: 0.0,
            thermal_t: 0.0,
            ltp_ltd_ratio: 1.0,
            services_healthy: 12,
            key_achievement: None,
        };
        let json = serde_json::to_string(&snap).unwrap();
        let back: HealthSnapshot = serde_json::from_str(&json).unwrap();
        assert!(back.key_achievement.is_none());
    }

    #[test]
    fn health_snapshot_clone() {
        let snap = snapshot(0.664);
        let cloned = snap.clone();
        assert!((cloned.ralph_fitness - snap.ralph_fitness).abs() < f64::EPSILON);
    }

    #[test]
    fn health_snapshot_debug_not_empty() {
        let snap = snapshot(0.664);
        assert!(!format!("{snap:?}").is_empty());
    }

    // -----------------------------------------------------------------------
    // CaptureResult fields and serde
    // -----------------------------------------------------------------------

    #[test]
    fn capture_result_fields_accessible() {
        let conn = mem_db();
        let result = capture_trajectory(&conn, 1, &snapshot(0.5)).unwrap();
        let _ = result.session_id;
        let _ = result.delta_summary.len();
        let _ = result.fitness_delta;
        let _ = result.inserted;
    }

    #[test]
    fn capture_result_serde_round_trip() {
        let result = CaptureResult {
            session_id: 109,
            delta_summary: "fitness +0.010 after session".to_string(),
            fitness_delta: Some(0.010),
            inserted: true,
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: CaptureResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.session_id, result.session_id);
        assert_eq!(back.delta_summary, result.delta_summary);
        assert!((back.fitness_delta.unwrap() - result.fitness_delta.unwrap()).abs() < f64::EPSILON);
        assert_eq!(back.inserted, result.inserted);
    }

    #[test]
    fn capture_result_serde_none_delta() {
        let result = CaptureResult {
            session_id: 1,
            delta_summary: "first trajectory point".to_string(),
            fitness_delta: None,
            inserted: true,
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: CaptureResult = serde_json::from_str(&json).unwrap();
        assert!(back.fitness_delta.is_none());
    }

    #[test]
    fn capture_result_clone() {
        let result = CaptureResult {
            session_id: 1,
            delta_summary: "first trajectory point".to_string(),
            fitness_delta: None,
            inserted: true,
        };
        let cloned = result.clone();
        assert_eq!(cloned.session_id, result.session_id);
    }

    #[test]
    fn capture_result_debug_not_empty() {
        let result = CaptureResult {
            session_id: 1,
            delta_summary: "x".to_string(),
            fitness_delta: None,
            inserted: false,
        };
        assert!(!format!("{result:?}").is_empty());
    }

    // -----------------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn capture_zero_fitness_stores_correctly() {
        let conn = mem_db();
        let snap = snapshot(0.0);
        let result = capture_trajectory(&conn, 1, &snap).unwrap();
        assert!(result.inserted);
        let row = get_by_session(&conn, 1).unwrap().unwrap();
        assert!(row.ralph_fitness.abs() < f64::EPSILON);
    }

    #[test]
    fn capture_fitness_one_stores_correctly() {
        let conn = mem_db();
        let result = capture_trajectory(&conn, 1, &snapshot(1.0)).unwrap();
        assert!(result.inserted);
        let row = get_by_session(&conn, 1).unwrap().unwrap();
        assert!((row.ralph_fitness - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn capture_very_small_positive_delta_below_epsilon_is_stable() {
        let conn = mem_db();
        seed_previous(&conn, 108, 0.500);
        // Delta 0.004 < 0.005 → stable
        let snap = snapshot(0.504);
        let result = capture_trajectory(&conn, 109, &snap).unwrap();
        assert!(result.delta_summary.contains("stable"), "got: {}", result.delta_summary);
    }

    #[test]
    fn capture_delta_at_epsilon_shows_numeric() {
        let conn = mem_db();
        seed_previous(&conn, 108, 0.500);
        // Delta exactly 0.005 → UP → numeric format
        let snap = snapshot(0.505);
        let result = capture_trajectory(&conn, 109, &snap).unwrap();
        assert!(
            result.delta_summary.contains('+'),
            "expected numeric delta in {:?}",
            result.delta_summary
        );
    }

    #[test]
    fn capture_multiple_sequential_sessions() {
        let conn = mem_db();
        for s in 100_u32..=104 {
            let fitness = 0.5 + f64::from(s - 99) * 0.01;
            let snap = snapshot(fitness);
            let result = capture_trajectory(&conn, s, &snap).unwrap();
            assert!(result.inserted, "session {s} should have been inserted");
        }
        let count = crate::m2_schema::m08_trajectory::count(&conn).unwrap();
        assert_eq!(count, 5);
    }

    #[test]
    fn capture_uses_immediately_preceding_session() {
        let conn = mem_db();
        // Session 100 at 0.5, session 105 at 0.7 — skip 101..104
        seed_previous(&conn, 100, 0.5);
        let result = capture_trajectory(&conn, 105, &snapshot(0.7)).unwrap();
        let delta = result.fitness_delta.unwrap();
        // Should be 0.7 - 0.5 = 0.2
        assert!((delta - 0.2).abs() < 1e-10, "expected 0.2, got {delta}");
    }

    #[test]
    fn capture_all_fields_stored_in_db() {
        let conn = mem_db();
        let snap = HealthSnapshot {
            ralph_fitness: 0.750,
            field_r: 0.920,
            thermal_t: 0.480,
            ltp_ltd_ratio: 6.5,
            services_healthy: 10,
            key_achievement: Some("new feature".to_string()),
        };
        capture_trajectory(&conn, 42, &snap).unwrap();
        let row = get_by_session(&conn, 42).unwrap().unwrap();
        assert!((row.ralph_fitness - 0.750).abs() < f64::EPSILON);
        assert!((row.field_r - 0.920).abs() < f64::EPSILON);
        assert!((row.thermal_t - 0.480).abs() < f64::EPSILON);
        assert!((row.ltp_ltd_ratio - 6.5).abs() < 1e-10);
        assert_eq!(row.services_healthy, 10);
        assert_eq!(row.key_achievement.as_deref(), Some("new feature"));
    }

    #[test]
    fn delta_summary_format_with_zero_previous_fitness() {
        let prev = make_trajectory_row(1, 0.0);
        let snap = snapshot(0.100);
        let summary = compute_delta_summary(&snap, Some(&prev));
        // 0.100 - 0.0 = 0.100 → "+0.100"
        assert!(summary.contains("+0.100"), "got: {summary}");
    }

    #[test]
    fn capture_result_inserted_false_has_delta_from_earlier_session() {
        let conn = mem_db();
        seed_previous(&conn, 108, 0.660);
        // Insert session 109 once
        capture_trajectory(&conn, 109, &snapshot(0.670)).unwrap();
        // Second call — idempotent, but delta should still be computed
        let result = capture_trajectory(&conn, 109, &snapshot(0.670)).unwrap();
        assert!(!result.inserted);
        let delta = result.fitness_delta.unwrap();
        assert!((delta - 0.010).abs() < 1e-10);
    }
}
