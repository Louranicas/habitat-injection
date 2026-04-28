//! `m27_auto_consolidate` — Scheduled cache-rebuild timer and `PostToolUse` counter.
//!
//! The daemon timer rebuilds the cache periodically (every 6 hours by default)
//! without running Hebbian decay — decay runs exclusively in `habitat-consolidate`.
//! The `PostToolUse` counter triggers a lightweight rebuild every N tool uses.
//!
//! ## Layer
//!
//! `m4_consolidation`
//!
//! ## Dependencies
//!
//! - L1: `m02_errors::SelfHealError`, `m05_constants`
//! - L2: `m06_schema`
//! - L3: `m13b_cache_light`
//! - L4: `m26_backup_clone`

#[cfg(feature = "sqlite")]
use std::path::PathBuf;
#[cfg(feature = "sqlite")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(feature = "sqlite")]
use std::sync::Arc;
#[cfg(feature = "sqlite")]
use std::thread::JoinHandle;
#[cfg(feature = "sqlite")]
use std::time::Duration;

#[cfg(feature = "sqlite")]
use tracing::{info, warn};

#[cfg(feature = "sqlite")]
use crate::m1_foundation::m02_errors::SelfHealError;
#[cfg(feature = "sqlite")]
use crate::m1_foundation::m05_constants::POST_TOOL_USE_REBUILD_THRESHOLD;

// ---------------------------------------------------------------------------
// Rebuild trigger tracking
// ---------------------------------------------------------------------------

/// What triggered a cache rebuild.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(feature = "sqlite")]
pub enum RebuildTrigger {
    /// Periodic timer fired.
    Timer,
    /// PostToolUse counter reached threshold.
    PostToolUse,
}

// ---------------------------------------------------------------------------
// Timer
// ---------------------------------------------------------------------------

/// Start a background thread that rebuilds the cache on a fixed interval.
///
/// The timer does **cache-rebuild only** — no Hebbian decay. Decay runs
/// exclusively in `habitat-consolidate` to prevent double-decay races.
///
/// Returns a `JoinHandle` that can be joined on shutdown.
#[cfg(feature = "sqlite")]
pub fn start_timer(
    db_path: PathBuf,
    interval: Duration,
    stop: Arc<AtomicBool>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        while !stop.load(Ordering::Relaxed) {
            std::thread::sleep(interval);
            if stop.load(Ordering::Relaxed) {
                break;
            }

            let conn = match crate::m2_schema::m06_schema::open_database(&db_path) {
                Ok(c) => c,
                Err(e) => {
                    warn!(error = %e, "consolidation timer: DB open failed");
                    continue;
                }
            };

            match crate::m3_injection::m13b_cache_light::rebuild_cache_light(&conn) {
                Ok(result) => {
                    info!(tokens = result.token_count, "timer: cache rebuilt");
                    let _ = crate::m4_consolidation::m26_backup_clone::create_backup(&db_path);
                    let _ = persist_trigger(&conn, RebuildTrigger::Timer);
                }
                Err(e) => warn!(error = %e, "timer: cache rebuild failed"),
            }
        }
    })
}

// ---------------------------------------------------------------------------
// PostToolUse counter
// ---------------------------------------------------------------------------

/// Increment the `PostToolUse` counter and trigger a rebuild if the threshold
/// is reached.
///
/// Counter persisted in `daemon_state` table so it survives daemon restarts.
///
/// # Errors
///
/// Returns [`SelfHealError`] if the database operation fails.
#[cfg(feature = "sqlite")]
pub fn tick_tool_use(
    db_path: &std::path::Path,
) -> Result<Option<crate::m3_injection::m13b_cache_light::LightRebuildResult>, SelfHealError> {
    let conn = crate::m2_schema::m06_schema::open_database(db_path)
        .map_err(|e| SelfHealError::RebuildFailed(format!("open DB for tick: {e}")))?;
    let count = increment_tool_counter(&conn)?;

    if count % POST_TOOL_USE_REBUILD_THRESHOLD == 0 {
        let result = crate::m3_injection::m13b_cache_light::rebuild_cache_light(&conn)
            .map_err(|e| SelfHealError::RebuildFailed(format!("tick rebuild: {e}")))?;
        let _ = persist_trigger(&conn, RebuildTrigger::PostToolUse);
        Ok(Some(result))
    } else {
        Ok(None)
    }
}

/// Read the current tool-use counter value.
///
/// # Errors
///
/// Returns [`SelfHealError`] if the database operation fails.
#[cfg(feature = "sqlite")]
pub fn get_tool_counter(db_path: &std::path::Path) -> Result<u32, SelfHealError> {
    let conn = crate::m2_schema::m06_schema::open_database(db_path)
        .map_err(|e| SelfHealError::RebuildFailed(format!("open DB for counter: {e}")))?;
    read_counter(&conn)
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

#[cfg(feature = "sqlite")]
fn increment_tool_counter(conn: &rusqlite::Connection) -> Result<u32, SelfHealError> {
    conn.execute(
        "INSERT INTO daemon_state (key, value, updated_at)
         VALUES ('tool_use_counter', '1', unixepoch())
         ON CONFLICT(key) DO UPDATE SET
            value = CAST(CAST(value AS INTEGER) + 1 AS TEXT),
            updated_at = unixepoch()",
        [],
    )
    .map_err(|e| SelfHealError::RebuildFailed(format!("increment counter: {e}")))?;

    read_counter(conn)
}

#[cfg(feature = "sqlite")]
fn read_counter(conn: &rusqlite::Connection) -> Result<u32, SelfHealError> {
    let val: String = conn
        .query_row(
            "SELECT value FROM daemon_state WHERE key = 'tool_use_counter'",
            [],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "0".to_string());
    val.parse::<u32>()
        .map_err(|e| SelfHealError::RebuildFailed(format!("parse counter: {e}")))
}

#[cfg(feature = "sqlite")]
fn persist_trigger(
    conn: &rusqlite::Connection,
    trigger: RebuildTrigger,
) -> Result<(), SelfHealError> {
    let label = match trigger {
        RebuildTrigger::Timer => "timer",
        RebuildTrigger::PostToolUse => "post_tool_use",
    };
    conn.execute(
        "INSERT INTO daemon_state (key, value, updated_at)
         VALUES ('last_rebuild_trigger', ?1, unixepoch())
         ON CONFLICT(key) DO UPDATE SET value = ?1, updated_at = unixepoch()",
        rusqlite::params![label],
    )
    .map_err(|e| SelfHealError::RebuildFailed(format!("persist trigger: {e}")))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use crate::m2_schema::m06_schema;
    use std::path::PathBuf;

    fn test_db(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("habitat_m27_{name}.db"));
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("db-wal"));
        let _ = std::fs::remove_file(path.with_extension("db-shm"));
        let conn = m06_schema::open_database(&path).unwrap();
        drop(conn);
        path
    }

    fn cleanup(path: &Path) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path.with_extension("db.bak"));
        let _ = std::fs::remove_file(path.with_extension("db-wal"));
        let _ = std::fs::remove_file(path.with_extension("db-shm"));
    }

    use std::path::Path;

    #[test]
    fn increment_counter_from_zero() {
        let path = test_db("cnt_zero");
        let conn = m06_schema::open_database(&path).unwrap();
        let c = increment_tool_counter(&conn).unwrap();
        assert_eq!(c, 1);
        cleanup(&path);
    }

    #[test]
    fn increment_counter_multiple() {
        let path = test_db("cnt_multi");
        let conn = m06_schema::open_database(&path).unwrap();
        for _ in 0..5 {
            increment_tool_counter(&conn).unwrap();
        }
        let c = read_counter(&conn).unwrap();
        assert_eq!(c, 5);
        cleanup(&path);
    }

    #[test]
    fn tick_tool_use_no_rebuild_below_threshold() {
        let path = test_db("tick_no");
        let result = tick_tool_use(&path).unwrap();
        assert!(result.is_none());
        cleanup(&path);
    }

    #[test]
    fn tick_tool_use_triggers_rebuild_at_threshold() {
        let path = test_db("tick_thresh");
        let conn = m06_schema::open_database(&path).unwrap();
        conn.execute(
            "INSERT INTO daemon_state (key, value) VALUES ('tool_use_counter', ?1)",
            rusqlite::params![(POST_TOOL_USE_REBUILD_THRESHOLD - 1).to_string()],
        )
        .unwrap();
        drop(conn);

        let result = tick_tool_use(&path).unwrap();
        assert!(result.is_some());
        cleanup(&path);
    }

    #[test]
    fn get_tool_counter_zero_on_empty() {
        let path = test_db("get_cnt_zero");
        let c = get_tool_counter(&path).unwrap();
        assert_eq!(c, 0);
        cleanup(&path);
    }

    #[test]
    fn get_tool_counter_after_ticks() {
        let path = test_db("get_cnt_ticks");
        for _ in 0..3 {
            tick_tool_use(&path).unwrap();
        }
        let c = get_tool_counter(&path).unwrap();
        assert_eq!(c, 3);
        cleanup(&path);
    }

    #[test]
    fn persist_trigger_timer() {
        let path = test_db("trig_timer");
        let conn = m06_schema::open_database(&path).unwrap();
        persist_trigger(&conn, RebuildTrigger::Timer).unwrap();
        let val: String = conn
            .query_row(
                "SELECT value FROM daemon_state WHERE key = 'last_rebuild_trigger'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(val, "timer");
        cleanup(&path);
    }

    #[test]
    fn persist_trigger_post_tool_use() {
        let path = test_db("trig_ptu");
        let conn = m06_schema::open_database(&path).unwrap();
        persist_trigger(&conn, RebuildTrigger::PostToolUse).unwrap();
        let val: String = conn
            .query_row(
                "SELECT value FROM daemon_state WHERE key = 'last_rebuild_trigger'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(val, "post_tool_use");
        cleanup(&path);
    }

    #[test]
    fn persist_trigger_overwrites() {
        let path = test_db("trig_overw");
        let conn = m06_schema::open_database(&path).unwrap();
        persist_trigger(&conn, RebuildTrigger::Timer).unwrap();
        persist_trigger(&conn, RebuildTrigger::PostToolUse).unwrap();
        let val: String = conn
            .query_row(
                "SELECT value FROM daemon_state WHERE key = 'last_rebuild_trigger'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(val, "post_tool_use");
        cleanup(&path);
    }

    #[test]
    fn timer_stops_on_signal() {
        let path = test_db("timer_stop");
        let stop = Arc::new(AtomicBool::new(false));
        let handle = start_timer(path.clone(), Duration::from_millis(50), stop.clone());
        std::thread::sleep(Duration::from_millis(20));
        stop.store(true, Ordering::Relaxed);
        handle.join().unwrap();
        cleanup(&path);
    }

    #[test]
    fn timer_immediate_stop() {
        let path = test_db("timer_imm");
        let stop = Arc::new(AtomicBool::new(true));
        let handle = start_timer(path.clone(), Duration::from_millis(10), stop);
        handle.join().unwrap();
        cleanup(&path);
    }

    #[test]
    fn rebuild_trigger_eq() {
        assert_eq!(RebuildTrigger::Timer, RebuildTrigger::Timer);
        assert_ne!(RebuildTrigger::Timer, RebuildTrigger::PostToolUse);
    }

    #[test]
    fn rebuild_trigger_debug() {
        let dbg = format!("{:?}", RebuildTrigger::Timer);
        assert!(dbg.contains("Timer"));
    }

    #[test]
    fn counter_survives_reconnect() {
        let path = test_db("cnt_reconnect");
        for _ in 0..3 {
            tick_tool_use(&path).unwrap();
        }
        let c = get_tool_counter(&path).unwrap();
        assert_eq!(c, 3);
        cleanup(&path);
    }

    #[test]
    fn tick_at_double_threshold() {
        let path = test_db("tick_double");
        let conn = m06_schema::open_database(&path).unwrap();
        conn.execute(
            "INSERT INTO daemon_state (key, value) VALUES ('tool_use_counter', ?1)",
            rusqlite::params![((POST_TOOL_USE_REBUILD_THRESHOLD * 2) - 1).to_string()],
        )
        .unwrap();
        drop(conn);

        let result = tick_tool_use(&path).unwrap();
        assert!(result.is_some());
        cleanup(&path);
    }

    #[test]
    fn tick_between_thresholds_no_rebuild() {
        let path = test_db("tick_between");
        let conn = m06_schema::open_database(&path).unwrap();
        conn.execute(
            "INSERT INTO daemon_state (key, value) VALUES ('tool_use_counter', '25')",
            [],
        )
        .unwrap();
        drop(conn);

        let result = tick_tool_use(&path).unwrap();
        assert!(result.is_none());
        cleanup(&path);
    }
}
