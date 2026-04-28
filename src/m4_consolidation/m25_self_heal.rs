//! `m25_self_heal` — Self-healing orchestrator for injection.db.
//!
//! Checks database health (existence, integrity, schema, cache freshness,
//! backup age) and performs the minimal corrective action. All APIs take
//! `db_path: &Path` — each thread opens its own `Connection`.
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
use std::path::Path;
#[cfg(feature = "sqlite")]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(feature = "sqlite")]
use tracing::{debug, info, warn};

use serde::{Deserialize, Serialize};

#[cfg(feature = "sqlite")]
use crate::m1_foundation::m02_errors::SelfHealError;
#[cfg(feature = "sqlite")]
use crate::m1_foundation::m05_constants::BACKUP_MAX_AGE_SECS;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Action taken by the self-heal orchestrator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealAction {
    /// Everything healthy — no action taken.
    NoActionNeeded,
    /// Primary DB was missing; swapped from backup.
    SwappedFromBackup,
    /// Primary DB was missing or corrupt with no backup; recreated from scratch.
    Rebuilt,
    /// Integrity check failed; repaired by swapping from backup.
    RepairedIntegrity,
    /// Backup was missing or stale; created a fresh one.
    CreatedBackup,
    /// Cache was stale; rebuilt from database tables.
    RebuildCache,
}

/// Health status of the injection database and cache.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct CacheHealth {
    /// Whether the primary database file exists.
    pub db_exists: bool,
    /// Whether `PRAGMA integrity_check` passes.
    pub db_integrity_ok: bool,
    /// Whether the injection cache row is fresh.
    pub cache_fresh: bool,
    /// Whether a backup file exists.
    pub backup_exists: bool,
    /// Whether the backup is younger than `BACKUP_MAX_AGE_SECS`.
    pub backup_fresh: bool,
    /// The corrective action taken, if any.
    pub last_heal_action: Option<HealAction>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Check database health and perform the minimal corrective action.
///
/// Decision tree (executed top-to-bottom, first match wins):
///
/// 1. Clean up orphan `.staging` / `.corrupt` files.
/// 2. DB missing? → swap from backup if available, else recreate.
/// 3. Integrity check fails? → swap from backup if available, else recreate.
/// 4. Schema version 0 or error? → run migrations.
/// 5. Cache stale or missing? → lightweight rebuild.
/// 6. Backup missing or stale? → create fresh backup.
/// 7. All healthy → no action.
///
/// # Errors
///
/// Returns [`SelfHealError`] only if an attempted heal action itself fails.
#[cfg(feature = "sqlite")]
pub fn check_and_heal(
    db_path: &Path,
    backup_path: &Path,
    cache_ttl_secs: u64,
) -> Result<CacheHealth, SelfHealError> {
    use crate::m2_schema::m06_schema;
    use crate::m3_injection::m13b_cache_light;
    use crate::m4_consolidation::m26_backup_clone;

    let mut health = CacheHealth::default();

    // Step 1: orphan cleanup
    m26_backup_clone::cleanup_orphans(db_path);

    // Step 2: DB missing?
    if !db_path.exists() {
        if backup_path.exists() && m26_backup_clone::verify_integrity(backup_path).unwrap_or(false)
        {
            m26_backup_clone::swap_backup_to_primary(db_path, backup_path)?;
            health.db_exists = true;
            health.db_integrity_ok = true;
            health.last_heal_action = Some(HealAction::SwappedFromBackup);
            info!("self-heal: DB missing, swapped from backup");
            return Ok(health);
        }
        m06_schema::open_database(db_path).map_err(|e| {
            SelfHealError::SchemaMigrationFailed(format!("recreate DB: {e}"))
        })?;
        health.db_exists = true;
        health.db_integrity_ok = true;
        health.last_heal_action = Some(HealAction::Rebuilt);
        info!("self-heal: DB missing, no backup, recreated");
        return Ok(health);
    }
    health.db_exists = true;

    // Step 3: integrity check
    let integrity_ok = m26_backup_clone::verify_integrity(db_path).unwrap_or(false);
    if !integrity_ok {
        if backup_path.exists() && m26_backup_clone::verify_integrity(backup_path).unwrap_or(false)
        {
            m26_backup_clone::swap_backup_to_primary(db_path, backup_path)?;
            health.db_integrity_ok = true;
            health.last_heal_action = Some(HealAction::RepairedIntegrity);
            info!("self-heal: integrity failed, swapped from backup");
            return Ok(health);
        }
        let _ = std::fs::remove_file(db_path);
        m06_schema::open_database(db_path).map_err(|e| {
            SelfHealError::SchemaMigrationFailed(format!("recreate after corruption: {e}"))
        })?;
        health.db_integrity_ok = true;
        health.last_heal_action = Some(HealAction::Rebuilt);
        info!("self-heal: integrity failed, no backup, recreated");
        return Ok(health);
    }
    health.db_integrity_ok = true;

    // Step 4: schema version check
    let conn = m06_schema::open_database(db_path).map_err(|e| {
        SelfHealError::SchemaMigrationFailed(format!("open for schema check: {e}"))
    })?;
    let version = m06_schema::schema_version(&conn).unwrap_or(0);
    if version == 0 {
        drop(conn);
        m06_schema::open_database(db_path).map_err(|e| {
            SelfHealError::SchemaMigrationFailed(format!("migration on v0: {e}"))
        })?;
        health.last_heal_action = Some(HealAction::Rebuilt);
        info!("self-heal: schema version 0, ran migrations");
        return Ok(health);
    }

    // Step 5: cache freshness
    let cache_fresh = is_cache_fresh(&conn, cache_ttl_secs);
    health.cache_fresh = cache_fresh;
    if !cache_fresh {
        match m13b_cache_light::rebuild_cache_light(&conn) {
            Ok(result) => {
                health.cache_fresh = true;
                health.last_heal_action = Some(HealAction::RebuildCache);
                debug!(tokens = result.token_count, "self-heal: cache rebuilt");
            }
            Err(e) => {
                warn!(error = %e, "self-heal: cache rebuild failed (non-fatal)");
            }
        }
    }
    drop(conn);

    // Step 6: backup freshness
    health.backup_exists = backup_path.exists();
    health.backup_fresh = health.backup_exists && is_file_fresh(backup_path, BACKUP_MAX_AGE_SECS);

    if !health.backup_exists || !health.backup_fresh {
        match m26_backup_clone::create_backup(db_path) {
            Ok(_) => {
                health.backup_exists = true;
                health.backup_fresh = true;
                if health.last_heal_action.is_none() {
                    health.last_heal_action = Some(HealAction::CreatedBackup);
                }
                debug!("self-heal: backup created/refreshed");
            }
            Err(e) => {
                warn!(error = %e, "self-heal: backup creation failed (non-fatal)");
            }
        }
    }

    if health.last_heal_action.is_none() {
        health.last_heal_action = Some(HealAction::NoActionNeeded);
    }

    Ok(health)
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

#[cfg(feature = "sqlite")]
fn is_cache_fresh(conn: &rusqlite::Connection, ttl_secs: u64) -> bool {
    let key = crate::m3_injection::m11_parallel_query::CACHE_SECTION_KEY;
    let result: Result<i64, _> = conn.query_row(
        "SELECT computed_at FROM injection_cache WHERE section = ?1",
        rusqlite::params![key],
        |r| r.get(0),
    );
    match result {
        Ok(ts) => {
            let now = now_secs();
            let ts_u64 = u64::try_from(ts.max(0)).unwrap_or(0);
            let age = now.saturating_sub(ts_u64);
            age <= ttl_secs
        }
        Err(_) => false,
    }
}

#[cfg(feature = "sqlite")]
fn is_file_fresh(path: &Path, max_age_secs: u64) -> bool {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .is_ok_and(|modified| {
            modified
                .elapsed()
                .is_ok_and(|age| age.as_secs() <= max_age_secs)
        })
}

#[cfg(feature = "sqlite")]
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use crate::m2_schema::m06_schema;
    use crate::m4_consolidation::m26_backup_clone;
    use std::path::PathBuf;

    fn test_db(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("habitat_m25_{name}.db"));
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("db.bak"));
        let _ = std::fs::remove_file(path.with_extension("db.bak.staging"));
        let _ = std::fs::remove_file(path.with_extension("db.corrupt"));
        let _ = std::fs::remove_file(path.with_extension("db-wal"));
        let _ = std::fs::remove_file(path.with_extension("db-shm"));
        path
    }

    fn seed_with_fresh_cache(path: &Path) {
        let conn = m06_schema::open_database(path).unwrap();
        conn.execute(
            "INSERT INTO causal_chain (origin_session, chain_type, label, description)
             VALUES (1, 'bug', 'test-bug', 'for testing')",
            [],
        )
        .unwrap();
        crate::m3_injection::m13b_cache_light::rebuild_cache_light(&conn).unwrap();
        drop(conn);
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
    fn healthy_db_no_action() {
        let path = test_db("healthy");
        seed_with_fresh_cache(&path);
        m26_backup_clone::create_backup(&path).unwrap();

        let bak = m26_backup_clone::backup_path(&path);
        let health = check_and_heal(&path, &bak, 86400).unwrap();
        assert!(health.db_exists);
        assert!(health.db_integrity_ok);
        assert!(health.cache_fresh);
        assert!(health.backup_exists);
        assert_eq!(health.last_heal_action, Some(HealAction::NoActionNeeded));
        cleanup(&path);
    }

    #[test]
    fn missing_db_no_backup_rebuilds() {
        let path = test_db("missing_nob");
        let bak = m26_backup_clone::backup_path(&path);
        let health = check_and_heal(&path, &bak, 86400).unwrap();
        assert!(health.db_exists);
        assert_eq!(health.last_heal_action, Some(HealAction::Rebuilt));
        assert!(path.exists());
        cleanup(&path);
    }

    #[test]
    fn missing_db_with_backup_swaps() {
        let path = test_db("missing_bak");
        seed_with_fresh_cache(&path);
        m26_backup_clone::create_backup(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("db-wal"));
        let _ = std::fs::remove_file(path.with_extension("db-shm"));

        let bak = m26_backup_clone::backup_path(&path);
        let health = check_and_heal(&path, &bak, 86400).unwrap();
        assert!(health.db_exists);
        assert_eq!(health.last_heal_action, Some(HealAction::SwappedFromBackup));
        assert!(path.exists());
        cleanup(&path);
    }

    #[test]
    fn corrupt_db_no_backup_rebuilds() {
        let path = test_db("corrupt_nob");
        std::fs::write(&path, b"not a sqlite file").unwrap();
        let bak = m26_backup_clone::backup_path(&path);
        let health = check_and_heal(&path, &bak, 86400).unwrap();
        assert!(health.db_exists);
        assert_eq!(health.last_heal_action, Some(HealAction::Rebuilt));
        cleanup(&path);
    }

    #[test]
    fn corrupt_db_with_backup_repairs() {
        let path = test_db("corrupt_bak");
        seed_with_fresh_cache(&path);
        m26_backup_clone::create_backup(&path).unwrap();
        std::fs::write(&path, b"corrupted").unwrap();

        let bak = m26_backup_clone::backup_path(&path);
        let health = check_and_heal(&path, &bak, 86400).unwrap();
        assert_eq!(
            health.last_heal_action,
            Some(HealAction::RepairedIntegrity)
        );
        cleanup(&path);
    }

    #[test]
    fn stale_cache_triggers_rebuild() {
        let path = test_db("stale_cache");
        let conn = m06_schema::open_database(&path).unwrap();
        let stale_ts = now_secs().saturating_sub(100_000);
        let key = crate::m3_injection::m11_parallel_query::CACHE_SECTION_KEY;
        conn.execute(
            "INSERT INTO injection_cache (section, payload, token_count, computed_at)
             VALUES (?1, 'old', 1, ?2)",
            rusqlite::params![key, stale_ts as i64],
        )
        .unwrap();
        drop(conn);

        let bak = m26_backup_clone::backup_path(&path);
        let health = check_and_heal(&path, &bak, 300).unwrap();
        assert!(health.cache_fresh);
        assert_eq!(health.last_heal_action, Some(HealAction::RebuildCache));
        cleanup(&path);
    }

    #[test]
    fn missing_backup_creates_one() {
        let path = test_db("no_backup");
        seed_with_fresh_cache(&path);
        let bak = m26_backup_clone::backup_path(&path);
        assert!(!bak.exists());

        let health = check_and_heal(&path, &bak, 86400).unwrap();
        assert!(health.backup_exists);
        assert!(health.backup_fresh);
        cleanup(&path);
    }

    #[test]
    fn cache_health_default() {
        let h = CacheHealth::default();
        assert!(!h.db_exists);
        assert!(!h.db_integrity_ok);
        assert!(!h.cache_fresh);
        assert!(!h.backup_exists);
        assert!(!h.backup_fresh);
        assert!(h.last_heal_action.is_none());
    }

    #[test]
    fn cache_health_debug() {
        let h = CacheHealth::default();
        let dbg = format!("{h:?}");
        assert!(dbg.contains("CacheHealth"));
    }

    #[test]
    fn cache_health_clone() {
        let h = CacheHealth {
            db_exists: true,
            last_heal_action: Some(HealAction::Rebuilt),
            ..CacheHealth::default()
        };
        let h2 = h.clone();
        assert_eq!(h2.last_heal_action, Some(HealAction::Rebuilt));
    }

    #[test]
    fn cache_health_serializable() {
        let h = CacheHealth {
            db_exists: true,
            db_integrity_ok: true,
            cache_fresh: true,
            backup_exists: true,
            backup_fresh: true,
            last_heal_action: Some(HealAction::NoActionNeeded),
        };
        let json = serde_json::to_string(&h).unwrap();
        let back: CacheHealth = serde_json::from_str(&json).unwrap();
        assert!(back.db_exists);
        assert_eq!(back.last_heal_action, Some(HealAction::NoActionNeeded));
    }

    #[test]
    fn heal_action_eq() {
        assert_eq!(HealAction::Rebuilt, HealAction::Rebuilt);
        assert_ne!(HealAction::Rebuilt, HealAction::CreatedBackup);
    }

    #[test]
    fn heal_action_serializable() {
        let a = HealAction::SwappedFromBackup;
        let json = serde_json::to_string(&a).unwrap();
        let back: HealAction = serde_json::from_str(&json).unwrap();
        assert_eq!(back, HealAction::SwappedFromBackup);
    }

    #[test]
    fn heal_action_debug() {
        let dbg = format!("{:?}", HealAction::RepairedIntegrity);
        assert!(dbg.contains("RepairedIntegrity"));
    }

    #[test]
    fn is_cache_fresh_with_fresh_row() {
        let conn = m06_schema::open_memory().unwrap();
        let key = crate::m3_injection::m11_parallel_query::CACHE_SECTION_KEY;
        conn.execute(
            "INSERT INTO injection_cache (section, payload, token_count, computed_at)
             VALUES (?1, 'p', 1, ?2)",
            rusqlite::params![key, now_secs() as i64],
        )
        .unwrap();
        assert!(is_cache_fresh(&conn, 300));
    }

    #[test]
    fn is_cache_fresh_with_stale_row() {
        let conn = m06_schema::open_memory().unwrap();
        let key = crate::m3_injection::m11_parallel_query::CACHE_SECTION_KEY;
        let old = now_secs().saturating_sub(1000);
        conn.execute(
            "INSERT INTO injection_cache (section, payload, token_count, computed_at)
             VALUES (?1, 'p', 1, ?2)",
            rusqlite::params![key, old as i64],
        )
        .unwrap();
        assert!(!is_cache_fresh(&conn, 300));
    }

    #[test]
    fn is_cache_fresh_with_no_row() {
        let conn = m06_schema::open_memory().unwrap();
        assert!(!is_cache_fresh(&conn, 300));
    }

    #[test]
    fn is_file_fresh_recent_file() {
        let path = std::env::temp_dir().join("habitat_m25_fresh.tmp");
        std::fs::write(&path, b"fresh").unwrap();
        assert!(is_file_fresh(&path, 60));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn is_file_fresh_missing_file() {
        let path = std::env::temp_dir().join("habitat_m25_nofile.tmp");
        let _ = std::fs::remove_file(&path);
        assert!(!is_file_fresh(&path, 60));
    }

    #[test]
    fn now_secs_positive() {
        assert!(now_secs() > 1_700_000_000);
    }

    #[test]
    fn full_heal_cycle_from_scratch() {
        let path = test_db("full_cycle");
        let bak = m26_backup_clone::backup_path(&path);

        // First call: DB missing → rebuild
        let h1 = check_and_heal(&path, &bak, 86400).unwrap();
        assert_eq!(h1.last_heal_action, Some(HealAction::Rebuilt));

        // Second call: DB exists but no cache → rebuild cache + create backup
        let h2 = check_and_heal(&path, &bak, 86400).unwrap();
        assert!(
            h2.last_heal_action == Some(HealAction::RebuildCache)
                || h2.last_heal_action == Some(HealAction::CreatedBackup)
        );

        // Third call: everything fresh → no action
        let h3 = check_and_heal(&path, &bak, 86400).unwrap();
        assert_eq!(h3.last_heal_action, Some(HealAction::NoActionNeeded));
        cleanup(&path);
    }

    #[test]
    fn heal_orphan_cleanup_runs() {
        let path = test_db("orphan_heal");
        seed_with_fresh_cache(&path);
        m26_backup_clone::create_backup(&path).unwrap();

        let staging = path.with_extension("db.bak.staging");
        std::fs::write(&staging, b"leftover").unwrap();

        let bak = m26_backup_clone::backup_path(&path);
        check_and_heal(&path, &bak, 86400).unwrap();
        assert!(!staging.exists());
        cleanup(&path);
    }

    #[test]
    fn heal_all_actions_enumerated() {
        let actions = [
            HealAction::NoActionNeeded,
            HealAction::SwappedFromBackup,
            HealAction::Rebuilt,
            HealAction::RepairedIntegrity,
            HealAction::CreatedBackup,
            HealAction::RebuildCache,
        ];
        for a in &actions {
            let json = serde_json::to_string(a).unwrap();
            let back: HealAction = serde_json::from_str(&json).unwrap();
            assert_eq!(*a, back);
        }
    }

    #[test]
    fn heal_corrupt_backup_does_not_swap() {
        let path = test_db("corrupt_backup");
        seed_with_fresh_cache(&path);
        let bak = m26_backup_clone::backup_path(&path);
        std::fs::write(&bak, b"not sqlite").unwrap();
        std::fs::write(&path, b"corrupted primary").unwrap();

        let health = check_and_heal(&path, &bak, 86400).unwrap();
        assert_eq!(health.last_heal_action, Some(HealAction::Rebuilt));
        cleanup(&path);
    }
}
