//! `m14_consent_filter` — Filters query results by [`ConsentLevel`].
//!
//! Only rows with `consent = "Emit"` pass through to the renderer.  Rows
//! with `consent = "Store"` or `consent = "Forget"` are dropped and logged
//! via [`tracing::debug!`] with the row type and identifier.
//!
//! This is the Security Architect's contribution (NA-R2 consent requirement).
//!
//! # Design
//!
//! The core primitive is [`filter_by_consent`], a generic function that works
//! over any `T: ConsentBearing`.  Four typed convenience wrappers
//! ([`filter_chains`], [`filter_trajectories`], [`filter_workstreams`],
//! [`filter_patterns`]) call through to it with a fixed context label so
//! callers do not have to supply one manually.
//!
//! # Layer
//!
//! `m3_injection`
//!
//! # Dependencies
//!
//! `m2_schema::{m07_causal_chain, m08_trajectory, m09_workstream, m10_pattern}`

use serde::{Deserialize, Serialize};

use crate::m2_schema::m07_causal_chain::CausalChainRow;
use crate::m2_schema::m08_trajectory::TrajectoryRow;
use crate::m2_schema::m09_workstream::WorkstreamRow;
use crate::m2_schema::m10_pattern::PatternRow;

// ---------------------------------------------------------------------------
// ConsentBearing trait
// ---------------------------------------------------------------------------

/// Implemented by any row type that carries a `consent` column.
///
/// The two methods expose the consent value and a human-readable identifier
/// used in [`tracing::debug!`] messages when a row is dropped.
pub trait ConsentBearing {
    /// Return the consent string for this row: `"Emit"`, `"Store"`, or
    /// `"Forget"`.
    fn consent(&self) -> &str;

    /// Return a human-readable identifier for log messages (e.g. a label,
    /// `pattern_id`, `ws_id`, or `session_id`).
    fn identifier(&self) -> String;
}

// ---------------------------------------------------------------------------
// ConsentBearing impls for the four schema row types
// ---------------------------------------------------------------------------

impl ConsentBearing for CausalChainRow {
    #[inline]
    fn consent(&self) -> &str {
        &self.consent
    }

    #[inline]
    fn identifier(&self) -> String {
        self.label.clone()
    }
}

impl ConsentBearing for TrajectoryRow {
    #[inline]
    fn consent(&self) -> &str {
        &self.consent
    }

    #[inline]
    fn identifier(&self) -> String {
        self.session_id.to_string()
    }
}

impl ConsentBearing for WorkstreamRow {
    #[inline]
    fn consent(&self) -> &str {
        &self.consent
    }

    #[inline]
    fn identifier(&self) -> String {
        self.ws_id.clone()
    }
}

impl ConsentBearing for PatternRow {
    #[inline]
    fn consent(&self) -> &str {
        &self.consent
    }

    #[inline]
    fn identifier(&self) -> String {
        self.pattern_id.clone()
    }
}

// ---------------------------------------------------------------------------
// FilterStats
// ---------------------------------------------------------------------------

/// Statistics collected during a single consent-filtering pass.
///
/// All counts fit in a `u32` — a single injection call processes at most a
/// few hundred rows.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilterStats {
    /// Number of rows that passed (consent = `"Emit"`).
    pub passed: u32,
    /// Number of rows dropped because consent = `"Store"`.
    pub dropped_store: u32,
    /// Number of rows dropped because consent = `"Forget"`.
    pub dropped_forget: u32,
    /// Total rows examined (`passed + dropped_store + dropped_forget`).
    pub total: u32,
}

// ---------------------------------------------------------------------------
// Core generic filter
// ---------------------------------------------------------------------------

/// Filter `items` by consent level, returning the passing rows and statistics.
///
/// Rows with `consent = "Emit"` are passed through.  All other rows are
/// dropped and a [`tracing::debug!`] message is emitted for each one.
///
/// `context` is a short label that identifies the call-site in log output,
/// for example `"causal_chain"` or `"pattern"`.
///
/// # Examples
///
/// ```rust
/// use habitat_injection::m3_injection::m14_consent_filter::{
///     ConsentBearing, filter_by_consent,
/// };
///
/// struct MyRow { consent: String, id: String }
///
/// impl ConsentBearing for MyRow {
///     fn consent(&self) -> &str { &self.consent }
///     fn identifier(&self) -> String { self.id.clone() }
/// }
///
/// let rows = vec![
///     MyRow { consent: "Emit".into(),  id: "r1".into() },
///     MyRow { consent: "Store".into(), id: "r2".into() },
/// ];
/// let (passed, stats) = filter_by_consent(rows, "example");
/// assert_eq!(passed.len(), 1);
/// assert_eq!(stats.passed, 1);
/// assert_eq!(stats.dropped_store, 1);
/// ```
#[must_use]
pub fn filter_by_consent<T: ConsentBearing>(
    items: Vec<T>,
    context: &str,
) -> (Vec<T>, FilterStats) {
    let total = u32::try_from(items.len()).unwrap_or(u32::MAX);
    let mut passed_rows: Vec<T> = Vec::with_capacity(items.len());
    let mut dropped_store: u32 = 0;
    let mut dropped_forget: u32 = 0;

    for item in items {
        match item.consent() {
            "Emit" => {
                passed_rows.push(item);
            }
            "Store" => {
                tracing::debug!(
                    context = context,
                    id = %item.identifier(),
                    "consent=Store: row dropped (not emitted to renderer)"
                );
                dropped_store = dropped_store.saturating_add(1);
            }
            "Forget" => {
                tracing::debug!(
                    context = context,
                    id = %item.identifier(),
                    "consent=Forget: row dropped (marked for deletion)"
                );
                dropped_forget = dropped_forget.saturating_add(1);
            }
            other => {
                // Unknown consent value — treat conservatively as non-Emit.
                tracing::debug!(
                    context = context,
                    id = %item.identifier(),
                    consent = other,
                    "unknown consent value: row dropped"
                );
                dropped_forget = dropped_forget.saturating_add(1);
            }
        }
    }

    let passed = u32::try_from(passed_rows.len()).unwrap_or(u32::MAX);
    let stats = FilterStats {
        passed,
        dropped_store,
        dropped_forget,
        total,
    };

    (passed_rows, stats)
}

// ---------------------------------------------------------------------------
// Typed convenience wrappers
// ---------------------------------------------------------------------------

/// Filter [`CausalChainRow`] items by consent level.
///
/// Convenience wrapper around [`filter_by_consent`] using the context label
/// `"causal_chain"`.
#[must_use]
pub fn filter_chains(chains: Vec<CausalChainRow>) -> (Vec<CausalChainRow>, FilterStats) {
    filter_by_consent(chains, "causal_chain")
}

/// Filter [`TrajectoryRow`] items by consent level.
///
/// Convenience wrapper around [`filter_by_consent`] using the context label
/// `"trajectory"`.
#[must_use]
pub fn filter_trajectories(
    trajectories: Vec<TrajectoryRow>,
) -> (Vec<TrajectoryRow>, FilterStats) {
    filter_by_consent(trajectories, "trajectory")
}

/// Filter [`WorkstreamRow`] items by consent level.
///
/// Convenience wrapper around [`filter_by_consent`] using the context label
/// `"workstream"`.
#[must_use]
pub fn filter_workstreams(
    workstreams: Vec<WorkstreamRow>,
) -> (Vec<WorkstreamRow>, FilterStats) {
    filter_by_consent(workstreams, "workstream")
}

/// Filter [`PatternRow`] items by consent level.
///
/// Convenience wrapper around [`filter_by_consent`] using the context label
/// `"pattern"`.
#[must_use]
pub fn filter_patterns(patterns: Vec<PatternRow>) -> (Vec<PatternRow>, FilterStats) {
    filter_by_consent(patterns, "pattern")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Test helpers — minimal in-memory row constructors
    // -----------------------------------------------------------------------

    fn make_chain(label: &str, consent: &str) -> CausalChainRow {
        CausalChainRow {
            id: 1,
            origin_session: 109,
            resolved_session: None,
            chain_type: "bug".into(),
            label: label.into(),
            description: "test".into(),
            reinforcement_count: 1,
            last_reinforced_session: None,
            consent: consent.into(),
        }
    }

    fn make_trajectory(session_id: u32, consent: &str) -> TrajectoryRow {
        TrajectoryRow {
            session_id,
            ralph_fitness: 0.664,
            field_r: 0.876,
            thermal_t: 0.515,
            ltp_ltd_ratio: 4.2,
            services_healthy: 11,
            delta_summary: "stable".into(),
            key_achievement: None,
            consent: consent.into(),
        }
    }

    fn make_workstream(ws_id: &str, consent: &str) -> WorkstreamRow {
        WorkstreamRow {
            ws_id: ws_id.into(),
            title: "Test Workstream".into(),
            status: "active".into(),
            blocker: None,
            priority: 5,
            last_touched_session: 109,
            items_total: None,
            items_done: None,
            resume_context: "ctx".into(),
            consent: consent.into(),
        }
    }

    fn make_pattern(pattern_id: &str, consent: &str) -> PatternRow {
        PatternRow {
            pattern_id: pattern_id.into(),
            category: "procedural".into(),
            description: "verify before ship".into(),
            anti_pattern: None,
            weight: 0.5,
            hit_count: 1,
            last_fired_session: None,
            natural_hit_count: 0,
            keywords: String::new(),
            consent: consent.into(),
        }
    }

    // -----------------------------------------------------------------------
    // ConsentBearing implementations
    // -----------------------------------------------------------------------

    #[test]
    fn chain_consent_bearing_returns_consent() {
        let row = make_chain("BUG-001", "Emit");
        assert_eq!(row.consent(), "Emit");
    }

    #[test]
    fn chain_consent_bearing_identifier_is_label() {
        let row = make_chain("BUG-001", "Emit");
        assert_eq!(row.identifier(), "BUG-001");
    }

    #[test]
    fn trajectory_consent_bearing_returns_consent() {
        let row = make_trajectory(109, "Store");
        assert_eq!(row.consent(), "Store");
    }

    #[test]
    fn trajectory_consent_bearing_identifier_is_session_id() {
        let row = make_trajectory(109, "Emit");
        assert_eq!(row.identifier(), "109");
    }

    #[test]
    fn workstream_consent_bearing_returns_consent() {
        let row = make_workstream("stdb-inject", "Forget");
        assert_eq!(row.consent(), "Forget");
    }

    #[test]
    fn workstream_consent_bearing_identifier_is_ws_id() {
        let row = make_workstream("stdb-inject", "Emit");
        assert_eq!(row.identifier(), "stdb-inject");
    }

    #[test]
    fn pattern_consent_bearing_returns_consent() {
        let row = make_pattern("verify-before-ship", "Emit");
        assert_eq!(row.consent(), "Emit");
    }

    #[test]
    fn pattern_consent_bearing_identifier_is_pattern_id() {
        let row = make_pattern("verify-before-ship", "Emit");
        assert_eq!(row.identifier(), "verify-before-ship");
    }

    // -----------------------------------------------------------------------
    // FilterStats — defaults and serde
    // -----------------------------------------------------------------------

    #[test]
    fn filter_stats_default_all_zero() {
        let stats = FilterStats::default();
        assert_eq!(stats.passed, 0);
        assert_eq!(stats.dropped_store, 0);
        assert_eq!(stats.dropped_forget, 0);
        assert_eq!(stats.total, 0);
    }

    #[test]
    fn filter_stats_partial_eq() {
        let a = FilterStats {
            passed: 3,
            dropped_store: 1,
            dropped_forget: 1,
            total: 5,
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn filter_stats_serde_roundtrip() {
        let stats = FilterStats {
            passed: 7,
            dropped_store: 2,
            dropped_forget: 1,
            total: 10,
        };
        let json = serde_json::to_string(&stats).expect("serialize");
        let back: FilterStats = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, stats);
    }

    #[test]
    fn filter_stats_serde_default_roundtrip() {
        let stats = FilterStats::default();
        let json = serde_json::to_string(&stats).expect("serialize");
        let back: FilterStats = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, stats);
    }

    #[test]
    fn filter_stats_debug_not_empty() {
        let stats = FilterStats {
            passed: 1,
            dropped_store: 0,
            dropped_forget: 0,
            total: 1,
        };
        assert!(!format!("{stats:?}").is_empty());
    }

    // -----------------------------------------------------------------------
    // filter_by_consent — generic filter with custom type
    // -----------------------------------------------------------------------

    struct MinimalRow {
        consent: String,
        id: String,
    }

    impl ConsentBearing for MinimalRow {
        fn consent(&self) -> &str {
            &self.consent
        }

        fn identifier(&self) -> String {
            self.id.clone()
        }
    }

    fn minimal(id: &str, consent: &str) -> MinimalRow {
        MinimalRow {
            consent: consent.into(),
            id: id.into(),
        }
    }

    #[test]
    fn empty_input_returns_empty_and_zero_stats() {
        let (rows, stats) = filter_by_consent::<MinimalRow>(vec![], "test");
        assert!(rows.is_empty());
        assert_eq!(stats, FilterStats::default());
    }

    #[test]
    fn all_emit_all_pass() {
        let items = vec![
            minimal("a", "Emit"),
            minimal("b", "Emit"),
            minimal("c", "Emit"),
        ];
        let (rows, stats) = filter_by_consent(items, "test");
        assert_eq!(rows.len(), 3);
        assert_eq!(stats.passed, 3);
        assert_eq!(stats.dropped_store, 0);
        assert_eq!(stats.dropped_forget, 0);
        assert_eq!(stats.total, 3);
    }

    #[test]
    fn all_store_all_dropped() {
        let items = vec![
            minimal("a", "Store"),
            minimal("b", "Store"),
        ];
        let (rows, stats) = filter_by_consent(items, "test");
        assert!(rows.is_empty());
        assert_eq!(stats.passed, 0);
        assert_eq!(stats.dropped_store, 2);
        assert_eq!(stats.dropped_forget, 0);
        assert_eq!(stats.total, 2);
    }

    #[test]
    fn all_forget_all_dropped() {
        let items = vec![
            minimal("a", "Forget"),
            minimal("b", "Forget"),
            minimal("c", "Forget"),
        ];
        let (rows, stats) = filter_by_consent(items, "test");
        assert!(rows.is_empty());
        assert_eq!(stats.passed, 0);
        assert_eq!(stats.dropped_store, 0);
        assert_eq!(stats.dropped_forget, 3);
        assert_eq!(stats.total, 3);
    }

    #[test]
    fn mixed_consent_correct_counts() {
        let items = vec![
            minimal("e1", "Emit"),
            minimal("s1", "Store"),
            minimal("e2", "Emit"),
            minimal("f1", "Forget"),
            minimal("e3", "Emit"),
            minimal("s2", "Store"),
        ];
        let (rows, stats) = filter_by_consent(items, "test");
        assert_eq!(rows.len(), 3);
        assert_eq!(stats.passed, 3);
        assert_eq!(stats.dropped_store, 2);
        assert_eq!(stats.dropped_forget, 1);
        assert_eq!(stats.total, 6);
    }

    #[test]
    fn stats_total_equals_passed_plus_dropped() {
        let items = vec![
            minimal("a", "Emit"),
            minimal("b", "Store"),
            minimal("c", "Forget"),
            minimal("d", "Emit"),
        ];
        let (_, stats) = filter_by_consent(items, "test");
        assert_eq!(
            stats.total,
            stats.passed + stats.dropped_store + stats.dropped_forget
        );
    }

    #[test]
    fn single_emit_item() {
        let items = vec![minimal("only", "Emit")];
        let (rows, stats) = filter_by_consent(items, "test");
        assert_eq!(rows.len(), 1);
        assert_eq!(stats.passed, 1);
        assert_eq!(stats.total, 1);
    }

    #[test]
    fn single_store_item() {
        let items = vec![minimal("only", "Store")];
        let (rows, stats) = filter_by_consent(items, "test");
        assert!(rows.is_empty());
        assert_eq!(stats.dropped_store, 1);
        assert_eq!(stats.total, 1);
    }

    #[test]
    fn single_forget_item() {
        let items = vec![minimal("only", "Forget")];
        let (rows, stats) = filter_by_consent(items, "test");
        assert!(rows.is_empty());
        assert_eq!(stats.dropped_forget, 1);
        assert_eq!(stats.total, 1);
    }

    #[test]
    fn unknown_consent_counted_as_dropped_forget() {
        let items = vec![minimal("x", "InvalidLevel")];
        let (rows, stats) = filter_by_consent(items, "test");
        assert!(rows.is_empty());
        assert_eq!(stats.dropped_store, 0);
        assert_eq!(stats.dropped_forget, 1);
        assert_eq!(stats.total, 1);
    }

    #[test]
    fn passed_rows_preserve_order() {
        let items = vec![
            minimal("first", "Emit"),
            minimal("dropped", "Store"),
            minimal("second", "Emit"),
            minimal("third", "Emit"),
        ];
        let (rows, _) = filter_by_consent(items, "test");
        assert_eq!(rows[0].id, "first");
        assert_eq!(rows[1].id, "second");
        assert_eq!(rows[2].id, "third");
    }

    #[test]
    fn large_collection_correct_stats() {
        let mut items = Vec::new();
        for i in 0..100 {
            let consent = match i % 3 {
                0 => "Emit",
                1 => "Store",
                _ => "Forget",
            };
            items.push(minimal(&format!("r{i}"), consent));
        }
        let (rows, stats) = filter_by_consent(items, "large");
        // 0..100: indices 0,3,6,...,99 are Emit → 34 items; 1,4,7,... Store → 33; 2,5,8,... Forget → 33
        assert_eq!(rows.len(), 34);
        assert_eq!(stats.passed, 34);
        assert_eq!(stats.dropped_store, 33);
        assert_eq!(stats.dropped_forget, 33);
        assert_eq!(stats.total, 100);
    }

    // -----------------------------------------------------------------------
    // filter_chains — convenience wrapper
    // -----------------------------------------------------------------------

    #[test]
    fn filter_chains_all_emit() {
        let chains = vec![
            make_chain("BUG-001", "Emit"),
            make_chain("BUG-002", "Emit"),
        ];
        let (passed, stats) = filter_chains(chains);
        assert_eq!(passed.len(), 2);
        assert_eq!(stats.passed, 2);
        assert_eq!(stats.dropped_store, 0);
        assert_eq!(stats.dropped_forget, 0);
    }

    #[test]
    fn filter_chains_store_dropped() {
        let chains = vec![
            make_chain("BUG-001", "Emit"),
            make_chain("TRAP-001", "Store"),
        ];
        let (passed, stats) = filter_chains(chains);
        assert_eq!(passed.len(), 1);
        assert_eq!(passed[0].label, "BUG-001");
        assert_eq!(stats.dropped_store, 1);
    }

    #[test]
    fn filter_chains_forget_dropped() {
        let chains = vec![make_chain("OLD-001", "Forget")];
        let (passed, stats) = filter_chains(chains);
        assert!(passed.is_empty());
        assert_eq!(stats.dropped_forget, 1);
    }

    #[test]
    fn filter_chains_empty_input() {
        let (passed, stats) = filter_chains(vec![]);
        assert!(passed.is_empty());
        assert_eq!(stats, FilterStats::default());
    }

    #[test]
    fn filter_chains_mixed() {
        let chains = vec![
            make_chain("A", "Emit"),
            make_chain("B", "Store"),
            make_chain("C", "Forget"),
            make_chain("D", "Emit"),
        ];
        let (passed, stats) = filter_chains(chains);
        assert_eq!(passed.len(), 2);
        assert_eq!(stats.passed, 2);
        assert_eq!(stats.dropped_store, 1);
        assert_eq!(stats.dropped_forget, 1);
        assert_eq!(stats.total, 4);
    }

    // -----------------------------------------------------------------------
    // filter_trajectories — convenience wrapper
    // -----------------------------------------------------------------------

    #[test]
    fn filter_trajectories_all_emit() {
        let rows = vec![
            make_trajectory(109, "Emit"),
            make_trajectory(110, "Emit"),
        ];
        let (passed, stats) = filter_trajectories(rows);
        assert_eq!(passed.len(), 2);
        assert_eq!(stats.passed, 2);
    }

    #[test]
    fn filter_trajectories_store_dropped() {
        let rows = vec![
            make_trajectory(109, "Emit"),
            make_trajectory(110, "Store"),
        ];
        let (passed, stats) = filter_trajectories(rows);
        assert_eq!(passed.len(), 1);
        assert_eq!(passed[0].session_id, 109);
        assert_eq!(stats.dropped_store, 1);
    }

    #[test]
    fn filter_trajectories_forget_dropped() {
        let rows = vec![make_trajectory(100, "Forget")];
        let (passed, stats) = filter_trajectories(rows);
        assert!(passed.is_empty());
        assert_eq!(stats.dropped_forget, 1);
    }

    #[test]
    fn filter_trajectories_empty_input() {
        let (passed, stats) = filter_trajectories(vec![]);
        assert!(passed.is_empty());
        assert_eq!(stats, FilterStats::default());
    }

    #[test]
    fn filter_trajectories_identifier_is_session_id_string() {
        let row = make_trajectory(42, "Emit");
        assert_eq!(row.identifier(), "42");
    }

    // -----------------------------------------------------------------------
    // filter_workstreams — convenience wrapper
    // -----------------------------------------------------------------------

    #[test]
    fn filter_workstreams_all_emit() {
        let rows = vec![
            make_workstream("stdb-inject", "Emit"),
            make_workstream("comms-v3", "Emit"),
        ];
        let (passed, stats) = filter_workstreams(rows);
        assert_eq!(passed.len(), 2);
        assert_eq!(stats.passed, 2);
    }

    #[test]
    fn filter_workstreams_store_dropped() {
        let rows = vec![
            make_workstream("ws-a", "Emit"),
            make_workstream("ws-b", "Store"),
        ];
        let (passed, stats) = filter_workstreams(rows);
        assert_eq!(passed.len(), 1);
        assert_eq!(passed[0].ws_id, "ws-a");
        assert_eq!(stats.dropped_store, 1);
    }

    #[test]
    fn filter_workstreams_forget_dropped() {
        let rows = vec![make_workstream("retired-ws", "Forget")];
        let (passed, stats) = filter_workstreams(rows);
        assert!(passed.is_empty());
        assert_eq!(stats.dropped_forget, 1);
    }

    #[test]
    fn filter_workstreams_empty_input() {
        let (passed, stats) = filter_workstreams(vec![]);
        assert!(passed.is_empty());
        assert_eq!(stats, FilterStats::default());
    }

    #[test]
    fn filter_workstreams_mixed() {
        let rows = vec![
            make_workstream("ws-1", "Emit"),
            make_workstream("ws-2", "Store"),
            make_workstream("ws-3", "Forget"),
        ];
        let (passed, stats) = filter_workstreams(rows);
        assert_eq!(passed.len(), 1);
        assert_eq!(stats.total, 3);
    }

    // -----------------------------------------------------------------------
    // filter_patterns — convenience wrapper
    // -----------------------------------------------------------------------

    #[test]
    fn filter_patterns_all_emit() {
        let rows = vec![
            make_pattern("verify-before-ship", "Emit"),
            make_pattern("read-only-forensics", "Emit"),
        ];
        let (passed, stats) = filter_patterns(rows);
        assert_eq!(passed.len(), 2);
        assert_eq!(stats.passed, 2);
    }

    #[test]
    fn filter_patterns_store_dropped() {
        let rows = vec![
            make_pattern("active", "Emit"),
            make_pattern("private", "Store"),
        ];
        let (passed, stats) = filter_patterns(rows);
        assert_eq!(passed.len(), 1);
        assert_eq!(passed[0].pattern_id, "active");
        assert_eq!(stats.dropped_store, 1);
    }

    #[test]
    fn filter_patterns_forget_dropped() {
        let rows = vec![make_pattern("stale", "Forget")];
        let (passed, stats) = filter_patterns(rows);
        assert!(passed.is_empty());
        assert_eq!(stats.dropped_forget, 1);
    }

    #[test]
    fn filter_patterns_empty_input() {
        let (passed, stats) = filter_patterns(vec![]);
        assert!(passed.is_empty());
        assert_eq!(stats, FilterStats::default());
    }

    #[test]
    fn filter_patterns_mixed() {
        let rows = vec![
            make_pattern("p1", "Emit"),
            make_pattern("p2", "Store"),
            make_pattern("p3", "Forget"),
            make_pattern("p4", "Emit"),
            make_pattern("p5", "Store"),
        ];
        let (passed, stats) = filter_patterns(rows);
        assert_eq!(passed.len(), 2);
        assert_eq!(stats.passed, 2);
        assert_eq!(stats.dropped_store, 2);
        assert_eq!(stats.dropped_forget, 1);
        assert_eq!(stats.total, 5);
    }

    // -----------------------------------------------------------------------
    // Cross-type: identical behaviour regardless of row type
    // -----------------------------------------------------------------------

    #[test]
    fn store_never_reaches_passed_vec_for_chains() {
        let chains = vec![make_chain("SECRET", "Store")];
        let (passed, _) = filter_chains(chains);
        assert!(passed.is_empty());
    }

    #[test]
    fn forget_never_reaches_passed_vec_for_patterns() {
        let patterns = vec![make_pattern("deleted", "Forget")];
        let (passed, _) = filter_patterns(patterns);
        assert!(passed.is_empty());
    }

    #[test]
    fn emit_is_case_sensitive_store_is_not_emit() {
        // "emit" (lowercase) should not pass — only exact "Emit" passes.
        let items = vec![
            minimal("lowercase", "emit"),
            minimal("uppercase", "Emit"),
        ];
        let (rows, stats) = filter_by_consent(items, "case");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "uppercase");
        assert_eq!(stats.passed, 1);
        assert_eq!(stats.dropped_forget, 1); // unknown → counts as dropped_forget
    }

    #[test]
    fn stats_total_always_equals_input_len() {
        for n in [0_usize, 1, 5, 10, 50] {
            let items: Vec<MinimalRow> = (0..n)
                .map(|i| minimal(&format!("r{i}"), if i % 2 == 0 { "Emit" } else { "Store" }))
                .collect();
            let input_len = u32::try_from(items.len()).unwrap_or(u32::MAX);
            let (_, stats) = filter_by_consent(items, "total-check");
            assert_eq!(stats.total, input_len, "n={n}");
        }
    }

    #[test]
    fn filter_stats_clone_is_equal_to_original() {
        let stats = FilterStats {
            passed: 5,
            dropped_store: 3,
            dropped_forget: 2,
            total: 10,
        };
        assert_eq!(stats.clone(), stats);
    }
}
