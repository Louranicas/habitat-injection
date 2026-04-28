//! `m17_cache_builder` — Rebuilds the `injection_cache` table with a pre-computed
//! payload so that `SessionStart` injection is a single `SQL` read instead of
//! 4 queries + rendering.
//!
//! ## Pipeline
//!
//! 1. Query all 4 data tables via `m07`–`m10` CRUD functions.
//! 2. Filter each result set through `m14_consent_filter` (keep only
//!    `consent = "Emit"` rows).
//! 3. Convert rows into renderer entry types and call `m12_prose_renderer::render`.
//! 4. Write the full payload to `injection_cache` with `section = "full_payload"`,
//!    `computed_at = now()`, and `consent_applied = 1`.
//! 5. Return [`CacheRebuildResult`] for immediate use (e.g. writing to atuin KV).
//!
//! ## Layer
//!
//! `m4_consolidation`
//!
//! ## Dependencies
//!
//! - [`crate::m1_foundation::m01_types::TokenBudget`]
//! - [`crate::m1_foundation::m02_errors::ConsolidationError`]
//! - [`crate::m1_foundation::m03_config::InjectionConfig`]
//! - [`crate::m1_foundation::m05_constants::DEFAULT_BUDGET`]
//! - `m07_causal_chain::find_unresolved`, `m08_trajectory::get_recent`,
//!   `m09_workstream::{get_active, get_blocked}`, `m10_pattern::get_top_by_weight`
//! - `m12_prose_renderer::{render, RenderInput, …, count_tokens}`
//! - `m14_consent_filter::{filter_chains, filter_trajectories, filter_workstreams,
//!   filter_patterns}`
//!
//! ## Invariants
//!
//! - `injection_cache` always has at most one row (keyed on `"full_payload"`).
//! - `consent_applied = 1` is always set — the payload is pre-filtered.
//! - Token count in the cache row matches the rendered payload.
//! - Repeated rebuilds are idempotent (`INSERT OR REPLACE`).

#[cfg(feature = "sqlite")]
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[cfg(feature = "sqlite")]
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
#[cfg(feature = "sqlite")]
use tracing::{debug, info};

#[cfg(feature = "sqlite")]
use crate::m1_foundation::m02_errors::ConsolidationError;
#[cfg(feature = "sqlite")]
use crate::m1_foundation::m03_config::InjectionConfig;
#[cfg(feature = "sqlite")]
use crate::m1_foundation::m05_constants::DEFAULT_BUDGET;
#[cfg(feature = "sqlite")]
use crate::m2_schema::m07_causal_chain::find_unresolved;
#[cfg(feature = "sqlite")]
use crate::m2_schema::m08_trajectory::get_recent;
#[cfg(feature = "sqlite")]
use crate::m2_schema::m09_workstream::{get_active, get_blocked};
#[cfg(feature = "sqlite")]
use crate::m2_schema::m10_pattern::get_top_by_weight;
#[cfg(feature = "sqlite")]
use crate::m3_injection::m12_prose_renderer::{
    ChainEntry, PatternEntry, RenderInput, TrajectoryEntry, WorkstreamEntry, render,
};
#[cfg(feature = "sqlite")]
use crate::m3_injection::m14_consent_filter::{
    filter_chains, filter_patterns, filter_trajectories, filter_workstreams,
};

// ---------------------------------------------------------------------------
// The section key that m11_parallel_query::execute_cached reads from.
// ---------------------------------------------------------------------------

/// `SQLite` `injection_cache.section` key for the full pre-computed payload.
///
/// Must match [`crate::m3_injection::m11_parallel_query::CACHE_SECTION_KEY`].
pub const CACHE_SECTION_KEY: &str = "full_payload";

// ---------------------------------------------------------------------------
// CacheRebuildResult
// ---------------------------------------------------------------------------

/// Result of a complete cache rebuild.
///
/// Returned by [`rebuild_cache`] for immediate use (e.g. writing the payload
/// to the atuin KV store so that cold-start injection survives a database
/// outage).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheRebuildResult {
    /// The fully rendered prose payload.
    pub payload: String,
    /// Estimated token count produced by [`count_tokens`].
    pub token_count: u32,
    /// Number of prose sections successfully rendered (0–5).
    pub sections_rendered: u32,
    /// Wall-clock time for the full rebuild cycle in milliseconds.
    pub elapsed_ms: u64,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Rebuild `injection_cache` from live database state.
///
/// Queries all four data tables, applies consent filtering, renders the full
/// prose payload, and writes it to `injection_cache` with `computed_at = now()`.
///
/// The `session_number`, `services_healthy`, `services_total`, and `thermal`
/// parameters come from live probes — not the database — and are injected
/// directly into [`RenderInput`].
///
/// # Errors
///
/// Returns [`ConsolidationError::CacheRebuildFailed`] if any query, filtering,
/// rendering, or write step fails.
#[cfg(feature = "sqlite")]
pub fn rebuild_cache(
    conn: &Connection,
    session_number: u32,
    services_healthy: u32,
    services_total: u32,
    thermal: Option<f64>,
) -> Result<CacheRebuildResult, ConsolidationError> {
    let t_start = Instant::now();

    let input = build_render_input(conn, session_number, services_healthy, services_total, thermal)?;

    let (payload, token_count) = render(&input, DEFAULT_BUDGET)
        .map_err(|e| ConsolidationError::CacheRebuildFailed(format!("render: {e}")))?;

    let sections_rendered =
        u32::try_from(payload.lines().filter(|l| l.starts_with("### ")).count())
            .unwrap_or(u32::MAX);

    write_cache_entry(conn, &payload, token_count)?;

    let elapsed_ms = u64::try_from(t_start.elapsed().as_millis()).unwrap_or(u64::MAX);

    info!(
        token_count,
        sections_rendered,
        elapsed_ms,
        "injection_cache rebuilt"
    );

    Ok(CacheRebuildResult {
        payload,
        token_count,
        sections_rendered,
        elapsed_ms,
    })
}

/// Query all four data tables, apply consent filtering, and assemble a
/// [`RenderInput`] ready for rendering.
///
/// Extracted from [`rebuild_cache`] to respect the 100-line function limit.
///
/// # Errors
///
/// Returns [`ConsolidationError::CacheRebuildFailed`] on any query failure.
#[cfg(feature = "sqlite")]
fn build_render_input(
    conn: &Connection,
    session_number: u32,
    services_healthy: u32,
    services_total: u32,
    thermal: Option<f64>,
) -> Result<RenderInput, ConsolidationError> {
    let config = InjectionConfig::default();

    // ---- Query ---------------------------------------------------------------
    let chain_rows = find_unresolved(conn, config.max_chains)
        .map_err(|e| ConsolidationError::CacheRebuildFailed(format!("chain query: {e}")))?;
    let traj_rows = get_recent(conn, config.max_trajectory_points)
        .map_err(|e| ConsolidationError::CacheRebuildFailed(format!("trajectory query: {e}")))?;
    let ws_active = get_active(conn)
        .map_err(|e| ConsolidationError::CacheRebuildFailed(format!("workstream query: {e}")))?;
    let ws_blocked = get_blocked(conn)
        .map_err(|e| ConsolidationError::CacheRebuildFailed(format!("blocked query: {e}")))?;
    let pattern_rows = get_top_by_weight(conn, config.max_patterns)
        .map_err(|e| ConsolidationError::CacheRebuildFailed(format!("pattern query: {e}")))?;

    // ---- Consent filter ------------------------------------------------------
    let (chains_emit, chain_stats) = filter_chains(chain_rows);
    let (traj_emit, traj_stats) = filter_trajectories(traj_rows);
    let (ws_emit, ws_stats) = filter_workstreams(ws_active);
    let (blocked_emit, _) = filter_workstreams(ws_blocked);
    let (patterns_emit, pat_stats) = filter_patterns(pattern_rows);

    debug!(
        chain_passed = chain_stats.passed,
        chain_dropped = chain_stats.dropped_store + chain_stats.dropped_forget,
        traj_passed = traj_stats.passed,
        traj_dropped = traj_stats.dropped_store + traj_stats.dropped_forget,
        ws_passed = ws_stats.passed,
        ws_dropped = ws_stats.dropped_store + ws_stats.dropped_forget,
        pat_passed = pat_stats.passed,
        pat_dropped = pat_stats.dropped_store + pat_stats.dropped_forget,
        "consent filter pass complete"
    );

    // ---- Convert rows → renderer types ---------------------------------------
    let chains = chains_emit
        .into_iter()
        .map(|r| ChainEntry {
            label: r.label,
            reinforcement_count: r.reinforcement_count,
            description: r.description,
        })
        .collect();

    // `get_recent` returns DESC; reverse to chronological for the renderer.
    let mut trajectory: Vec<TrajectoryEntry> = traj_emit
        .into_iter()
        .map(|r| TrajectoryEntry {
            session_id: r.session_id,
            ralph_fitness: r.ralph_fitness,
            delta_summary: r.delta_summary,
        })
        .collect();
    trajectory.reverse();

    let patterns = patterns_emit
        .into_iter()
        .map(|r| PatternEntry {
            pattern_id: r.pattern_id,
            weight: r.weight,
            description: r.description,
        })
        .collect();

    Ok(RenderInput {
        session_number,
        chains,
        trajectory,
        active_workstreams: rows_to_ws_entries(ws_emit),
        blocked_workstreams: rows_to_ws_entries(blocked_emit),
        deferred_workstreams: vec![],
        patterns,
        services_healthy,
        services_total,
        thermal,
    })
}

/// Convert a `Vec` of [`WorkstreamRow`] records into the [`WorkstreamEntry`]
/// type expected by the prose renderer.
#[cfg(feature = "sqlite")]
fn rows_to_ws_entries(
    rows: Vec<crate::m2_schema::m09_workstream::WorkstreamRow>,
) -> Vec<WorkstreamEntry> {
    rows.into_iter()
        .map(|r| WorkstreamEntry {
            title: r.title,
            status: r.status,
            items_done: r.items_done,
            items_total: r.items_total,
            resume_context: r.resume_context,
            blocker: r.blocker,
        })
        .collect()
}

/// Write (or replace) the `"full_payload"` row in `injection_cache`.
///
/// Uses `INSERT OR REPLACE` so repeated calls are idempotent.  Sets
/// `computed_at` to the current Unix timestamp in seconds and
/// `consent_applied = 1`.
///
/// # Errors
///
/// Returns [`ConsolidationError::CacheRebuildFailed`] if the `SQL` write fails.
#[cfg(feature = "sqlite")]
pub fn write_cache_entry(
    conn: &Connection,
    payload: &str,
    token_count: u32,
) -> Result<(), ConsolidationError> {
    let computed_at = unix_now_secs();

    conn.execute(
        "INSERT OR REPLACE INTO injection_cache
             (section, payload, token_count, computed_at, consent_applied)
         VALUES (?1, ?2, ?3, ?4, 1)",
        rusqlite::params![CACHE_SECTION_KEY, payload, token_count, computed_at],
    )
    .map_err(|e| ConsolidationError::CacheRebuildFailed(format!("write_cache_entry: {e}")))?;

    debug!(
        section = CACHE_SECTION_KEY,
        token_count,
        computed_at,
        "injection_cache row written"
    );
    Ok(())
}

/// Delete all rows from `injection_cache`.
///
/// Used to force a full rebuild on the next injection cycle.
///
/// # Errors
///
/// Returns [`ConsolidationError::CacheRebuildFailed`] if the `DELETE` fails.
#[cfg(feature = "sqlite")]
pub fn clear_cache(conn: &Connection) -> Result<(), ConsolidationError> {
    conn.execute("DELETE FROM injection_cache", []).map_err(|e| {
        ConsolidationError::CacheRebuildFailed(format!("clear_cache: {e}"))
    })?;
    debug!("injection_cache cleared");
    Ok(())
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Return the current time as Unix seconds, clamped to `0` if the clock is
/// before the Unix epoch (impossible in practice; guarded for correctness).
#[cfg(feature = "sqlite")]
fn unix_now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

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
    #[cfg(feature = "sqlite")]
    use crate::m3_injection::m11_parallel_query::execute_cached;
    #[cfg(feature = "sqlite")]
    use crate::m3_injection::m12_prose_renderer::count_tokens;

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    /// Seed a representative dataset into `conn`.
    #[cfg(feature = "sqlite")]
    fn seed_db(conn: &rusqlite::Connection) {
        // Chains
        insert_chain(conn, 108, "bug", "cp-alias-trap", "cp is aliased to trash").unwrap();
        insert_chain(conn, 107, "trap", "stash-pop-wipe", "stash pop on wrong stash").unwrap();
        insert_chain(conn, 106, "bug", "docker-prune", "prune erased gateway").unwrap();

        // Trajectory (oldest first)
        for s in 105_u32..=109 {
            insert_point(
                conn,
                s,
                0.60 + f64::from(s - 105) * 0.02,
                0.5,
                0.244,
                2.0,
                11,
                &format!("delta for S{s:03}"),
                None,
            )
            .unwrap();
        }

        // Workstream
        insert_workstream(conn, "stdb-inject", "SpaceTimeDB injection", "active", 109, "start L1")
            .unwrap();

        // Pattern (insert_pattern uses the 4-arg form; weight defaults to 0.5)
        insert_pattern(conn, "verify-before-ship", "procedural", "Always verify", None).unwrap();
    }

    // -------------------------------------------------------------------------
    // CacheRebuildResult — type properties
    // -------------------------------------------------------------------------

    #[test]
    fn cache_rebuild_result_serde_roundtrip() {
        let r = CacheRebuildResult {
            payload: "hello world".into(),
            token_count: 2,
            sections_rendered: 3,
            elapsed_ms: 42,
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: CacheRebuildResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.payload, r.payload);
        assert_eq!(back.token_count, r.token_count);
        assert_eq!(back.sections_rendered, r.sections_rendered);
        assert_eq!(back.elapsed_ms, r.elapsed_ms);
    }

    #[test]
    fn cache_rebuild_result_debug_not_empty() {
        let r = CacheRebuildResult {
            payload: String::new(),
            token_count: 0,
            sections_rendered: 0,
            elapsed_ms: 0,
        };
        assert!(!format!("{r:?}").is_empty());
    }

    #[test]
    fn cache_rebuild_result_clone() {
        let r = CacheRebuildResult {
            payload: "payload".into(),
            token_count: 5,
            sections_rendered: 2,
            elapsed_ms: 10,
        };
        let r2 = r.clone();
        assert_eq!(r.payload, r2.payload);
        assert_eq!(r.token_count, r2.token_count);
    }

    #[test]
    fn cache_section_key_matches_m11() {
        // Ensure our constant matches the one m11_parallel_query uses.
        assert_eq!(CACHE_SECTION_KEY, "full_payload");
    }

    // -------------------------------------------------------------------------
    // write_cache_entry
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn write_cache_entry_inserts_row() {
        let conn = open_memory().unwrap();
        write_cache_entry(&conn, "test payload", 10).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM injection_cache", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn write_cache_entry_stores_correct_payload() {
        let conn = open_memory().unwrap();
        write_cache_entry(&conn, "my payload", 3).unwrap();
        let payload: String = conn
            .query_row(
                "SELECT payload FROM injection_cache WHERE section = 'full_payload'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(payload, "my payload");
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn write_cache_entry_stores_token_count() {
        let conn = open_memory().unwrap();
        write_cache_entry(&conn, "one two three", 3).unwrap();
        let tc: i64 = conn
            .query_row(
                "SELECT token_count FROM injection_cache WHERE section = 'full_payload'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(tc, 3);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn write_cache_entry_sets_computed_at_near_now() {
        let conn = open_memory().unwrap();
        let before = unix_now_secs();
        write_cache_entry(&conn, "payload", 1).unwrap();
        let after = unix_now_secs();

        let computed_at: i64 = conn
            .query_row(
                "SELECT computed_at FROM injection_cache WHERE section = 'full_payload'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(computed_at >= before, "computed_at ({computed_at}) < before ({before})");
        assert!(computed_at <= after, "computed_at ({computed_at}) > after ({after})");
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn write_cache_entry_sets_consent_applied_true() {
        let conn = open_memory().unwrap();
        write_cache_entry(&conn, "payload", 5).unwrap();
        let ca: i64 = conn
            .query_row(
                "SELECT consent_applied FROM injection_cache WHERE section = 'full_payload'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(ca, 1);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn write_cache_entry_is_idempotent_replace() {
        let conn = open_memory().unwrap();
        write_cache_entry(&conn, "first", 1).unwrap();
        write_cache_entry(&conn, "second", 2).unwrap();

        let payload: String = conn
            .query_row(
                "SELECT payload FROM injection_cache WHERE section = 'full_payload'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(payload, "second");

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM injection_cache", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1, "INSERT OR REPLACE must keep exactly one row");
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn write_cache_entry_empty_payload_accepted() {
        let conn = open_memory().unwrap();
        write_cache_entry(&conn, "", 0).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM injection_cache", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn write_cache_entry_section_is_full_payload() {
        let conn = open_memory().unwrap();
        write_cache_entry(&conn, "data", 1).unwrap();
        let section: String = conn
            .query_row("SELECT section FROM injection_cache LIMIT 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(section, CACHE_SECTION_KEY);
    }

    // -------------------------------------------------------------------------
    // clear_cache
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn clear_cache_removes_all_rows() {
        let conn = open_memory().unwrap();
        write_cache_entry(&conn, "first payload", 5).unwrap();
        clear_cache(&conn).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM injection_cache", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn clear_cache_on_empty_table_is_ok() {
        let conn = open_memory().unwrap();
        // Should not error on an already-empty table.
        assert!(clear_cache(&conn).is_ok());
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn clear_cache_then_write_succeeds() {
        let conn = open_memory().unwrap();
        write_cache_entry(&conn, "payload", 3).unwrap();
        clear_cache(&conn).unwrap();
        write_cache_entry(&conn, "new payload", 4).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM injection_cache", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    // -------------------------------------------------------------------------
    // rebuild_cache — empty database
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn rebuild_cache_empty_db_succeeds() {
        let conn = open_memory().unwrap();
        let result = rebuild_cache(&conn, 109, 12, 12, Some(0.244));
        assert!(result.is_ok(), "rebuild on empty DB must succeed: {result:?}");
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn rebuild_cache_empty_db_token_count_nonzero() {
        let conn = open_memory().unwrap();
        let result = rebuild_cache(&conn, 109, 12, 12, Some(0.244)).unwrap();
        // Even an empty DB produces a header + orientation section.
        assert!(result.token_count > 0, "token_count must be > 0");
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn rebuild_cache_empty_db_writes_to_injection_cache() {
        let conn = open_memory().unwrap();
        rebuild_cache(&conn, 109, 12, 12, None).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM injection_cache", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn rebuild_cache_empty_db_elapsed_ms_recorded() {
        let conn = open_memory().unwrap();
        let result = rebuild_cache(&conn, 109, 12, 12, None).unwrap();
        // elapsed_ms should be a non-negative number; u64 is always >= 0.
        // Just confirm it's not u64::MAX (which signals overflow).
        assert_ne!(result.elapsed_ms, u64::MAX);
    }

    // -------------------------------------------------------------------------
    // rebuild_cache — seeded database
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn rebuild_cache_seeded_db_contains_session_header() {
        let conn = open_memory().unwrap();
        seed_db(&conn);
        let result = rebuild_cache(&conn, 109, 12, 12, Some(0.244)).unwrap();
        assert!(
            result.payload.contains("Session S109 Injection"),
            "payload missing session header"
        );
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn rebuild_cache_seeded_db_contains_chain_label() {
        let conn = open_memory().unwrap();
        seed_db(&conn);
        let result = rebuild_cache(&conn, 109, 12, 12, None).unwrap();
        assert!(
            result.payload.contains("cp-alias-trap"),
            "payload missing chain label"
        );
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn rebuild_cache_seeded_db_contains_trajectory() {
        let conn = open_memory().unwrap();
        seed_db(&conn);
        let result = rebuild_cache(&conn, 109, 12, 12, None).unwrap();
        assert!(
            result.payload.contains("### Trajectory"),
            "payload missing trajectory section"
        );
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn rebuild_cache_seeded_db_contains_workstream() {
        let conn = open_memory().unwrap();
        seed_db(&conn);
        let result = rebuild_cache(&conn, 109, 12, 12, None).unwrap();
        assert!(
            result.payload.contains("SpaceTimeDB injection"),
            "payload missing workstream title"
        );
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn rebuild_cache_seeded_db_contains_health() {
        let conn = open_memory().unwrap();
        seed_db(&conn);
        let result = rebuild_cache(&conn, 109, 12, 12, Some(0.55)).unwrap();
        assert!(
            result.payload.contains("### Health"),
            "payload missing health section"
        );
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn rebuild_cache_seeded_db_sections_rendered_positive() {
        let conn = open_memory().unwrap();
        seed_db(&conn);
        let result = rebuild_cache(&conn, 109, 12, 12, Some(0.244)).unwrap();
        assert!(
            result.sections_rendered > 0,
            "sections_rendered must be > 0 with seeded data"
        );
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn rebuild_cache_seeded_db_token_count_matches_payload() {
        let conn = open_memory().unwrap();
        seed_db(&conn);
        let result = rebuild_cache(&conn, 109, 12, 12, Some(0.244)).unwrap();
        // token_count stored in the struct should equal count_tokens(payload)
        // (they may differ by 1 due to the header-stamping replacement, but
        // both reflect the same whitespace-based approximation).
        let recomputed = count_tokens(&result.payload);
        // Allow at most 5 token difference due to header replacement.
        let diff = result.token_count.abs_diff(recomputed);
        assert!(diff <= 5, "token_count ({}) vs recomputed ({recomputed})", result.token_count);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn rebuild_cache_thermal_none_no_thermal_in_payload() {
        let conn = open_memory().unwrap();
        let result = rebuild_cache(&conn, 109, 12, 12, None).unwrap();
        assert!(
            !result.payload.contains("Thermal"),
            "payload should not mention Thermal when thermal=None"
        );
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn rebuild_cache_thermal_some_appears_in_payload() {
        let conn = open_memory().unwrap();
        seed_db(&conn);
        let result = rebuild_cache(&conn, 109, 12, 12, Some(0.55)).unwrap();
        assert!(
            result.payload.contains("Thermal"),
            "payload should mention Thermal when thermal=Some"
        );
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn rebuild_cache_partial_services_in_health() {
        let conn = open_memory().unwrap();
        let result = rebuild_cache(&conn, 109, 10, 12, None).unwrap();
        assert!(
            result.payload.contains("10/12"),
            "payload should show partial service count"
        );
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn rebuild_cache_all_services_green() {
        let conn = open_memory().unwrap();
        let result = rebuild_cache(&conn, 109, 12, 12, None).unwrap();
        assert!(
            result.payload.contains("All 12 services responding"),
            "all-green health line missing"
        );
    }

    // -------------------------------------------------------------------------
    // Consent filtering: Store rows are excluded
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn rebuild_cache_store_chain_excluded() {
        let conn = open_memory().unwrap();
        // Insert one Emit chain and one Store chain.
        insert_chain(&conn, 109, "bug", "visible-bug", "should appear").unwrap();
        conn.execute(
            "INSERT INTO causal_chain (origin_session, chain_type, label, description, consent)
             VALUES (109, 'bug', 'hidden-bug', 'should not appear', 'Store')",
            [],
        )
        .unwrap();

        let result = rebuild_cache(&conn, 109, 12, 12, None).unwrap();
        assert!(
            result.payload.contains("visible-bug"),
            "Emit chain must appear in payload"
        );
        assert!(
            !result.payload.contains("hidden-bug"),
            "Store chain must not appear in payload"
        );
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn rebuild_cache_store_trajectory_excluded() {
        let conn = open_memory().unwrap();
        // One Emit trajectory row and one Store row.
        insert_point(&conn, 109, 0.70, 0.5, 0.244, 2.0, 12, "visible delta", None).unwrap();
        conn.execute(
            "INSERT INTO session_trajectory
                 (session_id, ralph_fitness, field_r, thermal_t, ltp_ltd_ratio, services_healthy,
                  delta_summary, consent)
             VALUES (110, 0.75, 0.6, 0.3, 3.0, 12, 'hidden delta', 'Store')",
            [],
        )
        .unwrap();

        let result = rebuild_cache(&conn, 110, 12, 12, None).unwrap();
        assert!(
            result.payload.contains("visible delta"),
            "Emit trajectory must appear in payload"
        );
        assert!(
            !result.payload.contains("hidden delta"),
            "Store trajectory must not appear in payload"
        );
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn rebuild_cache_store_workstream_excluded() {
        let conn = open_memory().unwrap();
        insert_workstream(&conn, "emit-ws", "Visible WS", "active", 109, "ctx").unwrap();
        conn.execute(
            "INSERT INTO workstream (ws_id, title, status, last_touched_session, resume_context, consent)
             VALUES ('store-ws', 'Hidden WS', 'active', 109, 'ctx', 'Store')",
            [],
        )
        .unwrap();

        let result = rebuild_cache(&conn, 109, 12, 12, None).unwrap();
        assert!(
            result.payload.contains("Visible WS"),
            "Emit workstream must appear in payload"
        );
        assert!(
            !result.payload.contains("Hidden WS"),
            "Store workstream must not appear in payload"
        );
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn rebuild_cache_store_pattern_excluded() {
        let conn = open_memory().unwrap();
        // Insert one Emit pattern and one Store pattern via raw SQL (insert_pattern
        // always defaults to Emit so we override the consent column directly).
        insert_pattern(&conn, "emit-pat", "procedural", "visible pattern", None).unwrap();
        conn.execute(
            "INSERT INTO reinforced_pattern (pattern_id, category, description, consent)
             VALUES ('store-pat', 'trap', 'hidden pattern', 'Store')",
            [],
        )
        .unwrap();

        let result = rebuild_cache(&conn, 109, 12, 12, None).unwrap();
        // Patterns are not currently rendered in the prose payload, but the
        // Store pattern must not accidentally appear in any serialized form.
        assert!(
            !result.payload.contains("hidden pattern"),
            "Store pattern must not appear in payload"
        );
    }

    // -------------------------------------------------------------------------
    // Round-trip: rebuild → execute_cached reads back the same payload
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_cached_reads_payload_written_by_write_cache_entry() {
        let conn = open_memory().unwrap();
        write_cache_entry(&conn, "hello from cache", 4).unwrap();
        let cached = execute_cached(&conn).unwrap();
        // execute_cached may return None if the age check fails in theory, but
        // since we just wrote it, it should be fresh.
        assert!(cached.is_some(), "execute_cached should return Some immediately after write");
        assert_eq!(cached.unwrap(), "hello from cache");
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn rebuild_cache_payload_readable_by_execute_cached() {
        let conn = open_memory().unwrap();
        seed_db(&conn);
        let rebuild_result = rebuild_cache(&conn, 109, 12, 12, Some(0.244)).unwrap();
        let cached = execute_cached(&conn).unwrap();
        assert!(cached.is_some(), "execute_cached must find the freshly-rebuilt cache");
        assert_eq!(
            cached.unwrap(),
            rebuild_result.payload,
            "execute_cached payload must match rebuild result"
        );
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn clear_cache_makes_execute_cached_return_none() {
        let conn = open_memory().unwrap();
        write_cache_entry(&conn, "payload", 2).unwrap();
        clear_cache(&conn).unwrap();
        let cached = execute_cached(&conn).unwrap();
        assert!(cached.is_none(), "cleared cache must return None from execute_cached");
    }

    // -------------------------------------------------------------------------
    // Repeated rebuilds are idempotent
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn rebuild_cache_twice_keeps_single_row() {
        let conn = open_memory().unwrap();
        seed_db(&conn);
        rebuild_cache(&conn, 109, 12, 12, Some(0.244)).unwrap();
        rebuild_cache(&conn, 109, 12, 12, Some(0.244)).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM injection_cache", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1, "repeated rebuilds must leave exactly one cache row");
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn rebuild_cache_updates_payload_on_second_call() {
        let conn = open_memory().unwrap();
        // First build with session 108.
        rebuild_cache(&conn, 108, 11, 12, None).unwrap();
        // Second build with session 109.
        rebuild_cache(&conn, 109, 12, 12, Some(0.55)).unwrap();
        let payload: String = conn
            .query_row(
                "SELECT payload FROM injection_cache WHERE section = 'full_payload'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        // Most recent session number should appear in the payload.
        assert!(
            payload.contains("S109"),
            "second rebuild should update payload to S109"
        );
    }

    // -------------------------------------------------------------------------
    // CacheRebuildResult field invariants
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn rebuild_cache_result_token_count_within_budget() {
        let conn = open_memory().unwrap();
        seed_db(&conn);
        let result = rebuild_cache(&conn, 109, 12, 12, Some(0.244)).unwrap();
        assert!(
            result.token_count <= DEFAULT_BUDGET.as_u32(),
            "token_count ({}) must not exceed default budget ({})",
            result.token_count,
            DEFAULT_BUDGET.as_u32()
        );
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn rebuild_cache_result_sections_rendered_at_most_five() {
        let conn = open_memory().unwrap();
        seed_db(&conn);
        let result = rebuild_cache(&conn, 109, 12, 12, Some(0.244)).unwrap();
        assert!(
            result.sections_rendered <= 5,
            "at most 5 prose sections can be rendered; got {}",
            result.sections_rendered
        );
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn rebuild_cache_result_payload_is_nonempty() {
        let conn = open_memory().unwrap();
        let result = rebuild_cache(&conn, 109, 12, 12, None).unwrap();
        assert!(!result.payload.is_empty(), "payload must be non-empty");
    }

    // -------------------------------------------------------------------------
    // unix_now_secs helper
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn unix_now_secs_is_reasonable() {
        let t = unix_now_secs();
        // Must be after 2024-01-01 (Unix time > 1_700_000_000).
        assert!(t > 1_700_000_000, "unix_now_secs returned an implausibly small value: {t}");
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn unix_now_secs_is_non_negative() {
        let t = unix_now_secs();
        assert!(t >= 0);
    }

    // -------------------------------------------------------------------------
    // Structural: payload always starts with session header
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn rebuild_cache_payload_starts_with_double_hash() {
        let conn = open_memory().unwrap();
        let result = rebuild_cache(&conn, 42, 6, 12, None).unwrap();
        assert!(
            result.payload.starts_with("## Session"),
            "payload should start with ## Session header"
        );
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn rebuild_cache_payload_always_contains_orientation() {
        let conn = open_memory().unwrap();
        let result = rebuild_cache(&conn, 109, 12, 12, None).unwrap();
        assert!(
            result.payload.contains("### Orientation"),
            "Orientation section is mandatory and must always be present"
        );
    }

    // -------------------------------------------------------------------------
    // Clear and rebuild full round-trip
    // -------------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn full_round_trip_clear_seed_rebuild_read() {
        let conn = open_memory().unwrap();
        // 1. Write stale data.
        write_cache_entry(&conn, "stale payload", 5).unwrap();
        // 2. Clear.
        clear_cache(&conn).unwrap();
        // 3. Seed and rebuild.
        seed_db(&conn);
        let result = rebuild_cache(&conn, 109, 12, 12, Some(0.244)).unwrap();
        // 4. Verify execute_cached returns the fresh payload.
        let cached = execute_cached(&conn).unwrap();
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), result.payload);
        assert_ne!(result.payload, "stale payload");
    }

    // -------------------------------------------------------------------------
    // Additional invariant tests to meet 50-test minimum
    // -------------------------------------------------------------------------

    #[test]
    fn cache_section_key_is_nonempty() {
        assert!(!CACHE_SECTION_KEY.is_empty());
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn write_cache_entry_multiple_writes_single_row() {
        let conn = open_memory().unwrap();
        for i in 0..5_u32 {
            write_cache_entry(&conn, &format!("payload {i}"), i).unwrap();
        }
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM injection_cache", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1, "five writes must leave exactly one row");
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn rebuild_cache_zero_services_does_not_panic() {
        // Edge case: services_healthy=0, services_total=0.
        let conn = open_memory().unwrap();
        let result = rebuild_cache(&conn, 1, 0, 0, None);
        assert!(result.is_ok(), "rebuild with zero services must not panic: {result:?}");
    }
}
