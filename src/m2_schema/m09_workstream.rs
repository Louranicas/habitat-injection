//! `m09_workstream` — CRUD for `workstream` table.
//!
//! Provides `insert_workstream`, `update_status`, `set_blocker`, `clear_blocker`,
//! `get_active`, `get_blocked`, `touch`, `get_by_id`, `update_progress`, and
//! `count_by_status`.  The `workstream` table tracks every in-flight work item
//! with its status, optional blocker, priority, and rich resume context so a
//! fresh session can pick up where the previous one left off.
//!
//! Layer: `m2_schema`
//! Dependencies: `m01_types`, `m02_errors`, `m06_schema`
//! Implemented by: Historian + Practitioner (circle deliberation S109)
//! Session: S110

use rusqlite::{Connection, OptionalExtension as _, Row, params};
use serde::{Deserialize, Serialize};

use crate::m1_foundation::m02_errors::SchemaError;

use super::sqlite_err;

// ---------------------------------------------------------------------------
// WorkstreamRow — mirrors every column in the `workstream` table
// ---------------------------------------------------------------------------

/// A fully-hydrated row from the `workstream` table.
///
/// All columns are present, with `blocker`, `items_total`, and `items_done`
/// nullable as the schema defines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkstreamRow {
    /// Primary key — human-readable slug (e.g. `"comms-v3"`, `"stdb-inject"`).
    pub ws_id: String,
    /// Short human-readable title.
    pub title: String,
    /// Current lifecycle status: `active`, `blocked`, `deferred`, or `complete`.
    pub status: String,
    /// Human-readable description of what is blocking progress, if any.
    pub blocker: Option<String>,
    /// Lower number = higher urgency. Default is `5`.
    pub priority: i32,
    /// Session number at which this row was last modified.
    pub last_touched_session: u32,
    /// Total number of sub-items, if tracked.
    pub items_total: Option<u32>,
    /// Completed sub-items, if tracked.
    pub items_done: Option<u32>,
    /// Free-form text a new session needs to resume this workstream.
    pub resume_context: String,
    /// Consent level: `Emit`, `Store`, or `Forget`.
    pub consent: String,
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn row_to_workstream(row: &Row<'_>) -> rusqlite::Result<WorkstreamRow> {
    Ok(WorkstreamRow {
        ws_id: row.get(0)?,
        title: row.get(1)?,
        status: row.get(2)?,
        blocker: row.get(3)?,
        priority: row.get::<_, i32>(4)?,
        last_touched_session: row.get::<_, u32>(5)?,
        items_total: row.get::<_, Option<u32>>(6)?,
        items_done: row.get::<_, Option<u32>>(7)?,
        resume_context: row.get(8)?,
        consent: row.get(9)?,
    })
}

const SELECT_COLS: &str =
    "ws_id, title, status, blocker, priority, last_touched_session, \
     items_total, items_done, resume_context, consent";

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Insert a new workstream with the minimum required fields.
///
/// Optional fields (`blocker`, `priority`, `items_total`, `items_done`)
/// take their schema defaults (`NULL`, `5`, `NULL`, `NULL`).  `consent`
/// defaults to `Emit`.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on any `rusqlite` failure, including a
/// `UNIQUE` violation on `ws_id`.
pub fn insert_workstream(
    conn: &Connection,
    ws_id: &str,
    title: &str,
    status: &str,
    last_touched_session: u32,
    resume_context: &str,
) -> Result<(), SchemaError> {
    conn.execute(
        "INSERT INTO workstream (ws_id, title, status, last_touched_session, resume_context)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![ws_id, title, status, last_touched_session, resume_context],
    )
    .map_err(|e| sqlite_err(&e))?;
    Ok(())
}

/// Update the `status` column and touch `last_touched_session` atomically.
///
/// Returns `true` if a row with `ws_id` was found and updated, `false` if
/// no such row exists.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on any database error, including a
/// `CHECK` constraint violation for an invalid `status` value.
pub fn update_status(
    conn: &Connection,
    ws_id: &str,
    status: &str,
    session: u32,
) -> Result<bool, SchemaError> {
    let rows = conn
        .execute(
            "UPDATE workstream
             SET status = ?1, last_touched_session = ?2, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE ws_id = ?3",
            params![status, session, ws_id],
        )
        .map_err(|e| sqlite_err(&e))?;
    Ok(rows > 0)
}

/// Set a blocker on a workstream, transitioning its status to `blocked`.
///
/// Also updates `last_touched_session`.  Returns `true` if the row was found.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on any database error.
pub fn set_blocker(
    conn: &Connection,
    ws_id: &str,
    blocker: &str,
    session: u32,
) -> Result<bool, SchemaError> {
    let rows = conn
        .execute(
            "UPDATE workstream
             SET status = 'blocked', blocker = ?1, last_touched_session = ?2, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE ws_id = ?3",
            params![blocker, session, ws_id],
        )
        .map_err(|e| sqlite_err(&e))?;
    Ok(rows > 0)
}

/// Clear a blocker, transitioning the workstream back to `active`.
///
/// Sets `blocker` to `NULL`, `status` to `active`, and updates
/// `last_touched_session`.  Returns `true` if the row was found.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on any database error.
pub fn clear_blocker(
    conn: &Connection,
    ws_id: &str,
    session: u32,
) -> Result<bool, SchemaError> {
    let rows = conn
        .execute(
            "UPDATE workstream
             SET status = 'active', blocker = NULL, last_touched_session = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE ws_id = ?2",
            params![session, ws_id],
        )
        .map_err(|e| sqlite_err(&e))?;
    Ok(rows > 0)
}

/// Return all workstreams with `status = 'active'`, ordered by `priority ASC`.
///
/// Lower priority numbers surface first (higher urgency).
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on any database error.
pub fn get_active(conn: &Connection) -> Result<Vec<WorkstreamRow>, SchemaError> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {SELECT_COLS} FROM workstream WHERE status = 'active' ORDER BY priority ASC"
        ))
        .map_err(|e| sqlite_err(&e))?;

    let rows = stmt
        .query_map([], row_to_workstream)
        .map_err(|e| sqlite_err(&e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| sqlite_err(&e))?;

    Ok(rows)
}

/// Return all workstreams with `status = 'blocked'`.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on any database error.
pub fn get_blocked(conn: &Connection) -> Result<Vec<WorkstreamRow>, SchemaError> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {SELECT_COLS} FROM workstream WHERE status = 'blocked'"
        ))
        .map_err(|e| sqlite_err(&e))?;

    let rows = stmt
        .query_map([], row_to_workstream)
        .map_err(|e| sqlite_err(&e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| sqlite_err(&e))?;

    Ok(rows)
}

/// Update only `last_touched_session` for `ws_id`.
///
/// Returns `true` if the row was found, `false` if it does not exist.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on any database error.
pub fn touch(conn: &Connection, ws_id: &str, session: u32) -> Result<bool, SchemaError> {
    let rows = conn
        .execute(
            "UPDATE workstream SET last_touched_session = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE ws_id = ?2",
            params![session, ws_id],
        )
        .map_err(|e| sqlite_err(&e))?;
    Ok(rows > 0)
}

/// Fetch a single workstream by its `ws_id`.
///
/// Returns `None` if no row with that `ws_id` exists.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on any database error.
pub fn get_by_id(
    conn: &Connection,
    ws_id: &str,
) -> Result<Option<WorkstreamRow>, SchemaError> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {SELECT_COLS} FROM workstream WHERE ws_id = ?1"
        ))
        .map_err(|e| sqlite_err(&e))?;

    stmt.query_row([ws_id], row_to_workstream)
        .optional()
        .map_err(|e| sqlite_err(&e))
}

/// Update `items_done` and `items_total` progress counters, and touch session.
///
/// Returns `true` if the row was found, `false` if it does not exist.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on any database error.
pub fn update_progress(
    conn: &Connection,
    ws_id: &str,
    items_done: u32,
    items_total: u32,
    session: u32,
) -> Result<bool, SchemaError> {
    let rows = conn
        .execute(
            "UPDATE workstream
             SET items_done = ?1, items_total = ?2, last_touched_session = ?3, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE ws_id = ?4",
            params![items_done, items_total, session, ws_id],
        )
        .map_err(|e| sqlite_err(&e))?;
    Ok(rows > 0)
}

/// Count workstreams with the given `status`.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on any database error.
pub fn count_by_status(conn: &Connection, status: &str) -> Result<u64, SchemaError> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM workstream WHERE status = ?1",
            [status],
            |row| row.get(0),
        )
        .map_err(|e| sqlite_err(&e))?;
    Ok(count.cast_unsigned())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::m2_schema::m06_schema::open_memory;

    // ---- helpers ----

    fn db() -> Connection {
        open_memory().expect("open_memory should never fail in tests")
    }

    fn insert_basic(conn: &Connection, ws_id: &str, status: &str) {
        insert_workstream(conn, ws_id, "Test Title", status, 109, "resume ctx")
            .expect("insert should succeed");
    }

    // ---- insert_workstream ----

    #[test]
    fn insert_basic_succeeds() {
        let conn = db();
        assert!(insert_workstream(&conn, "ws-1", "Title", "active", 109, "ctx").is_ok());
    }

    #[test]
    fn insert_round_trip_via_get_by_id() {
        let conn = db();
        insert_workstream(&conn, "ws-rt", "RT Title", "active", 110, "resume").unwrap();
        let row = get_by_id(&conn, "ws-rt").unwrap().unwrap();
        assert_eq!(row.ws_id, "ws-rt");
        assert_eq!(row.title, "RT Title");
        assert_eq!(row.status, "active");
        assert_eq!(row.last_touched_session, 110);
        assert_eq!(row.resume_context, "resume");
    }

    #[test]
    fn insert_defaults_priority_5() {
        let conn = db();
        insert_basic(&conn, "ws-p", "active");
        let row = get_by_id(&conn, "ws-p").unwrap().unwrap();
        assert_eq!(row.priority, 5);
    }

    #[test]
    fn insert_defaults_consent_emit() {
        let conn = db();
        insert_basic(&conn, "ws-c", "active");
        let row = get_by_id(&conn, "ws-c").unwrap().unwrap();
        assert_eq!(row.consent, "Emit");
    }

    #[test]
    fn insert_blocker_is_null_by_default() {
        let conn = db();
        insert_basic(&conn, "ws-b", "active");
        let row = get_by_id(&conn, "ws-b").unwrap().unwrap();
        assert!(row.blocker.is_none());
    }

    #[test]
    fn insert_items_null_by_default() {
        let conn = db();
        insert_basic(&conn, "ws-i", "active");
        let row = get_by_id(&conn, "ws-i").unwrap().unwrap();
        assert!(row.items_total.is_none());
        assert!(row.items_done.is_none());
    }

    #[test]
    fn insert_duplicate_ws_id_fails() {
        let conn = db();
        insert_basic(&conn, "ws-dup", "active");
        let result = insert_workstream(&conn, "ws-dup", "T2", "active", 110, "ctx2");
        assert!(result.is_err());
    }

    #[test]
    fn insert_invalid_status_fails() {
        let conn = db();
        let result = insert_workstream(&conn, "ws-bad", "T", "invalid_status", 1, "ctx");
        assert!(result.is_err());
    }

    #[test]
    fn insert_all_valid_statuses() {
        let conn = db();
        for status in &["active", "blocked", "deferred", "complete"] {
            let id = format!("ws-{status}");
            assert!(
                insert_workstream(&conn, &id, "T", status, 1, "ctx").is_ok(),
                "status={status} should be valid"
            );
        }
    }

    // ---- get_by_id ----

    #[test]
    fn get_by_id_missing_returns_none() {
        let conn = db();
        let row = get_by_id(&conn, "no-such-ws").unwrap();
        assert!(row.is_none());
    }

    #[test]
    fn get_by_id_after_insert_returns_some() {
        let conn = db();
        insert_basic(&conn, "ws-exist", "active");
        let row = get_by_id(&conn, "ws-exist").unwrap();
        assert!(row.is_some());
    }

    // ---- update_status ----

    #[test]
    fn update_status_changes_status() {
        let conn = db();
        insert_basic(&conn, "ws-us", "active");
        let found = update_status(&conn, "ws-us", "deferred", 111).unwrap();
        assert!(found);
        let row = get_by_id(&conn, "ws-us").unwrap().unwrap();
        assert_eq!(row.status, "deferred");
    }

    #[test]
    fn update_status_touches_session() {
        let conn = db();
        insert_basic(&conn, "ws-ts", "active");
        update_status(&conn, "ws-ts", "complete", 200).unwrap();
        let row = get_by_id(&conn, "ws-ts").unwrap().unwrap();
        assert_eq!(row.last_touched_session, 200);
    }

    #[test]
    fn update_status_missing_returns_false() {
        let conn = db();
        let found = update_status(&conn, "no-ws", "active", 1).unwrap();
        assert!(!found);
    }

    #[test]
    fn update_status_invalid_status_fails() {
        let conn = db();
        insert_basic(&conn, "ws-inv", "active");
        let result = update_status(&conn, "ws-inv", "bad_status", 1);
        assert!(result.is_err());
    }

    #[test]
    fn update_status_all_transitions() {
        let conn = db();
        insert_basic(&conn, "ws-trans", "active");
        for status in &["blocked", "deferred", "complete", "active"] {
            assert!(update_status(&conn, "ws-trans", status, 1).unwrap());
        }
        let row = get_by_id(&conn, "ws-trans").unwrap().unwrap();
        assert_eq!(row.status, "active");
    }

    // ---- set_blocker ----

    #[test]
    fn set_blocker_sets_status_to_blocked() {
        let conn = db();
        insert_basic(&conn, "ws-sb", "active");
        let found = set_blocker(&conn, "ws-sb", "waiting on devenv BUG-001b", 112).unwrap();
        assert!(found);
        let row = get_by_id(&conn, "ws-sb").unwrap().unwrap();
        assert_eq!(row.status, "blocked");
        assert_eq!(
            row.blocker.as_deref(),
            Some("waiting on devenv BUG-001b")
        );
    }

    #[test]
    fn set_blocker_touches_session() {
        let conn = db();
        insert_basic(&conn, "ws-sbts", "active");
        set_blocker(&conn, "ws-sbts", "dep missing", 300).unwrap();
        let row = get_by_id(&conn, "ws-sbts").unwrap().unwrap();
        assert_eq!(row.last_touched_session, 300);
    }

    #[test]
    fn set_blocker_missing_ws_returns_false() {
        let conn = db();
        let found = set_blocker(&conn, "ghost", "whatever", 1).unwrap();
        assert!(!found);
    }

    #[test]
    fn set_blocker_on_already_blocked() {
        let conn = db();
        insert_basic(&conn, "ws-sab", "blocked");
        set_blocker(&conn, "ws-sab", "new blocker text", 115).unwrap();
        let row = get_by_id(&conn, "ws-sab").unwrap().unwrap();
        assert_eq!(row.blocker.as_deref(), Some("new blocker text"));
    }

    // ---- clear_blocker ----

    #[test]
    fn clear_blocker_sets_status_to_active() {
        let conn = db();
        insert_basic(&conn, "ws-cb", "blocked");
        set_blocker(&conn, "ws-cb", "stuck", 110).unwrap();
        let found = clear_blocker(&conn, "ws-cb", 113).unwrap();
        assert!(found);
        let row = get_by_id(&conn, "ws-cb").unwrap().unwrap();
        assert_eq!(row.status, "active");
        assert!(row.blocker.is_none());
    }

    #[test]
    fn clear_blocker_touches_session() {
        let conn = db();
        insert_basic(&conn, "ws-cbts", "blocked");
        clear_blocker(&conn, "ws-cbts", 400).unwrap();
        let row = get_by_id(&conn, "ws-cbts").unwrap().unwrap();
        assert_eq!(row.last_touched_session, 400);
    }

    #[test]
    fn clear_blocker_missing_ws_returns_false() {
        let conn = db();
        let found = clear_blocker(&conn, "phantom", 1).unwrap();
        assert!(!found);
    }

    #[test]
    fn clear_blocker_nulls_blocker_field() {
        let conn = db();
        insert_basic(&conn, "ws-null", "active");
        set_blocker(&conn, "ws-null", "temporary", 1).unwrap();
        clear_blocker(&conn, "ws-null", 2).unwrap();
        let row = get_by_id(&conn, "ws-null").unwrap().unwrap();
        assert!(row.blocker.is_none());
    }

    // ---- set_blocker / clear_blocker round-trip ----

    #[test]
    fn blocker_round_trip() {
        let conn = db();
        insert_basic(&conn, "ws-rtp", "active");
        set_blocker(&conn, "ws-rtp", "waiting on PR", 110).unwrap();
        {
            let row = get_by_id(&conn, "ws-rtp").unwrap().unwrap();
            assert_eq!(row.status, "blocked");
            assert!(row.blocker.is_some());
        }
        clear_blocker(&conn, "ws-rtp", 111).unwrap();
        {
            let row = get_by_id(&conn, "ws-rtp").unwrap().unwrap();
            assert_eq!(row.status, "active");
            assert!(row.blocker.is_none());
        }
    }

    // ---- get_active ----

    #[test]
    fn get_active_returns_only_active() {
        let conn = db();
        insert_basic(&conn, "ws-a1", "active");
        insert_basic(&conn, "ws-a2", "active");
        insert_basic(&conn, "ws-b1", "blocked");
        insert_basic(&conn, "ws-d1", "deferred");
        insert_basic(&conn, "ws-c1", "complete");
        let active = get_active(&conn).unwrap();
        assert_eq!(active.len(), 2);
        assert!(active.iter().all(|r| r.status == "active"));
    }

    #[test]
    fn get_active_empty_when_none() {
        let conn = db();
        let active = get_active(&conn).unwrap();
        assert!(active.is_empty());
    }

    #[test]
    fn get_active_ordered_by_priority_asc() {
        let conn = db();
        insert_basic(&conn, "ws-p3", "active");
        insert_basic(&conn, "ws-p1", "active");
        insert_basic(&conn, "ws-p2", "active");
        // Override priority via raw SQL (insert_workstream uses default 5)
        conn.execute(
            "UPDATE workstream SET priority = 3 WHERE ws_id = 'ws-p3'",
            [],
        )
        .unwrap();
        conn.execute(
            "UPDATE workstream SET priority = 1 WHERE ws_id = 'ws-p1'",
            [],
        )
        .unwrap();
        conn.execute(
            "UPDATE workstream SET priority = 2 WHERE ws_id = 'ws-p2'",
            [],
        )
        .unwrap();
        let active = get_active(&conn).unwrap();
        let ids: Vec<&str> = active.iter().map(|r| r.ws_id.as_str()).collect();
        assert_eq!(ids, vec!["ws-p1", "ws-p2", "ws-p3"]);
    }

    #[test]
    fn get_active_excludes_blocked() {
        let conn = db();
        insert_basic(&conn, "ws-excl", "blocked");
        let active = get_active(&conn).unwrap();
        assert!(active.is_empty());
    }

    // ---- get_blocked ----

    #[test]
    fn get_blocked_returns_only_blocked() {
        let conn = db();
        insert_basic(&conn, "ws-gb1", "blocked");
        insert_basic(&conn, "ws-gb2", "blocked");
        insert_basic(&conn, "ws-ga1", "active");
        let blocked = get_blocked(&conn).unwrap();
        assert_eq!(blocked.len(), 2);
        assert!(blocked.iter().all(|r| r.status == "blocked"));
    }

    #[test]
    fn get_blocked_empty_when_none() {
        let conn = db();
        let blocked = get_blocked(&conn).unwrap();
        assert!(blocked.is_empty());
    }

    #[test]
    fn get_blocked_contains_blocker_text() {
        let conn = db();
        insert_basic(&conn, "ws-bt", "active");
        set_blocker(&conn, "ws-bt", "waiting on CI", 109).unwrap();
        let blocked = get_blocked(&conn).unwrap();
        assert_eq!(blocked.len(), 1);
        assert_eq!(blocked[0].blocker.as_deref(), Some("waiting on CI"));
    }

    // ---- touch ----

    #[test]
    fn touch_updates_session() {
        let conn = db();
        insert_basic(&conn, "ws-touch", "active");
        let found = touch(&conn, "ws-touch", 999).unwrap();
        assert!(found);
        let row = get_by_id(&conn, "ws-touch").unwrap().unwrap();
        assert_eq!(row.last_touched_session, 999);
    }

    #[test]
    fn touch_does_not_change_status() {
        let conn = db();
        insert_basic(&conn, "ws-tncs", "deferred");
        touch(&conn, "ws-tncs", 999).unwrap();
        let row = get_by_id(&conn, "ws-tncs").unwrap().unwrap();
        assert_eq!(row.status, "deferred");
    }

    #[test]
    fn touch_missing_ws_returns_false() {
        let conn = db();
        let found = touch(&conn, "no-ws", 1).unwrap();
        assert!(!found);
    }

    // ---- update_progress ----

    #[test]
    fn update_progress_sets_items() {
        let conn = db();
        insert_basic(&conn, "ws-up", "active");
        let found = update_progress(&conn, "ws-up", 3, 10, 112).unwrap();
        assert!(found);
        let row = get_by_id(&conn, "ws-up").unwrap().unwrap();
        assert_eq!(row.items_done, Some(3));
        assert_eq!(row.items_total, Some(10));
    }

    #[test]
    fn update_progress_touches_session() {
        let conn = db();
        insert_basic(&conn, "ws-upts", "active");
        update_progress(&conn, "ws-upts", 1, 5, 500).unwrap();
        let row = get_by_id(&conn, "ws-upts").unwrap().unwrap();
        assert_eq!(row.last_touched_session, 500);
    }

    #[test]
    fn update_progress_missing_returns_false() {
        let conn = db();
        let found = update_progress(&conn, "ghost", 1, 5, 1).unwrap();
        assert!(!found);
    }

    #[test]
    fn update_progress_to_complete() {
        let conn = db();
        insert_basic(&conn, "ws-done", "active");
        update_progress(&conn, "ws-done", 10, 10, 110).unwrap();
        let row = get_by_id(&conn, "ws-done").unwrap().unwrap();
        assert_eq!(row.items_done, Some(10));
        assert_eq!(row.items_total, Some(10));
    }

    #[test]
    fn update_progress_overwrites_previous() {
        let conn = db();
        insert_basic(&conn, "ws-ovr", "active");
        update_progress(&conn, "ws-ovr", 2, 8, 110).unwrap();
        update_progress(&conn, "ws-ovr", 5, 8, 111).unwrap();
        let row = get_by_id(&conn, "ws-ovr").unwrap().unwrap();
        assert_eq!(row.items_done, Some(5));
    }

    // ---- count_by_status ----

    #[test]
    fn count_by_status_active() {
        let conn = db();
        insert_basic(&conn, "ws-ca1", "active");
        insert_basic(&conn, "ws-ca2", "active");
        insert_basic(&conn, "ws-cb3", "blocked");
        assert_eq!(count_by_status(&conn, "active").unwrap(), 2);
    }

    #[test]
    fn count_by_status_blocked() {
        let conn = db();
        insert_basic(&conn, "ws-blk1", "blocked");
        assert_eq!(count_by_status(&conn, "blocked").unwrap(), 1);
    }

    #[test]
    fn count_by_status_returns_zero_when_empty() {
        let conn = db();
        assert_eq!(count_by_status(&conn, "complete").unwrap(), 0);
    }

    #[test]
    fn count_by_status_unknown_status_returns_zero() {
        let conn = db();
        insert_basic(&conn, "ws-cunk", "active");
        assert_eq!(count_by_status(&conn, "nonexistent").unwrap(), 0);
    }

    #[test]
    fn count_by_status_all_statuses() {
        let conn = db();
        for (i, status) in ["active", "blocked", "deferred", "complete"]
            .iter()
            .enumerate()
        {
            let id = format!("ws-all-{i}");
            insert_basic(&conn, &id, status);
        }
        assert_eq!(count_by_status(&conn, "active").unwrap(), 1);
        assert_eq!(count_by_status(&conn, "blocked").unwrap(), 1);
        assert_eq!(count_by_status(&conn, "deferred").unwrap(), 1);
        assert_eq!(count_by_status(&conn, "complete").unwrap(), 1);
    }

    // ---- serde round-trip for WorkstreamRow ----

    #[test]
    fn workstream_row_serde_roundtrip() {
        let conn = db();
        insert_basic(&conn, "ws-serde", "active");
        let row = get_by_id(&conn, "ws-serde").unwrap().unwrap();
        let json = serde_json::to_string(&row).unwrap();
        let back: WorkstreamRow = serde_json::from_str(&json).unwrap();
        assert_eq!(back.ws_id, row.ws_id);
        assert_eq!(back.status, row.status);
        assert_eq!(back.priority, row.priority);
    }

    #[test]
    fn workstream_row_debug_not_empty() {
        let conn = db();
        insert_basic(&conn, "ws-dbg", "active");
        let row = get_by_id(&conn, "ws-dbg").unwrap().unwrap();
        let dbg = format!("{row:?}");
        assert!(!dbg.is_empty());
        assert!(dbg.contains("ws-dbg"));
    }

    // ---- composite scenarios ----

    #[test]
    fn full_lifecycle_active_blocked_active() {
        let conn = db();
        insert_workstream(&conn, "ws-life", "Full lifecycle", "active", 109, "start ctx")
            .unwrap();
        // block it
        set_blocker(&conn, "ws-life", "apt locked", 110).unwrap();
        assert_eq!(count_by_status(&conn, "blocked").unwrap(), 1);
        // clear it
        clear_blocker(&conn, "ws-life", 111).unwrap();
        assert_eq!(count_by_status(&conn, "active").unwrap(), 1);
        // complete it
        update_progress(&conn, "ws-life", 5, 5, 112).unwrap();
        update_status(&conn, "ws-life", "complete", 112).unwrap();
        assert_eq!(count_by_status(&conn, "complete").unwrap(), 1);
    }

    #[test]
    fn priority_ordering_preserved_across_insertions() {
        let conn = db();
        // insert in descending priority order
        for (i, priority) in [10u32, 3, 7, 1, 5].iter().enumerate() {
            let id = format!("ws-ord-{i}");
            insert_workstream(&conn, &id, "T", "active", 109, "ctx").unwrap();
            conn.execute(
                "UPDATE workstream SET priority = ?1 WHERE ws_id = ?2",
                params![priority, id],
            )
            .unwrap();
        }
        let active = get_active(&conn).unwrap();
        let priorities: Vec<i32> = active.iter().map(|r| r.priority).collect();
        let mut sorted = priorities.clone();
        sorted.sort_unstable();
        assert_eq!(priorities, sorted, "get_active must return rows ORDER BY priority ASC");
    }

    #[test]
    fn multiple_blocked_workstreams_all_returned() {
        let conn = db();
        for i in 0..5 {
            let id = format!("ws-mb-{i}");
            insert_basic(&conn, &id, "active");
            set_blocker(&conn, &id, "shared blocker", 109).unwrap();
        }
        let blocked = get_blocked(&conn).unwrap();
        assert_eq!(blocked.len(), 5);
    }

    #[test]
    fn touch_updates_without_affecting_other_fields() {
        let conn = db();
        insert_workstream(&conn, "ws-touch2", "Stable", "deferred", 109, "ctx for resume")
            .unwrap();
        touch(&conn, "ws-touch2", 150).unwrap();
        let row = get_by_id(&conn, "ws-touch2").unwrap().unwrap();
        assert_eq!(row.title, "Stable");
        assert_eq!(row.status, "deferred");
        assert_eq!(row.resume_context, "ctx for resume");
        assert_eq!(row.last_touched_session, 150);
    }
}
