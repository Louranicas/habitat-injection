//! `m26_backup_clone` — Online backup and atomic hot-swap for injection.db.
//!
//! Uses `rusqlite::backup::Backup` for incremental page-by-page copy under a
//! shared lock — concurrent reads proceed unblocked. Writes to a `.staging`
//! file first, verifies integrity, then performs an atomic rename (with
//! cross-filesystem fallback).
//!
//! ## Layer
//!
//! `m4_consolidation`
//!
//! ## Dependencies
//!
//! - L1: `m02_errors::SelfHealError`

#[cfg(feature = "sqlite")]
use std::path::{Path, PathBuf};
#[cfg(feature = "sqlite")]
use std::time::{Duration, Instant};

#[cfg(feature = "sqlite")]
use rusqlite::{Connection, OpenFlags};
#[cfg(feature = "sqlite")]
use tracing::{debug, info, warn};

use serde::{Deserialize, Serialize};

#[cfg(feature = "sqlite")]
use crate::m1_foundation::m02_errors::SelfHealError;

// ---------------------------------------------------------------------------
// Result type
// ---------------------------------------------------------------------------

/// Result of a successful backup creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupResult {
    /// Path to the backup file.
    pub path: PathBuf,
    /// Size of the backup file in bytes.
    pub size_bytes: u64,
    /// Wall-clock time for the backup operation in milliseconds.
    pub elapsed_ms: u64,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Create a backup using the `rusqlite` online backup API (shared lock).
///
/// Writes to a staging path first, verifies integrity, then atomically
/// renames to the final backup path. Concurrent inject reads proceed
/// unblocked during the copy.
///
/// # Errors
///
/// Returns [`SelfHealError`] if the backup, verification, or rename fails.
#[cfg(feature = "sqlite")]
pub fn create_backup(primary_path: &Path) -> Result<BackupResult, SelfHealError> {
    let t_start = Instant::now();
    let staging = staging_path(primary_path);
    let backup_path = backup_path(primary_path);

    let _ = std::fs::remove_file(&staging);

    let src = Connection::open_with_flags(primary_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| SelfHealError::BackupCreationFailed(format!("open source: {e}")))?;
    let mut dst = Connection::open(&staging)
        .map_err(|e| SelfHealError::BackupCreationFailed(format!("open staging: {e}")))?;

    {
        let backup = rusqlite::backup::Backup::new(&src, &mut dst)
            .map_err(|e| SelfHealError::BackupCreationFailed(format!("init backup: {e}")))?;
        backup
            .run_to_completion(100, Duration::from_millis(10), None)
            .map_err(|e| SelfHealError::BackupCreationFailed(format!("backup run: {e}")))?;
    }
    drop(dst);
    drop(src);

    if !verify_integrity_file(&staging)? {
        let _ = std::fs::remove_file(&staging);
        return Err(SelfHealError::BackupVerificationFailed(
            "staging file failed integrity check".into(),
        ));
    }

    atomic_rename_or_copy(&staging, &backup_path)?;

    let size_bytes = std::fs::metadata(&backup_path).map_or(0, |m| m.len());
    let elapsed_ms = u64::try_from(t_start.elapsed().as_millis()).unwrap_or(u64::MAX);

    info!(
        size_bytes,
        elapsed_ms,
        path = %backup_path.display(),
        "backup created"
    );

    Ok(BackupResult {
        path: backup_path,
        size_bytes,
        elapsed_ms,
    })
}

/// Verify `SQLite` integrity of a database file.
///
/// Returns `true` if `PRAGMA integrity_check` reports "ok".
///
/// # Errors
///
/// Returns [`SelfHealError`] if the file cannot be opened.
#[cfg(feature = "sqlite")]
pub fn verify_integrity(db_path: &Path) -> Result<bool, SelfHealError> {
    if !db_path.exists() {
        return Ok(false);
    }
    verify_integrity_file(db_path)
}

/// Swap a verified backup into the primary position.
///
/// Renames the current primary to `.corrupt` (if it exists), then renames
/// the backup to the primary path.
///
/// # Errors
///
/// Returns [`SelfHealError`] if the swap fails.
#[cfg(feature = "sqlite")]
pub fn swap_backup_to_primary(
    primary_path: &Path,
    backup_path: &Path,
) -> Result<(), SelfHealError> {
    if !backup_path.exists() {
        return Err(SelfHealError::AtomicSwapFailed(
            "backup file does not exist".into(),
        ));
    }

    if primary_path.exists() {
        let corrupt = primary_path.with_extension("db.corrupt");
        atomic_rename_or_copy(primary_path, &corrupt)?;
        debug!(from = %primary_path.display(), to = %corrupt.display(), "primary moved to .corrupt");
    }

    atomic_rename_or_copy(backup_path, primary_path)?;
    info!(
        backup = %backup_path.display(),
        primary = %primary_path.display(),
        "backup swapped to primary"
    );
    Ok(())
}

/// Remove orphan files left by interrupted operations.
///
/// Cleans up `.staging` and `.corrupt` files associated with `db_path`.
#[cfg(feature = "sqlite")]
pub fn cleanup_orphans(db_path: &Path) {
    for ext in &["db.bak.staging", "db.corrupt"] {
        let orphan = db_path.with_extension(ext);
        if orphan.exists() {
            match std::fs::remove_file(&orphan) {
                Ok(()) => debug!(path = %orphan.display(), "orphan removed"),
                Err(e) => warn!(path = %orphan.display(), error = %e, "orphan removal failed"),
            }
        }
    }
}

/// Return the canonical backup path for a given primary database path.
#[cfg(feature = "sqlite")]
#[must_use]
pub fn backup_path(primary: &Path) -> PathBuf {
    primary.with_extension("db.bak")
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

#[cfg(feature = "sqlite")]
fn staging_path(primary: &Path) -> PathBuf {
    primary.with_extension("db.bak.staging")
}

#[cfg(feature = "sqlite")]
fn verify_integrity_file(path: &Path) -> Result<bool, SelfHealError> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| SelfHealError::IntegrityCheckFailed(format!("open: {e}")))?;
    let result: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(|e| SelfHealError::IntegrityCheckFailed(format!("pragma: {e}")))?;
    Ok(result == "ok")
}

/// Rename with cross-filesystem fallback: copy + remove when rename fails.
#[cfg(feature = "sqlite")]
fn atomic_rename_or_copy(from: &Path, to: &Path) -> Result<(), SelfHealError> {
    match std::fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(rename_err) => {
            debug!(
                from = %from.display(),
                to = %to.display(),
                error = %rename_err,
                "rename failed, falling back to copy+remove"
            );
            std::fs::copy(from, to).map_err(|e| {
                SelfHealError::AtomicSwapFailed(format!("copy fallback: {e}"))
            })?;
            let _ = std::fs::remove_file(from);
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use crate::m2_schema::m06_schema;

    fn test_db(name: &str) -> (PathBuf, Connection) {
        let path = std::env::temp_dir().join(format!("habitat_m26_{name}.db"));
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("db.bak"));
        let _ = std::fs::remove_file(path.with_extension("db.bak.staging"));
        let _ = std::fs::remove_file(path.with_extension("db.corrupt"));
        let conn = m06_schema::open_database(&path).unwrap();
        (path, conn)
    }

    fn seed_data(conn: &Connection) {
        conn.execute(
            "INSERT INTO causal_chain (origin_session, chain_type, label, description)
             VALUES (1, 'bug', 'test-chain', 'test description')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO session_trajectory
                 (session_id, ralph_fitness, field_r, thermal_t, ltp_ltd_ratio,
                  services_healthy, delta_summary)
             VALUES (100, 0.7, 0.5, 0.3, 2.0, 11, 'test')",
            [],
        )
        .unwrap();
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
    fn create_backup_succeeds_on_populated_db() {
        let (path, conn) = test_db("backup_pop");
        seed_data(&conn);
        drop(conn);
        let result = create_backup(&path).unwrap();
        assert!(result.path.exists());
        assert!(result.size_bytes > 0);
        cleanup(&path);
    }

    #[test]
    fn create_backup_succeeds_on_empty_db() {
        let (path, conn) = test_db("backup_empty");
        drop(conn);
        let result = create_backup(&path).unwrap();
        assert!(result.path.exists());
        assert!(result.size_bytes > 0);
        cleanup(&path);
    }

    #[test]
    fn create_backup_result_path_is_bak() {
        let (path, conn) = test_db("backup_path");
        drop(conn);
        let result = create_backup(&path).unwrap();
        assert!(result.path.to_string_lossy().ends_with(".db.bak"));
        cleanup(&path);
    }

    #[test]
    fn create_backup_staging_removed() {
        let (path, conn) = test_db("backup_staging");
        drop(conn);
        create_backup(&path).unwrap();
        let staging = staging_path(&path);
        assert!(!staging.exists());
        cleanup(&path);
    }

    #[test]
    fn create_backup_elapsed_ms_reasonable() {
        let (path, conn) = test_db("backup_elapsed");
        drop(conn);
        let result = create_backup(&path).unwrap();
        assert!(result.elapsed_ms < 5000);
        cleanup(&path);
    }

    #[test]
    fn create_backup_preserves_data() {
        let (path, conn) = test_db("backup_data");
        seed_data(&conn);
        drop(conn);
        let result = create_backup(&path).unwrap();

        let bak_conn =
            Connection::open_with_flags(&result.path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let label: String = bak_conn
            .query_row(
                "SELECT label FROM causal_chain WHERE label = 'test-chain'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(label, "test-chain");
        cleanup(&path);
    }

    #[test]
    fn create_backup_is_idempotent() {
        let (path, conn) = test_db("backup_idemp");
        seed_data(&conn);
        drop(conn);
        let r1 = create_backup(&path).unwrap();
        let r2 = create_backup(&path).unwrap();
        assert_eq!(r1.size_bytes, r2.size_bytes);
        cleanup(&path);
    }

    #[test]
    fn create_backup_independent_of_primary() {
        let (path, conn) = test_db("backup_indep");
        seed_data(&conn);
        drop(conn);
        create_backup(&path).unwrap();

        let primary = Connection::open(&path).unwrap();
        primary
            .execute(
                "INSERT INTO causal_chain (origin_session, chain_type, label, description)
                 VALUES (2, 'trap', 'new-chain', 'added after backup')",
                [],
            )
            .unwrap();
        drop(primary);

        let bak = Connection::open_with_flags(
            backup_path(&path),
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let count: i64 = bak
            .query_row("SELECT COUNT(*) FROM causal_chain", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1, "backup should not contain post-backup data");
        cleanup(&path);
    }

    #[test]
    fn verify_integrity_valid_db() {
        let (path, conn) = test_db("integ_valid");
        seed_data(&conn);
        drop(conn);
        assert!(verify_integrity(&path).unwrap());
        cleanup(&path);
    }

    #[test]
    fn verify_integrity_missing_file() {
        let path = std::env::temp_dir().join("habitat_m26_integ_missing.db");
        let _ = std::fs::remove_file(&path);
        assert!(!verify_integrity(&path).unwrap());
    }

    #[test]
    fn verify_integrity_corrupt_file() {
        let path = std::env::temp_dir().join("habitat_m26_integ_corrupt.db");
        std::fs::write(&path, b"this is not a sqlite database").unwrap();
        let result = verify_integrity(&path);
        assert!(result.is_err() || !result.unwrap());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn swap_backup_to_primary_succeeds() {
        let (path, conn) = test_db("swap_ok");
        seed_data(&conn);
        drop(conn);
        create_backup(&path).unwrap();

        let _ = std::fs::remove_file(&path);
        swap_backup_to_primary(&path, &backup_path(&path)).unwrap();
        assert!(path.exists());
        assert!(!backup_path(&path).exists());
        cleanup(&path);
    }

    #[test]
    fn swap_backup_preserves_data() {
        let (path, conn) = test_db("swap_data");
        seed_data(&conn);
        drop(conn);
        create_backup(&path).unwrap();

        let _ = std::fs::remove_file(&path);
        swap_backup_to_primary(&path, &backup_path(&path)).unwrap();

        let conn =
            Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let label: String = conn
            .query_row(
                "SELECT label FROM causal_chain WHERE label = 'test-chain'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(label, "test-chain");
        cleanup(&path);
    }

    #[test]
    fn swap_backup_moves_primary_to_corrupt() {
        let (path, conn) = test_db("swap_corrupt");
        seed_data(&conn);
        drop(conn);
        create_backup(&path).unwrap();

        swap_backup_to_primary(&path, &backup_path(&path)).unwrap();
        let corrupt = path.with_extension("db.corrupt");
        assert!(corrupt.exists());
        cleanup(&path);
    }

    #[test]
    fn swap_backup_missing_errors() {
        let path = std::env::temp_dir().join("habitat_m26_swap_missing.db");
        let backup = path.with_extension("db.bak");
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&backup);
        let result = swap_backup_to_primary(&path, &backup);
        assert!(result.is_err());
    }

    #[test]
    fn cleanup_orphans_removes_staging() {
        let path = std::env::temp_dir().join("habitat_m26_orphan_staging.db");
        let staging = path.with_extension("db.bak.staging");
        std::fs::write(&staging, b"orphan").unwrap();
        cleanup_orphans(&path);
        assert!(!staging.exists());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn cleanup_orphans_removes_corrupt() {
        let path = std::env::temp_dir().join("habitat_m26_orphan_corrupt.db");
        let corrupt = path.with_extension("db.corrupt");
        std::fs::write(&corrupt, b"orphan").unwrap();
        cleanup_orphans(&path);
        assert!(!corrupt.exists());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn cleanup_orphans_no_files_is_noop() {
        let path = std::env::temp_dir().join("habitat_m26_orphan_none.db");
        let _ = std::fs::remove_file(path.with_extension("db.bak.staging"));
        let _ = std::fs::remove_file(path.with_extension("db.corrupt"));
        cleanup_orphans(&path);
    }

    #[test]
    fn backup_path_canonical() {
        let p = PathBuf::from("/tmp/test.db");
        assert_eq!(backup_path(&p), PathBuf::from("/tmp/test.db.bak"));
    }

    #[test]
    fn staging_path_canonical() {
        let p = PathBuf::from("/tmp/test.db");
        assert_eq!(staging_path(&p), PathBuf::from("/tmp/test.db.bak.staging"));
    }

    #[test]
    fn atomic_rename_succeeds() {
        let from = std::env::temp_dir().join("habitat_m26_rename_from.tmp");
        let to = std::env::temp_dir().join("habitat_m26_rename_to.tmp");
        std::fs::write(&from, b"content").unwrap();
        let _ = std::fs::remove_file(&to);
        atomic_rename_or_copy(&from, &to).unwrap();
        assert!(!from.exists());
        assert!(to.exists());
        let _ = std::fs::remove_file(&to);
    }

    #[test]
    fn backup_result_serializable() {
        let r = BackupResult {
            path: PathBuf::from("/tmp/test.db.bak"),
            size_bytes: 4096,
            elapsed_ms: 15,
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: BackupResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.size_bytes, 4096);
        assert_eq!(back.elapsed_ms, 15);
    }

    #[test]
    fn backup_result_debug() {
        let r = BackupResult {
            path: PathBuf::from("/tmp/test.db.bak"),
            size_bytes: 4096,
            elapsed_ms: 15,
        };
        let dbg = format!("{r:?}");
        assert!(dbg.contains("BackupResult"));
        assert!(dbg.contains("4096"));
    }

    #[test]
    fn backup_result_clone() {
        let r1 = BackupResult {
            path: PathBuf::from("/tmp/test.db.bak"),
            size_bytes: 4096,
            elapsed_ms: 15,
        };
        let r2 = r1.clone();
        assert_eq!(r1.size_bytes, r2.size_bytes);
    }

    #[test]
    fn create_backup_errors_on_missing_primary() {
        let path = std::env::temp_dir().join("habitat_m26_missing_primary.db");
        let _ = std::fs::remove_file(&path);
        let result = create_backup(&path);
        assert!(result.is_err());
    }

    #[test]
    fn create_then_verify_backup() {
        let (path, conn) = test_db("bak_verify");
        seed_data(&conn);
        drop(conn);
        let result = create_backup(&path).unwrap();
        assert!(verify_integrity(&result.path).unwrap());
        cleanup(&path);
    }

    #[test]
    fn full_cycle_create_verify_swap() {
        let (path, conn) = test_db("full_cycle");
        seed_data(&conn);
        drop(conn);

        create_backup(&path).unwrap();
        let bak = backup_path(&path);
        assert!(verify_integrity(&bak).unwrap());

        let _ = std::fs::remove_file(&path);
        swap_backup_to_primary(&path, &bak).unwrap();
        assert!(verify_integrity(&path).unwrap());
        cleanup(&path);
    }

    #[test]
    fn cleanup_then_create_backup() {
        let (path, conn) = test_db("clean_create");
        seed_data(&conn);
        drop(conn);

        let staging = staging_path(&path);
        std::fs::write(&staging, b"leftover").unwrap();

        cleanup_orphans(&path);
        assert!(!staging.exists());

        let result = create_backup(&path).unwrap();
        assert!(result.path.exists());
        cleanup(&path);
    }

    #[test]
    fn backup_of_backup_works() {
        let (path, conn) = test_db("bak_of_bak");
        seed_data(&conn);
        drop(conn);
        create_backup(&path).unwrap();
        let bak = backup_path(&path);
        let result = create_backup(&bak);
        assert!(result.is_ok());
        let _ = std::fs::remove_file(backup_path(&bak));
        cleanup(&path);
    }

    #[test]
    fn swap_when_primary_does_not_exist() {
        let path = std::env::temp_dir().join("habitat_m26_swap_noprimary.db");
        let bak = path.with_extension("db.bak");
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&bak);

        let conn = m06_schema::open_database(&bak).unwrap();
        drop(conn);

        swap_backup_to_primary(&path, &bak).unwrap();
        assert!(path.exists());
        assert!(!bak.exists());
        cleanup(&path);
    }

    #[test]
    fn verify_integrity_backup_file() {
        let (path, conn) = test_db("verify_bak");
        seed_data(&conn);
        drop(conn);
        create_backup(&path).unwrap();
        assert!(verify_integrity(&backup_path(&path)).unwrap());
        cleanup(&path);
    }

    #[test]
    fn multiple_backups_overwrite() {
        let (path, conn) = test_db("multi_bak");
        seed_data(&conn);
        drop(conn);

        let r1 = create_backup(&path).unwrap();
        let s1 = r1.size_bytes;

        let conn2 = Connection::open(&path).unwrap();
        for i in 0..100 {
            conn2
                .execute(
                    "INSERT INTO causal_chain (origin_session, chain_type, label, description)
                     VALUES (?1, 'bug', ?2, 'padding')",
                    rusqlite::params![i + 10, format!("chain-{i}")],
                )
                .unwrap();
        }
        drop(conn2);

        let r2 = create_backup(&path).unwrap();
        assert!(r2.size_bytes >= s1);
        cleanup(&path);
    }
}
