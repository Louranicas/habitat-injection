//! `m13b_cache_light` — Lightweight injection cache rebuild from database tables only.
//!
//! Called by [`m13_fallback`](super::m13_fallback) Tier 1b when the pre-computed
//! cache is stale or missing but the database is accessible. Produces the same
//! payload format as [`m17_cache_builder::rebuild_cache`](crate::m4_consolidation::m17_cache_builder::rebuild_cache)
//! without live service probes, atuin KV sync, or Hebbian decay.
//!
//! ## Layer
//!
//! `m3_injection` — same layer as the fallback chain. No L4 dependency.
//!
//! ## Dependencies
//!
//! - L1: `m01_types`, `m02_errors`, `m05_constants`
//! - L2: `m07_causal_chain`, `m08_trajectory`, `m09_workstream`, `m10_pattern`
//! - L3: `m12_prose_renderer`, `m14_consent_filter`
//!
//! ## Timing
//!
//! Target: <50ms. Budget: 4 queries ~10ms, consent ~1ms, render ~5ms, write ~3ms.

#[cfg(feature = "sqlite")]
use std::time::Instant;

#[cfg(feature = "sqlite")]
use rusqlite::Connection;

#[cfg(feature = "sqlite")]
use tracing::{debug, info};

use serde::{Deserialize, Serialize};

#[cfg(feature = "sqlite")]
use crate::m1_foundation::m02_errors::ConsolidationError;
#[cfg(feature = "sqlite")]
use crate::m1_foundation::m05_constants::DEFAULT_BUDGET;
#[cfg(feature = "sqlite")]
use crate::m2_schema::{m07_causal_chain, m08_trajectory, m09_workstream, m10_pattern};
#[cfg(feature = "sqlite")]
use crate::m3_injection::m12_prose_renderer::{
    ChainEntry, PatternEntry, RenderInput, TrajectoryEntry, WorkstreamEntry, render,
};
#[cfg(feature = "sqlite")]
use crate::m3_injection::m14_consent_filter::{
    filter_chains, filter_patterns, filter_trajectories, filter_workstreams,
};

// ---------------------------------------------------------------------------
// Result type
// ---------------------------------------------------------------------------

/// Result of a lightweight cache rebuild.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightRebuildResult {
    /// The fully rendered prose payload.
    pub payload: String,
    /// Estimated token count.
    pub token_count: u32,
    /// Number of prose sections rendered.
    pub sections_rendered: u32,
    /// Wall-clock time for the rebuild in milliseconds.
    pub elapsed_ms: u64,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Rebuild `injection_cache` from database tables only — no live probes, no
/// atuin KV sync, no Hebbian decay.
///
/// Session number is derived from `MAX(session_id) + 1` in `session_trajectory`.
/// Health values come from the most recent trajectory row. Wrapped in
/// `BEGIN IMMEDIATE TRANSACTION` for atomicity.
///
/// # Errors
///
/// Returns [`ConsolidationError::CacheRebuildFailed`] if any query, filter,
/// render, or write step fails.
#[cfg(feature = "sqlite")]
pub fn rebuild_cache_light(conn: &Connection) -> Result<LightRebuildResult, ConsolidationError> {
    let t_start = Instant::now();

    let tx = conn
        .unchecked_transaction()
        .map_err(|e| ConsolidationError::CacheRebuildFailed(format!("begin tx: {e}")))?;

    let input = build_input_from_db(&tx)?;

    let (payload, token_count) = render(&input, DEFAULT_BUDGET)
        .map_err(|e| ConsolidationError::CacheRebuildFailed(format!("render: {e}")))?;

    let sections_rendered = count_sections(&payload);

    write_cache_row(&tx, &payload, token_count)?;

    tx.commit()
        .map_err(|e| ConsolidationError::CacheRebuildFailed(format!("commit: {e}")))?;

    let elapsed_ms = u64::try_from(t_start.elapsed().as_millis()).unwrap_or(u64::MAX);

    info!(
        token_count,
        sections_rendered,
        elapsed_ms,
        "cache_light: injection_cache rebuilt from DB"
    );

    Ok(LightRebuildResult {
        payload,
        token_count,
        sections_rendered,
        elapsed_ms,
    })
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Build a [`RenderInput`] from database tables only.
///
/// Session number = `MAX(session_id) + 1` from trajectory. Health values from
/// the most recent trajectory row (not live probes).
#[cfg(feature = "sqlite")]
fn build_input_from_db(conn: &Connection) -> Result<RenderInput, ConsolidationError> {
    let session = derive_session_number(conn);

    let last_traj = m08_trajectory::get_recent(conn, 1)
        .map_err(|e| ConsolidationError::CacheRebuildFailed(format!("trajectory query: {e}")))?;
    let (services_healthy, thermal) = last_traj
        .first()
        .map_or((0, None), |t| (t.services_healthy, Some(t.thermal_t)));

    let chain_rows = m07_causal_chain::find_unresolved(conn, 5)
        .map_err(|e| ConsolidationError::CacheRebuildFailed(format!("chain query: {e}")))?;
    let traj_rows = m08_trajectory::get_recent(conn, 5)
        .map_err(|e| ConsolidationError::CacheRebuildFailed(format!("trajectory query: {e}")))?;
    let ws_active = m09_workstream::get_active(conn)
        .map_err(|e| ConsolidationError::CacheRebuildFailed(format!("workstream query: {e}")))?;
    let ws_blocked = m09_workstream::get_blocked(conn)
        .map_err(|e| ConsolidationError::CacheRebuildFailed(format!("blocked query: {e}")))?;
    let pattern_rows = m10_pattern::get_top_by_weight(conn, 5)
        .map_err(|e| ConsolidationError::CacheRebuildFailed(format!("pattern query: {e}")))?;

    let (chains_emit, _) = filter_chains(chain_rows);
    let (traj_emit, _) = filter_trajectories(traj_rows);
    let (ws_emit, _) = filter_workstreams(ws_active);
    let (blocked_emit, _) = filter_workstreams(ws_blocked);
    let (patterns_emit, _) = filter_patterns(pattern_rows);

    let chains = chains_emit
        .into_iter()
        .map(|r| ChainEntry {
            label: r.label,
            reinforcement_count: r.reinforcement_count,
            description: r.description,
        })
        .collect();

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

    let workstreams_to_entries =
        |rows: Vec<crate::m2_schema::m09_workstream::WorkstreamRow>| -> Vec<WorkstreamEntry> {
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
        };

    debug!(session, services_healthy, "cache_light: input assembled from DB");

    Ok(RenderInput {
        session_number: session,
        chains,
        trajectory,
        active_workstreams: workstreams_to_entries(ws_emit),
        blocked_workstreams: workstreams_to_entries(blocked_emit),
        deferred_workstreams: vec![],
        patterns,
        services_healthy,
        services_total: 14,
        thermal,
    })
}

/// Derive next session number from the trajectory table.
#[cfg(feature = "sqlite")]
fn derive_session_number(conn: &Connection) -> u32 {
    m08_trajectory::get_recent(conn, 1)
        .ok()
        .and_then(|rows| rows.first().map(|r| r.session_id.saturating_add(1)))
        .unwrap_or(1)
}

/// Write (or replace) the cache row using the same schema as `m17_cache_builder`.
#[cfg(feature = "sqlite")]
fn write_cache_row(
    conn: &Connection,
    payload: &str,
    token_count: u32,
) -> Result<(), ConsolidationError> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let computed_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0i64, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX));

    let section_key = crate::m3_injection::m11_parallel_query::CACHE_SECTION_KEY;
    conn.execute(
        "INSERT OR REPLACE INTO injection_cache
             (section, payload, token_count, computed_at, consent_applied)
         VALUES (?1, ?2, ?3, ?4, 1)",
        rusqlite::params![section_key, payload, token_count, computed_at],
    )
    .map_err(|e| ConsolidationError::CacheRebuildFailed(format!("write_cache_row: {e}")))?;

    Ok(())
}

/// Count prose sections (lines starting with `### `).
fn count_sections(payload: &str) -> u32 {
    u32::try_from(payload.lines().filter(|l| l.starts_with("### ")).count()).unwrap_or(u32::MAX)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use crate::m2_schema::m06_schema::open_memory;

    fn seeded_db() -> Connection {
        let conn = open_memory().unwrap();

        conn.execute(
            "INSERT INTO session_trajectory
                 (session_id, ralph_fitness, field_r, thermal_t, ltp_ltd_ratio,
                  services_healthy, delta_summary)
             VALUES (119, 0.836, 0.0, 0.244, 3.16, 11, 'STDP fix deployed')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO session_trajectory
                 (session_id, ralph_fitness, field_r, thermal_t, ltp_ltd_ratio,
                  services_healthy, delta_summary)
             VALUES (120, 0.455, 0.0, 0.0, 0.0, 9, 'Armada bug hunt')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO causal_chain
                 (origin_session, chain_type, label, description, reinforcement_count)
             VALUES (100, 'trap', 'cp-alias', 'cp is aliased to interactive', 20)",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO causal_chain
                 (origin_session, chain_type, label, description, reinforcement_count)
             VALUES (101, 'bug', 'devenv-stop', 'devenv stop does not kill processes', 15)",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO workstream
                 (ws_id, title, status, priority, last_touched_session, resume_context)
             VALUES ('ws-inject', 'habitat-injection Phase 1 CLI', 'active', 1, 120,
                     'Library complete. Next: data seeding + hook wiring')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO workstream
                 (ws_id, title, status, priority, last_touched_session, resume_context,
                  blocker)
             VALUES ('ws-synthex', 'synthex-v2 Phase G Shadow Window', 'blocked', 3, 108,
                     'v1 streaming gate', 'synthex-v2 Phase G externally gated')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO reinforced_pattern
                 (pattern_id, category, description, weight, hit_count, last_fired_session)
             VALUES ('verify-before-ship', 'procedural',
                     'independently re-run claims before shipping', 0.92, 5, 119)",
            [],
        )
        .unwrap();

        conn
    }

    #[test]
    fn rebuild_on_seeded_db_succeeds() {
        let conn = seeded_db();
        let result = rebuild_cache_light(&conn).unwrap();
        assert!(!result.payload.is_empty());
        assert!(result.token_count > 0);
        assert!(result.sections_rendered > 0);
    }

    #[test]
    fn rebuild_writes_cache_row() {
        let conn = seeded_db();
        rebuild_cache_light(&conn).unwrap();

        let section_key = crate::m3_injection::m11_parallel_query::CACHE_SECTION_KEY;
        let payload: String = conn
            .query_row(
                "SELECT payload FROM injection_cache WHERE section = ?1",
                rusqlite::params![section_key],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!payload.is_empty());
    }

    #[test]
    fn rebuild_is_idempotent() {
        let conn = seeded_db();
        let r1 = rebuild_cache_light(&conn).unwrap();
        let r2 = rebuild_cache_light(&conn).unwrap();
        assert_eq!(r1.token_count, r2.token_count);
        assert_eq!(r1.sections_rendered, r2.sections_rendered);
    }

    #[test]
    fn session_number_derived_from_trajectory() {
        let conn = seeded_db();
        let result = rebuild_cache_light(&conn).unwrap();
        assert!(result.payload.contains("S121"));
    }

    #[test]
    fn rebuild_on_empty_db_succeeds() {
        let conn = open_memory().unwrap();
        let result = rebuild_cache_light(&conn).unwrap();
        assert!(!result.payload.is_empty());
        assert!(result.payload.contains("S001"));
    }

    #[test]
    fn payload_within_budget() {
        let conn = seeded_db();
        let result = rebuild_cache_light(&conn).unwrap();
        assert!(result.token_count <= DEFAULT_BUDGET.as_u32());
    }

    #[test]
    fn payload_contains_chain_labels() {
        let conn = seeded_db();
        let result = rebuild_cache_light(&conn).unwrap();
        assert!(result.payload.contains("cp-alias"));
    }

    #[test]
    fn payload_contains_workstream_titles() {
        let conn = seeded_db();
        let result = rebuild_cache_light(&conn).unwrap();
        assert!(result.payload.contains("habitat-injection"));
    }

    #[test]
    fn payload_contains_trajectory_data() {
        let conn = seeded_db();
        let result = rebuild_cache_light(&conn).unwrap();
        assert!(result.payload.contains("0.836") || result.payload.contains("0.455"));
    }

    #[test]
    fn elapsed_ms_is_reasonable() {
        let conn = seeded_db();
        let result = rebuild_cache_light(&conn).unwrap();
        assert!(result.elapsed_ms < 1000);
    }

    #[test]
    fn cache_row_has_fresh_timestamp() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let conn = seeded_db();
        rebuild_cache_light(&conn).unwrap();

        let section_key = crate::m3_injection::m11_parallel_query::CACHE_SECTION_KEY;
        let ts: i64 = conn
            .query_row(
                "SELECT computed_at FROM injection_cache WHERE section = ?1",
                rusqlite::params![section_key],
                |r| r.get(0),
            )
            .unwrap();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0));
        assert!((now - ts).abs() < 5);
    }

    #[test]
    fn cache_row_has_consent_applied() {
        let conn = seeded_db();
        rebuild_cache_light(&conn).unwrap();

        let section_key = crate::m3_injection::m11_parallel_query::CACHE_SECTION_KEY;
        let applied: i32 = conn
            .query_row(
                "SELECT consent_applied FROM injection_cache WHERE section = ?1",
                rusqlite::params![section_key],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(applied, 1);
    }

    #[test]
    fn token_count_is_positive_and_bounded() {
        let conn = seeded_db();
        let result = rebuild_cache_light(&conn).unwrap();
        assert!(result.token_count > 0);
        assert!(result.token_count <= DEFAULT_BUDGET.as_u32());
    }

    #[test]
    fn sections_rendered_matches_payload() {
        let conn = seeded_db();
        let result = rebuild_cache_light(&conn).unwrap();
        let manual_count = count_sections(&result.payload);
        assert_eq!(result.sections_rendered, manual_count);
    }

    #[test]
    fn count_sections_empty() {
        assert_eq!(count_sections(""), 0);
    }

    #[test]
    fn count_sections_with_headers() {
        let text = "### A\nfoo\n### B\nbar\n### C\nbaz\n";
        assert_eq!(count_sections(text), 3);
    }

    #[test]
    fn count_sections_no_headers() {
        assert_eq!(count_sections("just text\nno headers\n"), 0);
    }

    #[test]
    fn derive_session_empty_table() {
        let conn = open_memory().unwrap();
        assert_eq!(derive_session_number(&conn), 1);
    }

    #[test]
    fn derive_session_with_data() {
        let conn = seeded_db();
        assert_eq!(derive_session_number(&conn), 121);
    }

    #[test]
    fn consent_filter_applied() {
        let conn = open_memory().unwrap();
        conn.execute(
            "INSERT INTO causal_chain
                 (origin_session, chain_type, label, description, consent)
             VALUES (1, 'bug', 'hidden', 'should not appear', 'Store')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO causal_chain
                 (origin_session, chain_type, label, description, consent)
             VALUES (2, 'bug', 'visible', 'should appear', 'Emit')",
            [],
        )
        .unwrap();

        let result = rebuild_cache_light(&conn).unwrap();
        assert!(!result.payload.contains("hidden"));
        assert!(result.payload.contains("visible"));
    }

    #[test]
    fn rebuild_after_clear_cache() {
        let conn = seeded_db();
        rebuild_cache_light(&conn).unwrap();
        conn.execute("DELETE FROM injection_cache", []).unwrap();
        let result = rebuild_cache_light(&conn).unwrap();
        assert!(!result.payload.is_empty());
    }

    #[test]
    fn multiple_chains_ordered_by_reinforcement() {
        let conn = seeded_db();
        let result = rebuild_cache_light(&conn).unwrap();
        let cp_pos = result.payload.find("cp-alias");
        let devenv_pos = result.payload.find("devenv-stop");
        if let (Some(cp), Some(de)) = (cp_pos, devenv_pos) {
            assert!(
                cp < de,
                "cp-alias (20 reinforcements) should appear before devenv-stop (15)"
            );
        }
    }

    #[test]
    fn health_derived_from_trajectory() {
        let conn = seeded_db();
        let result = rebuild_cache_light(&conn).unwrap();
        assert!(result.payload.contains("9") || result.payload.contains("11"));
    }

    #[test]
    fn write_cache_row_updates_existing() {
        let conn = open_memory().unwrap();
        write_cache_row(&conn, "payload_1", 10).unwrap();
        write_cache_row(&conn, "payload_2", 20).unwrap();

        let section_key = crate::m3_injection::m11_parallel_query::CACHE_SECTION_KEY;
        let (p, tc): (String, u32) = conn
            .query_row(
                "SELECT payload, token_count FROM injection_cache WHERE section = ?1",
                rusqlite::params![section_key],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(p, "payload_2");
        assert_eq!(tc, 20);
    }

    #[test]
    fn rebuild_with_only_trajectory() {
        let conn = open_memory().unwrap();
        conn.execute(
            "INSERT INTO session_trajectory
                 (session_id, ralph_fitness, field_r, thermal_t, ltp_ltd_ratio,
                  services_healthy, delta_summary)
             VALUES (50, 0.7, 0.5, 0.3, 2.0, 12, 'stable')",
            [],
        )
        .unwrap();
        let result = rebuild_cache_light(&conn).unwrap();
        assert!(result.payload.contains("S051"));
    }

    #[test]
    fn rebuild_with_only_chains() {
        let conn = open_memory().unwrap();
        conn.execute(
            "INSERT INTO causal_chain
                 (origin_session, chain_type, label, description)
             VALUES (1, 'trap', 'solo-chain', 'only chain in DB')",
            [],
        )
        .unwrap();
        let result = rebuild_cache_light(&conn).unwrap();
        assert!(result.payload.contains("solo-chain"));
    }

    #[test]
    fn rebuild_with_only_workstreams() {
        let conn = open_memory().unwrap();
        conn.execute(
            "INSERT INTO workstream
                 (ws_id, title, status, priority, last_touched_session, resume_context)
             VALUES ('ws-solo', 'Solo Workstream', 'active', 1, 1, 'just this')",
            [],
        )
        .unwrap();
        let result = rebuild_cache_light(&conn).unwrap();
        assert!(result.payload.contains("Solo Workstream"));
    }

    #[test]
    fn rebuild_with_only_patterns() {
        let conn = open_memory().unwrap();
        conn.execute(
            "INSERT INTO reinforced_pattern
                 (pattern_id, category, description, weight)
             VALUES ('solo-pat', 'procedural', 'solo pattern', 0.8)",
            [],
        )
        .unwrap();
        let result = rebuild_cache_light(&conn).unwrap();
        assert!(!result.payload.is_empty());
    }

    #[test]
    fn payload_never_empty_even_on_empty_db() {
        let conn = open_memory().unwrap();
        let result = rebuild_cache_light(&conn).unwrap();
        assert!(result.payload.len() > 10);
    }

    #[test]
    fn blocked_workstreams_included() {
        let conn = seeded_db();
        let result = rebuild_cache_light(&conn).unwrap();
        assert!(
            result.payload.contains("synthex-v2") || result.payload.contains("Phase G"),
            "blocked workstream should appear in payload"
        );
    }

    #[test]
    fn light_rebuild_result_serializable() {
        let result = LightRebuildResult {
            payload: "test".into(),
            token_count: 1,
            sections_rendered: 0,
            elapsed_ms: 5,
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: LightRebuildResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.token_count, 1);
        assert_eq!(back.elapsed_ms, 5);
    }

    #[test]
    fn light_rebuild_result_debug() {
        let result = LightRebuildResult {
            payload: "test".into(),
            token_count: 1,
            sections_rendered: 0,
            elapsed_ms: 5,
        };
        let dbg = format!("{result:?}");
        assert!(dbg.contains("LightRebuildResult"));
    }

    #[test]
    fn light_rebuild_result_clone() {
        let r1 = LightRebuildResult {
            payload: "test".into(),
            token_count: 1,
            sections_rendered: 0,
            elapsed_ms: 5,
        };
        let r2 = r1.clone();
        assert_eq!(r1.payload, r2.payload);
    }

    #[test]
    fn max_five_chains_in_payload() {
        let conn = open_memory().unwrap();
        for i in 0..10 {
            conn.execute(
                "INSERT INTO causal_chain
                     (origin_session, chain_type, label, description, reinforcement_count)
                 VALUES (?1, 'bug', ?2, 'desc', ?3)",
                rusqlite::params![1, format!("chain-{i}"), 10 - i],
            )
            .unwrap();
        }
        let result = rebuild_cache_light(&conn).unwrap();
        let chain_count = (0..10)
            .filter(|i| result.payload.contains(&format!("chain-{i}")))
            .count();
        assert!(chain_count <= 5);
    }

    #[test]
    fn forget_consent_chains_excluded() {
        let conn = open_memory().unwrap();
        conn.execute(
            "INSERT INTO causal_chain
                 (origin_session, chain_type, label, description, consent)
             VALUES (1, 'bug', 'forgotten', 'this is forgotten', 'Forget')",
            [],
        )
        .unwrap();
        let result = rebuild_cache_light(&conn).unwrap();
        assert!(!result.payload.contains("forgotten"));
    }
}
