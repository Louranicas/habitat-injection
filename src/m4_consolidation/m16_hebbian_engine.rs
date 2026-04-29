//! `m16_hebbian_engine` — Decay + reinforce algorithm.
//!
//! Implements the four-step Hebbian consolidation cycle run at session close:
//!
//! 1. **Decay** — `weight *= DECAY_RATE` for all patterns where
//!    `last_fired_session IS NOT NULL` (i.e. patterns that have been fired at
//!    least once).  Delegates to [`m10_pattern::decay_all`].
//! 2. **Reinforce** — `weight += REINFORCE_RATE * (1 - weight)`, `hit_count++`
//!    for each explicitly named fired pattern.  Delegates to
//!    [`m10_pattern::reinforce`] per `pattern_id`.
//! 3. **Prune** — `DELETE` where `weight < PRUNE_THRESHOLD`.  Delegates to
//!    [`m10_pattern::prune_weak`].
//! 4. **Auto-resolve** — resolves causal chains that have not been reinforced
//!    for [`AUTO_RESOLVE_SESSIONS`] sessions.  Delegates to
//!    [`m07_causal_chain::auto_resolve_stale`].
//!
//! # Layer
//!
//! `m4_consolidation`
//!
//! # Dependencies
//!
//! `m01_types`, `m02_errors`, `m05_constants`, `m07_causal_chain`, `m10_pattern`

#[cfg(feature = "sqlite")]
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[cfg(feature = "sqlite")]
use crate::m1_foundation::m02_errors::ConsolidationError;
#[cfg(feature = "sqlite")]
use crate::m1_foundation::m05_constants::{
    AUTO_RESOLVE_PLAN_SESSIONS, AUTO_RESOLVE_SESSIONS, BUOY_RATE, BUOY_THRESHOLD, BUOY_TTL_SESSIONS,
    DECAY_RATE, PRUNE_THRESHOLD,
};
#[cfg(feature = "sqlite")]
use crate::m2_schema::m07_causal_chain::auto_resolve_stale_typed;
#[cfg(feature = "sqlite")]
use crate::m2_schema::m10_pattern::{buoy_qualified, decay_all, natural_reinforce_weighted, prune_weak};

// ---------------------------------------------------------------------------
// ConsolidationResult
// ---------------------------------------------------------------------------

/// Result of a full Hebbian consolidation cycle.
///
/// Accumulates the counts from each of the four steps: decay → reinforce →
/// prune → auto-resolve.  All fields are non-negative; zero means the step
/// had no matching rows.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsolidationResult {
    /// Number of [`reinforced_pattern`] rows whose `weight` was decayed.
    pub patterns_decayed: u32,
    /// Number of qualified pathways that received a buoy maintenance pulse.
    pub patterns_buoyed: u32,
    /// Number of named fired patterns that were actually found and reinforced.
    pub patterns_reinforced: u32,
    /// Number of [`reinforced_pattern`] rows deleted by the prune step.
    pub patterns_pruned: u32,
    /// Number of [`causal_chain`] rows auto-resolved in this cycle.
    pub chains_auto_resolved: u32,
}

// ---------------------------------------------------------------------------
// Public API (sqlite feature-gated)
// ---------------------------------------------------------------------------

/// Run the full four-step Hebbian consolidation cycle and return aggregate
/// statistics.
///
/// The steps execute in strict order:
/// 1. [`decay_patterns`] — decay all previously fired patterns
/// 2. [`reinforce_patterns`] — reinforce the named fired patterns
/// 3. [`prune_patterns`] — delete patterns whose weight dropped below threshold
/// 4. [`auto_resolve_chains`] — resolve causal chains inactive for too long
///
/// All four steps are executed inside a single `SQLite` transaction opened with
/// [`Connection::unchecked_transaction`].  If any step fails the transaction
/// is rolled back automatically, leaving the database unchanged.
///
/// # Errors
///
/// Returns [`ConsolidationError`] if any step fails or if the transaction
/// cannot be started or committed.
#[cfg(feature = "sqlite")]
pub fn run_consolidation(
    conn: &Connection,
    session: u32,
    fired_patterns: &[&str],
) -> Result<ConsolidationResult, ConsolidationError> {
    run_consolidation_weighted(conn, session, fired_patterns, 1.0)
}

/// Run consolidation with session intensity weighting for reinforcement.
///
/// `intensity` is `min(1.0, tool_uses / INTENSITY_BASELINE)`.
///
/// # Errors
///
/// Returns [`ConsolidationError`] if any step fails.
#[cfg(feature = "sqlite")]
pub fn run_consolidation_weighted(
    conn: &Connection,
    session: u32,
    fired_patterns: &[&str],
    intensity: f64,
) -> Result<ConsolidationResult, ConsolidationError> {
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| ConsolidationError::DecayFailed(e.to_string()))?;

    let patterns_decayed = decay_patterns(&tx)?;
    let patterns_buoyed = buoy_patterns(&tx, session)?;
    let patterns_reinforced = reinforce_patterns(&tx, session, fired_patterns, intensity)?;
    let patterns_pruned = prune_patterns(&tx)?;
    let chains_auto_resolved = auto_resolve_chains(&tx, session)?;

    tx.commit()
        .map_err(|e| ConsolidationError::DecayFailed(e.to_string()))?;

    Ok(ConsolidationResult {
        patterns_decayed,
        patterns_buoyed,
        patterns_reinforced,
        patterns_pruned,
        chains_auto_resolved,
    })
}

/// Apply Hebbian decay to all patterns that have been fired at least once.
///
/// Calls [`decay_all`] with the [`DECAY_RATE`] constant (`weight *= 0.98`).
/// Only patterns where `last_fired_session IS NOT NULL` are affected; patterns
/// that have never been fired are left unchanged.
///
/// Returns the number of rows updated.
///
/// # Errors
///
/// Returns [`ConsolidationError::DecayFailed`] if the underlying database
/// operation fails.
#[cfg(feature = "sqlite")]
pub fn decay_patterns(conn: &Connection) -> Result<u32, ConsolidationError> {
    decay_all(conn, DECAY_RATE).map_err(|e| ConsolidationError::DecayFailed(e.to_string()))
}

/// Apply buoy maintenance pulse to qualified pathways (`hit_count >= BUOY_THRESHOLD`).
///
/// Fires after decay, before reinforce. Weaker than full reinforcement
/// (`BUOY_RATE=0.02` vs `REINFORCE_RATE=0.1`), maintains equilibrium at ~0.5.
/// Permanent: once qualified, always buoyed.
///
/// # Errors
///
/// Returns [`ConsolidationError::DecayFailed`] if the underlying database
/// operation fails.
#[cfg(feature = "sqlite")]
pub fn buoy_patterns(conn: &Connection, session: u32) -> Result<u32, ConsolidationError> {
    buoy_qualified(conn, BUOY_RATE, BUOY_THRESHOLD, session, BUOY_TTL_SESSIONS)
        .map_err(|e| ConsolidationError::DecayFailed(e.to_string()))
}

/// Reinforce each pattern in `pattern_ids` for the given `session`.
///
/// Calls [`reinforce`] once per `pattern_id` and counts only the patterns
/// that were actually found in the database.  Missing `pattern_id` values
/// are silently skipped (they contribute 0 to the returned count).
///
/// Returns the count of patterns that were found and reinforced.
///
/// # Errors
///
/// Returns [`ConsolidationError::ReinforceFailed`] if any underlying
/// database operation fails.
#[cfg(feature = "sqlite")]
pub fn reinforce_patterns(
    conn: &Connection,
    session: u32,
    pattern_ids: &[&str],
    intensity: f64,
) -> Result<u32, ConsolidationError> {
    use crate::m1_foundation::m05_constants::MAX_NATURAL_REINFORCE_PER_CHAIN;
    use crate::m2_schema::m10_pattern::get_by_id;

    let mut found: u32 = 0;
    for &pid in pattern_ids {
        if let Ok(Some(row)) = get_by_id(conn, pid) {
            let should_skip = row
                .last_fired_session
                .is_some_and(|last| {
                    session.saturating_sub(last) < 3
                        && row.natural_hit_count >= MAX_NATURAL_REINFORCE_PER_CHAIN
                });
            if should_skip {
                continue;
            }
        }
        let was_found = natural_reinforce_weighted(conn, pid, session, intensity).map_err(|e| {
            ConsolidationError::ReinforceFailed {
                pattern_id: pid.to_owned(),
                reason: e.to_string(),
            }
        })?;
        if was_found {
            found = found.saturating_add(1);
        }
    }
    Ok(found)
}

/// Delete all patterns whose `weight` is strictly below [`PRUNE_THRESHOLD`].
///
/// Calls [`prune_weak`] with the `PRUNE_THRESHOLD` constant (`0.05`).
///
/// Returns the number of rows deleted.
///
/// # Errors
///
/// Returns [`ConsolidationError::CacheRebuildFailed`] if the underlying
/// database operation fails.
#[cfg(feature = "sqlite")]
pub fn prune_patterns(conn: &Connection) -> Result<u32, ConsolidationError> {
    prune_weak(conn, PRUNE_THRESHOLD)
        .map_err(|e| ConsolidationError::CacheRebuildFailed(e.to_string()))
}

/// Auto-resolve all causal chains that have been inactive for at least
/// [`AUTO_RESOLVE_SESSIONS`] sessions.
///
/// Calls [`auto_resolve_stale`] with `current_session` and
/// `AUTO_RESOLVE_SESSIONS` as the threshold.
///
/// Returns the number of chains resolved.
///
/// # Errors
///
/// Returns [`ConsolidationError::ChainReinforcementFailed`] if the underlying
/// database operation fails.
#[cfg(feature = "sqlite")]
pub fn auto_resolve_chains(conn: &Connection, session: u32) -> Result<u32, ConsolidationError> {
    auto_resolve_stale_typed(conn, session, AUTO_RESOLVE_SESSIONS, AUTO_RESOLVE_PLAN_SESSIONS)
        .map_err(|e| {
        ConsolidationError::ChainReinforcementFailed {
            chain_id: 0,
            reason: e.to_string(),
        }
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Only compile the sqlite-dependent tests when the feature is active.
    #[cfg(feature = "sqlite")]
    mod sqlite_tests {
        use super::*;
        use crate::m1_foundation::m05_constants::{
            DECAY_RATE, PRUNE_THRESHOLD, REINFORCE_RATE, AUTO_RESOLVE_SESSIONS,
        };
        use crate::m2_schema::m06_schema::open_memory;
        use crate::m2_schema::m07_causal_chain::{find_by_label, insert_chain};
        use crate::m2_schema::m10_pattern::{get_by_id, insert_pattern, reinforce as pat_reinforce};

        // ------------------------------------------------------------------
        // Helpers
        // ------------------------------------------------------------------

        fn mem() -> rusqlite::Connection {
            open_memory().expect("open_memory must succeed")
        }

        /// Insert a pattern that has been fired (so decay will touch it).
        fn seed_fired(conn: &rusqlite::Connection, id: &str) {
            insert_pattern(conn, id, "procedural", "test pattern", None)
                .expect("insert_pattern");
            pat_reinforce(conn, id, 100).expect("reinforce");
        }

        /// Insert a pattern that has never been fired (decay skips it).
        fn seed_unfired(conn: &rusqlite::Connection, id: &str) {
            insert_pattern(conn, id, "trap", "unfired pattern", None)
                .expect("insert_pattern");
        }

        // ------------------------------------------------------------------
        // ConsolidationResult struct
        // ------------------------------------------------------------------

        #[test]
        fn consolidation_result_default_is_all_zeros() {
            let r = ConsolidationResult::default();
            assert_eq!(r.patterns_decayed, 0);
            assert_eq!(r.patterns_reinforced, 0);
            assert_eq!(r.patterns_pruned, 0);
            assert_eq!(r.chains_auto_resolved, 0);
        }

        #[test]
        fn consolidation_result_derives_eq() {
            let a = ConsolidationResult {
                patterns_decayed: 1,
                patterns_buoyed: 0,
                patterns_reinforced: 2,
                patterns_pruned: 3,
                chains_auto_resolved: 4,
            };
            let b = a.clone();
            assert_eq!(a, b);
        }

        #[test]
        fn consolidation_result_serde_roundtrip_zeroes() {
            let r = ConsolidationResult::default();
            let json = serde_json::to_string(&r).expect("serialize");
            let back: ConsolidationResult = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(r, back);
        }

        #[test]
        fn consolidation_result_serde_roundtrip_nonzero() {
            let r = ConsolidationResult {
                patterns_decayed: 7,
                patterns_buoyed: 0,
                patterns_reinforced: 3,
                patterns_pruned: 2,
                chains_auto_resolved: 1,
            };
            let json = serde_json::to_string(&r).expect("serialize");
            let back: ConsolidationResult = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(r, back);
        }

        #[test]
        fn consolidation_result_debug_contains_fields() {
            let r = ConsolidationResult {
                patterns_decayed: 5,
                patterns_buoyed: 0,
                patterns_reinforced: 0,
                patterns_pruned: 1,
                chains_auto_resolved: 2,
            };
            let dbg = format!("{r:?}");
            assert!(dbg.contains("patterns_decayed"));
            assert!(dbg.contains("patterns_reinforced"));
            assert!(dbg.contains("patterns_pruned"));
            assert!(dbg.contains("chains_auto_resolved"));
        }

        // ------------------------------------------------------------------
        // decay_patterns
        // ------------------------------------------------------------------

        #[test]
        fn decay_patterns_empty_db_returns_zero() {
            let conn = mem();
            let n = decay_patterns(&conn).expect("decay_patterns");
            assert_eq!(n, 0);
        }

        #[test]
        fn decay_patterns_clamps_unfired_at_floor() {
            use crate::m1_foundation::m05_constants::SEEDED_PATTERN_FLOOR;
            let conn = mem();
            seed_unfired(&conn, "p-unfired");
            // Unfired at 0.5 > floor 0.3 — gets decayed but clamped
            let n = decay_patterns(&conn).expect("decay_patterns");
            assert_eq!(n, 1);
            let row = get_by_id(&conn, "p-unfired").expect("get_by_id").expect("row");
            assert!(row.weight >= SEEDED_PATTERN_FLOOR);
        }

        #[test]
        fn decay_patterns_affects_fired_patterns() {
            let conn = mem();
            seed_fired(&conn, "p-fired");
            let n = decay_patterns(&conn).expect("decay_patterns");
            assert_eq!(n, 1);
        }

        #[test]
        fn decay_patterns_math_correct() {
            let conn = mem();
            seed_fired(&conn, "p-math");
            // weight after one reinforce from 0.5: 0.5 + 0.1*(1-0.5) = 0.55
            let before = get_by_id(&conn, "p-math").expect("g").expect("r").weight;
            decay_patterns(&conn).expect("decay_patterns");
            let after = get_by_id(&conn, "p-math").expect("g").expect("r").weight;
            let expected = before * DECAY_RATE;
            assert!((after - expected).abs() < 1e-10);
        }

        #[test]
        fn decay_patterns_returns_count_of_updated_rows() {
            let conn = mem();
            for i in 0..5_u32 {
                seed_fired(&conn, &format!("pf-{i}"));
            }
            seed_unfired(&conn, "pu-0");
            // 5 fired + 1 unfired above floor = 6
            let n = decay_patterns(&conn).expect("decay_patterns");
            assert_eq!(n, 6);
        }

        // ------------------------------------------------------------------
        // reinforce_patterns
        // ------------------------------------------------------------------

        #[test]
        fn reinforce_patterns_empty_slice_returns_zero() {
            let conn = mem();
            seed_unfired(&conn, "p0");
            let n = reinforce_patterns(&conn, 110, &[], 1.0).expect("reinforce_patterns");
            assert_eq!(n, 0);
        }

        #[test]
        fn reinforce_patterns_all_missing_returns_zero() {
            let conn = mem();
            let n = reinforce_patterns(&conn, 110, &["no-such-id", "also-missing"], 1.0)
                .expect("reinforce_patterns");
            assert_eq!(n, 0);
        }

        #[test]
        fn reinforce_patterns_found_pattern_returns_one() {
            let conn = mem();
            seed_unfired(&conn, "p-found");
            let n = reinforce_patterns(&conn, 110, &["p-found"], 1.0).expect("reinforce_patterns");
            assert_eq!(n, 1);
        }

        #[test]
        fn reinforce_patterns_mixed_found_and_missing() {
            let conn = mem();
            seed_unfired(&conn, "p-exists");
            let n = reinforce_patterns(&conn, 110, &["p-exists", "p-missing"], 1.0)
                .expect("reinforce_patterns");
            assert_eq!(n, 1);
        }

        #[test]
        fn reinforce_patterns_multiple_found() {
            let conn = mem();
            for i in 0..4_u32 {
                seed_unfired(&conn, &format!("prf-{i}"));
            }
            let ids = ["prf-0", "prf-1", "prf-2", "prf-3"];
            let n = reinforce_patterns(&conn, 110, &ids, 1.0).expect("reinforce_patterns");
            assert_eq!(n, 4);
        }

        #[test]
        fn reinforce_patterns_updates_last_fired_session() {
            let conn = mem();
            seed_unfired(&conn, "p-sess");
            reinforce_patterns(&conn, 42, &["p-sess"], 1.0).expect("reinforce_patterns");
            let row = get_by_id(&conn, "p-sess").expect("g").expect("r");
            assert_eq!(row.last_fired_session, Some(42));
        }

        #[test]
        fn reinforce_patterns_increments_weight() {
            let conn = mem();
            seed_unfired(&conn, "p-wt");
            let before = get_by_id(&conn, "p-wt").expect("g").expect("r").weight;
            reinforce_patterns(&conn, 110, &["p-wt"], 1.0).expect("reinforce_patterns");
            let after = get_by_id(&conn, "p-wt").expect("g").expect("r").weight;
            let expected = before + REINFORCE_RATE * (1.0 - before);
            assert!((after - expected).abs() < 1e-10);
        }

        #[test]
        fn reinforce_patterns_increments_hit_count() {
            let conn = mem();
            seed_unfired(&conn, "p-hc");
            let before = get_by_id(&conn, "p-hc").expect("g").expect("r").hit_count;
            reinforce_patterns(&conn, 110, &["p-hc"], 1.0).expect("reinforce_patterns");
            let after = get_by_id(&conn, "p-hc").expect("g").expect("r").hit_count;
            assert_eq!(after, before + 1);
        }

        #[test]
        fn reinforce_patterns_duplicate_ids_reinforce_twice() {
            // Providing the same id twice counts as two found hits.
            let conn = mem();
            seed_unfired(&conn, "p-dup");
            let n = reinforce_patterns(&conn, 110, &["p-dup", "p-dup"], 1.0)
                .expect("reinforce_patterns");
            assert_eq!(n, 2);
            let row = get_by_id(&conn, "p-dup").expect("g").expect("r");
            // started at hit_count=1; two reinforcements → 3
            assert_eq!(row.hit_count, 3);
        }

        // ------------------------------------------------------------------
        // prune_patterns
        // ------------------------------------------------------------------

        #[test]
        fn prune_patterns_empty_db_returns_zero() {
            let conn = mem();
            let n = prune_patterns(&conn).expect("prune_patterns");
            assert_eq!(n, 0);
        }

        #[test]
        fn prune_patterns_leaves_above_threshold() {
            let conn = mem();
            seed_unfired(&conn, "p-above"); // weight = 0.5
            let n = prune_patterns(&conn).expect("prune_patterns");
            assert_eq!(n, 0);
        }

        #[test]
        fn prune_patterns_deletes_below_threshold() {
            let conn = mem();
            // Insert a pattern, fire it, then zero its weight via raw SQL.
            seed_fired(&conn, "p-low");
            conn.execute(
                "UPDATE reinforced_pattern SET weight = 0.0 WHERE pattern_id = 'p-low'",
                [],
            )
            .expect("manual weight zero");
            let n = prune_patterns(&conn).expect("prune_patterns");
            assert_eq!(n, 1);
        }

        #[test]
        fn prune_patterns_exactly_at_threshold_survives() {
            let conn = mem();
            conn.execute(
                "INSERT INTO reinforced_pattern (pattern_id, category, description, weight)
                 VALUES ('p-exact', 'trap', 'desc', ?1)",
                rusqlite::params![PRUNE_THRESHOLD],
            )
            .expect("insert at threshold");
            // weight == PRUNE_THRESHOLD is NOT < threshold; should survive
            let n = prune_patterns(&conn).expect("prune_patterns");
            assert_eq!(n, 0);
        }

        #[test]
        fn prune_patterns_counts_multiple_deletions() {
            let conn = mem();
            for i in 0..3_u32 {
                let pid = format!("pp-low-{i}");
                seed_fired(&conn, &pid);
                conn.execute(
                    "UPDATE reinforced_pattern SET weight = 0.01 WHERE pattern_id = ?1",
                    rusqlite::params![pid],
                )
                .expect("set low weight");
            }
            seed_unfired(&conn, "pp-high"); // weight = 0.5, survives
            let n = prune_patterns(&conn).expect("prune_patterns");
            assert_eq!(n, 3);
        }

        // ------------------------------------------------------------------
        // auto_resolve_chains
        // ------------------------------------------------------------------

        #[test]
        fn auto_resolve_chains_empty_db_returns_zero() {
            let conn = mem();
            let n = auto_resolve_chains(&conn, 120).expect("auto_resolve_chains");
            assert_eq!(n, 0);
        }

        #[test]
        fn auto_resolve_chains_fresh_chain_not_resolved() {
            let conn = mem();
            // origin 119, current 120, threshold 10 → gap 1 < 10 → not stale
            insert_chain(&conn, 119, "trap", "FRESH-CHAIN", "x").expect("insert_chain");
            let n = auto_resolve_chains(&conn, 120).expect("auto_resolve_chains");
            assert_eq!(n, 0);
        }

        #[test]
        fn auto_resolve_chains_stale_chain_resolved() {
            let conn = mem();
            // origin 100, current 120, threshold=AUTO_RESOLVE_SESSIONS=10 → 20 ≥ 10 → stale
            insert_chain(&conn, 100, "trap", "STALE-CHAIN", "x").expect("insert_chain");
            let n = auto_resolve_chains(&conn, 120).expect("auto_resolve_chains");
            assert_eq!(n, 1);
            let row = find_by_label(&conn, "STALE-CHAIN").expect("g").expect("r");
            assert_eq!(row.resolved_session, Some(120));
        }

        #[test]
        fn auto_resolve_chains_threshold_boundary_exclusive() {
            let conn = mem();
            // gap = AUTO_RESOLVE_SESSIONS - 1 → NOT stale
            let current = 120_u32;
            let origin = current - AUTO_RESOLVE_SESSIONS + 1;
            insert_chain(&conn, origin, "trap", "BOUND-EX", "x").expect("insert_chain");
            let n = auto_resolve_chains(&conn, current).expect("auto_resolve_chains");
            assert_eq!(n, 0);
        }

        #[test]
        fn auto_resolve_chains_threshold_boundary_inclusive() {
            let conn = mem();
            // gap == AUTO_RESOLVE_SESSIONS → stale
            let current = 120_u32;
            let origin = current - AUTO_RESOLVE_SESSIONS;
            insert_chain(&conn, origin, "trap", "BOUND-IN", "x").expect("insert_chain");
            let n = auto_resolve_chains(&conn, current).expect("auto_resolve_chains");
            assert_eq!(n, 1);
        }

        #[test]
        fn auto_resolve_chains_multiple_stale_resolved() {
            let conn = mem();
            for i in 0..3_u32 {
                insert_chain(&conn, 100, "trap", &format!("STALE-M{i}"), "x")
                    .expect("insert_chain");
            }
            let n = auto_resolve_chains(&conn, 120).expect("auto_resolve_chains");
            assert_eq!(n, 3);
        }

        // ------------------------------------------------------------------
        // run_consolidation — ordering and integration
        // ------------------------------------------------------------------

        #[test]
        fn run_consolidation_empty_db_returns_zeros() {
            let conn = mem();
            let r = run_consolidation(&conn, 110, &[]).expect("run_consolidation");
            assert_eq!(r, ConsolidationResult::default());
        }

        #[test]
        fn run_consolidation_full_cycle_seeded_data() {
            let conn = mem();
            // Two fired patterns (will be decayed), one unfired (decayed with floor).
            seed_fired(&conn, "fired-a");
            seed_fired(&conn, "fired-b");
            seed_unfired(&conn, "unfired-c");
            // One stale causal chain (trap — eligible for auto-resolve).
            insert_chain(&conn, 100, "trap", "STALE-CC", "x").expect("insert_chain");

            let r = run_consolidation(&conn, 120, &["fired-a"]).expect("run_consolidation");

            // decay touched 2 fired + 1 unfired above floor = 3
            assert_eq!(r.patterns_decayed, 3);
            // reinforce found "fired-a" only
            assert_eq!(r.patterns_reinforced, 1);
            // nothing pruned (all weights are still well above threshold)
            assert_eq!(r.patterns_pruned, 0);
            // the stale chain was auto-resolved
            assert_eq!(r.chains_auto_resolved, 1);
        }

        #[test]
        fn run_consolidation_decay_before_reinforce() {
            // After decay: weight = w_d = w_before * 0.95
            // After reinforce: weight = w_d + 0.1*(1-w_d)
            //
            // If order were reversed (reinforce first) the weight would be
            // (w_before + 0.1*(1-w_before)) * 0.95, which is different.
            let conn = mem();
            seed_fired(&conn, "order-test");
            let w0 = get_by_id(&conn, "order-test").expect("g").expect("r").weight;

            let expected_after_decay = w0 * DECAY_RATE;
            let expected_final = expected_after_decay + REINFORCE_RATE * (1.0 - expected_after_decay);

            run_consolidation(&conn, 110, &["order-test"]).expect("run_consolidation");

            let actual = get_by_id(&conn, "order-test").expect("g").expect("r").weight;
            assert!(
                (actual - expected_final).abs() < 1e-10,
                "expected {expected_final}, got {actual}"
            );
        }

        #[test]
        fn run_consolidation_prune_after_decay_removes_weakened_patterns() {
            // Force a pattern's weight to just above the prune threshold such that
            // a single decay step pushes it strictly below.
            //
            // We need: weight * DECAY_RATE < PRUNE_THRESHOLD
            // i.e.     weight < PRUNE_THRESHOLD / DECAY_RATE
            //
            // Use the midpoint between PRUNE_THRESHOLD and PRUNE_THRESHOLD/DECAY_RATE
            // so the decayed result is clearly below (not floating-point-equal to) the
            // prune threshold.
            let conn = mem();
            let upper = PRUNE_THRESHOLD / DECAY_RATE; // ≈ 0.052631
            let mid = (PRUNE_THRESHOLD + upper) / 2.0; // ≈ 0.051316  (above 0.05, below 0.05263)
            let pid = "p-prune-after-decay";
            insert_pattern(&conn, pid, "trap", "desc", None).expect("insert");
            pat_reinforce(&conn, pid, 100).expect("reinforce");
            conn.execute(
                "UPDATE reinforced_pattern SET weight = ?1 WHERE pattern_id = ?2",
                rusqlite::params![mid, pid],
            )
            .expect("set weight");

            // Verify the pre-condition: mid * DECAY_RATE < PRUNE_THRESHOLD
            assert!(
                mid * DECAY_RATE < PRUNE_THRESHOLD,
                "pre-condition: {mid} * {DECAY_RATE} = {} must be < {PRUNE_THRESHOLD}",
                mid * DECAY_RATE
            );

            let r = run_consolidation(&conn, 110, &[]).expect("run_consolidation");
            assert_eq!(r.patterns_decayed, 1);
            assert_eq!(r.patterns_pruned, 1);
            assert!(get_by_id(&conn, pid).expect("g").is_none(), "pruned row must be gone");
        }

        #[test]
        fn run_consolidation_reinforce_before_prune() {
            // A pattern just below the prune threshold, then reinforced: it should
            // survive because reinforce runs before prune.
            let conn = mem();
            let just_below = PRUNE_THRESHOLD - 1e-9;
            let pid = "p-reinforce-saves";
            insert_pattern(&conn, pid, "trap", "desc", None).expect("insert");
            // Do NOT fire it yet so decay skips it; set weight manually.
            conn.execute(
                "UPDATE reinforced_pattern SET weight = ?1 WHERE pattern_id = ?2",
                rusqlite::params![just_below, pid],
            )
            .expect("set weight");

            // Reinforce this pattern (now weight will rise above threshold, then prune skips it).
            let r = run_consolidation(&conn, 110, &[pid]).expect("run_consolidation");
            // decay skipped it (last_fired_session IS NULL still), reinforce found it.
            assert_eq!(r.patterns_reinforced, 1);
            assert_eq!(r.patterns_pruned, 0);
            let row = get_by_id(&conn, pid).expect("g").expect("r");
            assert!(row.weight > PRUNE_THRESHOLD);
        }

        #[test]
        fn run_consolidation_accumulates_all_result_fields() {
            let conn = mem();
            // 3 fired patterns
            for i in 0..3_u32 {
                seed_fired(&conn, &format!("acc-f{i}"));
            }
            // 2 unfired
            for i in 0..2_u32 {
                seed_unfired(&conn, &format!("acc-u{i}"));
            }
            // Force two fired patterns to near-zero so they get pruned after decay.
            for id in &["acc-f1", "acc-f2"] {
                conn.execute(
                    "UPDATE reinforced_pattern SET weight = 0.001 WHERE pattern_id = ?1",
                    rusqlite::params![id],
                )
                .expect("set low weight");
            }
            // 2 stale causal chains
            for i in 0..2_u32 {
                insert_chain(&conn, 100, "trap", &format!("acc-cc{i}"), "x").expect("insert_chain");
            }

            // Fire "acc-f0" explicitly; "acc-f1" and "acc-f2" are fired but will be pruned.
            let r = run_consolidation(&conn, 120, &["acc-f0"]).expect("run_consolidation");

            assert_eq!(r.patterns_decayed, 5, "3 fired + 2 unfired above floor decayed");
            assert_eq!(r.patterns_reinforced, 1, "only acc-f0 reinforced");
            assert_eq!(r.patterns_pruned, 2, "acc-f1 and acc-f2 pruned");
            assert_eq!(r.chains_auto_resolved, 2, "both stale chains resolved");
        }

        #[test]
        fn run_consolidation_multiple_fired_patterns() {
            let conn = mem();
            for i in 0..5_u32 {
                seed_unfired(&conn, &format!("mfp-{i}"));
            }
            let fired = ["mfp-0", "mfp-1", "mfp-2"];
            let r = run_consolidation(&conn, 110, &fired).expect("run_consolidation");
            assert_eq!(r.patterns_reinforced, 3);
        }

        #[test]
        fn run_consolidation_fired_pattern_not_in_db_returns_zero_reinforce() {
            let conn = mem();
            let r = run_consolidation(&conn, 110, &["does-not-exist"])
                .expect("run_consolidation");
            assert_eq!(r.patterns_reinforced, 0);
        }

        #[test]
        fn run_consolidation_no_stale_chains_chains_resolved_zero() {
            let conn = mem();
            // Chain that is NOT stale: origin = 115, current = 116, threshold = 10
            insert_chain(&conn, 115, "plan", "FRESH-ONLY", "x").expect("insert_chain");
            let r = run_consolidation(&conn, 116, &[]).expect("run_consolidation");
            assert_eq!(r.chains_auto_resolved, 0);
        }

        // ------------------------------------------------------------------
        // Mathematical verification
        // ------------------------------------------------------------------

        #[test]
        fn decay_reinforce_interaction_math_verified() {
            // Start at weight w0.  After one full cycle (fired pattern):
            //   step 1 decay:     w1 = w0 * DECAY_RATE
            //   step 2 reinforce: w2 = w1 + REINFORCE_RATE * (1 - w1)
            //                        = w0 * D + R * (1 - w0 * D)
            //                        = w0*D + R - R*w0*D
            //                        = R + w0*D*(1-R)
            let conn = mem();
            seed_fired(&conn, "math-v");
            let w0 = get_by_id(&conn, "math-v").expect("g").expect("r").weight;
            run_consolidation(&conn, 110, &["math-v"]).expect("run");
            let actual = get_by_id(&conn, "math-v").expect("g").expect("r").weight;

            let d = DECAY_RATE;
            let r = REINFORCE_RATE;
            let expected = r + w0 * d * (1.0 - r);
            assert!((actual - expected).abs() < 1e-10, "expected {expected}, got {actual}");
        }

        #[test]
        fn decay_only_path_does_not_reinforce() {
            // A fired pattern not listed in fired_patterns: decayed only.
            let conn = mem();
            seed_fired(&conn, "decay-only");
            let w0 = get_by_id(&conn, "decay-only").expect("g").expect("r").weight;
            run_consolidation(&conn, 110, &[]).expect("run");
            let w1 = get_by_id(&conn, "decay-only").expect("g").expect("r").weight;
            let expected = w0 * DECAY_RATE;
            assert!((w1 - expected).abs() < 1e-10);
        }

        #[test]
        fn unfired_pattern_weight_decayed_then_reinforced() {
            // An unfired pattern listed in fired_patterns: decayed with floor, then reinforced.
            let conn = mem();
            seed_unfired(&conn, "unfired-in-list");
            let w0 = get_by_id(&conn, "unfired-in-list").expect("g").expect("r").weight;
            run_consolidation(&conn, 110, &["unfired-in-list"]).expect("run");
            let w1 = get_by_id(&conn, "unfired-in-list").expect("g").expect("r").weight;
            // Decay applied first (w0*DECAY_RATE), then reinforce.
            let after_decay = w0 * DECAY_RATE;
            let expected = after_decay + REINFORCE_RATE * (1.0 - after_decay);
            assert!((w1 - expected).abs() < 1e-10);
        }

        #[test]
        fn prune_threshold_constant_matches_expected_value() {
            // Regression guard: if the constant changes unintentionally this fails.
            assert!((PRUNE_THRESHOLD - 0.05).abs() < f64::EPSILON);
        }

        #[test]
        fn decay_rate_constant_matches_expected_value() {
            assert!((DECAY_RATE - 0.98).abs() < f64::EPSILON);
        }

        #[test]
        fn reinforce_rate_constant_matches_expected_value() {
            assert!((REINFORCE_RATE - 0.1).abs() < f64::EPSILON);
        }

        #[test]
        fn auto_resolve_sessions_constant_matches_expected_value() {
            assert_eq!(AUTO_RESOLVE_SESSIONS, 10);
        }

        // ------------------------------------------------------------------
        // Repeated / idempotent consolidation
        // ------------------------------------------------------------------

        #[test]
        fn multiple_consolidation_cycles_converge_toward_prune() {
            let conn = mem();
            seed_fired(&conn, "converge");
            // After enough decay-only cycles the weight should drop below the
            // prune threshold and the pattern will be pruned.
            for s in 110..350_u32 {
                let r = run_consolidation(&conn, s, &[]).expect("run");
                if r.patterns_pruned > 0 {
                    assert!(get_by_id(&conn, "converge").expect("g").is_none());
                    return;
                }
            }
            panic!("pattern never pruned after 240 decay-only cycles");
        }

        #[test]
        fn consolidation_on_already_empty_db_is_idempotent() {
            let conn = mem();
            for s in 0..3_u32 {
                let r = run_consolidation(&conn, s, &[]).expect("run");
                assert_eq!(r, ConsolidationResult::default());
            }
        }

        #[test]
        fn run_consolidation_session_zero_accepted() {
            // Edge case: session = 0 must not panic or error.
            let conn = mem();
            let r = run_consolidation(&conn, 0, &[]).expect("run_consolidation");
            assert_eq!(r, ConsolidationResult::default());
        }

        #[test]
        fn run_consolidation_high_session_number_accepted() {
            let conn = mem();
            let r = run_consolidation(&conn, u32::MAX, &[]).expect("run_consolidation");
            assert_eq!(r, ConsolidationResult::default());
        }

        #[test]
        fn reinforce_patterns_large_slice_all_missing() {
            let conn = mem();
            let ids: Vec<String> = (0..50).map(|i| format!("missing-{i}")).collect();
            let id_refs: Vec<&str> = ids.iter().map(String::as_str).collect();
            let n = reinforce_patterns(&conn, 110, &id_refs, 1.0).expect("reinforce_patterns");
            assert_eq!(n, 0);
        }

        #[test]
        fn prune_patterns_after_zero_weight_decay_clears_table() {
            let conn = mem();
            for i in 0..5_u32 {
                seed_fired(&conn, &format!("clr-{i}"));
            }
            // Zero-out all weights manually.
            conn.execute(
                "UPDATE reinforced_pattern SET weight = 0.0",
                [],
            )
            .expect("zero weights");
            let n = prune_patterns(&conn).expect("prune_patterns");
            assert_eq!(n, 5);
        }

        // ------------------------------------------------------------------
        // ConsolidationResult Eq / ne
        // ------------------------------------------------------------------

        #[test]
        fn consolidation_result_ne_when_fields_differ() {
            let a = ConsolidationResult { patterns_decayed: 1, ..Default::default() };
            let b = ConsolidationResult { patterns_decayed: 2, ..Default::default() };
            assert_ne!(a, b);
        }

        #[test]
        fn consolidation_result_eq_same_values() {
            let a = ConsolidationResult {
                patterns_decayed: 3,
                patterns_buoyed: 0,
                patterns_reinforced: 2,
                patterns_pruned: 1,
                chains_auto_resolved: 0,
            };
            let b = a.clone();
            assert_eq!(a, b);
        }
    }

    // Compile-time smoke test: ConsolidationResult is usable without sqlite feature.
    #[test]
    fn consolidation_result_is_always_available() {
        let r: ConsolidationResult = ConsolidationResult::default();
        assert_eq!(r.patterns_decayed, 0);
    }
}
