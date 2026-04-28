//! `m28_health_watchdog` — Periodic health monitor with cooldown and cached metrics.
//!
//! Runs `check_and_heal` on a fixed interval (5 minutes by default), caches
//! the result for serving `/health` without a database query, and enforces
//! a cooldown period between consecutive heal actions to prevent storms.
//!
//! ## Layer
//!
//! `m4_consolidation`
//!
//! ## Dependencies
//!
//! - L1: `m05_constants`
//! - L4: `m25_self_heal`

#[cfg(feature = "sqlite")]
use std::path::PathBuf;
#[cfg(feature = "sqlite")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(feature = "sqlite")]
use std::sync::{Arc, RwLock};
#[cfg(feature = "sqlite")]
use std::time::{Duration, Instant};

#[cfg(feature = "sqlite")]
use tracing::{debug, info, warn};

#[cfg(feature = "sqlite")]
use crate::m4_consolidation::m25_self_heal::{CacheHealth, HealAction};

// ---------------------------------------------------------------------------
// WatchdogHandle
// ---------------------------------------------------------------------------

/// Handle for the background watchdog thread.
#[cfg(feature = "sqlite")]
pub struct WatchdogHandle {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    /// Cached health metrics from the last check — serves `/health` without a
    /// database query.
    pub cached_health: Arc<RwLock<CacheHealth>>,
}

#[cfg(feature = "sqlite")]
impl WatchdogHandle {
    /// Gracefully shut down the watchdog thread (2-second join timeout via
    /// the loop's sleep granularity).
    pub fn shutdown(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Start the health watchdog on a background thread.
///
/// Runs `check_and_heal` every `check_interval`, caches the result in
/// `cached_health`, and enforces `cooldown_secs` between consecutive
/// heal actions.
#[cfg(feature = "sqlite")]
pub fn start_watchdog(
    db_path: PathBuf,
    backup_path: PathBuf,
    check_interval: Duration,
    cache_ttl: Duration,
    stop: &Arc<AtomicBool>,
) -> WatchdogHandle {
    let cached = Arc::new(RwLock::new(CacheHealth::default()));
    let cached_clone = cached.clone();
    let cooldown_secs = crate::m1_foundation::m05_constants::WATCHDOG_HEAL_COOLDOWN_SECS;
    let stop_clone = Arc::clone(stop);

    let thread = std::thread::spawn(move || {
        let stop = stop_clone;
        let mut last_heal_at = Instant::now()
            .checked_sub(Duration::from_secs(cooldown_secs + 1))
            .unwrap_or_else(Instant::now);

        while !stop.load(Ordering::Relaxed) {
            std::thread::sleep(check_interval);
            if stop.load(Ordering::Relaxed) {
                break;
            }

            if last_heal_at.elapsed() < Duration::from_secs(cooldown_secs) {
                debug!("watchdog: cooldown active, skipping check");
                continue;
            }

            match crate::m4_consolidation::m25_self_heal::check_and_heal(
                &db_path,
                &backup_path,
                cache_ttl.as_secs(),
            ) {
                Ok(health) => {
                    let took_action = health.last_heal_action
                        != Some(HealAction::NoActionNeeded);
                    if took_action {
                        last_heal_at = Instant::now();
                        info!(action = ?health.last_heal_action, "watchdog healed");
                    }
                    if let Ok(mut cached) = cached_clone.write() {
                        *cached = health;
                    }
                }
                Err(e) => warn!(error = %e, "watchdog check failed"),
            }
        }
    });

    WatchdogHandle {
        stop: Arc::clone(stop),
        thread: Some(thread),
        cached_health: cached,
    }
}

/// Read the last cached health status without touching the database.
#[cfg(feature = "sqlite")]
#[must_use]
pub fn read_cached_health(handle: &WatchdogHandle) -> CacheHealth {
    handle
        .cached_health
        .read()
        .map(|h| h.clone())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use crate::m2_schema::m06_schema;
    use crate::m4_consolidation::m26_backup_clone;
    use std::path::Path;

    fn test_db(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("habitat_m28_{name}.db"));
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("db.bak"));
        let _ = std::fs::remove_file(path.with_extension("db.bak.staging"));
        let _ = std::fs::remove_file(path.with_extension("db.corrupt"));
        let _ = std::fs::remove_file(path.with_extension("db-wal"));
        let _ = std::fs::remove_file(path.with_extension("db-shm"));
        let conn = m06_schema::open_database(&path).unwrap();
        drop(conn);
        path
    }

    fn cleanup(path: &Path) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path.with_extension("db.bak"));
        let _ = std::fs::remove_file(path.with_extension("db.bak.staging"));
        let _ = std::fs::remove_file(path.with_extension("db.corrupt"));
        let _ = std::fs::remove_file(path.with_extension("db-wal"));
        let _ = std::fs::remove_file(path.with_extension("db-shm"));
    }

    #[test]
    fn watchdog_starts_and_stops() {
        let path = test_db("wd_start_stop");
        let bak = m26_backup_clone::backup_path(&path);
        let stop = Arc::new(AtomicBool::new(false));

        let handle = start_watchdog(
            path.clone(),
            bak,
            Duration::from_millis(50),
            Duration::from_secs(86400),
            &stop,
        );

        std::thread::sleep(Duration::from_millis(20));
        stop.store(true, Ordering::Relaxed);
        handle.shutdown();
        cleanup(&path);
    }

    #[test]
    fn watchdog_immediate_stop() {
        let path = test_db("wd_imm");
        let bak = m26_backup_clone::backup_path(&path);
        let stop = Arc::new(AtomicBool::new(true));

        let handle = start_watchdog(
            path.clone(),
            bak,
            Duration::from_millis(10),
            Duration::from_secs(86400),
            &stop,
        );

        handle.shutdown();
        cleanup(&path);
    }

    #[test]
    fn watchdog_cached_health_updates() {
        let path = test_db("wd_cached");
        let bak = m26_backup_clone::backup_path(&path);
        let stop = Arc::new(AtomicBool::new(false));

        let handle = start_watchdog(
            path.clone(),
            bak,
            Duration::from_millis(30),
            Duration::from_secs(86400),
            &stop,
        );

        std::thread::sleep(Duration::from_millis(200));
        let health = read_cached_health(&handle);
        assert!(health.db_exists, "watchdog should have run at least once within 200ms");

        stop.store(true, Ordering::Relaxed);
        handle.shutdown();
        cleanup(&path);
    }

    #[test]
    fn watchdog_default_health_before_first_check() {
        let path = test_db("wd_default");
        let bak = m26_backup_clone::backup_path(&path);
        let stop = Arc::new(AtomicBool::new(true));

        let handle = start_watchdog(
            path.clone(),
            bak,
            Duration::from_secs(3600),
            Duration::from_secs(86400),
            &stop,
        );

        let health = read_cached_health(&handle);
        assert!(!health.db_exists);
        handle.shutdown();
        cleanup(&path);
    }

    #[test]
    fn read_cached_health_returns_default_on_new() {
        let h = CacheHealth::default();
        assert!(!h.db_exists);
        assert!(h.last_heal_action.is_none());
    }

    #[test]
    fn watchdog_handle_has_cached_health() {
        let path = test_db("wd_has_cached");
        let bak = m26_backup_clone::backup_path(&path);
        let stop = Arc::new(AtomicBool::new(true));

        let handle = start_watchdog(
            path.clone(),
            bak,
            Duration::from_millis(10),
            Duration::from_secs(86400),
            &stop,
        );

        {
            let _h = handle.cached_health.read().unwrap();
        }
        handle.shutdown();
        cleanup(&path);
    }

    #[test]
    fn watchdog_multiple_cycles() {
        let path = test_db("wd_multi");
        let bak = m26_backup_clone::backup_path(&path);
        let stop = Arc::new(AtomicBool::new(false));

        let handle = start_watchdog(
            path.clone(),
            bak,
            Duration::from_millis(20),
            Duration::from_secs(86400),
            &stop,
        );

        std::thread::sleep(Duration::from_millis(120));
        stop.store(true, Ordering::Relaxed);

        let health = read_cached_health(&handle);
        assert!(health.db_exists);
        handle.shutdown();
        cleanup(&path);
    }
}
