//! Schema creation, migration runner, and version tracking.
//!
//! Idempotent — safe to call on an existing database.

#[cfg(feature = "sqlite")]
use std::path::Path;

#[cfg(feature = "sqlite")]
use rusqlite::Connection;

#[cfg(feature = "sqlite")]
use crate::m1_foundation::m02_errors::SchemaError;

#[cfg(feature = "sqlite")]
const CURRENT_VERSION: u32 = 5;

/// Open (or create) the injection database and ensure schema is current.
///
/// # Errors
///
/// Returns [`SchemaError`] on database open or migration failure.
#[cfg(feature = "sqlite")]
pub fn open_database(path: &Path) -> Result<Connection, SchemaError> {
    let needs_create = !path.exists();

    if let Some(parent) = path.parent().filter(|p| !p.exists()) {
        std::fs::create_dir_all(parent).map_err(|e| SchemaError::DatabaseOpenFailed {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })?;
    }

    let conn = Connection::open(path).map_err(|e| SchemaError::DatabaseOpenFailed {
        path: path.to_path_buf(),
        reason: e.to_string(),
    })?;

    configure_connection(&conn)?;

    if needs_create {
        create_all_tables(&conn)?;
        set_schema_version(&conn, CURRENT_VERSION)?;
    } else {
        let version = get_schema_version(&conn)?;
        if version < CURRENT_VERSION {
            migrate(&conn, version, CURRENT_VERSION)?;
        }
    }

    Ok(conn)
}

/// Open an in-memory database with full schema. Useful for tests.
///
/// # Errors
///
/// Returns [`SchemaError`] if schema creation fails.
#[cfg(feature = "sqlite")]
pub fn open_memory() -> Result<Connection, SchemaError> {
    let conn = Connection::open_in_memory().map_err(|e| SchemaError::DatabaseOpenFailed {
        path: ":memory:".into(),
        reason: e.to_string(),
    })?;
    configure_connection(&conn)?;
    create_all_tables(&conn)?;
    set_schema_version(&conn, CURRENT_VERSION)?;
    Ok(conn)
}

#[cfg(feature = "sqlite")]
fn configure_connection(conn: &Connection) -> Result<(), SchemaError> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA busy_timeout = 5000;
         PRAGMA foreign_keys = ON;
         PRAGMA synchronous = NORMAL;
         PRAGMA wal_autocheckpoint = 100;",
    )
    .map_err(|e| SchemaError::Sqlite(e.to_string()))
}

/// Create all 7 tables and their indexes.
///
/// # Errors
///
/// Returns [`SchemaError::TableCreationFailed`] on failure.
#[cfg(feature = "sqlite")]
pub fn create_all_tables(conn: &Connection) -> Result<(), SchemaError> {
    create_causal_chain(conn)?;
    create_session_trajectory(conn)?;
    create_workstream(conn)?;
    create_reinforced_pattern(conn)?;
    create_injection_cache(conn)?;
    create_session_checkpoint(conn)?;
    create_injection_script(conn)?;
    create_daemon_state(conn)?;
    Ok(())
}

#[cfg(feature = "sqlite")]
fn create_causal_chain(conn: &Connection) -> Result<(), SchemaError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS causal_chain (
            id                      INTEGER PRIMARY KEY AUTOINCREMENT,
            origin_session          INTEGER NOT NULL,
            resolved_session        INTEGER,
            chain_type              TEXT NOT NULL CHECK(chain_type IN ('bug', 'trap', 'plan', 'pattern')),
            label                   TEXT NOT NULL,
            description             TEXT NOT NULL,
            reinforcement_count     INTEGER NOT NULL DEFAULT 1,
            last_reinforced_session INTEGER,
            consent                 TEXT NOT NULL DEFAULT 'Emit'
                CHECK(consent IN ('Emit', 'Store', 'Forget')),
            created_at              TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
            updated_at              TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
        );
        CREATE INDEX IF NOT EXISTS idx_causal_unresolved
            ON causal_chain(reinforcement_count DESC)
            WHERE resolved_session IS NULL;
        CREATE INDEX IF NOT EXISTS idx_causal_label ON causal_chain(label);",
    )
    .map_err(|e| SchemaError::TableCreationFailed {
        table: "causal_chain".into(),
        reason: e.to_string(),
    })
}

#[cfg(feature = "sqlite")]
fn create_session_trajectory(conn: &Connection) -> Result<(), SchemaError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS session_trajectory (
            session_id          INTEGER PRIMARY KEY,
            ralph_fitness       REAL NOT NULL,
            field_r             REAL NOT NULL,
            thermal_t           REAL NOT NULL,
            ltp_ltd_ratio       REAL NOT NULL,
            services_healthy    INTEGER NOT NULL,
            delta_summary       TEXT NOT NULL,
            key_achievement     TEXT,
            consent             TEXT NOT NULL DEFAULT 'Emit'
                CHECK(consent IN ('Emit', 'Store', 'Forget'))
        );
        CREATE INDEX IF NOT EXISTS idx_trajectory_recent
            ON session_trajectory(session_id DESC);",
    )
    .map_err(|e| SchemaError::TableCreationFailed {
        table: "session_trajectory".into(),
        reason: e.to_string(),
    })
}

#[cfg(feature = "sqlite")]
fn create_workstream(conn: &Connection) -> Result<(), SchemaError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS workstream (
            ws_id                TEXT PRIMARY KEY,
            title                TEXT NOT NULL,
            status               TEXT NOT NULL CHECK(status IN ('active', 'blocked', 'deferred', 'complete')),
            blocker              TEXT,
            priority             INTEGER NOT NULL DEFAULT 5,
            last_touched_session INTEGER NOT NULL,
            items_total          INTEGER,
            items_done           INTEGER,
            resume_context       TEXT NOT NULL,
            consent              TEXT NOT NULL DEFAULT 'Emit'
                CHECK(consent IN ('Emit', 'Store', 'Forget')),
            created_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
            updated_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
        );
        CREATE INDEX IF NOT EXISTS idx_workstream_active
            ON workstream(status) WHERE status IN ('active', 'blocked');",
    )
    .map_err(|e| SchemaError::TableCreationFailed {
        table: "workstream".into(),
        reason: e.to_string(),
    })
}

#[cfg(feature = "sqlite")]
fn create_reinforced_pattern(conn: &Connection) -> Result<(), SchemaError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS reinforced_pattern (
            pattern_id          TEXT PRIMARY KEY,
            category            TEXT NOT NULL CHECK(category IN ('procedural', 'semantic', 'trap', 'feedback')),
            description         TEXT NOT NULL,
            anti_pattern        TEXT,
            weight              REAL NOT NULL DEFAULT 0.5,
            hit_count           INTEGER NOT NULL DEFAULT 1,
            last_fired_session  INTEGER,
            natural_hit_count   INTEGER NOT NULL DEFAULT 0,
            keywords            TEXT NOT NULL DEFAULT '',
            consent             TEXT NOT NULL DEFAULT 'Emit'
                CHECK(consent IN ('Emit', 'Store', 'Forget')),
            created_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
            updated_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
        );
        CREATE INDEX IF NOT EXISTS idx_pattern_weight
            ON reinforced_pattern(weight DESC);",
    )
    .map_err(|e| SchemaError::TableCreationFailed {
        table: "reinforced_pattern".into(),
        reason: e.to_string(),
    })
}

#[cfg(feature = "sqlite")]
fn create_injection_cache(conn: &Connection) -> Result<(), SchemaError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS injection_cache (
            section             TEXT PRIMARY KEY,
            payload             TEXT NOT NULL,
            token_count         INTEGER NOT NULL,
            computed_at         INTEGER NOT NULL,
            consent_applied     INTEGER NOT NULL DEFAULT 1
        );",
    )
    .map_err(|e| SchemaError::TableCreationFailed {
        table: "injection_cache".into(),
        reason: e.to_string(),
    })
}

#[cfg(feature = "sqlite")]
fn create_session_checkpoint(conn: &Connection) -> Result<(), SchemaError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS session_checkpoint (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            label               TEXT NOT NULL UNIQUE,
            session_number      INTEGER,
            timestamp_utc       TEXT NOT NULL,
            pane_id             TEXT,
            tab                 TEXT,
            session_name        TEXT,
            cwd                 TEXT,
            git_sha             TEXT,
            git_branch          TEXT,
            git_dirty_files     INTEGER DEFAULT 0,
            git_last_commit     TEXT,
            services_alive      INTEGER NOT NULL,
            services_total      INTEGER NOT NULL DEFAULT 12,
            services_alive_ports TEXT,
            watcher_ready       INTEGER DEFAULT 0,
            watcher_reason      TEXT,
            persona             TEXT,
            scope_constraint    TEXT,
            accomplished_json   TEXT NOT NULL,
            in_progress_json    TEXT NOT NULL,
            blocked_json        TEXT NOT NULL,
            key_findings_json   TEXT NOT NULL,
            resume_instructions TEXT NOT NULL,
            conversation_anchors TEXT,
            source_file         TEXT NOT NULL,
            consent             TEXT NOT NULL DEFAULT 'Emit'
                CHECK(consent IN ('Emit', 'Store', 'Forget'))
        );
        CREATE INDEX IF NOT EXISTS idx_checkpoint_label ON session_checkpoint(label);
        CREATE INDEX IF NOT EXISTS idx_checkpoint_ts ON session_checkpoint(timestamp_utc DESC);",
    )
    .map_err(|e| SchemaError::TableCreationFailed {
        table: "session_checkpoint".into(),
        reason: e.to_string(),
    })
}

#[cfg(feature = "sqlite")]
fn create_injection_script(conn: &Connection) -> Result<(), SchemaError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS injection_script (
            id                  TEXT PRIMARY KEY,
            name                TEXT NOT NULL UNIQUE,
            description         TEXT NOT NULL,
            tags                TEXT NOT NULL DEFAULT '',
            shebang             TEXT NOT NULL DEFAULT '#!/usr/bin/env bash',
            script_body         TEXT NOT NULL,
            template_vars_json  TEXT NOT NULL DEFAULT '{}',
            created_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
            updated_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
            last_run            TEXT,
            run_count           INTEGER NOT NULL DEFAULT 0,
            exit_code_last      INTEGER,
            consent             TEXT NOT NULL DEFAULT 'Emit'
                CHECK(consent IN ('Emit', 'Store', 'Forget'))
        );
        CREATE INDEX IF NOT EXISTS idx_script_name ON injection_script(name);
        CREATE INDEX IF NOT EXISTS idx_script_tags ON injection_script(tags);",
    )
    .map_err(|e| SchemaError::TableCreationFailed {
        table: "injection_script".into(),
        reason: e.to_string(),
    })
}

#[cfg(feature = "sqlite")]
fn create_daemon_state(conn: &Connection) -> Result<(), SchemaError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS daemon_state (
            key         TEXT PRIMARY KEY,
            value       TEXT NOT NULL,
            updated_at  INTEGER NOT NULL DEFAULT (unixepoch())
        );",
    )
    .map_err(|e| SchemaError::TableCreationFailed {
        table: "daemon_state".into(),
        reason: e.to_string(),
    })
}

#[cfg(feature = "sqlite")]
fn migrate_v4_to_v5(conn: &Connection) -> Result<(), SchemaError> {
    let has_col = |col: &str| -> bool {
        conn.prepare(&format!("SELECT {col} FROM reinforced_pattern LIMIT 0"))
            .is_ok()
    };
    if !has_col("natural_hit_count") {
        conn.execute(
            "ALTER TABLE reinforced_pattern ADD COLUMN natural_hit_count INTEGER NOT NULL DEFAULT 0",
            [],
        )
        .map_err(|e| SchemaError::MigrationFailed {
            version: 5,
            reason: e.to_string(),
        })?;
    }
    if !has_col("keywords") {
        conn.execute(
            "ALTER TABLE reinforced_pattern ADD COLUMN keywords TEXT NOT NULL DEFAULT ''",
            [],
        )
        .map_err(|e| SchemaError::MigrationFailed {
            version: 5,
            reason: e.to_string(),
        })?;
    }
    Ok(())
}

#[cfg(feature = "sqlite")]
fn get_schema_version(conn: &Connection) -> Result<u32, SchemaError> {
    let version: u32 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|e| SchemaError::Sqlite(e.to_string()))?;
    Ok(version)
}

#[cfg(feature = "sqlite")]
fn set_schema_version(conn: &Connection, version: u32) -> Result<(), SchemaError> {
    conn.pragma_update(None, "user_version", version)
        .map_err(|e| SchemaError::Sqlite(e.to_string()))
}

#[cfg(feature = "sqlite")]
fn migrate(conn: &Connection, from: u32, to: u32) -> Result<(), SchemaError> {
    for v in from..to {
        match v {
            0 => {
                create_all_tables(conn)?;
            }
            1 => {
                migrate_v1_to_v2(conn)?;
            }
            2 => {
                create_injection_script(conn)?;
            }
            3 => {
                create_daemon_state(conn)?;
            }
            4 => {
                migrate_v4_to_v5(conn)?;
            }
            _ => {
                return Err(SchemaError::MigrationFailed {
                    version: v + 1,
                    reason: format!("no migration path from version {v}"),
                });
            }
        }
        set_schema_version(conn, v + 1)?;
    }
    Ok(())
}

#[cfg(feature = "sqlite")]
fn migrate_v1_to_v2(conn: &Connection) -> Result<(), SchemaError> {
    conn.execute_batch("BEGIN;")
        .map_err(|e| SchemaError::MigrationFailed { version: 2, reason: e.to_string() })?;

    let result = migrate_v1_to_v2_inner(conn);

    match &result {
        Ok(()) => {
            conn.execute_batch("COMMIT;")
                .map_err(|e| SchemaError::MigrationFailed { version: 2, reason: e.to_string() })?;
        }
        Err(_) => {
            let _ = conn.execute_batch("ROLLBACK;");
        }
    }
    result
}

#[cfg(feature = "sqlite")]
fn migrate_v1_to_v2_inner(conn: &Connection) -> Result<(), SchemaError> {
    let ts_default = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";
    for table in ["causal_chain", "workstream", "reinforced_pattern"] {
        for col in ["created_at", "updated_at"] {
            if !column_exists(conn, table, col)? {
                conn.execute_batch(&format!(
                    "ALTER TABLE {table} ADD COLUMN {col} TEXT NOT NULL DEFAULT ({ts_default});"
                ))
                .map_err(|e| SchemaError::MigrationFailed {
                    version: 2,
                    reason: format!("{table}.{col}: {e}"),
                })?;
            }
        }
    }
    Ok(())
}

#[cfg(feature = "sqlite")]
fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool, SchemaError> {
    debug_assert!(
        table.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'),
        "table name must be alphanumeric: {table}"
    );
    debug_assert!(
        column.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'),
        "column name must be alphanumeric: {column}"
    );
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|e| SchemaError::Sqlite(e.to_string()))?;
    let found = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| SchemaError::Sqlite(e.to_string()))?
        .any(|r| r.is_ok_and(|name| name == column));
    Ok(found)
}

/// Returns the list of table names in the database.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on query failure.
#[cfg(feature = "sqlite")]
pub fn list_tables(conn: &Connection) -> Result<Vec<String>, SchemaError> {
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .map_err(|e| SchemaError::Sqlite(e.to_string()))?;
    let names = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| SchemaError::Sqlite(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| SchemaError::Sqlite(e.to_string()))?;
    Ok(names)
}

/// Returns the current schema version.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on query failure.
#[cfg(feature = "sqlite")]
pub fn schema_version(conn: &Connection) -> Result<u32, SchemaError> {
    get_schema_version(conn)
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;

    fn mem_db() -> Connection {
        open_memory().unwrap()
    }

    #[test]
    fn open_memory_succeeds() {
        let _conn = mem_db();
    }

    #[test]
    fn schema_version_is_current() {
        let conn = mem_db();
        assert_eq!(schema_version(&conn).unwrap(), CURRENT_VERSION);
    }

    #[test]
    fn all_seven_tables_exist() {
        let conn = mem_db();
        let tables = list_tables(&conn).unwrap();
        assert!(tables.contains(&"causal_chain".to_string()));
        assert!(tables.contains(&"session_trajectory".to_string()));
        assert!(tables.contains(&"workstream".to_string()));
        assert!(tables.contains(&"reinforced_pattern".to_string()));
        assert!(tables.contains(&"injection_cache".to_string()));
        assert!(tables.contains(&"session_checkpoint".to_string()));
        assert!(tables.contains(&"injection_script".to_string()));
        assert!(tables.contains(&"daemon_state".to_string()));
    }

    #[test]
    fn table_count_is_eight() {
        let conn = mem_db();
        assert_eq!(list_tables(&conn).unwrap().len(), 8);
    }

    #[test]
    fn idempotent_create() {
        let conn = mem_db();
        create_all_tables(&conn).unwrap();
        create_all_tables(&conn).unwrap();
        assert_eq!(list_tables(&conn).unwrap().len(), 8);
    }

    #[test]
    fn causal_chain_insert() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO causal_chain (origin_session, chain_type, label, description)
             VALUES (109, 'bug', 'BUG-064i', 'pathway update discarded')",
            [],
        )
        .unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM causal_chain", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn causal_chain_type_constraint() {
        let conn = mem_db();
        let result = conn.execute(
            "INSERT INTO causal_chain (origin_session, chain_type, label, description)
             VALUES (1, 'invalid_type', 'x', 'y')",
            [],
        );
        assert!(result.is_err());
    }

    #[test]
    fn causal_chain_consent_constraint() {
        let conn = mem_db();
        let result = conn.execute(
            "INSERT INTO causal_chain (origin_session, chain_type, label, description, consent)
             VALUES (1, 'bug', 'x', 'y', 'InvalidConsent')",
            [],
        );
        assert!(result.is_err());
    }

    #[test]
    fn causal_chain_default_reinforcement() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO causal_chain (origin_session, chain_type, label, description)
             VALUES (1, 'trap', 'cp-alias', 'cp is aliased')",
            [],
        )
        .unwrap();
        let rc: i64 = conn
            .query_row(
                "SELECT reinforcement_count FROM causal_chain WHERE label = 'cp-alias'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(rc, 1);
    }

    #[test]
    fn causal_chain_default_consent() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO causal_chain (origin_session, chain_type, label, description)
             VALUES (1, 'bug', 'x', 'y')",
            [],
        )
        .unwrap();
        let consent: String = conn
            .query_row("SELECT consent FROM causal_chain WHERE label = 'x'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(consent, "Emit");
    }

    #[test]
    fn causal_chain_valid_types() {
        let conn = mem_db();
        for ct in &["bug", "trap", "plan", "pattern"] {
            conn.execute(
                "INSERT INTO causal_chain (origin_session, chain_type, label, description) VALUES (1, ?1, ?2, 'desc')",
                rusqlite::params![ct, format!("label_{ct}")],
            )
            .unwrap();
        }
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM causal_chain", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 4);
    }

    #[test]
    fn causal_chain_autoincrement() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO causal_chain (origin_session, chain_type, label, description) VALUES (1, 'bug', 'a', 'x')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO causal_chain (origin_session, chain_type, label, description) VALUES (2, 'trap', 'b', 'y')",
            [],
        )
        .unwrap();
        let ids: Vec<i64> = {
            let mut stmt = conn
                .prepare("SELECT id FROM causal_chain ORDER BY id")
                .unwrap();
            stmt.query_map([], |r| r.get(0))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        };
        assert_eq!(ids.len(), 2);
        assert!(ids[1] > ids[0]);
    }

    #[test]
    fn causal_chain_consent_valid_values() {
        let conn = mem_db();
        for (i, consent) in ["Emit", "Store", "Forget"].iter().enumerate() {
            conn.execute(
                "INSERT INTO causal_chain (origin_session, chain_type, label, description, consent) VALUES (1, 'bug', ?1, 'desc', ?2)",
                rusqlite::params![format!("c{i}"), consent],
            )
            .unwrap();
        }
        assert_eq!(
            conn.query_row::<i64, _, _>("SELECT COUNT(*) FROM causal_chain", [], |r| r.get(0))
                .unwrap(),
            3
        );
    }

    #[test]
    fn trajectory_insert_and_query() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO session_trajectory (session_id, ralph_fitness, field_r, thermal_t, ltp_ltd_ratio, services_healthy, delta_summary)
             VALUES (109, 0.664, 0.876, 0.515, 4.2, 11, 'fitness stable')",
            [],
        )
        .unwrap();
        let fit: f64 = conn
            .query_row(
                "SELECT ralph_fitness FROM session_trajectory WHERE session_id = 109",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!((fit - 0.664).abs() < f64::EPSILON);
    }

    #[test]
    fn trajectory_session_id_is_pk() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO session_trajectory (session_id, ralph_fitness, field_r, thermal_t, ltp_ltd_ratio, services_healthy, delta_summary)
             VALUES (1, 0.5, 0.0, 0.5, 1.0, 11, 'first')",
            [],
        )
        .unwrap();
        let result = conn.execute(
            "INSERT INTO session_trajectory (session_id, ralph_fitness, field_r, thermal_t, ltp_ltd_ratio, services_healthy, delta_summary)
             VALUES (1, 0.6, 0.0, 0.5, 1.0, 11, 'duplicate')",
            [],
        );
        assert!(result.is_err());
    }

    #[test]
    fn trajectory_consent_constraint() {
        let conn = mem_db();
        let result = conn.execute(
            "INSERT INTO session_trajectory (session_id, ralph_fitness, field_r, thermal_t, ltp_ltd_ratio, services_healthy, delta_summary, consent)
             VALUES (1, 0.5, 0.0, 0.5, 1.0, 11, 'x', 'BAD')",
            [],
        );
        assert!(result.is_err());
    }

    #[test]
    fn trajectory_ordering_by_session_desc() {
        let conn = mem_db();
        for s in [105, 108, 106, 107] {
            conn.execute(
                "INSERT INTO session_trajectory (session_id, ralph_fitness, field_r, thermal_t, ltp_ltd_ratio, services_healthy, delta_summary)
                 VALUES (?1, 0.5, 0.0, 0.5, 1.0, 11, ?2)",
                rusqlite::params![s, format!("session {s}")],
            )
            .unwrap();
        }
        let first: i64 = conn
            .query_row(
                "SELECT session_id FROM session_trajectory ORDER BY session_id DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(first, 108);
    }

    #[test]
    fn trajectory_key_achievement_nullable() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO session_trajectory (session_id, ralph_fitness, field_r, thermal_t, ltp_ltd_ratio, services_healthy, delta_summary)
             VALUES (1, 0.5, 0.0, 0.5, 1.0, 11, 'x')",
            [],
        )
        .unwrap();
        let ka: Option<String> = conn
            .query_row(
                "SELECT key_achievement FROM session_trajectory WHERE session_id = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(ka.is_none());
    }

    #[test]
    fn workstream_insert_and_query() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO workstream (ws_id, title, status, last_touched_session, resume_context)
             VALUES ('comms-v3', 'Comms Layer v3', 'active', 109, 'WS-0 P4 next')",
            [],
        )
        .unwrap();
        let status: String = conn
            .query_row(
                "SELECT status FROM workstream WHERE ws_id = 'comms-v3'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, "active");
    }

    #[test]
    fn workstream_status_constraint() {
        let conn = mem_db();
        let result = conn.execute(
            "INSERT INTO workstream (ws_id, title, status, last_touched_session, resume_context)
             VALUES ('x', 'y', 'invalid_status', 1, 'z')",
            [],
        );
        assert!(result.is_err());
    }

    #[test]
    fn workstream_blocker_nullable() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO workstream (ws_id, title, status, last_touched_session, resume_context)
             VALUES ('x', 'y', 'active', 1, 'z')",
            [],
        )
        .unwrap();
        let blocker: Option<String> = conn
            .query_row("SELECT blocker FROM workstream WHERE ws_id = 'x'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert!(blocker.is_none());
    }

    #[test]
    fn workstream_valid_statuses() {
        let conn = mem_db();
        for (i, status) in ["active", "blocked", "deferred", "complete"]
            .iter()
            .enumerate()
        {
            conn.execute(
                "INSERT INTO workstream (ws_id, title, status, last_touched_session, resume_context) VALUES (?1, 't', ?2, 1, 'ctx')",
                rusqlite::params![format!("ws{i}"), status],
            )
            .unwrap();
        }
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM workstream", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 4);
    }

    #[test]
    fn workstream_priority_default() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO workstream (ws_id, title, status, last_touched_session, resume_context)
             VALUES ('x', 'y', 'active', 1, 'z')",
            [],
        )
        .unwrap();
        let priority: i64 = conn
            .query_row(
                "SELECT priority FROM workstream WHERE ws_id = 'x'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(priority, 5);
    }

    #[test]
    fn pattern_insert_and_query() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO reinforced_pattern (pattern_id, category, description, weight)
             VALUES ('verify-before-ship', 'procedural', 'Always verify before deploying', 0.8)",
            [],
        )
        .unwrap();
        let w: f64 = conn
            .query_row(
                "SELECT weight FROM reinforced_pattern WHERE pattern_id = 'verify-before-ship'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!((w - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn pattern_category_constraint() {
        let conn = mem_db();
        let result = conn.execute(
            "INSERT INTO reinforced_pattern (pattern_id, category, description)
             VALUES ('x', 'bogus', 'y')",
            [],
        );
        assert!(result.is_err());
    }

    #[test]
    fn pattern_default_weight() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO reinforced_pattern (pattern_id, category, description)
             VALUES ('x', 'trap', 'y')",
            [],
        )
        .unwrap();
        let w: f64 = conn
            .query_row(
                "SELECT weight FROM reinforced_pattern WHERE pattern_id = 'x'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!((w - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn pattern_default_hit_count() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO reinforced_pattern (pattern_id, category, description)
             VALUES ('x', 'feedback', 'y')",
            [],
        )
        .unwrap();
        let hc: i64 = conn
            .query_row(
                "SELECT hit_count FROM reinforced_pattern WHERE pattern_id = 'x'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hc, 1);
    }

    #[test]
    fn pattern_valid_categories() {
        let conn = mem_db();
        for (i, cat) in ["procedural", "semantic", "trap", "feedback"]
            .iter()
            .enumerate()
        {
            conn.execute(
                "INSERT INTO reinforced_pattern (pattern_id, category, description) VALUES (?1, ?2, 'desc')",
                rusqlite::params![format!("p{i}"), cat],
            )
            .unwrap();
        }
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM reinforced_pattern", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 4);
    }

    #[test]
    fn pattern_anti_pattern_nullable() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO reinforced_pattern (pattern_id, category, description)
             VALUES ('x', 'semantic', 'y')",
            [],
        )
        .unwrap();
        let ap: Option<String> = conn
            .query_row(
                "SELECT anti_pattern FROM reinforced_pattern WHERE pattern_id = 'x'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(ap.is_none());
    }

    #[test]
    fn injection_cache_insert_and_query() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO injection_cache (section, payload, token_count, computed_at)
             VALUES ('orientation', 'Session 109', 10, 1713970000)",
            [],
        )
        .unwrap();
        let tc: i64 = conn
            .query_row(
                "SELECT token_count FROM injection_cache WHERE section = 'orientation'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(tc, 10);
    }

    #[test]
    fn injection_cache_section_is_pk() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO injection_cache (section, payload, token_count, computed_at)
             VALUES ('trajectory', 'v1', 5, 100)",
            [],
        )
        .unwrap();
        let result = conn.execute(
            "INSERT INTO injection_cache (section, payload, token_count, computed_at)
             VALUES ('trajectory', 'v2', 5, 200)",
            [],
        );
        assert!(result.is_err());
    }

    #[test]
    fn injection_cache_consent_applied_default() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO injection_cache (section, payload, token_count, computed_at)
             VALUES ('test', 'data', 5, 100)",
            [],
        )
        .unwrap();
        let ca: i64 = conn
            .query_row(
                "SELECT consent_applied FROM injection_cache WHERE section = 'test'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(ca, 1);
    }

    #[test]
    fn checkpoint_insert_and_query() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO session_checkpoint (label, session_number, timestamp_utc, services_alive, accomplished_json, in_progress_json, blocked_json, key_findings_json, resume_instructions, source_file)
             VALUES ('s109-close', 109, '2026-04-24T12:00:00Z', 11, '[]', '[]', '[]', '[]', 'start L2', '/tmp/s109.md')",
            [],
        )
        .unwrap();
        let sn: i64 = conn
            .query_row(
                "SELECT session_number FROM session_checkpoint WHERE label = 's109-close'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(sn, 109);
    }

    #[test]
    fn checkpoint_label_unique() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO session_checkpoint (label, timestamp_utc, services_alive, accomplished_json, in_progress_json, blocked_json, key_findings_json, resume_instructions, source_file)
             VALUES ('dup', '2026-01-01', 11, '[]', '[]', '[]', '[]', 'x', 'f')",
            [],
        )
        .unwrap();
        let result = conn.execute(
            "INSERT INTO session_checkpoint (label, timestamp_utc, services_alive, accomplished_json, in_progress_json, blocked_json, key_findings_json, resume_instructions, source_file)
             VALUES ('dup', '2026-01-02', 10, '[]', '[]', '[]', '[]', 'y', 'g')",
            [],
        );
        assert!(result.is_err());
    }

    #[test]
    fn checkpoint_consent_constraint() {
        let conn = mem_db();
        let result = conn.execute(
            "INSERT INTO session_checkpoint (label, timestamp_utc, services_alive, accomplished_json, in_progress_json, blocked_json, key_findings_json, resume_instructions, source_file, consent)
             VALUES ('x', '2026-01-01', 11, '[]', '[]', '[]', '[]', 'r', 'f', 'BAD')",
            [],
        );
        assert!(result.is_err());
    }

    #[test]
    fn checkpoint_default_services_total() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO session_checkpoint (label, timestamp_utc, services_alive, accomplished_json, in_progress_json, blocked_json, key_findings_json, resume_instructions, source_file)
             VALUES ('x', '2026-01-01', 11, '[]', '[]', '[]', '[]', 'r', 'f')",
            [],
        )
        .unwrap();
        let total: i64 = conn
            .query_row(
                "SELECT services_total FROM session_checkpoint WHERE label = 'x'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(total, 12);
    }

    #[test]
    fn checkpoint_default_consent() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO session_checkpoint (label, timestamp_utc, services_alive, accomplished_json, in_progress_json, blocked_json, key_findings_json, resume_instructions, source_file)
             VALUES ('y', '2026-01-01', 11, '[]', '[]', '[]', '[]', 'r', 'f')",
            [],
        )
        .unwrap();
        let consent: String = conn
            .query_row(
                "SELECT consent FROM session_checkpoint WHERE label = 'y'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(consent, "Emit");
    }

    #[test]
    fn wal_mode_enabled() {
        let conn = mem_db();
        let mode: String = conn
            .pragma_query_value(None, "journal_mode", |r| r.get(0))
            .unwrap();
        assert!(mode == "wal" || mode == "memory");
    }

    #[test]
    fn foreign_keys_enabled() {
        let conn = mem_db();
        let fk: i64 = conn
            .pragma_query_value(None, "foreign_keys", |r| r.get(0))
            .unwrap();
        assert_eq!(fk, 1);
    }

    #[test]
    fn busy_timeout_set() {
        let conn = mem_db();
        let timeout: i64 = conn
            .pragma_query_value(None, "busy_timeout", |r| r.get(0))
            .unwrap();
        assert_eq!(timeout, 5000);
    }

    #[test]
    fn migrate_from_zero() {
        let conn = Connection::open_in_memory().unwrap();
        configure_connection(&conn).unwrap();
        set_schema_version(&conn, 0).unwrap();
        migrate(&conn, 0, CURRENT_VERSION).unwrap();
        assert_eq!(get_schema_version(&conn).unwrap(), CURRENT_VERSION);
        assert_eq!(list_tables(&conn).unwrap().len(), 8);
    }

    #[test]
    fn migrate_unknown_version_errors() {
        let conn = Connection::open_in_memory().unwrap();
        configure_connection(&conn).unwrap();
        let result = migrate(&conn, 99, 100);
        assert!(result.is_err());
    }

    #[test]
    fn open_database_creates_file() {
        let dir = std::env::temp_dir().join("habitat_test_m06_open");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test_open.db");
        let _ = std::fs::remove_file(&path);

        let conn = open_database(&path).unwrap();
        assert!(path.exists());
        assert_eq!(list_tables(&conn).unwrap().len(), 8);

        drop(conn);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn open_database_idempotent() {
        let dir = std::env::temp_dir().join("habitat_test_m06_idem");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test_idem.db");
        let _ = std::fs::remove_file(&path);

        let conn1 = open_database(&path).unwrap();
        conn1
            .execute(
                "INSERT INTO causal_chain (origin_session, chain_type, label, description)
                 VALUES (1, 'bug', 'test', 'test')",
                [],
            )
            .unwrap();
        drop(conn1);

        let conn2 = open_database(&path).unwrap();
        let count: i64 = conn2
            .query_row("SELECT COUNT(*) FROM causal_chain", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);

        drop(conn2);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn open_database_creates_parent_dirs() {
        let dir = std::env::temp_dir()
            .join("habitat_test_m06_deep")
            .join("nested")
            .join("dir");
        let path = dir.join("test.db");
        let _ = std::fs::remove_file(&path);

        let conn = open_database(&path).unwrap();
        assert!(path.exists());

        drop(conn);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(std::env::temp_dir().join("habitat_test_m06_deep"));
    }

    #[test]
    fn unresolved_index_query() {
        let conn = mem_db();
        for i in 0..5 {
            conn.execute(
                "INSERT INTO causal_chain (origin_session, chain_type, label, description, reinforcement_count)
                 VALUES (1, 'bug', ?1, 'desc', ?2)",
                rusqlite::params![format!("bug-{i}"), 5 - i],
            )
            .unwrap();
        }
        conn.execute(
            "INSERT INTO causal_chain (origin_session, resolved_session, chain_type, label, description, reinforcement_count)
             VALUES (1, 2, 'bug', 'resolved', 'desc', 100)",
            [],
        )
        .unwrap();
        let top_label: String = conn
            .query_row(
                "SELECT label FROM causal_chain WHERE resolved_session IS NULL
                 ORDER BY reinforcement_count DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(top_label, "bug-0");
    }

    #[test]
    fn active_workstream_index_query() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO workstream (ws_id, title, status, last_touched_session, resume_context)
             VALUES ('ws1', 'Active', 'active', 109, 'ctx')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO workstream (ws_id, title, status, last_touched_session, resume_context)
             VALUES ('ws2', 'Done', 'complete', 100, 'ctx')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO workstream (ws_id, title, status, last_touched_session, resume_context)
             VALUES ('ws3', 'Stuck', 'blocked', 108, 'ctx')",
            [],
        )
        .unwrap();
        let active_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM workstream WHERE status IN ('active', 'blocked')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(active_count, 2);
    }

    #[test]
    fn pattern_weight_desc_index() {
        let conn = mem_db();
        for (i, w) in [0.3, 0.9, 0.1, 0.7].iter().enumerate() {
            conn.execute(
                "INSERT INTO reinforced_pattern (pattern_id, category, description, weight) VALUES (?1, 'trap', 'desc', ?2)",
                rusqlite::params![format!("p{i}"), w],
            )
            .unwrap();
        }
        let top: String = conn
            .query_row(
                "SELECT pattern_id FROM reinforced_pattern ORDER BY weight DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(top, "p1");
    }

    #[test]
    fn workstream_items_nullable() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO workstream (ws_id, title, status, last_touched_session, resume_context)
             VALUES ('x', 'y', 'active', 1, 'z')",
            [],
        )
        .unwrap();
        let total: Option<i64> = conn
            .query_row(
                "SELECT items_total FROM workstream WHERE ws_id = 'x'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(total.is_none());
    }

    #[test]
    fn trajectory_multiple_sessions() {
        let conn = mem_db();
        for s in 105..=110 {
            conn.execute(
                "INSERT INTO session_trajectory (session_id, ralph_fitness, field_r, thermal_t, ltp_ltd_ratio, services_healthy, delta_summary)
                 VALUES (?1, ?2, 0.5, 0.5, 2.0, 11, 'ok')",
                rusqlite::params![s, 0.5 + f64::from(s - 105) * 0.01],
            )
            .unwrap();
        }
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM session_trajectory", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 6);
        let recent: Vec<i64> = {
            let mut stmt = conn
                .prepare("SELECT session_id FROM session_trajectory ORDER BY session_id DESC LIMIT 3")
                .unwrap();
            stmt.query_map([], |r| r.get(0))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        };
        assert_eq!(recent, vec![110, 109, 108]);
    }

    #[test]
    fn causal_chain_has_created_at() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO causal_chain (origin_session, chain_type, label, description)
             VALUES (1, 'bug', 'ts-test', 'timestamp test')",
            [],
        )
        .unwrap();
        let ts: String = conn
            .query_row(
                "SELECT created_at FROM causal_chain WHERE label = 'ts-test'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(ts.contains('T'), "created_at should be ISO-8601: {ts}");
    }

    #[test]
    fn causal_chain_has_updated_at() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO causal_chain (origin_session, chain_type, label, description)
             VALUES (1, 'trap', 'ua-test', 'updated_at test')",
            [],
        )
        .unwrap();
        let ts: String = conn
            .query_row(
                "SELECT updated_at FROM causal_chain WHERE label = 'ua-test'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(ts.contains('T'));
    }

    #[test]
    fn workstream_has_timestamps() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO workstream (ws_id, title, status, last_touched_session, resume_context)
             VALUES ('ws-ts', 'TS', 'active', 1, 'ctx')",
            [],
        )
        .unwrap();
        let (ca, ua): (String, String) = conn
            .query_row(
                "SELECT created_at, updated_at FROM workstream WHERE ws_id = 'ws-ts'",
                [],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .unwrap();
        assert!(ca.contains('T'));
        assert!(ua.contains('T'));
    }

    #[test]
    fn reinforced_pattern_has_timestamps() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO reinforced_pattern (pattern_id, category, description)
             VALUES ('p-ts', 'trap', 'timestamp test')",
            [],
        )
        .unwrap();
        let (ca, ua): (String, String) = conn
            .query_row(
                "SELECT created_at, updated_at FROM reinforced_pattern WHERE pattern_id = 'p-ts'",
                [],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .unwrap();
        assert!(ca.contains('T'));
        assert!(ua.contains('T'));
    }

    #[test]
    #[cfg(feature = "sqlite")]
fn column_exists_returns_true_for_known_column() {
        let conn = mem_db();
        assert!(column_exists(&conn, "causal_chain", "created_at").unwrap());
        assert!(column_exists(&conn, "workstream", "updated_at").unwrap());
        assert!(column_exists(&conn, "reinforced_pattern", "created_at").unwrap());
    }

    #[test]
    #[cfg(feature = "sqlite")]
fn column_exists_returns_false_for_unknown_column() {
        let conn = mem_db();
        assert!(!column_exists(&conn, "causal_chain", "nonexistent_col").unwrap());
    }

    #[test]
    fn migrate_v1_to_v2_is_idempotent() {
        let conn = mem_db();
        migrate_v1_to_v2(&conn).unwrap();
        migrate_v1_to_v2(&conn).unwrap();
        assert!(column_exists(&conn, "causal_chain", "created_at").unwrap());
    }

    #[test]
    fn daemon_state_insert_and_read() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO daemon_state (key, value) VALUES ('tool_use_counter', '42')",
            [],
        )
        .unwrap();
        let val: String = conn
            .query_row(
                "SELECT value FROM daemon_state WHERE key = 'tool_use_counter'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(val, "42");
    }

    #[test]
    fn daemon_state_key_is_pk() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO daemon_state (key, value) VALUES ('k1', 'v1')",
            [],
        )
        .unwrap();
        let result = conn.execute(
            "INSERT INTO daemon_state (key, value) VALUES ('k1', 'v2')",
            [],
        );
        assert!(result.is_err());
    }

    #[test]
    fn daemon_state_upsert() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO daemon_state (key, value) VALUES ('k', 'v1')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO daemon_state (key, value, updated_at) VALUES ('k', 'v2', unixepoch())",
            [],
        )
        .unwrap();
        let val: String = conn
            .query_row(
                "SELECT value FROM daemon_state WHERE key = 'k'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(val, "v2");
    }

    #[test]
    fn daemon_state_updated_at_has_default() {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO daemon_state (key, value) VALUES ('ts_test', 'val')",
            [],
        )
        .unwrap();
        let ts: i64 = conn
            .query_row(
                "SELECT updated_at FROM daemon_state WHERE key = 'ts_test'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(ts > 0);
    }

    #[test]
    fn migration_3_to_4_creates_daemon_state() {
        let conn = Connection::open_in_memory().unwrap();
        configure_connection(&conn).unwrap();
        create_causal_chain(&conn).unwrap();
        create_session_trajectory(&conn).unwrap();
        create_workstream(&conn).unwrap();
        create_reinforced_pattern(&conn).unwrap();
        create_injection_cache(&conn).unwrap();
        create_session_checkpoint(&conn).unwrap();
        create_injection_script(&conn).unwrap();
        set_schema_version(&conn, 3).unwrap();

        migrate(&conn, 3, 4).unwrap();

        let tables = list_tables(&conn).unwrap();
        assert!(tables.contains(&"daemon_state".to_string()));
        assert_eq!(schema_version(&conn).unwrap(), 4);
    }

    #[test]
    fn wal_autocheckpoint_is_set() {
        let conn = mem_db();
        let val: i32 = conn
            .pragma_query_value(None, "wal_autocheckpoint", |r| r.get(0))
            .unwrap();
        assert_eq!(val, 100);
    }
}
