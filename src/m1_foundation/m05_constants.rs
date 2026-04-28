//! `m05_constants` — Named constants for the habitat-injection system.
//!
//! Centralises all magic numbers. Every constant has a doc comment
//! explaining its purpose and where it is consumed.
//!
//! ## Layer
//!
//! `m1_foundation`
//!
//! ## Dependencies
//!
//! - [`crate::m1_foundation::m01_types::TokenBudget`] — `DEFAULT_BUDGET` type
//! - [`crate::m1_foundation::m01_types::Severity`] — `WATCHER_SEVERITY_THRESHOLD` type
//!
//! ## Invariants
//!
//! - `DECAY_RATE` and `REINFORCE_RATE` are in `(0.0, 1.0)`.
//! - `PRUNE_THRESHOLD` is reachable by finite applications of `DECAY_RATE`.
//! - Per-section token budgets sum to less than `DEFAULT_BUDGET`.
//! - Port constants do not collide with existing habitat service ports.

use crate::m1_foundation::m01_types::{Severity, TokenBudget};

/// Default injection token budget — ~2KB of structured text.
pub const DEFAULT_BUDGET: TokenBudget = TokenBudget::new(1100);

/// Hebbian decay factor applied to unfired pattern weights each cycle.
/// `weight *= DECAY_RATE` → 0.98 means ~2% decay per cycle (half-life ~35 sessions).
pub const DECAY_RATE: f64 = 0.98;

/// Reinforcement rate for fired patterns.
/// `weight += REINFORCE_RATE * (1.0 - weight)` → asymptotic approach to 1.0.
pub const REINFORCE_RATE: f64 = 0.1;

/// Sessions of inactivity before a causal chain auto-resolves.
pub const AUTO_RESOLVE_SESSIONS: u32 = 10;

/// Injection cache rebuild interval in seconds (24h — cache valid until next consolidation run).
pub const CACHE_REBUILD_SECS: u64 = 86_400;

/// Maximum causal chains injected per bootstrap payload.
pub const MAX_CHAINS_INJECTED: usize = 5;

/// Maximum reinforced patterns injected per bootstrap payload.
pub const MAX_PATTERNS_INJECTED: usize = 10;

/// Maximum trajectory data points in bootstrap payload.
pub const MAX_TRAJECTORY_POINTS: usize = 5;

/// Maximum workstreams in bootstrap payload.
pub const MAX_WORKSTREAMS: usize = 10;

/// Pattern weights below this are candidates for pruning.
pub const PRUNE_THRESHOLD: f64 = 0.05;

/// Minimum severity that triggers Watcher observation creation.
pub const WATCHER_SEVERITY_THRESHOLD: Severity = Severity::new(7);

/// Maximum payload size in bytes for injection (~15 KB target).
pub const MAX_PAYLOAD_BYTES: usize = 15_360;

/// Default database path relative to `$HOME`.
pub const DEFAULT_DB_RELATIVE_PATH: &str = ".local/share/habitat/injection.db";

/// STDB default port.
pub const STDB_PORT: u16 = 3000;

/// Ingester health check port.
pub const INGESTER_HEALTH_PORT: u16 = 3001;

/// ORAC polling interval in seconds.
pub const ORAC_POLL_INTERVAL_SECS: u64 = 30;

/// SYNTHEX polling interval in seconds.
pub const SYNTHEX_POLL_INTERVAL_SECS: u64 = 60;

/// POVM sync interval in seconds.
pub const POVM_SYNC_INTERVAL_SECS: u64 = 300;

/// Gradient snapshot capture interval in seconds.
pub const GRADIENT_CAPTURE_INTERVAL_SECS: u64 = 60;

/// Retention: days before event payloads are stripped (envelope-only).
pub const RETENTION_ENVELOPE_DAYS: u32 = 30;

/// Retention: days before events are fully deleted.
pub const RETENTION_DELETE_DAYS: u32 = 90;

/// Retention: days before gradient snapshots are downsampled to 1/hour.
pub const GRADIENT_DOWNSAMPLE_HOURLY_DAYS: u32 = 7;

/// Retention: days before gradient snapshots are downsampled to 1/day.
pub const GRADIENT_DOWNSAMPLE_DAILY_DAYS: u32 = 30;

/// Decay scheduler interval in seconds (6 hours).
pub const DECAY_INTERVAL_SECS: u64 = 6 * 60 * 60;

/// Compaction scheduler interval in seconds (24 hours).
pub const COMPACTION_INTERVAL_SECS: u64 = 24 * 60 * 60;

/// Maximum injection latency target in milliseconds.
pub const MAX_INJECTION_LATENCY_MS: u64 = 100;

/// Watchdog health-check interval in seconds (5 minutes).
pub const WATCHDOG_CHECK_INTERVAL_SECS: u64 = 300;

/// Auto-consolidation timer interval in seconds (6 hours).
/// Cache-rebuild only — no Hebbian decay (that runs exclusively in `habitat-consolidate`).
pub const AUTO_CONSOLIDATE_INTERVAL_SECS: u64 = 21_600;

/// `PostToolUse` counter threshold — rebuild cache every Nth tool use.
pub const POST_TOOL_USE_REBUILD_THRESHOLD: u32 = 50;

/// Maximum age of backup clone before forced refresh (6 hours).
pub const BACKUP_MAX_AGE_SECS: u64 = 21_600;

/// Minimum interval between consecutive watchdog heal actions (60 seconds).
pub const WATCHDOG_HEAL_COOLDOWN_SECS: u64 = 60;

/// WAL auto-checkpoint threshold (pages).
pub const WAL_AUTOCHECKPOINT_PAGES: u32 = 100;

/// POVM consolidation interval in ticks (for POVM-origin edges in STDB).
pub const POVM_CONSOLIDATION_TICKS: u64 = 300;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_budget_value() {
        assert_eq!(DEFAULT_BUDGET.as_u32(), 1100);
    }

    #[test]
    fn decay_rate_in_range() {
        assert!(DECAY_RATE > 0.0);
        assert!(DECAY_RATE < 1.0);
    }

    #[test]
    fn reinforce_rate_in_range() {
        assert!(REINFORCE_RATE > 0.0);
        assert!(REINFORCE_RATE < 1.0);
    }

    #[test]
    fn auto_resolve_positive() {
        assert!(AUTO_RESOLVE_SESSIONS > 0);
    }

    #[test]
    fn cache_rebuild_positive() {
        assert!(CACHE_REBUILD_SECS > 0);
    }

    #[test]
    fn max_chains_positive() {
        assert!(MAX_CHAINS_INJECTED > 0);
    }

    #[test]
    fn max_patterns_positive() {
        assert!(MAX_PATTERNS_INJECTED > 0);
    }

    #[test]
    fn max_trajectory_positive() {
        assert!(MAX_TRAJECTORY_POINTS > 0);
    }

    #[test]
    fn max_workstreams_positive() {
        assert!(MAX_WORKSTREAMS > 0);
    }

    #[test]
    fn prune_threshold_in_range() {
        assert!(PRUNE_THRESHOLD > 0.0);
        assert!(PRUNE_THRESHOLD < 1.0);
    }

    #[test]
    fn watcher_threshold_is_seven() {
        assert_eq!(WATCHER_SEVERITY_THRESHOLD.as_u8(), 7);
    }

    #[test]
    fn max_payload_reasonable() {
        assert!(MAX_PAYLOAD_BYTES >= 1024);
        assert!(MAX_PAYLOAD_BYTES <= 65_536);
    }

    #[test]
    fn db_path_not_empty() {
        assert!(!DEFAULT_DB_RELATIVE_PATH.is_empty());
        assert!(DEFAULT_DB_RELATIVE_PATH.ends_with(".db"));
    }

    #[test]
    fn stdb_port_not_privileged() {
        assert!(STDB_PORT > 1024);
    }

    #[test]
    fn ingester_port_not_privileged() {
        assert!(INGESTER_HEALTH_PORT > 1024);
    }

    #[test]
    fn ingester_port_differs_from_stdb() {
        assert_ne!(STDB_PORT, INGESTER_HEALTH_PORT);
    }

    #[test]
    fn poll_intervals_ordered() {
        assert!(ORAC_POLL_INTERVAL_SECS <= SYNTHEX_POLL_INTERVAL_SECS);
        assert!(SYNTHEX_POLL_INTERVAL_SECS <= POVM_SYNC_INTERVAL_SECS);
    }

    #[test]
    fn retention_ordering() {
        assert!(RETENTION_ENVELOPE_DAYS < RETENTION_DELETE_DAYS);
    }

    #[test]
    fn gradient_downsample_ordering() {
        assert!(GRADIENT_DOWNSAMPLE_HOURLY_DAYS < GRADIENT_DOWNSAMPLE_DAILY_DAYS);
    }

    #[test]
    fn decay_interval_is_six_hours() {
        assert_eq!(DECAY_INTERVAL_SECS, 21600);
    }

    #[test]
    fn compaction_interval_is_one_day() {
        assert_eq!(COMPACTION_INTERVAL_SECS, 86400);
    }

    #[test]
    fn max_injection_latency_reasonable() {
        assert!(MAX_INJECTION_LATENCY_MS > 0);
        assert!(MAX_INJECTION_LATENCY_MS <= 1000);
    }

    #[test]
    fn povm_consolidation_ticks_positive() {
        assert!(POVM_CONSOLIDATION_TICKS > 0);
    }

    #[test]
    fn decay_produces_convergence() {
        let mut w = 1.0_f64;
        for _ in 0..1000 {
            w *= DECAY_RATE;
        }
        assert!(w < PRUNE_THRESHOLD);
    }

    #[test]
    fn reinforce_produces_growth() {
        let mut w = 0.0_f64;
        for _ in 0..50 {
            w += REINFORCE_RATE * (1.0 - w);
        }
        assert!(w > 0.99);
    }

    #[test]
    fn all_max_limits_consistent() {
        let estimated_tokens = MAX_CHAINS_INJECTED * 50
            + MAX_PATTERNS_INJECTED * 20
            + MAX_TRAJECTORY_POINTS * 30
            + MAX_WORKSTREAMS * 25;
        assert!(
            estimated_tokens < DEFAULT_BUDGET.as_u32() as usize,
            "max limits exceed default budget"
        );
    }

    // -- Mathematical property tests --

    #[test]
    fn decay_and_reinforce_are_inverse_tendencies() {
        let w = 0.5_f64;
        let decayed = w * DECAY_RATE;
        let reinforced = w + REINFORCE_RATE * (1.0 - w);
        assert!(decayed < w);
        assert!(reinforced > w);
    }

    #[test]
    fn decay_fixed_point_is_zero() {
        let w = 0.0_f64;
        let after = w * DECAY_RATE;
        assert!((after).abs() < f64::EPSILON);
    }

    #[test]
    fn reinforce_fixed_point_is_one() {
        let w = 1.0_f64;
        let after = w + REINFORCE_RATE * (1.0 - w);
        assert!((after - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn prune_threshold_reachable_by_decay() {
        let mut w = 1.0_f64;
        let mut cycles = 0u32;
        while w >= PRUNE_THRESHOLD {
            w *= DECAY_RATE;
            cycles += 1;
            assert!(cycles < 10_000, "decay never reaches prune threshold");
        }
        assert!(cycles > 0);
    }

    #[test]
    fn reinforce_from_prune_recovers_above_threshold() {
        let mut w = PRUNE_THRESHOLD;
        w += REINFORCE_RATE * (1.0 - w);
        assert!(w > PRUNE_THRESHOLD);
    }

    #[test]
    fn decay_rate_preserves_ordering() {
        let a = 0.3_f64;
        let b = 0.7_f64;
        assert!(a * DECAY_RATE < b * DECAY_RATE);
    }

    #[test]
    fn reinforce_rate_preserves_ordering() {
        let a = 0.3_f64;
        let b = 0.7_f64;
        let ra = a + REINFORCE_RATE * (1.0 - a);
        let rb = b + REINFORCE_RATE * (1.0 - b);
        assert!(ra < rb);
    }

    // -- Interval relationship tests --

    #[test]
    fn cache_rebuild_within_compaction_window() {
        assert!(CACHE_REBUILD_SECS <= COMPACTION_INTERVAL_SECS);
    }

    #[test]
    fn decay_faster_than_compaction() {
        assert!(DECAY_INTERVAL_SECS < COMPACTION_INTERVAL_SECS);
    }

    #[test]
    fn gradient_capture_leq_synthex_poll() {
        assert!(GRADIENT_CAPTURE_INTERVAL_SECS <= SYNTHEX_POLL_INTERVAL_SECS);
    }

    #[test]
    fn orac_poll_faster_than_povm_sync() {
        assert!(ORAC_POLL_INTERVAL_SECS < POVM_SYNC_INTERVAL_SECS);
    }

    // -- Payload budget tests --

    #[test]
    fn max_payload_bytes_fits_default_budget() {
        let avg_bytes_per_token = 4;
        let token_capacity = MAX_PAYLOAD_BYTES / avg_bytes_per_token;
        assert!(
            token_capacity >= DEFAULT_BUDGET.as_u32() as usize,
            "max payload bytes cannot fit default token budget"
        );
    }

    #[test]
    fn injection_latency_allows_query_plus_render() {
        assert!(MAX_INJECTION_LATENCY_MS >= 50);
    }

    // -- Port safety --

    #[test]
    fn stdb_port_not_in_habitat_range() {
        let habitat_ports = [8082, 8083, 8092, 8111, 8120, 8125, 8130, 8132, 8133, 8140, 8180, 10002];
        assert!(
            !habitat_ports.contains(&STDB_PORT),
            "STDB port conflicts with existing habitat service"
        );
    }

    #[test]
    fn ingester_port_not_in_habitat_range() {
        let habitat_ports = [8082, 8083, 8092, 8111, 8120, 8125, 8130, 8132, 8133, 8140, 8180, 10002];
        assert!(
            !habitat_ports.contains(&INGESTER_HEALTH_PORT),
            "ingester port conflicts with existing habitat service"
        );
    }

    // -- Path validation --

    #[test]
    fn db_path_is_relative() {
        assert!(!DEFAULT_DB_RELATIVE_PATH.starts_with('/'));
    }

    #[test]
    fn db_path_has_correct_directory() {
        assert!(DEFAULT_DB_RELATIVE_PATH.contains("habitat"));
    }

    // -- Auto-resolve threshold --

    #[test]
    fn auto_resolve_reasonable_range() {
        assert!(AUTO_RESOLVE_SESSIONS >= 3);
        assert!(AUTO_RESOLVE_SESSIONS <= 100);
    }

    // -- POVM consolidation --

    #[test]
    fn povm_consolidation_not_too_frequent() {
        assert!(POVM_CONSOLIDATION_TICKS >= 100);
    }

    // -- Watcher threshold --

    #[test]
    fn watcher_threshold_below_max_severity() {
        assert!(WATCHER_SEVERITY_THRESHOLD.as_u8() < 10);
    }

    #[test]
    fn watcher_threshold_above_noise() {
        assert!(WATCHER_SEVERITY_THRESHOLD.as_u8() > 3);
    }

    #[test]
    fn watcher_threshold_triggers_correctly() {
        assert!(WATCHER_SEVERITY_THRESHOLD.triggers_watcher());
    }

    // -- System-level invariants --

    #[test]
    fn retention_allows_multi_session_history() {
        assert!(
            RETENTION_ENVELOPE_DAYS >= 7,
            "envelope retention too short for meaningful trajectory"
        );
    }

    #[test]
    fn gradient_downsample_allows_weekly_resolution() {
        assert!(GRADIENT_DOWNSAMPLE_HOURLY_DAYS >= 1);
    }

    #[test]
    fn max_chains_and_patterns_both_nonzero() {
        assert!(MAX_CHAINS_INJECTED > 0);
        assert!(MAX_PATTERNS_INJECTED > 0);
    }

    // -- Self-manage constants --

    #[test]
    fn watchdog_check_interval_five_minutes() {
        assert_eq!(WATCHDOG_CHECK_INTERVAL_SECS, 300);
    }

    #[test]
    fn auto_consolidate_interval_six_hours() {
        assert_eq!(AUTO_CONSOLIDATE_INTERVAL_SECS, 21_600);
    }

    #[test]
    fn post_tool_use_threshold_positive() {
        assert!(POST_TOOL_USE_REBUILD_THRESHOLD > 0);
        assert!(POST_TOOL_USE_REBUILD_THRESHOLD <= 200);
    }

    #[test]
    fn backup_max_age_matches_consolidate() {
        assert_eq!(BACKUP_MAX_AGE_SECS, AUTO_CONSOLIDATE_INTERVAL_SECS);
    }

    #[test]
    fn watchdog_cooldown_shorter_than_check_interval() {
        assert!(WATCHDOG_HEAL_COOLDOWN_SECS < WATCHDOG_CHECK_INTERVAL_SECS);
    }

    #[test]
    fn wal_autocheckpoint_reasonable() {
        assert!(WAL_AUTOCHECKPOINT_PAGES >= 10);
        assert!(WAL_AUTOCHECKPOINT_PAGES <= 10_000);
    }

    #[test]
    fn watchdog_interval_shorter_than_consolidate() {
        assert!(WATCHDOG_CHECK_INTERVAL_SECS < AUTO_CONSOLIDATE_INTERVAL_SECS);
    }

    #[test]
    fn all_intervals_are_nonzero() {
        assert!(CACHE_REBUILD_SECS > 0);
        assert!(DECAY_INTERVAL_SECS > 0);
        assert!(COMPACTION_INTERVAL_SECS > 0);
        assert!(ORAC_POLL_INTERVAL_SECS > 0);
        assert!(SYNTHEX_POLL_INTERVAL_SECS > 0);
        assert!(POVM_SYNC_INTERVAL_SECS > 0);
        assert!(GRADIENT_CAPTURE_INTERVAL_SECS > 0);
        assert!(WATCHDOG_CHECK_INTERVAL_SECS > 0);
        assert!(AUTO_CONSOLIDATE_INTERVAL_SECS > 0);
        assert!(WATCHDOG_HEAL_COOLDOWN_SECS > 0);
        assert!(BACKUP_MAX_AGE_SECS > 0);
    }

    #[test]
    fn all_max_limits_are_nonzero() {
        assert!(MAX_CHAINS_INJECTED > 0);
        assert!(MAX_PATTERNS_INJECTED > 0);
        assert!(MAX_TRAJECTORY_POINTS > 0);
        assert!(MAX_WORKSTREAMS > 0);
        assert!(MAX_PAYLOAD_BYTES > 0);
    }
}
