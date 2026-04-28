//! `m11_parallel_query` — Sequential query executor with per-query timing and
//! staleness annotation. Runs 4 `SQLite` queries in the injection pipeline —
//! `causal_chain`, `session_trajectory`, `workstream`, `reinforced_pattern` —
//! each timed independently and annotated with a staleness flag when the query
//! exceeds half the 100 ms total budget (i.e. `> 50 ms`).
//!
//! ## Design note: sequential, not parallel
//!
//! `rusqlite::Connection` is neither `Send` nor `Sync`, so the 4 queries must
//! run on the same thread. The public surface (`execute_all`) is deliberately
//! named to match the future `std::thread::scope`-based Phase 2 API that will
//! be wired once the `SpaceTimeDB` substrate is in place. The "parallel" in the
//! module name refers to the aspirational Phase 2 architecture; in Phase 1
//! (`SQLite`) we achieve the contract via sequential execution with independent
//! per-query timers.
//!
//! ## Cache path
//!
//! [`execute_cached`] checks the `injection_cache` table for a pre-computed
//! full payload. If the row's `computed_at` timestamp is within
//! [`CACHE_REBUILD_SECS`] seconds of `now()`, the cached string is returned
//! directly — skipping all four queries.
//!
//! Layer: `m3_injection`
//!
//! Dependencies:
//! - `crate::m1_foundation::m02_errors::InjectionError`
//! - `crate::m1_foundation::m03_config::InjectionConfig`
//! - `crate::m1_foundation::m05_constants`
//! - `crate::m2_schema::m07_causal_chain::{find_unresolved, CausalChainRow}`
//! - `crate::m2_schema::m08_trajectory::{get_recent, TrajectoryRow}`
//! - `crate::m2_schema::m09_workstream::{get_active, get_blocked, WorkstreamRow}`
//! - `crate::m2_schema::m10_pattern::{get_top_by_weight, PatternRow}`

#[cfg(feature = "sqlite")]
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[cfg(feature = "sqlite")]
use crate::m1_foundation::m03_config::InjectionConfig;
#[cfg(feature = "sqlite")]
use crate::m1_foundation::m05_constants;
#[cfg(feature = "sqlite")]
use crate::m1_foundation::m02_errors::InjectionError;
#[cfg(feature = "sqlite")]
use crate::m2_schema::m07_causal_chain::{CausalChainRow, find_unresolved};
#[cfg(feature = "sqlite")]
use crate::m2_schema::m08_trajectory::{TrajectoryRow, get_recent};
#[cfg(feature = "sqlite")]
use crate::m2_schema::m09_workstream::{WorkstreamRow, get_active, get_blocked};
#[cfg(feature = "sqlite")]
use crate::m2_schema::m10_pattern::{PatternRow, get_top_by_weight};

// ---------------------------------------------------------------------------
// Staleness threshold
// ---------------------------------------------------------------------------

/// Queries taking longer than this are marked stale — half of the 100 ms total
/// injection budget.
pub const STALE_THRESHOLD_MS: u64 = 50;

/// Section key used in `injection_cache` for the consolidated full payload.
pub const CACHE_SECTION_KEY: &str = "full_payload";

// ---------------------------------------------------------------------------
// QueryResult
// ---------------------------------------------------------------------------

/// Wraps a query result with per-query timing and staleness metadata.
///
/// `stale` is `true` when `elapsed_ms > STALE_THRESHOLD_MS` (50 ms), which
/// signals that this query consumed more than half the injection latency budget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult<T> {
    /// The rows returned by the query (empty `Vec` when the table is empty).
    pub data: T,
    /// Wall-clock time taken by this query in milliseconds.
    pub elapsed_ms: u64,
    /// `true` if `elapsed_ms > 50` — consumed more than half the latency budget.
    pub stale: bool,
}

impl<T> QueryResult<T> {
    /// Construct a [`QueryResult`], computing `stale` from `elapsed_ms`.
    #[must_use]
    pub fn new(data: T, elapsed_ms: u64) -> Self {
        Self {
            data,
            elapsed_ms,
            stale: elapsed_ms > STALE_THRESHOLD_MS,
        }
    }

    /// Returns `true` if this result is within the latency budget.
    #[must_use]
    pub fn is_fresh(&self) -> bool {
        !self.stale
    }
}

// ---------------------------------------------------------------------------
// InjectionData
// ---------------------------------------------------------------------------

/// Aggregated results from all four injection queries.
///
/// Each field is a [`QueryResult`] wrapping the rows from its corresponding
/// table. `total_elapsed_ms` is the sum of all four `elapsed_ms` values and
/// represents the end-to-end query phase duration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectionData {
    /// Unresolved causal chains ordered by `reinforcement_count DESC`.
    #[cfg(feature = "sqlite")]
    pub chains: QueryResult<Vec<CausalChainRow>>,

    /// Recent session trajectory rows ordered by `session_id DESC`.
    #[cfg(feature = "sqlite")]
    pub trajectory: QueryResult<Vec<TrajectoryRow>>,

    /// Active and blocked workstreams, active ordered by `priority ASC`.
    #[cfg(feature = "sqlite")]
    pub workstreams: QueryResult<Vec<WorkstreamRow>>,

    /// Top reinforced patterns ordered by `weight DESC`.
    #[cfg(feature = "sqlite")]
    pub patterns: QueryResult<Vec<PatternRow>>,

    /// Sum of all four per-query `elapsed_ms` values.
    pub total_elapsed_ms: u64,
}

#[cfg(feature = "sqlite")]
impl InjectionData {
    /// Returns `true` if every query result is within the latency budget.
    #[must_use]
    pub fn all_fresh(&self) -> bool {
        self.chains.is_fresh()
            && self.trajectory.is_fresh()
            && self.workstreams.is_fresh()
            && self.patterns.is_fresh()
    }

    /// Returns `true` if any query result exceeded the staleness threshold.
    #[must_use]
    pub fn any_stale(&self) -> bool {
        !self.all_fresh()
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Execute all four injection queries sequentially, recording independent
/// per-query timings and staleness annotations.
///
/// Although the function is named `execute_all` to match the future
/// `std::thread::scope`-based Phase 2 parallel API, the `SQLite` Phase 1
/// implementation runs queries on a single thread because
/// `rusqlite::Connection` is neither `Send` nor `Sync`.
///
/// Workstreams are merged from `get_active` and `get_blocked` in priority
/// order (active first, then blocked), capped at `config.max_workstreams`.
///
/// # Consent filtering
///
/// The returned rows are **unfiltered** — they include rows with any
/// `consent` value (`Emit`, `Store`, `Forget`). Callers **must** apply
/// [`crate::m3_injection::m14_consent_filter`] before passing results to
/// the renderer. The separation is intentional: the query layer retrieves
/// raw data; the consent layer gates what reaches the context window.
///
/// # Errors
///
/// Returns [`InjectionError::QueryFailed`] if any individual query fails,
/// with the underlying `rusqlite` error message embedded.
#[cfg(feature = "sqlite")]
pub fn execute_all(
    conn: &Connection,
    config: &InjectionConfig,
) -> Result<InjectionData, InjectionError> {
    use std::time::Instant;

    // Query 1: causal chains
    let t0 = Instant::now();
    let chain_rows = find_unresolved(conn, config.max_chains)
        .map_err(|e| InjectionError::QueryFailed(e.to_string()))?;
    let chains = QueryResult::new(chain_rows, elapsed_ms(t0));

    // Query 2: session trajectory
    let t1 = Instant::now();
    let traj_rows = get_recent(conn, config.max_trajectory_points)
        .map_err(|e| InjectionError::QueryFailed(e.to_string()))?;
    let trajectory = QueryResult::new(traj_rows, elapsed_ms(t1));

    // Query 3: workstreams (active + blocked, capped)
    let t2 = Instant::now();
    let mut ws_rows = get_active(conn)
        .map_err(|e| InjectionError::QueryFailed(e.to_string()))?;
    let blocked_rows = get_blocked(conn)
        .map_err(|e| InjectionError::QueryFailed(e.to_string()))?;
    ws_rows.extend(blocked_rows);
    ws_rows.truncate(config.max_workstreams);
    let workstreams = QueryResult::new(ws_rows, elapsed_ms(t2));

    // Query 4: patterns
    let t3 = Instant::now();
    let pat_rows = get_top_by_weight(conn, config.max_patterns)
        .map_err(|e| InjectionError::QueryFailed(e.to_string()))?;
    let patterns = QueryResult::new(pat_rows, elapsed_ms(t3));

    let total_elapsed_ms =
        chains.elapsed_ms + trajectory.elapsed_ms + workstreams.elapsed_ms + patterns.elapsed_ms;

    Ok(InjectionData {
        chains,
        trajectory,
        workstreams,
        patterns,
        total_elapsed_ms,
    })
}

/// Try to return a pre-computed full payload from the `injection_cache` table.
///
/// Returns `Some(payload)` if a row keyed `"full_payload"` exists and its
/// `computed_at` Unix timestamp is within [`m05_constants::CACHE_REBUILD_SECS`]
/// seconds of the current time. Returns `None` if the cache is absent, stale,
/// or if `computed_at` cannot be parsed.
///
/// # Errors
///
/// Returns [`InjectionError::QueryFailed`] if the SQL query itself fails.
#[cfg(feature = "sqlite")]
pub fn execute_cached(conn: &Connection) -> Result<Option<String>, InjectionError> {
    use rusqlite::OptionalExtension as _;

    let row: Option<(String, i64)> = conn
        .query_row(
            "SELECT payload, computed_at FROM injection_cache WHERE section = ?1",
            rusqlite::params![CACHE_SECTION_KEY],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(|e| InjectionError::QueryFailed(e.to_string()))?;

    let Some((payload, computed_at)) = row else {
        return Ok(None);
    };

    let now = unix_now_secs();
    let age = now.saturating_sub(computed_at.max(0).cast_unsigned());

    if age <= m05_constants::CACHE_REBUILD_SECS {
        Ok(Some(payload))
    } else {
        Ok(None)
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Convert an [`std::time::Instant`] elapsed time to whole milliseconds.
#[cfg(feature = "sqlite")]
#[inline]
fn elapsed_ms(start: std::time::Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// Return the current time as a Unix timestamp in seconds.
///
/// Falls back to `0` if the system clock is before the Unix epoch (impossible
/// in practice; guarded for correctness).
#[cfg(feature = "sqlite")]
fn unix_now_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Bring in SQLite-gated items only when the feature is available.
    #[cfg(feature = "sqlite")]
    use crate::m1_foundation::m03_config::InjectionConfig;
    #[cfg(feature = "sqlite")]
    use crate::m2_schema::m06_schema::open_memory;
    #[cfg(feature = "sqlite")]
    use crate::m2_schema::m07_causal_chain::insert_chain;
    #[cfg(feature = "sqlite")]
    use crate::m2_schema::m08_trajectory::insert_point;
    #[cfg(feature = "sqlite")]
    use crate::m2_schema::m09_workstream::insert_workstream;
    #[cfg(feature = "sqlite")]
    use crate::m2_schema::m10_pattern::insert_pattern;

    // -------------------------------------------------------------------------
    // QueryResult construction + stale flag
    // -------------------------------------------------------------------------

    #[test]
    fn query_result_fresh_when_under_threshold() {
        let qr: QueryResult<Vec<i32>> = QueryResult::new(vec![], 49);
        assert!(!qr.stale);
        assert!(qr.is_fresh());
    }

    #[test]
    fn query_result_stale_exactly_at_threshold() {
        // Threshold is > 50, so 50 is still fresh.
        let qr: QueryResult<Vec<i32>> = QueryResult::new(vec![], 50);
        assert!(!qr.stale);
    }

    #[test]
    fn query_result_stale_above_threshold() {
        let qr: QueryResult<Vec<i32>> = QueryResult::new(vec![], 51);
        assert!(qr.stale);
        assert!(!qr.is_fresh());
    }

    #[test]
    fn query_result_zero_elapsed_is_fresh() {
        let qr: QueryResult<Vec<String>> = QueryResult::new(vec![], 0);
        assert!(!qr.stale);
        assert!(qr.is_fresh());
    }

    #[test]
    fn query_result_max_elapsed_is_stale() {
        let qr: QueryResult<()> = QueryResult::new((), u64::MAX);
        assert!(qr.stale);
    }

    #[test]
    fn query_result_data_preserved() {
        let data = vec![1u32, 2, 3];
        let qr = QueryResult::new(data.clone(), 5);
        assert_eq!(qr.data, data);
    }

    #[test]
    fn query_result_elapsed_preserved() {
        let qr: QueryResult<()> = QueryResult::new((), 77);
        assert_eq!(qr.elapsed_ms, 77);
    }

    // -------------------------------------------------------------------------
    // QueryResult — Clone / Debug / Serde
    // -------------------------------------------------------------------------

    #[test]
    fn query_result_clone() {
        let qr = QueryResult::new(vec![42u32], 10);
        let cloned = qr.clone();
        assert_eq!(cloned.elapsed_ms, 10);
        assert_eq!(cloned.data, vec![42u32]);
    }

    #[test]
    fn query_result_debug_not_empty() {
        let qr = QueryResult::new(vec![1i32], 3);
        assert!(!format!("{qr:?}").is_empty());
    }

    #[test]
    fn query_result_serde_roundtrip_json() {
        let qr = QueryResult::new(vec![1u32, 2, 3], 25);
        let json = serde_json::to_string(&qr).unwrap();
        let parsed: QueryResult<Vec<u32>> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.data, vec![1u32, 2, 3]);
        assert_eq!(parsed.elapsed_ms, 25);
        assert!(!parsed.stale);
    }

    #[test]
    fn query_result_serde_roundtrip_stale() {
        let qr = QueryResult::new(String::from("hello"), 200);
        let json = serde_json::to_string(&qr).unwrap();
        let parsed: QueryResult<String> = serde_json::from_str(&json).unwrap();
        assert!(parsed.stale);
    }

    #[test]
    fn query_result_serde_empty_vec() {
        let qr: QueryResult<Vec<u8>> = QueryResult::new(vec![], 0);
        let json = serde_json::to_string(&qr).unwrap();
        let parsed: QueryResult<Vec<u8>> = serde_json::from_str(&json).unwrap();
        assert!(parsed.data.is_empty());
    }

    // -------------------------------------------------------------------------
    // Constants
    // -------------------------------------------------------------------------

    #[test]
    fn stale_threshold_is_fifty() {
        assert_eq!(STALE_THRESHOLD_MS, 50);
    }

    #[test]
    fn cache_section_key_is_expected() {
        assert_eq!(CACHE_SECTION_KEY, "full_payload");
    }

    // -------------------------------------------------------------------------
    // InjectionData — total_elapsed_ms field (sqlite-independent)
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn injection_data_total_elapsed_sum() {
        let conn = open_memory().unwrap();
        let config = InjectionConfig::default();
        let data = execute_all(&conn, &config).unwrap();
        assert_eq!(
            data.total_elapsed_ms,
            data.chains.elapsed_ms
                + data.trajectory.elapsed_ms
                + data.workstreams.elapsed_ms
                + data.patterns.elapsed_ms
        );
    }

    // -------------------------------------------------------------------------
    // execute_all — empty database
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_all_empty_db_returns_empty_vecs() {
        let conn = open_memory().unwrap();
        let config = InjectionConfig::default();
        let data = execute_all(&conn, &config).unwrap();
        assert!(data.chains.data.is_empty());
        assert!(data.trajectory.data.is_empty());
        assert!(data.workstreams.data.is_empty());
        assert!(data.patterns.data.is_empty());
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_all_empty_db_total_elapsed_geq_parts() {
        let conn = open_memory().unwrap();
        let config = InjectionConfig::default();
        let data = execute_all(&conn, &config).unwrap();
        assert_eq!(
            data.total_elapsed_ms,
            data.chains.elapsed_ms
                + data.trajectory.elapsed_ms
                + data.workstreams.elapsed_ms
                + data.patterns.elapsed_ms
        );
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_all_empty_db_all_fresh_on_empty_tables() {
        // An empty DB should execute extremely quickly — all queries fresh.
        let conn = open_memory().unwrap();
        let config = InjectionConfig::default();
        let data = execute_all(&conn, &config).unwrap();
        // We cannot guarantee sub-50ms in all CI environments, so we only
        // check the invariant that stale == (elapsed > 50).
        assert_eq!(data.chains.stale, data.chains.elapsed_ms > STALE_THRESHOLD_MS);
        assert_eq!(data.trajectory.stale, data.trajectory.elapsed_ms > STALE_THRESHOLD_MS);
        assert_eq!(data.workstreams.stale, data.workstreams.elapsed_ms > STALE_THRESHOLD_MS);
        assert_eq!(data.patterns.stale, data.patterns.elapsed_ms > STALE_THRESHOLD_MS);
    }

    // -------------------------------------------------------------------------
    // execute_all — seeded data
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_all_chains_seeded() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 109, "bug", "stale-cache-miss", "Cache not refreshed on startup").unwrap();
        insert_chain(&conn, 108, "trap", "unwrap-in-prod", "unwrap() outside tests").unwrap();

        let config = InjectionConfig::default();
        let data = execute_all(&conn, &config).unwrap();
        assert_eq!(data.chains.data.len(), 2);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_all_chains_respects_max_chains() {
        let conn = open_memory().unwrap();
        for i in 0..20u32 {
            insert_chain(&conn, i, "bug", &format!("label-{i}"), "desc").unwrap();
        }
        let config = InjectionConfig::default();
        let data = execute_all(&conn, &config).unwrap();
        assert!(data.chains.data.len() <= config.max_chains);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_all_trajectory_seeded() {
        let conn = open_memory().unwrap();
        insert_point(&conn, 109, 0.74, 0.92, 0.50, 3.2, 11, "S109 close", None).unwrap();
        insert_point(&conn, 108, 0.69, 0.88, 0.44, 2.1, 10, "S108 close", Some("Watcher born")).unwrap();

        let config = InjectionConfig::default();
        let data = execute_all(&conn, &config).unwrap();
        assert_eq!(data.trajectory.data.len(), 2);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_all_trajectory_respects_max_points() {
        let conn = open_memory().unwrap();
        for i in 100u32..115 {
            insert_point(&conn, i, 0.7, 0.8, 0.5, 2.0, 11, "summary", None).unwrap();
        }
        let config = InjectionConfig::default();
        let data = execute_all(&conn, &config).unwrap();
        assert!(data.trajectory.data.len() <= config.max_trajectory_points);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_all_workstreams_active_included() {
        let conn = open_memory().unwrap();
        insert_workstream(&conn, "stdb-inject", "STDB injection", "active", 109, "resume here").unwrap();

        let config = InjectionConfig::default();
        let data = execute_all(&conn, &config).unwrap();
        assert_eq!(data.workstreams.data.len(), 1);
        assert_eq!(data.workstreams.data[0].ws_id, "stdb-inject");
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_all_workstreams_blocked_included() {
        let conn = open_memory().unwrap();
        insert_workstream(&conn, "stdb-inject", "STDB injection", "blocked", 109, "blocked by apt").unwrap();

        let config = InjectionConfig::default();
        let data = execute_all(&conn, &config).unwrap();
        assert_eq!(data.workstreams.data.len(), 1);
        assert_eq!(data.workstreams.data[0].status, "blocked");
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_all_workstreams_deferred_excluded() {
        let conn = open_memory().unwrap();
        insert_workstream(&conn, "deferred-ws", "Deferred work", "deferred", 109, "later").unwrap();

        let config = InjectionConfig::default();
        let data = execute_all(&conn, &config).unwrap();
        // get_active and get_blocked only — deferred must not appear
        assert!(
            data.workstreams.data.iter().all(|w| w.status != "deferred"),
            "deferred workstream should not be included in injection"
        );
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_all_workstreams_capped_at_max() {
        let conn = open_memory().unwrap();
        for i in 0..20u32 {
            insert_workstream(
                &conn,
                &format!("ws-{i}"),
                "title",
                "active",
                109,
                "resume",
            ).unwrap();
        }
        let config = InjectionConfig::default();
        let data = execute_all(&conn, &config).unwrap();
        assert!(data.workstreams.data.len() <= config.max_workstreams);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_all_patterns_seeded() {
        let conn = open_memory().unwrap();
        insert_pattern(&conn, "verify-before-ship", "feedback", "Always verify before shipping", None).unwrap();

        let config = InjectionConfig::default();
        let data = execute_all(&conn, &config).unwrap();
        assert_eq!(data.patterns.data.len(), 1);
        assert_eq!(data.patterns.data[0].pattern_id, "verify-before-ship");
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_all_patterns_respects_max_patterns() {
        let conn = open_memory().unwrap();
        for i in 0..20u32 {
            insert_pattern(
                &conn,
                &format!("pattern-{i}"),
                "procedural",
                "desc",
                None,
            ).unwrap();
        }
        let config = InjectionConfig::default();
        let data = execute_all(&conn, &config).unwrap();
        assert!(data.patterns.data.len() <= config.max_patterns);
    }

    // -------------------------------------------------------------------------
    // execute_all — mixed seeded data
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_all_all_tables_seeded() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 109, "bug", "some-bug", "desc").unwrap();
        insert_point(&conn, 109, 0.7, 0.9, 0.5, 3.0, 11, "summary", None).unwrap();
        insert_workstream(&conn, "ws-1", "Work 1", "active", 109, "resume").unwrap();
        insert_pattern(&conn, "pat-1", "procedural", "desc", None).unwrap();

        let config = InjectionConfig::default();
        let data = execute_all(&conn, &config).unwrap();
        assert!(!data.chains.data.is_empty());
        assert!(!data.trajectory.data.is_empty());
        assert!(!data.workstreams.data.is_empty());
        assert!(!data.patterns.data.is_empty());
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_all_fields_accessible() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 109, "trap", "my-trap", "my description").unwrap();

        let config = InjectionConfig::default();
        let data = execute_all(&conn, &config).unwrap();
        assert_eq!(data.chains.data[0].label, "my-trap");
        assert_eq!(data.chains.data[0].description, "my description");
    }

    // -------------------------------------------------------------------------
    // InjectionData — all_fresh / any_stale helpers
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn injection_data_all_fresh_on_fast_queries() {
        let conn = open_memory().unwrap();
        let config = InjectionConfig::default();
        let data = execute_all(&conn, &config).unwrap();
        // Just ensure the predicate is consistent with the individual flags.
        assert_eq!(
            data.all_fresh(),
            !(data.chains.stale
                || data.trajectory.stale
                || data.workstreams.stale
                || data.patterns.stale)
        );
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn injection_data_any_stale_is_negation_of_all_fresh() {
        let conn = open_memory().unwrap();
        let config = InjectionConfig::default();
        let data = execute_all(&conn, &config).unwrap();
        assert_eq!(data.any_stale(), !data.all_fresh());
    }

    // -------------------------------------------------------------------------
    // InjectionData — Clone / Debug / Serde
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn injection_data_clone() {
        let conn = open_memory().unwrap();
        let config = InjectionConfig::default();
        let data = execute_all(&conn, &config).unwrap();
        let cloned = data.clone();
        assert_eq!(cloned.total_elapsed_ms, data.total_elapsed_ms);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn injection_data_debug_not_empty() {
        let conn = open_memory().unwrap();
        let config = InjectionConfig::default();
        let data = execute_all(&conn, &config).unwrap();
        assert!(!format!("{data:?}").is_empty());
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn injection_data_serde_roundtrip() {
        let conn = open_memory().unwrap();
        let config = InjectionConfig::default();
        let data = execute_all(&conn, &config).unwrap();
        let json = serde_json::to_string(&data).unwrap();
        let parsed: InjectionData = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.total_elapsed_ms, data.total_elapsed_ms);
        assert_eq!(parsed.chains.elapsed_ms, data.chains.elapsed_ms);
    }

    // -------------------------------------------------------------------------
    // execute_cached — missing cache
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_cached_empty_table_returns_none() {
        let conn = open_memory().unwrap();
        let result = execute_cached(&conn).unwrap();
        assert!(result.is_none());
    }

    // -------------------------------------------------------------------------
    // execute_cached — fresh cache
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_cached_fresh_row_returns_payload() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let conn = open_memory().unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO injection_cache (section, payload, token_count, computed_at)
             VALUES (?1, ?2, 100, ?3)",
            rusqlite::params![CACHE_SECTION_KEY, "## Injection\nfresh payload", now],
        ).unwrap();

        let result = execute_cached(&conn).unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains("fresh payload"));
    }

    // -------------------------------------------------------------------------
    // execute_cached — stale cache
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_cached_stale_row_returns_none() {
        use crate::m1_foundation::m05_constants::CACHE_REBUILD_SECS;
        use std::time::{SystemTime, UNIX_EPOCH};
        let conn = open_memory().unwrap();
        let old_ts = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64)
            - (CACHE_REBUILD_SECS as i64) - 10;

        conn.execute(
            "INSERT INTO injection_cache (section, payload, token_count, computed_at)
             VALUES (?1, ?2, 100, ?3)",
            rusqlite::params![CACHE_SECTION_KEY, "stale payload", old_ts],
        ).unwrap();

        let result = execute_cached(&conn).unwrap();
        assert!(result.is_none(), "stale cache should return None");
    }

    // -------------------------------------------------------------------------
    // execute_cached — boundary: exactly at cache_rebuild_secs
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_cached_boundary_exactly_rebuild_secs_is_fresh() {
        use crate::m1_foundation::m05_constants::CACHE_REBUILD_SECS;
        use std::time::{SystemTime, UNIX_EPOCH};
        let conn = open_memory().unwrap();
        // age == CACHE_REBUILD_SECS is still within budget (<=)
        let ts = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64)
            - CACHE_REBUILD_SECS as i64;

        conn.execute(
            "INSERT INTO injection_cache (section, payload, token_count, computed_at)
             VALUES (?1, ?2, 100, ?3)",
            rusqlite::params![CACHE_SECTION_KEY, "boundary payload", ts],
        ).unwrap();

        let result = execute_cached(&conn).unwrap();
        assert!(result.is_some(), "cache exactly at rebuild boundary should be fresh");
    }

    // -------------------------------------------------------------------------
    // execute_cached — wrong section key
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_cached_wrong_section_returns_none() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let conn = open_memory().unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO injection_cache (section, payload, token_count, computed_at)
             VALUES ('other_section', 'some payload', 50, ?1)",
            rusqlite::params![now],
        ).unwrap();

        let result = execute_cached(&conn).unwrap();
        assert!(result.is_none(), "wrong section key should return None");
    }

    // -------------------------------------------------------------------------
    // execute_cached — multiple sections, only correct one returned
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_cached_multiple_sections_returns_correct() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let conn = open_memory().unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO injection_cache (section, payload, token_count, computed_at) VALUES ('section_a', 'payload A', 10, ?1)",
            rusqlite::params![now],
        ).unwrap();
        conn.execute(
            "INSERT INTO injection_cache (section, payload, token_count, computed_at) VALUES (?1, 'payload FULL', 200, ?2)",
            rusqlite::params![CACHE_SECTION_KEY, now],
        ).unwrap();

        let result = execute_cached(&conn).unwrap();
        assert_eq!(result.unwrap(), "payload FULL");
    }

    // -------------------------------------------------------------------------
    // Staleness annotation invariant
    // -------------------------------------------------------------------------

    #[test]
    fn stale_flag_always_matches_threshold_formula() {
        for ms in [0u64, 49, 50, 51, 100, 999, u64::MAX] {
            let qr: QueryResult<()> = QueryResult::new((), ms);
            assert_eq!(qr.stale, ms > STALE_THRESHOLD_MS, "ms={ms}");
        }
    }

    // -------------------------------------------------------------------------
    // InjectionData total_elapsed invariant (unit-level)
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn total_elapsed_equals_sum_of_parts_seeded() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 1, "bug", "lbl", "d").unwrap();
        insert_point(&conn, 1, 0.5, 0.5, 0.5, 1.0, 11, "s", None).unwrap();
        insert_workstream(&conn, "ws", "t", "active", 1, "r").unwrap();
        insert_pattern(&conn, "p", "procedural", "d", None).unwrap();

        let config = InjectionConfig::default();
        let data = execute_all(&conn, &config).unwrap();
        assert_eq!(
            data.total_elapsed_ms,
            data.chains.elapsed_ms
                + data.trajectory.elapsed_ms
                + data.workstreams.elapsed_ms
                + data.patterns.elapsed_ms
        );
    }

    // -------------------------------------------------------------------------
    // execute_all returns error on corrupted table name (negative path)
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_all_ok_on_normal_db() {
        // Regression: ensure function does not panic or return Err on a
        // freshly-created in-memory DB.
        let conn = open_memory().unwrap();
        let config = InjectionConfig::default();
        assert!(execute_all(&conn, &config).is_ok());
    }

    // -------------------------------------------------------------------------
    // InjectionConfig limits are respected together
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_all_respects_all_limits_simultaneously() {
        let conn = open_memory().unwrap();
        for i in 0..20u32 {
            insert_chain(&conn, i, "bug", &format!("c-{i}"), "d").unwrap();
            insert_pattern(&conn, &format!("p-{i}"), "procedural", "d", None).unwrap();
            insert_workstream(&conn, &format!("ws-{i}"), "t", "active", i, "r").unwrap();
        }
        for i in 100u32..120 {
            insert_point(&conn, i, 0.7, 0.8, 0.5, 2.0, 11, "s", None).unwrap();
        }

        let config = InjectionConfig::default();
        let data = execute_all(&conn, &config).unwrap();
        assert!(data.chains.data.len() <= config.max_chains);
        assert!(data.trajectory.data.len() <= config.max_trajectory_points);
        assert!(data.workstreams.data.len() <= config.max_workstreams);
        assert!(data.patterns.data.len() <= config.max_patterns);
    }

    // -------------------------------------------------------------------------
    // QueryResult — generic type flexibility
    // -------------------------------------------------------------------------

    #[test]
    fn query_result_with_option_type() {
        let qr: QueryResult<Option<String>> = QueryResult::new(Some("hello".into()), 5);
        assert_eq!(qr.data.as_deref(), Some("hello"));
    }

    #[test]
    fn query_result_with_nested_vec() {
        let qr: QueryResult<Vec<Vec<u8>>> = QueryResult::new(vec![vec![1, 2], vec![3]], 10);
        assert_eq!(qr.data.len(), 2);
    }

    #[test]
    fn query_result_with_unit_type() {
        let qr: QueryResult<()> = QueryResult::new((), 0);
        assert!(!qr.stale);
    }

    // -------------------------------------------------------------------------
    // execute_all — workstream merging
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_all_merges_active_and_blocked_workstreams() {
        let conn = open_memory().unwrap();
        insert_workstream(&conn, "ws-active", "Active", "active", 109, "ctx").unwrap();
        insert_workstream(&conn, "ws-blocked", "Blocked", "blocked", 108, "ctx").unwrap();

        let config = InjectionConfig::default();
        let data = execute_all(&conn, &config).unwrap();
        assert_eq!(data.workstreams.data.len(), 2);
        let statuses: Vec<&str> = data.workstreams.data.iter().map(|w| w.status.as_str()).collect();
        assert!(statuses.contains(&"active"));
        assert!(statuses.contains(&"blocked"));
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_all_complete_workstreams_excluded() {
        let conn = open_memory().unwrap();
        insert_workstream(&conn, "ws-done", "Done", "complete", 100, "ctx").unwrap();

        let config = InjectionConfig::default();
        let data = execute_all(&conn, &config).unwrap();
        assert!(data.workstreams.data.is_empty());
    }

    // -------------------------------------------------------------------------
    // execute_cached — negative timestamp treated as stale
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_cached_negative_timestamp_is_stale() {
        let conn = open_memory().unwrap();
        conn.execute(
            "INSERT INTO injection_cache (section, payload, token_count, computed_at)
             VALUES (?1, 'payload', 50, -100)",
            rusqlite::params![CACHE_SECTION_KEY],
        ).unwrap();

        let result = execute_cached(&conn).unwrap();
        assert!(result.is_none());
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_cached_zero_timestamp_is_stale() {
        let conn = open_memory().unwrap();
        conn.execute(
            "INSERT INTO injection_cache (section, payload, token_count, computed_at)
             VALUES (?1, 'payload', 50, 0)",
            rusqlite::params![CACHE_SECTION_KEY],
        ).unwrap();

        let result = execute_cached(&conn).unwrap();
        assert!(result.is_none());
    }

    // -------------------------------------------------------------------------
    // X2: Full L3 integration test — seed → query → filter → render → cache
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn integration_full_pipeline_seed_to_cached_payload() {
        use crate::m1_foundation::m01_types::TokenBudget;
        use crate::m3_injection::m12_prose_renderer::{
            render, ChainEntry, RenderInput, TrajectoryEntry, WorkstreamEntry,
        };
        use crate::m3_injection::m14_consent_filter::{
            filter_chains, filter_patterns, filter_trajectories, filter_workstreams,
        };
        use crate::m4_consolidation::m17_cache_builder::write_cache_entry;

        let conn = open_memory().unwrap();

        // ---- Seed L2 tables ----
        insert_chain(&conn, 109, "bug", "BUG-064i", "pathway update discarded").unwrap();
        insert_chain(&conn, 108, "trap", "cp-alias", "cp aliased to interactive").unwrap();
        insert_point(&conn, 108, 0.660, 0.0, 0.272, 0.0, 11, "L7+L8 sealed", None).unwrap();
        insert_point(&conn, 109, 0.669, 0.876, 0.515, 4.2, 11, "Watcher persona + WCP v1", Some("Watcher born")).unwrap();
        insert_workstream(&conn, "comms-v3", "Comms Layer v3", "active", 109, "WS-6 habitat-wire next").unwrap();
        insert_workstream(&conn, "phase-g", "synthex-v2 Phase G", "blocked", 109, "external gate").unwrap();
        insert_pattern(&conn, "verify-before-ship", "procedural", "Always verify before deploying", None).unwrap();

        // ---- Step 1: Execute queries ----
        let config = InjectionConfig::default();
        let data = execute_all(&conn, &config).unwrap();

        assert_eq!(data.chains.data.len(), 2);
        assert_eq!(data.trajectory.data.len(), 2);
        assert_eq!(data.workstreams.data.len(), 2);
        assert_eq!(data.patterns.data.len(), 1);

        // ---- Step 2: Consent filter ----
        let (chains, chain_stats) = filter_chains(data.chains.data);
        let (trajectory, _) = filter_trajectories(data.trajectory.data);
        let (workstreams, _) = filter_workstreams(data.workstreams.data);
        let (patterns, _) = filter_patterns(data.patterns.data);

        assert_eq!(chain_stats.passed, 2);
        assert_eq!(chains.len(), 2);

        // ---- Step 3: Build RenderInput ----
        let chain_entries: Vec<ChainEntry> = chains
            .iter()
            .map(|c| ChainEntry {
                label: c.label.clone(),
                reinforcement_count: c.reinforcement_count.try_into().unwrap_or(0),
                description: c.description.clone(),
            })
            .collect();

        let traj_entries: Vec<TrajectoryEntry> = trajectory
            .iter()
            .map(|t| TrajectoryEntry {
                session_id: t.session_id,
                ralph_fitness: t.ralph_fitness,
                delta_summary: t.delta_summary.clone(),
            })
            .collect();

        let active_ws: Vec<WorkstreamEntry> = workstreams
            .iter()
            .filter(|w| w.status == "active")
            .map(|w| WorkstreamEntry {
                title: w.title.clone(),
                status: w.status.clone(),
                items_done: w.items_done,
                items_total: w.items_total,
                resume_context: w.resume_context.clone(),
                blocker: w.blocker.clone(),
            })
            .collect();

        let blocked_ws: Vec<WorkstreamEntry> = workstreams
            .iter()
            .filter(|w| w.status == "blocked")
            .map(|w| WorkstreamEntry {
                title: w.title.clone(),
                status: w.status.clone(),
                items_done: w.items_done,
                items_total: w.items_total,
                resume_context: w.resume_context.clone(),
                blocker: w.blocker.clone(),
            })
            .collect();

        let input = RenderInput {
            session_number: 110,
            chains: chain_entries,
            trajectory: traj_entries,
            active_workstreams: active_ws,
            blocked_workstreams: blocked_ws,
            deferred_workstreams: vec![],
            patterns: vec![],
            services_healthy: 11,
            services_total: 12,
            thermal: Some(0.515),
        };

        // ---- Step 4: Render ----
        let (payload, tokens) = render(&input, TokenBudget::new(1100)).unwrap();

        assert!(tokens > 0);
        assert!(tokens <= 1100);
        assert!(payload.contains("### Orientation"));
        assert!(payload.contains("### Trajectory"));
        assert!(payload.contains("Comms Layer v3"));
        assert!(payload.contains("BUG-064i") || payload.contains("cp-alias"));
        assert!(payload.contains("S110 Injection"));

        // ---- Step 5: Write to cache ----
        write_cache_entry(&conn, &payload, tokens).unwrap();

        // ---- Step 6: Read back from cache ----
        let cached = execute_cached(&conn).unwrap();
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), payload);
    }
}
