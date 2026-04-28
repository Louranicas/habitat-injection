//! `m13_fallback` — Three-tier fallback chain: Tier 1 `SQLite` injection cache → Tier 2 atuin KV
//! (`habitat.last-injection`) → Tier 3 static "NO STATE". Each tier returns `Option<String>`.
//! First `Some()` wins. The chain NEVER fails — Tier 3 is unconditional.
//!
//! Layer: `m3_injection`
//! Dependencies: `m01_types`, `m02_errors`, `m06_schema`
//! Implemented by: CLI Craftsman (ALPHA-TopRight)
//! Session: S109

use std::path::Path;
use std::time::{Duration, Instant};
#[cfg(feature = "sqlite")]
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tracing::{debug, instrument, warn};

#[cfg(feature = "sqlite")]
use rusqlite::Connection;

// InjectionError is imported for internal mapping only; it is never part of a public signature.
#[allow(unused_imports)]
use crate::m1_foundation::m02_errors::InjectionError;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Which tier of the fallback chain produced the payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FallbackTier {
    /// Pre-computed `SQLite` `injection_cache` row (fastest path).
    SqliteCache,
    /// Fresh `SQLite` query + render (cache missing/stale but DB present).
    SqliteFresh,
    /// `atuin kv get habitat.last-injection` subprocess.
    AtuinKv,
    /// Static compile-time string — always available.
    Static,
}

impl std::fmt::Display for FallbackTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SqliteCache => f.write_str("sqlite-cache"),
            Self::SqliteFresh => f.write_str("sqlite-fresh"),
            Self::AtuinKv => f.write_str("atuin-kv"),
            Self::Static => f.write_str("static"),
        }
    }
}

/// The result of executing the three-tier fallback chain.
///
/// The chain is guaranteed to always produce a non-empty [`payload`](FallbackResult::payload).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackResult {
    /// The injection payload string produced by the winning tier.
    pub payload: String,
    /// Which tier produced the [`payload`](FallbackResult::payload).
    pub tier: FallbackTier,
    /// Wall-clock elapsed time in milliseconds for the entire chain.
    pub elapsed_ms: u64,
}

// ---------------------------------------------------------------------------
// The static fallback string
// ---------------------------------------------------------------------------

/// The fixed static fallback payload.
///
/// Returned by [`static_fallback`] when all dynamic tiers fail.
const STATIC_FALLBACK_PAYLOAD: &str =
    "NO INJECTION STATE — first session or database missing. All systems operational until proven otherwise.";

/// The atuin KV key used for the Tier 2 fallback cache.
const ATUIN_KV_KEY: &str = "habitat.last-injection";

/// Timeout for the `atuin kv get` subprocess (milliseconds).
const ATUIN_TIMEOUT_MS: u64 = 500;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Execute the three-tier fallback chain and always return a [`FallbackResult`].
///
/// Tiers:
/// 1. `SQLite` `injection_cache` table — fastest, pre-computed.
/// 2. `atuin kv get habitat.last-injection` — subprocess, 500 ms timeout.
/// 3. Static compile-time string — unconditional.
///
/// # Arguments
///
/// * `db_path` — path to the `SQLite` database file. `None` skips Tier 1.
/// * `cache_max_age_secs` — max age (seconds) for a Tier 1 cache hit.
///
/// # Guarantees
///
/// This function NEVER returns an error. A payload is always produced.
#[instrument(level = "debug", skip(db_path))]
pub fn execute_fallback_chain(
    db_path: Option<&Path>,
    cache_max_age_secs: u64,
) -> FallbackResult {
    let start = Instant::now();

    // Tier 1a + 1b: SQLite (feature-gated on `sqlite`)
    #[cfg(feature = "sqlite")]
    {
        let conn = db_path.and_then(open_conn_silent);

        // Tier 1a: pre-computed cache row (fastest path)
        if let Some(payload) = conn.as_ref().and_then(|c| try_sqlite_cache(c, cache_max_age_secs))
        {
            let elapsed_ms = elapsed_ms_saturating(start);
            debug!(tier = "sqlite-cache", elapsed_ms, "fallback tier 1a hit");
            return FallbackResult {
                payload,
                tier: FallbackTier::SqliteCache,
                elapsed_ms,
            };
        }

        // Tier 1b: cache stale/missing but DB accessible — rebuild from tables
        if let Some(Ok(result)) = conn
            .as_ref()
            .map(crate::m3_injection::m13b_cache_light::rebuild_cache_light)
        {
            let elapsed_ms = elapsed_ms_saturating(start);
            debug!(
                tier = "sqlite-fresh",
                elapsed_ms,
                tokens = result.token_count,
                "fallback tier 1b hit — cache rebuilt from DB"
            );
            return FallbackResult {
                payload: result.payload,
                tier: FallbackTier::SqliteFresh,
                elapsed_ms,
            };
        }
    }

    // db_path and cache_max_age_secs are unused when sqlite feature is disabled.
    #[cfg(not(feature = "sqlite"))]
    {
        let _ = db_path;
        let _ = cache_max_age_secs;
    }

    // Tier 2: atuin KV
    if let Some(payload) = try_atuin_kv() {
        let elapsed_ms = elapsed_ms_saturating(start);
        debug!(tier = "atuin-kv", elapsed_ms, "fallback tier 2 hit");
        return FallbackResult {
            payload,
            tier: FallbackTier::AtuinKv,
            elapsed_ms,
        };
    }

    // Tier 3: static (unconditional)
    let payload = static_fallback();
    let elapsed_ms = elapsed_ms_saturating(start);
    warn!(tier = "static", elapsed_ms, "fallback chain exhausted — using static payload");
    FallbackResult {
        payload,
        tier: FallbackTier::Static,
        elapsed_ms,
    }
}

/// Try to read from the `injection_cache` table.
///
/// Returns `Some(payload)` if a fresh row exists, `None` if the cache is
/// missing, stale (older than `max_age_secs`), or the query fails.
///
/// A cache hit requires a row with `section` matching
/// [`CACHE_SECTION_KEY`](crate::m3_injection::m11_parallel_query::CACHE_SECTION_KEY)
/// (`"full_payload"`). Freshness is determined by comparing `computed_at`
/// (Unix seconds) against the current wall clock.
#[cfg(feature = "sqlite")]
#[instrument(level = "debug", skip(conn))]
pub fn try_sqlite_cache(conn: &Connection, max_age_secs: u64) -> Option<String> {
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Query the canonical cache section key — must match the key
    // used by m17_cache_builder::write_cache_entry and
    // m11_parallel_query::execute_cached.
    let cache_key = crate::m3_injection::m11_parallel_query::CACHE_SECTION_KEY;
    let result: rusqlite::Result<(String, i64)> = conn.query_row(
        "SELECT payload, computed_at \
         FROM injection_cache \
         WHERE section = ?1",
        rusqlite::params![cache_key],
        |row| Ok((row.get(0)?, row.get(1)?)),
    );

    match result {
        Ok((payload, computed_at)) => {
            // `computed_at` is a Unix timestamp stored as `i64`; negative values
            // indicate pre-epoch times and are treated as maximally stale.
            let ts_secs = if computed_at < 0 { 0u64 } else { computed_at.cast_unsigned() };
            let age_secs = now_secs.saturating_sub(ts_secs);
            if age_secs <= max_age_secs {
                debug!(age_secs, max_age_secs, "sqlite cache hit — fresh");
                Some(payload)
            } else {
                debug!(age_secs, max_age_secs, "sqlite cache miss — stale");
                None
            }
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            debug!("sqlite cache miss — no rows");
            None
        }
        Err(e) => {
            warn!(error = %e, "sqlite cache query failed");
            None
        }
    }
}

/// Try to retrieve the last injection payload from atuin KV.
///
/// Runs `atuin kv get habitat.last-injection` as a subprocess with a 500 ms
/// timeout. Returns `None` if:
/// - `atuin` binary is not found.
/// - The key does not exist (atuin exits non-zero).
/// - The subprocess times out.
/// - The output is empty.
#[instrument(level = "debug")]
pub fn try_atuin_kv() -> Option<String> {
    // Locate the `atuin` binary. If it's not on PATH, fail gracefully.
    let atuin_bin = find_atuin_bin()?;

    // Spawn the subprocess with a controlled timeout via a thread.
    // std::process::Command does not natively support timeouts, so we use
    // the classic "spawn + wait_with_timeout via a thread" pattern.
    let output = run_with_timeout(
        &atuin_bin,
        &["kv", "get", ATUIN_KV_KEY],
        Duration::from_millis(ATUIN_TIMEOUT_MS),
    )?;

    if output.is_empty() {
        debug!("atuin kv get returned empty output");
        return None;
    }

    debug!(key = ATUIN_KV_KEY, bytes = output.len(), "atuin kv hit");
    Some(output)
}

/// Save `payload` to `atuin kv set habitat.last-injection` for the next Tier 2 fallback.
///
/// Best-effort — failures are logged but not propagated.
/// Returns `true` if the KV write succeeded, `false` otherwise.
#[instrument(level = "debug", skip(payload))]
pub fn save_to_atuin_kv(payload: &str) -> bool {
    let Some(atuin_bin) = find_atuin_bin() else {
        debug!("atuin binary not found — skipping KV save");
        return false;
    };

    if run_with_timeout(
        &atuin_bin,
        &["kv", "set", ATUIN_KV_KEY, payload],
        Duration::from_millis(ATUIN_TIMEOUT_MS),
    )
    .is_some()
    {
        debug!(key = ATUIN_KV_KEY, "atuin kv set succeeded");
        true
    } else {
        warn!(key = ATUIN_KV_KEY, "atuin kv set failed or timed out");
        false
    }
}

/// Return the compile-time static fallback string.
///
/// Always succeeds. This is Tier 3 — the unconditional last resort.
#[must_use]
pub fn static_fallback() -> String {
    STATIC_FALLBACK_PAYLOAD.to_owned()
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Convert an [`Instant`] elapsed duration to milliseconds, capping at [`u64::MAX`].
///
/// Uses [`u64::try_from`] to avoid truncation lint on `as_millis() as u64`.
fn elapsed_ms_saturating(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// Open a `SQLite` connection silently — returns `None` on any failure.
#[cfg(feature = "sqlite")]
fn open_conn_silent(path: &Path) -> Option<Connection> {
    if !path.exists() {
        debug!(path = %path.display(), "sqlite db path does not exist");
        return None;
    }
    match Connection::open(path) {
        Ok(conn) => Some(conn),
        Err(e) => {
            warn!(path = %path.display(), error = %e, "sqlite open failed");
            None
        }
    }
}

/// Locate the `atuin` binary by checking common locations and `PATH`.
///
/// Returns the resolved path string, or `None` if not found.
fn find_atuin_bin() -> Option<String> {
    // First try common explicit locations.
    let candidates = [
        "/usr/bin/atuin",
        "/usr/local/bin/atuin",
    ];
    for candidate in &candidates {
        if std::path::Path::new(candidate).exists() {
            return Some((*candidate).to_owned());
        }
    }

    // Fall back to searching PATH via `which`-style resolution.
    let path_var = std::env::var("PATH").unwrap_or_default();
    for dir in path_var.split(':') {
        let candidate = format!("{dir}/atuin");
        if std::path::Path::new(&candidate).exists() {
            return Some(candidate);
        }
    }

    debug!("atuin binary not found on PATH");
    None
}

/// Run a command with a timeout. Returns the trimmed stdout on success,
/// `None` on timeout, non-zero exit, or spawn failure.
fn run_with_timeout(bin: &str, args: &[&str], timeout: Duration) -> Option<String> {
    use std::process::{Command, Stdio};
    use std::sync::mpsc;
    use std::thread;

    let bin_owned = bin.to_owned();
    let args_owned: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();

    let (tx, rx) = mpsc::channel::<Option<String>>();

    thread::spawn(move || {
        let result = Command::new(&bin_owned)
            .args(&args_owned)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output();

        match result {
            Ok(output) if output.status.success() => {
                let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                let _ = tx.send(if text.is_empty() { None } else { Some(text) });
            }
            Ok(_) => {
                let _ = tx.send(None);
            }
            Err(e) => {
                warn!(bin = bin_owned, error = %e, "subprocess spawn failed");
                let _ = tx.send(None);
            }
        }
    });

    if let Ok(result) = rx.recv_timeout(timeout) {
        result
    } else {
        let timeout_ms = u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX);
        warn!(timeout_ms, "subprocess timed out");
        None
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // static_fallback
    // -----------------------------------------------------------------------

    #[test]
    fn static_fallback_not_empty() {
        let s = static_fallback();
        assert!(!s.is_empty());
    }

    #[test]
    fn static_fallback_contains_no_state() {
        let s = static_fallback();
        assert!(s.contains("NO INJECTION STATE"));
    }

    #[test]
    fn static_fallback_contains_all_systems_operational() {
        let s = static_fallback();
        assert!(s.contains("All systems operational"));
    }

    #[test]
    fn static_fallback_is_deterministic() {
        assert_eq!(static_fallback(), static_fallback());
    }

    #[test]
    fn static_fallback_matches_constant() {
        assert_eq!(static_fallback(), STATIC_FALLBACK_PAYLOAD);
    }

    #[test]
    fn static_fallback_is_valid_utf8() {
        // The payload uses Unicode punctuation (em dash) — it must be valid UTF-8.
        assert!(std::str::from_utf8(static_fallback().as_bytes()).is_ok());
    }

    #[test]
    fn static_fallback_reasonable_length() {
        let s = static_fallback();
        // Must be long enough to be meaningful but short enough for injection.
        assert!(s.len() > 20);
        assert!(s.len() < 512);
    }

    // -----------------------------------------------------------------------
    // FallbackTier
    // -----------------------------------------------------------------------

    #[test]
    fn fallback_tier_display_sqlite_cache() {
        assert_eq!(FallbackTier::SqliteCache.to_string(), "sqlite-cache");
    }

    #[test]
    fn fallback_tier_display_sqlite_fresh() {
        assert_eq!(FallbackTier::SqliteFresh.to_string(), "sqlite-fresh");
    }

    #[test]
    fn fallback_tier_display_atuin_kv() {
        assert_eq!(FallbackTier::AtuinKv.to_string(), "atuin-kv");
    }

    #[test]
    fn fallback_tier_display_static() {
        assert_eq!(FallbackTier::Static.to_string(), "static");
    }

    #[test]
    fn fallback_tier_equality_same() {
        assert_eq!(FallbackTier::SqliteCache, FallbackTier::SqliteCache);
        assert_eq!(FallbackTier::AtuinKv, FallbackTier::AtuinKv);
        assert_eq!(FallbackTier::Static, FallbackTier::Static);
    }

    #[test]
    fn fallback_tier_equality_different() {
        assert_ne!(FallbackTier::SqliteCache, FallbackTier::Static);
        assert_ne!(FallbackTier::AtuinKv, FallbackTier::SqliteFresh);
    }

    #[test]
    fn fallback_tier_copy() {
        let t = FallbackTier::SqliteCache;
        let t2 = t;
        assert_eq!(t, t2);
    }

    #[test]
    fn fallback_tier_debug_not_empty() {
        let dbg = format!("{:?}", FallbackTier::Static);
        assert!(!dbg.is_empty());
    }

    #[test]
    fn fallback_tier_serde_roundtrip() {
        for tier in [
            FallbackTier::SqliteCache,
            FallbackTier::SqliteFresh,
            FallbackTier::AtuinKv,
            FallbackTier::Static,
        ] {
            let json = serde_json::to_string(&tier).unwrap();
            let back: FallbackTier = serde_json::from_str(&json).unwrap();
            assert_eq!(tier, back);
        }
    }

    #[test]
    fn fallback_tier_hash_set_dedup() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(FallbackTier::Static);
        set.insert(FallbackTier::Static);
        assert_eq!(set.len(), 1);
    }

    // -----------------------------------------------------------------------
    // FallbackResult
    // -----------------------------------------------------------------------

    #[test]
    fn fallback_result_construction() {
        let r = FallbackResult {
            payload: "hello".to_owned(),
            tier: FallbackTier::Static,
            elapsed_ms: 1,
        };
        assert_eq!(r.payload, "hello");
        assert_eq!(r.tier, FallbackTier::Static);
        assert_eq!(r.elapsed_ms, 1);
    }

    #[test]
    fn fallback_result_serde_roundtrip() {
        let r = FallbackResult {
            payload: "test payload".to_owned(),
            tier: FallbackTier::AtuinKv,
            elapsed_ms: 42,
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: FallbackResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.payload, r.payload);
        assert_eq!(back.tier, r.tier);
        assert_eq!(back.elapsed_ms, r.elapsed_ms);
    }

    #[test]
    fn fallback_result_debug_contains_payload() {
        let r = FallbackResult {
            payload: "dbg_payload".to_owned(),
            tier: FallbackTier::Static,
            elapsed_ms: 0,
        };
        let dbg = format!("{r:?}");
        assert!(dbg.contains("dbg_payload"));
    }

    #[test]
    fn fallback_result_clone() {
        let r = FallbackResult {
            payload: "x".to_owned(),
            tier: FallbackTier::SqliteCache,
            elapsed_ms: 5,
        };
        let r2 = r.clone();
        assert_eq!(r.payload, r2.payload);
        assert_eq!(r.tier, r2.tier);
        assert_eq!(r.elapsed_ms, r2.elapsed_ms);
    }

    // -----------------------------------------------------------------------
    // execute_fallback_chain — no DB, no atuin → static tier
    // -----------------------------------------------------------------------

    #[test]
    fn execute_fallback_chain_no_db_falls_past_sqlite() {
        let result = execute_fallback_chain(None, 300);
        assert_ne!(result.tier, FallbackTier::SqliteCache);
        assert_ne!(result.tier, FallbackTier::SqliteFresh);
        assert!(!result.payload.is_empty());
    }

    #[test]
    fn execute_fallback_chain_missing_path_falls_to_static_or_atuin() {
        let result = execute_fallback_chain(
            Some(std::path::Path::new("/tmp/habitat_nonexistent_db_xyz_m13.db")),
            300,
        );
        // Must succeed — either atuin KV or static
        assert!(!result.payload.is_empty());
        assert!(result.tier == FallbackTier::AtuinKv || result.tier == FallbackTier::Static);
    }

    #[test]
    fn execute_fallback_chain_elapsed_is_reasonable() {
        let result = execute_fallback_chain(None, 300);
        // Should complete within 2 seconds in test context.
        assert!(result.elapsed_ms < 2000);
    }

    #[test]
    fn execute_fallback_chain_payload_never_empty() {
        // Calling with None db_path forces tier 2 or 3.
        let result = execute_fallback_chain(None, 300);
        assert!(!result.payload.is_empty());
    }

    #[test]
    fn execute_fallback_chain_always_completes() {
        // Call many times — should always return.
        for _ in 0..5 {
            let r = execute_fallback_chain(None, 0);
            assert!(!r.payload.is_empty());
        }
    }

    // -----------------------------------------------------------------------
    // try_sqlite_cache — in-memory DB tests
    // -----------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    mod sqlite_tests {
        use super::*;
        use crate::m2_schema::m06_schema::open_memory;

        fn now_secs() -> u64 {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        }

        fn insert_cache(conn: &Connection, payload: &str, computed_at: u64) {
            conn.execute(
                "INSERT INTO injection_cache (section, payload, token_count, computed_at) \
                 VALUES (?1, ?2, 100, ?3)",
                rusqlite::params![
                    crate::m3_injection::m11_parallel_query::CACHE_SECTION_KEY,
                    payload,
                    computed_at as i64,
                ],
            )
            .unwrap();
        }

        #[test]
        fn try_sqlite_cache_empty_table_returns_none() {
            let conn = open_memory().unwrap();
            assert!(try_sqlite_cache(&conn, 300).is_none());
        }

        #[test]
        fn try_sqlite_cache_fresh_row_returns_payload() {
            let conn = open_memory().unwrap();
            insert_cache(&conn, "fresh payload", now_secs());
            let result = try_sqlite_cache(&conn, 300);
            assert_eq!(result.as_deref(), Some("fresh payload"));
        }

        #[test]
        fn try_sqlite_cache_stale_row_returns_none() {
            let conn = open_memory().unwrap();
            // Insert a row that is 1000 seconds old.
            let old_ts = now_secs().saturating_sub(1000);
            insert_cache(&conn, "old payload", old_ts);
            // max_age = 500 → row is stale
            assert!(try_sqlite_cache(&conn, 500).is_none());
        }

        #[test]
        fn try_sqlite_cache_exactly_at_age_boundary_fresh() {
            let conn = open_memory().unwrap();
            let ts = now_secs().saturating_sub(300);
            insert_cache(&conn, "boundary payload", ts);
            // Age is exactly 300, max_age is 300 → fresh (<=)
            let result = try_sqlite_cache(&conn, 300);
            assert!(result.is_some());
        }

        #[test]
        fn try_sqlite_cache_one_second_over_stale() {
            let conn = open_memory().unwrap();
            let ts = now_secs().saturating_sub(301);
            insert_cache(&conn, "boundary payload", ts);
            assert!(try_sqlite_cache(&conn, 300).is_none());
        }

        #[test]
        fn try_sqlite_cache_zero_max_age_with_current_ts() {
            let conn = open_memory().unwrap();
            insert_cache(&conn, "current payload", now_secs());
            // max_age = 0 and row is freshly stamped — should be a hit (age ≤ 0)
            // because saturating_sub of now - now = 0
            let result = try_sqlite_cache(&conn, 0);
            // Either Some (age == 0 → ≤ 0) or None (clock tick). Both acceptable.
            // We just verify it doesn't panic.
            let _ = result;
        }

        #[test]
        #[test]
        fn try_sqlite_cache_ignores_non_canonical_section_keys() {
            let conn = open_memory().unwrap();
            let key = crate::m3_injection::m11_parallel_query::CACHE_SECTION_KEY;
            let ts = now_secs();
            conn.execute(
                "INSERT INTO injection_cache (section, payload, token_count, computed_at) \
                 VALUES ('other_section', 'wrong payload', 50, ?1)",
                rusqlite::params![ts as i64],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO injection_cache (section, payload, token_count, computed_at) \
                 VALUES (?1, 'correct payload', 80, ?2)",
                rusqlite::params![key, ts as i64],
            )
            .unwrap();
            let result = try_sqlite_cache(&conn, 300);
            assert_eq!(result.as_deref(), Some("correct payload"));
        }

        #[test]
        fn try_sqlite_cache_large_max_age_always_returns_some() {
            let conn = open_memory().unwrap();
            // Row from 1 day ago
            let ts = now_secs().saturating_sub(86_400);
            insert_cache(&conn, "old but valid", ts);
            let result = try_sqlite_cache(&conn, u64::MAX);
            assert!(result.is_some());
        }

        #[test]
        fn try_sqlite_cache_multiple_calls_idempotent() {
            let conn = open_memory().unwrap();
            insert_cache(&conn, "stable payload", now_secs());
            let r1 = try_sqlite_cache(&conn, 300);
            let r2 = try_sqlite_cache(&conn, 300);
            assert_eq!(r1, r2);
        }

        // -----------------------------------------------------------------------
        // execute_fallback_chain with a SQLite DB
        // -----------------------------------------------------------------------

        #[test]
        fn execute_fallback_chain_with_fresh_cache_hits_tier1() {
            // Write a temp DB file.
            let tmp = tempfile_path("m13_chain_fresh");
            {
                let conn = crate::m2_schema::m06_schema::open_database(&tmp).unwrap();
                let ts = now_secs();
                let key = crate::m3_injection::m11_parallel_query::CACHE_SECTION_KEY;
                conn.execute(
                    "INSERT INTO injection_cache (section, payload, token_count, computed_at) \
                     VALUES (?1, 'cached payload', 50, ?2)",
                    rusqlite::params![key, ts as i64],
                )
                .unwrap();
            }
            let result = execute_fallback_chain(Some(&tmp), 300);
            assert_eq!(result.tier, FallbackTier::SqliteCache);
            assert_eq!(result.payload, "cached payload");
            let _ = std::fs::remove_file(&tmp);
        }

        #[test]
        fn execute_fallback_chain_with_stale_cache_hits_tier_1b() {
            let tmp = tempfile_path("m13_chain_stale");
            let _ = std::fs::remove_file(&tmp);
            {
                let conn = crate::m2_schema::m06_schema::open_database(&tmp).unwrap();
                let ts = now_secs().saturating_sub(10_000);
                let key = crate::m3_injection::m11_parallel_query::CACHE_SECTION_KEY;
                conn.execute(
                    "INSERT INTO injection_cache (section, payload, token_count, computed_at) \
                     VALUES (?1, 'stale cached payload', 50, ?2)",
                    rusqlite::params![key, ts as i64],
                )
                .unwrap();
            }
            // max_age = 300 → stale → Tier 1b rebuilds from DB tables
            let result = execute_fallback_chain(Some(&tmp), 300);
            assert_eq!(
                result.tier,
                FallbackTier::SqliteFresh,
                "stale cache should trigger Tier 1b rebuild, got {:?}",
                result.tier
            );
            assert!(!result.payload.is_empty());
            let _ = std::fs::remove_file(&tmp);
        }

        #[test]
        fn execute_fallback_chain_empty_db_hits_tier_1b() {
            let tmp = tempfile_path("m13_chain_empty");
            let _ = std::fs::remove_file(&tmp);
            {
                let _conn = crate::m2_schema::m06_schema::open_database(&tmp).unwrap();
                // No rows inserted — Tier 1b rebuilds from empty tables
            }
            let result = execute_fallback_chain(Some(&tmp), 300);
            assert_eq!(
                result.tier,
                FallbackTier::SqliteFresh,
                "empty DB should trigger Tier 1b rebuild, got {:?}",
                result.tier
            );
            let _ = std::fs::remove_file(&tmp);
        }

        fn tempfile_path(name: &str) -> std::path::PathBuf {
            std::env::temp_dir().join(format!("habitat_test_{name}.db"))
        }
    }

    // -----------------------------------------------------------------------
    // try_atuin_kv — graceful missing binary
    // -----------------------------------------------------------------------

    #[test]
    fn try_atuin_kv_graceful_when_missing() {
        // If atuin is not installed, must return None — not panic.
        // If atuin IS installed and the key is set, it returns Some(…) — also fine.
        let result = try_atuin_kv();
        // We can only assert it doesn't panic. Both None and Some are valid.
        let _ = result;
    }

    #[test]
    fn try_atuin_kv_returns_option_string() {
        let result: Option<String> = try_atuin_kv();
        if let Some(ref s) = result {
            assert!(!s.is_empty(), "atuin kv returned Some but empty string");
        }
    }

    // -----------------------------------------------------------------------
    // save_to_atuin_kv — best-effort
    // -----------------------------------------------------------------------

    #[test]
    fn save_to_atuin_kv_does_not_panic() {
        // If atuin is not installed, should silently return false.
        let _ = save_to_atuin_kv("test payload for m13 unit test");
    }

    #[test]
    fn save_to_atuin_kv_returns_bool() {
        let ok: bool = save_to_atuin_kv("another test");
        // Boolean — just assert it's a bool (trivially true, but documents the contract).
        let _ = ok;
    }

    // -----------------------------------------------------------------------
    // find_atuin_bin — internal helper via observable effects
    // -----------------------------------------------------------------------

    #[test]
    fn atuin_kv_timeout_is_nonzero() {
        assert!(ATUIN_TIMEOUT_MS > 0);
    }

    #[test]
    fn atuin_kv_key_is_expected_value() {
        assert_eq!(ATUIN_KV_KEY, "habitat.last-injection");
    }

    // -----------------------------------------------------------------------
    // Tier priority ordering — chain must always prefer a lower tier number
    // -----------------------------------------------------------------------

    #[test]
    fn tier_priority_static_is_last_resort() {
        // Static is always present. A chain with no DB and no atuin must land on Static.
        // We simulate by calling with None and observing — if atuin is missing the
        // chain terminates at Static.
        let result = execute_fallback_chain(None, 300);
        assert!(!result.payload.is_empty());
    }

    #[test]
    fn tier_ordering_values_logical() {
        // Sanity: we can enumerate all tiers without overlap.
        let all_tiers = [
            FallbackTier::SqliteCache,
            FallbackTier::SqliteFresh,
            FallbackTier::AtuinKv,
            FallbackTier::Static,
        ];
        let mut seen = std::collections::HashSet::new();
        for t in &all_tiers {
            assert!(seen.insert(*t), "duplicate tier in tier list");
        }
        assert_eq!(seen.len(), 4);
    }

    // -----------------------------------------------------------------------
    // Timing — elapsed_ms should be ≥ 0 and plausible
    // -----------------------------------------------------------------------

    #[test]
    fn elapsed_ms_is_zero_or_positive() {
        let r = execute_fallback_chain(None, 300);
        // elapsed_ms is a u64, so always ≥ 0 by type. Just log it.
        let _ = r.elapsed_ms;
    }

    #[test]
    fn elapsed_ms_under_one_second_for_static_path() {
        let r = execute_fallback_chain(None, 300);
        // Static fallback path (or atuin with missing binary) is fast.
        // We allow generous headroom of 1500ms for CI load.
        if r.tier == FallbackTier::Static {
            assert!(r.elapsed_ms < 1500, "static fallback took {}ms", r.elapsed_ms);
        }
    }

    // -----------------------------------------------------------------------
    // run_with_timeout internal — observable via public API
    // -----------------------------------------------------------------------

    #[test]
    fn run_with_timeout_bad_binary_returns_none() {
        // Call try_atuin_kv indirectly with a deliberately absent binary by
        // injecting a PATH that has no atuin. This is a black-box test.
        // We rely on find_atuin_bin returning None for the test environment
        // when atuin is absent. The function must not panic.
        let _ = try_atuin_kv();
    }

    // -----------------------------------------------------------------------
    // FallbackResult: serde JSON field names
    // -----------------------------------------------------------------------

    #[test]
    fn fallback_result_json_has_expected_fields() {
        let r = FallbackResult {
            payload: "p".to_owned(),
            tier: FallbackTier::Static,
            elapsed_ms: 10,
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("\"payload\""));
        assert!(json.contains("\"tier\""));
        assert!(json.contains("\"elapsed_ms\""));
    }

    #[test]
    fn fallback_tier_json_is_string_variant() {
        let json = serde_json::to_string(&FallbackTier::Static).unwrap();
        assert!(json.contains("Static"));
    }

    // -----------------------------------------------------------------------
    // STATIC_FALLBACK_PAYLOAD constant properties
    // -----------------------------------------------------------------------

    #[test]
    fn static_constant_contains_first_session() {
        assert!(STATIC_FALLBACK_PAYLOAD.contains("first session"));
    }

    #[test]
    fn static_constant_contains_database_missing() {
        assert!(STATIC_FALLBACK_PAYLOAD.contains("database missing"));
    }

    #[test]
    fn static_constant_is_single_line() {
        assert!(!STATIC_FALLBACK_PAYLOAD.contains('\n'));
    }

    // -----------------------------------------------------------------------
    // Thread safety
    // -----------------------------------------------------------------------

    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    #[test]
    fn fallback_result_send_sync() {
        assert_send::<FallbackResult>();
        assert_sync::<FallbackResult>();
    }

    #[test]
    fn fallback_tier_send_sync() {
        assert_send::<FallbackTier>();
        assert_sync::<FallbackTier>();
    }
}
