//! `m07_causal_chain` — CRUD for the `causal_chain` table.
//!
//! Provides [`insert_chain`], [`resolve_chain`], [`reinforce_chain`],
//! [`find_unresolved`], [`find_by_label`], [`auto_resolve_stale`], and
//! [`count_unresolved`].
//!
//! This is the key table from the Historian — every unresolved trap, bug,
//! plan, or pattern that needs reinforcement lives here, sorted by
//! `reinforcement_count DESC` so the hottest issues surface first.
//!
//! # Layer
//!
//! `m2_schema`
//!
//! # Dependencies
//!
//! `m01_types`, `m02_errors`, `m06_schema`

#[cfg(feature = "sqlite")]
use rusqlite::{Connection, OptionalExtension as _, params};
use serde::{Deserialize, Serialize};

#[cfg(feature = "sqlite")]
use crate::m1_foundation::m02_errors::SchemaError;

#[cfg(feature = "sqlite")]
use super::sqlite_err;

// ---------------------------------------------------------------------------
// CausalChainRow — plain data record mirroring the table columns
// ---------------------------------------------------------------------------

/// A single row from the `causal_chain` table.
///
/// All columns are present; `resolved_session` and `last_reinforced_session`
/// are nullable and represented as [`Option`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalChainRow {
    /// Primary key, auto-assigned by `SQLite`.
    pub id: i64,
    /// Session number in which this chain was first observed.
    pub origin_session: u32,
    /// Session number in which this chain was resolved, or `None` if still open.
    pub resolved_session: Option<u32>,
    /// One of `"bug"`, `"trap"`, `"plan"`, or `"pattern"`.
    pub chain_type: String,
    /// Short, unique label used as a human-readable key.
    pub label: String,
    /// Full description of the chain — what happened and why it matters.
    pub description: String,
    /// How many sessions have reinforced (re-triggered) this chain.
    pub reinforcement_count: u32,
    /// Session number of the most recent reinforcement, or `None` if never
    /// reinforced after creation.
    pub last_reinforced_session: Option<u32>,
    /// Consent level: one of `"Emit"`, `"Store"`, or `"Forget"`.
    pub consent: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse a single [`CausalChainRow`] from a prepared-statement row.
#[cfg(feature = "sqlite")]
fn parse_row(row: &rusqlite::Row<'_>) -> Result<CausalChainRow, rusqlite::Error> {
    Ok(CausalChainRow {
        id: row.get(0)?,
        origin_session: row.get::<_, u32>(1)?,
        resolved_session: row.get::<_, Option<u32>>(2)?,
        chain_type: row.get::<_, String>(3)?,
        label: row.get::<_, String>(4)?,
        description: row.get::<_, String>(5)?,
        reinforcement_count: row.get::<_, u32>(6)?,
        last_reinforced_session: row.get::<_, Option<u32>>(7)?,
        consent: row.get::<_, String>(8)?,
    })
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Insert a new chain into the `causal_chain` table.
///
/// `reinforcement_count` defaults to 1, `consent` defaults to `"Emit"`,
/// and `resolved_session` / `last_reinforced_session` are left `NULL`.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] if the INSERT fails (e.g. a `CHECK`
/// constraint violation for an invalid `chain_type`).
#[cfg(feature = "sqlite")]
pub fn insert_chain(
    conn: &Connection,
    origin_session: u32,
    chain_type: &str,
    label: &str,
    description: &str,
) -> Result<i64, SchemaError> {
    conn.execute(
        "INSERT INTO causal_chain
             (origin_session, chain_type, label, description)
         VALUES (?1, ?2, ?3, ?4)",
        params![origin_session, chain_type, label, description],
    )
    .map_err(|e| sqlite_err(&e))?;
    Ok(conn.last_insert_rowid())
}

/// Mark a chain as resolved in `resolved_session`.
///
/// Returns `true` if a row with `id` existed and was updated, `false` if no
/// such row was found.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on database failure.
#[cfg(feature = "sqlite")]
pub fn resolve_chain(
    conn: &Connection,
    id: i64,
    resolved_session: u32,
) -> Result<bool, SchemaError> {
    let rows_changed = conn
        .execute(
            "UPDATE causal_chain SET resolved_session = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?2",
            params![resolved_session, id],
        )
        .map_err(|e| sqlite_err(&e))?;
    Ok(rows_changed > 0)
}

/// Reinforce an existing chain by `label`, incrementing `reinforcement_count`
/// and updating `last_reinforced_session`.
///
/// If no chain with `label` exists yet, a new row is inserted with
/// `chain_type = "trap"` and `reinforcement_count = 1` so that previously
/// unknown labels can be seeded automatically.
///
/// Returns `true` if an existing row was found and updated, `false` if a new
/// row was created.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on database failure.
#[cfg(feature = "sqlite")]
pub fn reinforce_chain(
    conn: &Connection,
    label: &str,
    session: u32,
) -> Result<bool, SchemaError> {
    let rows_changed = conn
        .execute(
            "UPDATE causal_chain
             SET reinforcement_count     = reinforcement_count + 1,
                 last_reinforced_session = ?1,
                 updated_at              = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE label = ?2",
            params![session, label],
        )
        .map_err(|e| sqlite_err(&e))?;

    if rows_changed > 0 {
        return Ok(true);
    }

    // Not found — create a seed row so callers can reinforce without a prior insert.
    conn.execute(
        "INSERT INTO causal_chain
             (origin_session, chain_type, label, description, last_reinforced_session)
         VALUES (?1, 'trap', ?2, 'auto-seeded by reinforce_chain', ?1)",
        params![session, label],
    )
    .map_err(|e| sqlite_err(&e))?;

    Ok(false)
}

/// Fetch all unresolved chains ordered by `reinforcement_count DESC`, limited
/// to `limit` rows.
///
/// "Unresolved" means `resolved_session IS NULL`.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on query or row-parse failure.
#[cfg(feature = "sqlite")]
pub fn find_unresolved(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<CausalChainRow>, SchemaError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, origin_session, resolved_session, chain_type, label,
                    description, reinforcement_count, last_reinforced_session, consent
             FROM causal_chain
             WHERE resolved_session IS NULL
             ORDER BY reinforcement_count DESC
             LIMIT ?1",
        )
        .map_err(|e| sqlite_err(&e))?;

    // SQLite LIMIT takes i64; usize → i64 is safe up to i64::MAX rows, which
    // exceeds any realistic result set.
    #[allow(clippy::cast_possible_wrap)]
    let limit_i64 = limit as i64;
    let rows = stmt
        .query_map(params![limit_i64], parse_row)
        .map_err(|e| sqlite_err(&e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| sqlite_err(&e))?;

    Ok(rows)
}

/// Find a chain by its `label`, returning `None` if not found.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on query or row-parse failure.
#[cfg(feature = "sqlite")]
pub fn find_by_label(
    conn: &Connection,
    label: &str,
) -> Result<Option<CausalChainRow>, SchemaError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, origin_session, resolved_session, chain_type, label,
                    description, reinforcement_count, last_reinforced_session, consent
             FROM causal_chain
             WHERE label = ?1
             LIMIT 1",
        )
        .map_err(|e| sqlite_err(&e))?;

    stmt.query_row(params![label], parse_row)
        .optional()
        .map_err(|e| sqlite_err(&e))
}

/// Auto-resolve all unresolved chains that have not been reinforced for
/// `threshold` or more sessions.
///
/// A chain is considered stale when:
/// - `resolved_session IS NULL`, AND
/// - `last_reinforced_session IS NOT NULL` AND
///   `current_session - last_reinforced_session >= threshold`, OR
/// - `last_reinforced_session IS NULL` AND
///   `current_session - origin_session >= threshold`
///
/// Returns the number of rows resolved.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on database failure.
#[cfg(feature = "sqlite")]
pub fn auto_resolve_stale(
    conn: &Connection,
    current_session: u32,
    threshold: u32,
) -> Result<u32, SchemaError> {
    auto_resolve_stale_typed(conn, current_session, threshold, threshold)
}

/// Type-aware auto-resolve: traps/patterns use `trap_threshold`, plans use
/// `plan_threshold`, bugs are **never** auto-resolved.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on database failure.
#[cfg(feature = "sqlite")]
pub fn auto_resolve_stale_typed(
    conn: &Connection,
    current_session: u32,
    trap_threshold: u32,
    plan_threshold: u32,
) -> Result<u32, SchemaError> {
    let trap_resolved = conn
        .execute(
            "UPDATE causal_chain
             SET resolved_session = ?1,
                 updated_at       = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE resolved_session IS NULL
               AND chain_type IN ('trap', 'pattern')
               AND (
                     (last_reinforced_session IS NOT NULL
                      AND (?1 - last_reinforced_session) >= ?2)
                  OR
                     (last_reinforced_session IS NULL
                      AND (?1 - origin_session) >= ?2)
               )",
            params![current_session, trap_threshold],
        )
        .map_err(|e| sqlite_err(&e))?;

    let plan_resolved = conn
        .execute(
            "UPDATE causal_chain
             SET resolved_session = ?1,
                 updated_at       = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE resolved_session IS NULL
               AND chain_type = 'plan'
               AND (
                     (last_reinforced_session IS NOT NULL
                      AND (?1 - last_reinforced_session) >= ?2)
                  OR
                     (last_reinforced_session IS NULL
                      AND (?1 - origin_session) >= ?2)
               )",
            params![current_session, plan_threshold],
        )
        .map_err(|e| sqlite_err(&e))?;

    #[allow(clippy::cast_possible_truncation)]
    Ok((trap_resolved + plan_resolved) as u32)
}

/// Count the number of unresolved chains (`resolved_session IS NULL`).
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on query failure.
#[cfg(feature = "sqlite")]
pub fn count_unresolved(conn: &Connection) -> Result<u64, SchemaError> {
    conn.query_row(
        "SELECT COUNT(*) FROM causal_chain WHERE resolved_session IS NULL",
        [],
        |row| row.get::<_, i64>(0),
    )
    .map(i64::cast_unsigned)
    .map_err(|e| sqlite_err(&e))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use crate::m2_schema::m06_schema::open_memory;

    // ------------------------------------------------------------------
    // insert_chain
    // ------------------------------------------------------------------

    #[test]
    #[cfg(feature = "sqlite")]
    fn insert_returns_positive_id() {
        let conn = open_memory().unwrap();
        let id = insert_chain(&conn, 109, "bug", "BUG-001", "test bug").unwrap();
        assert!(id > 0);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn insert_two_ids_are_distinct() {
        let conn = open_memory().unwrap();
        let id1 = insert_chain(&conn, 109, "bug", "BUG-001", "first").unwrap();
        let id2 = insert_chain(&conn, 109, "bug", "BUG-002", "second").unwrap();
        assert_ne!(id1, id2);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn insert_ids_are_monotonically_increasing() {
        let conn = open_memory().unwrap();
        let id1 = insert_chain(&conn, 109, "bug", "A", "a").unwrap();
        let id2 = insert_chain(&conn, 109, "bug", "B", "b").unwrap();
        let id3 = insert_chain(&conn, 109, "bug", "C", "c").unwrap();
        assert!(id1 < id2);
        assert!(id2 < id3);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn insert_chain_type_bug() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 1, "bug", "L1", "desc").unwrap();
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn insert_chain_type_trap() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 1, "trap", "L2", "desc").unwrap();
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn insert_chain_type_plan() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 1, "plan", "L3", "desc").unwrap();
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn insert_chain_type_pattern() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 1, "pattern", "L4", "desc").unwrap();
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn insert_invalid_chain_type_errors() {
        let conn = open_memory().unwrap();
        let result = insert_chain(&conn, 1, "invalid", "L5", "desc");
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn insert_empty_chain_type_errors() {
        let conn = open_memory().unwrap();
        let result = insert_chain(&conn, 1, "", "L6", "desc");
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn insert_default_reinforcement_count_is_one() {
        let conn = open_memory().unwrap();
        let id = insert_chain(&conn, 109, "bug", "RC-1", "test").unwrap();
        let row = find_by_label(&conn, "RC-1").unwrap().unwrap();
        assert_eq!(row.id, id);
        assert_eq!(row.reinforcement_count, 1);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn insert_default_consent_is_emit() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 109, "bug", "CONSENT-1", "test").unwrap();
        let row = find_by_label(&conn, "CONSENT-1").unwrap().unwrap();
        assert_eq!(row.consent, "Emit");
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn insert_resolved_session_is_null_by_default() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 109, "bug", "RS-1", "test").unwrap();
        let row = find_by_label(&conn, "RS-1").unwrap().unwrap();
        assert!(row.resolved_session.is_none());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn insert_last_reinforced_session_is_null_by_default() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 109, "bug", "LR-1", "test").unwrap();
        let row = find_by_label(&conn, "LR-1").unwrap().unwrap();
        assert!(row.last_reinforced_session.is_none());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn insert_stores_origin_session() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 42, "plan", "PLAN-1", "plan desc").unwrap();
        let row = find_by_label(&conn, "PLAN-1").unwrap().unwrap();
        assert_eq!(row.origin_session, 42);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn insert_stores_label_and_description() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 1, "bug", "LABEL-X", "My description").unwrap();
        let row = find_by_label(&conn, "LABEL-X").unwrap().unwrap();
        assert_eq!(row.label, "LABEL-X");
        assert_eq!(row.description, "My description");
    }

    // ------------------------------------------------------------------
    // resolve_chain
    // ------------------------------------------------------------------

    #[test]
    #[cfg(feature = "sqlite")]
    fn resolve_returns_true_for_known_id() {
        let conn = open_memory().unwrap();
        let id = insert_chain(&conn, 109, "bug", "BUG-R1", "resolve me").unwrap();
        let found = resolve_chain(&conn, id, 110).unwrap();
        assert!(found);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn resolve_returns_false_for_unknown_id() {
        let conn = open_memory().unwrap();
        let found = resolve_chain(&conn, 999_999, 110).unwrap();
        assert!(!found);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn resolve_sets_resolved_session() {
        let conn = open_memory().unwrap();
        let id = insert_chain(&conn, 109, "bug", "BUG-R2", "x").unwrap();
        resolve_chain(&conn, id, 115).unwrap();
        let row = find_by_label(&conn, "BUG-R2").unwrap().unwrap();
        assert_eq!(row.resolved_session, Some(115));
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn resolve_twice_overwrites_session() {
        let conn = open_memory().unwrap();
        let id = insert_chain(&conn, 109, "bug", "BUG-R3", "x").unwrap();
        resolve_chain(&conn, id, 110).unwrap();
        resolve_chain(&conn, id, 120).unwrap();
        let row = find_by_label(&conn, "BUG-R3").unwrap().unwrap();
        assert_eq!(row.resolved_session, Some(120));
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn resolve_does_not_affect_other_rows() {
        let conn = open_memory().unwrap();
        let id1 = insert_chain(&conn, 109, "bug", "OTHER-1", "x").unwrap();
        let id2 = insert_chain(&conn, 109, "bug", "OTHER-2", "y").unwrap();
        resolve_chain(&conn, id1, 110).unwrap();
        let row2 = find_by_label(&conn, "OTHER-2").unwrap().unwrap();
        assert!(row2.resolved_session.is_none());
        let _ = id2;
    }

    // ------------------------------------------------------------------
    // reinforce_chain
    // ------------------------------------------------------------------

    #[test]
    #[cfg(feature = "sqlite")]
    fn reinforce_existing_returns_true() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 109, "bug", "REINF-1", "x").unwrap();
        let found = reinforce_chain(&conn, "REINF-1", 110).unwrap();
        assert!(found);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn reinforce_missing_returns_false_and_creates_row() {
        let conn = open_memory().unwrap();
        let found = reinforce_chain(&conn, "NEW-LABEL", 110).unwrap();
        assert!(!found);
        let row = find_by_label(&conn, "NEW-LABEL").unwrap();
        assert!(row.is_some());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn reinforce_increments_count() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 109, "bug", "CNT-1", "x").unwrap();
        reinforce_chain(&conn, "CNT-1", 110).unwrap();
        let row = find_by_label(&conn, "CNT-1").unwrap().unwrap();
        assert_eq!(row.reinforcement_count, 2);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn reinforce_multiple_times_accumulates() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 109, "trap", "CNT-2", "trap").unwrap();
        reinforce_chain(&conn, "CNT-2", 110).unwrap();
        reinforce_chain(&conn, "CNT-2", 111).unwrap();
        reinforce_chain(&conn, "CNT-2", 112).unwrap();
        let row = find_by_label(&conn, "CNT-2").unwrap().unwrap();
        assert_eq!(row.reinforcement_count, 4);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn reinforce_updates_last_reinforced_session() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 109, "bug", "LRS-1", "x").unwrap();
        reinforce_chain(&conn, "LRS-1", 115).unwrap();
        let row = find_by_label(&conn, "LRS-1").unwrap().unwrap();
        assert_eq!(row.last_reinforced_session, Some(115));
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn reinforce_overwrites_last_session_on_second_call() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 109, "bug", "LRS-2", "x").unwrap();
        reinforce_chain(&conn, "LRS-2", 110).unwrap();
        reinforce_chain(&conn, "LRS-2", 120).unwrap();
        let row = find_by_label(&conn, "LRS-2").unwrap().unwrap();
        assert_eq!(row.last_reinforced_session, Some(120));
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn reinforce_auto_seeds_with_trap_type() {
        let conn = open_memory().unwrap();
        reinforce_chain(&conn, "AUTO-TRAP", 109).unwrap();
        let row = find_by_label(&conn, "AUTO-TRAP").unwrap().unwrap();
        assert_eq!(row.chain_type, "trap");
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn reinforce_auto_seeded_count_is_one() {
        let conn = open_memory().unwrap();
        reinforce_chain(&conn, "AUTO-COUNT", 109).unwrap();
        let row = find_by_label(&conn, "AUTO-COUNT").unwrap().unwrap();
        assert_eq!(row.reinforcement_count, 1);
    }

    // ------------------------------------------------------------------
    // find_unresolved
    // ------------------------------------------------------------------

    #[test]
    #[cfg(feature = "sqlite")]
    fn find_unresolved_empty_db_returns_empty() {
        let conn = open_memory().unwrap();
        let rows = find_unresolved(&conn, 10).unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn find_unresolved_excludes_resolved_rows() {
        let conn = open_memory().unwrap();
        let id = insert_chain(&conn, 109, "bug", "DONE", "resolved").unwrap();
        resolve_chain(&conn, id, 110).unwrap();
        insert_chain(&conn, 109, "bug", "OPEN", "open").unwrap();
        let rows = find_unresolved(&conn, 10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].label, "OPEN");
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn find_unresolved_ordered_by_reinforcement_desc() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 109, "bug", "LOW", "x").unwrap();
        let id_high = insert_chain(&conn, 109, "bug", "HIGH", "x").unwrap();
        // reinforce HIGH twice → count 3
        reinforce_chain(&conn, "HIGH", 110).unwrap();
        reinforce_chain(&conn, "HIGH", 111).unwrap();
        let _ = id_high;
        let rows = find_unresolved(&conn, 10).unwrap();
        assert_eq!(rows[0].label, "HIGH");
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn find_unresolved_respects_limit() {
        let conn = open_memory().unwrap();
        for i in 0..10_u32 {
            insert_chain(&conn, 109, "bug", &format!("L{i}"), "x").unwrap();
        }
        let rows = find_unresolved(&conn, 3).unwrap();
        assert_eq!(rows.len(), 3);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn find_unresolved_limit_zero_returns_empty() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 109, "bug", "Z1", "x").unwrap();
        let rows = find_unresolved(&conn, 0).unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn find_unresolved_all_resolved_returns_empty() {
        let conn = open_memory().unwrap();
        let id1 = insert_chain(&conn, 109, "bug", "X1", "x").unwrap();
        let id2 = insert_chain(&conn, 109, "bug", "X2", "y").unwrap();
        resolve_chain(&conn, id1, 110).unwrap();
        resolve_chain(&conn, id2, 110).unwrap();
        let rows = find_unresolved(&conn, 10).unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn find_unresolved_rows_have_null_resolved_session() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 109, "bug", "NULL-RS", "x").unwrap();
        let rows = find_unresolved(&conn, 5).unwrap();
        for row in &rows {
            assert!(row.resolved_session.is_none());
        }
    }

    // ------------------------------------------------------------------
    // find_by_label
    // ------------------------------------------------------------------

    #[test]
    #[cfg(feature = "sqlite")]
    fn find_by_label_returns_none_for_unknown() {
        let conn = open_memory().unwrap();
        let result = find_by_label(&conn, "DOES-NOT-EXIST").unwrap();
        assert!(result.is_none());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn find_by_label_returns_row_for_known() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 109, "bug", "FOUND", "here").unwrap();
        let row = find_by_label(&conn, "FOUND").unwrap();
        assert!(row.is_some());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn find_by_label_returns_correct_fields() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 42, "plan", "PLAN-42", "my plan").unwrap();
        let row = find_by_label(&conn, "PLAN-42").unwrap().unwrap();
        assert_eq!(row.origin_session, 42);
        assert_eq!(row.chain_type, "plan");
        assert_eq!(row.label, "PLAN-42");
        assert_eq!(row.description, "my plan");
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn find_by_label_is_case_sensitive() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 109, "bug", "case-LABEL", "x").unwrap();
        let lower = find_by_label(&conn, "case-label").unwrap();
        assert!(lower.is_none());
        let upper = find_by_label(&conn, "case-LABEL").unwrap();
        assert!(upper.is_some());
    }

    // ------------------------------------------------------------------
    // auto_resolve_stale
    // ------------------------------------------------------------------

    #[test]
    #[cfg(feature = "sqlite")]
    fn auto_resolve_returns_zero_when_no_stale() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 109, "bug", "FRESH", "x").unwrap();
        // chain was inserted at session 109, current is 110, threshold 10 → not stale
        let count = auto_resolve_stale(&conn, 110, 10).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn auto_resolve_resolves_stale_by_origin() {
        let conn = open_memory().unwrap();
        // origin 100, current 115, threshold 10 → 15 ≥ 10 → stale
        insert_chain(&conn, 100, "trap", "STALE-1", "x").unwrap();
        let count = auto_resolve_stale(&conn, 115, 10).unwrap();
        assert_eq!(count, 1);
        let row = find_by_label(&conn, "STALE-1").unwrap().unwrap();
        assert_eq!(row.resolved_session, Some(115));
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn auto_resolve_resolves_stale_by_last_reinforced() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 100, "trap", "STALE-2", "x").unwrap();
        reinforce_chain(&conn, "STALE-2", 102).unwrap();
        // last reinforced = 102, current = 120, threshold = 10 → 18 ≥ 10 → stale
        let count = auto_resolve_stale(&conn, 120, 10).unwrap();
        assert_eq!(count, 1);
        let row = find_by_label(&conn, "STALE-2").unwrap().unwrap();
        assert_eq!(row.resolved_session, Some(120));
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn auto_resolve_does_not_touch_recently_reinforced() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 100, "bug", "ACTIVE", "x").unwrap();
        reinforce_chain(&conn, "ACTIVE", 118).unwrap();
        // last reinforced = 118, current = 120, threshold = 10 → 2 < 10 → not stale
        let count = auto_resolve_stale(&conn, 120, 10).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn auto_resolve_skips_already_resolved() {
        let conn = open_memory().unwrap();
        let id = insert_chain(&conn, 100, "bug", "ALREADY", "x").unwrap();
        resolve_chain(&conn, id, 105).unwrap();
        let count = auto_resolve_stale(&conn, 200, 1).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn auto_resolve_returns_correct_count_for_multiple() {
        let conn = open_memory().unwrap();
        // Three stale rows (trap — eligible for auto-resolve)
        for i in 0..3_u32 {
            insert_chain(&conn, 100, "trap", &format!("STALE-M{i}"), "x").unwrap();
        }
        // One fresh row
        insert_chain(&conn, 119, "trap", "FRESH-M", "x").unwrap();
        let count = auto_resolve_stale(&conn, 120, 10).unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn auto_resolve_threshold_boundary_exclusive() {
        let conn = open_memory().unwrap();
        // origin 110, current 119, threshold 10 → gap 9 < 10 → NOT stale
        insert_chain(&conn, 110, "bug", "BOUNDARY", "x").unwrap();
        let count = auto_resolve_stale(&conn, 119, 10).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn auto_resolve_threshold_boundary_inclusive() {
        let conn = open_memory().unwrap();
        // origin 110, current 120, threshold 10 → gap 10 ≥ 10 → stale
        insert_chain(&conn, 110, "trap", "BOUNDARY2", "x").unwrap();
        let count = auto_resolve_stale(&conn, 120, 10).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn auto_resolve_never_resolves_bugs() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 100, "bug", "BUG-FOREVER", "x").unwrap();
        let count = auto_resolve_stale(&conn, 500, 10).unwrap();
        assert_eq!(count, 0);
        let row = find_by_label(&conn, "BUG-FOREVER").unwrap().unwrap();
        assert!(row.resolved_session.is_none());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn auto_resolve_typed_plans_use_plan_threshold() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 100, "plan", "PLAN-LONG", "x").unwrap();
        // At session 140 with plan_threshold=50 → gap 40 < 50 → not stale
        let count = auto_resolve_stale_typed(&conn, 140, 10, 50).unwrap();
        assert_eq!(count, 0);
        // At session 155 → gap 55 ≥ 50 → stale
        let count = auto_resolve_stale_typed(&conn, 155, 10, 50).unwrap();
        assert_eq!(count, 1);
    }

    // ------------------------------------------------------------------
    // count_unresolved
    // ------------------------------------------------------------------

    #[test]
    #[cfg(feature = "sqlite")]
    fn count_unresolved_empty_db_is_zero() {
        let conn = open_memory().unwrap();
        assert_eq!(count_unresolved(&conn).unwrap(), 0);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn count_unresolved_counts_only_open() {
        let conn = open_memory().unwrap();
        let id1 = insert_chain(&conn, 109, "bug", "CU-1", "x").unwrap();
        insert_chain(&conn, 109, "bug", "CU-2", "y").unwrap();
        resolve_chain(&conn, id1, 110).unwrap();
        assert_eq!(count_unresolved(&conn).unwrap(), 1);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn count_unresolved_increases_on_insert() {
        let conn = open_memory().unwrap();
        assert_eq!(count_unresolved(&conn).unwrap(), 0);
        insert_chain(&conn, 109, "bug", "CU-INC-1", "x").unwrap();
        assert_eq!(count_unresolved(&conn).unwrap(), 1);
        insert_chain(&conn, 109, "bug", "CU-INC-2", "y").unwrap();
        assert_eq!(count_unresolved(&conn).unwrap(), 2);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn count_unresolved_decreases_on_resolve() {
        let conn = open_memory().unwrap();
        let id = insert_chain(&conn, 109, "bug", "CU-DEC", "x").unwrap();
        assert_eq!(count_unresolved(&conn).unwrap(), 1);
        resolve_chain(&conn, id, 110).unwrap();
        assert_eq!(count_unresolved(&conn).unwrap(), 0);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn count_unresolved_after_auto_resolve() {
        let conn = open_memory().unwrap();
        for i in 0..5_u32 {
            insert_chain(&conn, 100, "trap", &format!("AR-{i}"), "x").unwrap();
        }
        assert_eq!(count_unresolved(&conn).unwrap(), 5);
        auto_resolve_stale(&conn, 120, 10).unwrap();
        assert_eq!(count_unresolved(&conn).unwrap(), 0);
    }

    // ------------------------------------------------------------------
    // CausalChainRow struct and field completeness
    // ------------------------------------------------------------------

    #[test]
    #[cfg(feature = "sqlite")]
    fn row_struct_all_fields_present() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 109, "pattern", "STRUCT-1", "pattern desc").unwrap();
        let row = find_by_label(&conn, "STRUCT-1").unwrap().unwrap();
        assert!(row.id > 0);
        assert_eq!(row.origin_session, 109);
        assert!(row.resolved_session.is_none());
        assert_eq!(row.chain_type, "pattern");
        assert_eq!(row.label, "STRUCT-1");
        assert_eq!(row.description, "pattern desc");
        assert_eq!(row.reinforcement_count, 1);
        assert!(row.last_reinforced_session.is_none());
        assert_eq!(row.consent, "Emit");
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn row_is_clone() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 109, "bug", "CLONE-1", "x").unwrap();
        let row = find_by_label(&conn, "CLONE-1").unwrap().unwrap();
        let _cloned = row.clone();
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn row_debug_not_empty() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 109, "bug", "DBG-1", "x").unwrap();
        let row = find_by_label(&conn, "DBG-1").unwrap().unwrap();
        let dbg = format!("{row:?}");
        assert!(dbg.contains("DBG-1"));
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn row_serializes_to_json() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 109, "trap", "JSON-1", "json test").unwrap();
        let row = find_by_label(&conn, "JSON-1").unwrap().unwrap();
        let json = serde_json::to_string(&row).unwrap();
        assert!(json.contains("JSON-1"));
        assert!(json.contains("trap"));
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn row_roundtrips_through_json() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 42, "plan", "RT-1", "roundtrip").unwrap();
        let row = find_by_label(&conn, "RT-1").unwrap().unwrap();
        let json = serde_json::to_string(&row).unwrap();
        let decoded: CausalChainRow = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.label, row.label);
        assert_eq!(decoded.origin_session, row.origin_session);
        assert_eq!(decoded.chain_type, row.chain_type);
    }

    // ------------------------------------------------------------------
    // Edge cases / integration
    // ------------------------------------------------------------------

    #[test]
    #[cfg(feature = "sqlite")]
    fn all_chain_types_roundtrip() {
        let conn = open_memory().unwrap();
        for ct in &["bug", "trap", "plan", "pattern"] {
            let label = format!("ALL-{ct}");
            insert_chain(&conn, 1, ct, &label, "desc").unwrap();
            let row = find_by_label(&conn, &label).unwrap().unwrap();
            assert_eq!(&row.chain_type, ct);
        }
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn large_description_stores_correctly() {
        let conn = open_memory().unwrap();
        let long_desc = "x".repeat(10_000);
        insert_chain(&conn, 109, "bug", "LONG-DESC", &long_desc).unwrap();
        let row = find_by_label(&conn, "LONG-DESC").unwrap().unwrap();
        assert_eq!(row.description.len(), 10_000);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn unicode_label_and_description() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 109, "bug", "🐛-BUG", "emoji in label 🌍").unwrap();
        let row = find_by_label(&conn, "🐛-BUG").unwrap().unwrap();
        assert_eq!(row.label, "🐛-BUG");
        assert_eq!(row.description, "emoji in label 🌍");
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn find_unresolved_correct_ordering_with_many_rows() {
        let conn = open_memory().unwrap();
        // Insert rows with decreasing reinforcement counts
        for (i, extra_reinforcements) in [0_u32, 5, 2, 10, 1].iter().enumerate() {
            let label = format!("ORD-{i}");
            insert_chain(&conn, 109, "bug", &label, "x").unwrap();
            for s in 0..*extra_reinforcements {
                reinforce_chain(&conn, &label, 110 + s).unwrap();
            }
        }
        let rows = find_unresolved(&conn, 5).unwrap();
        // Should be ordered: ORD-3 (11), ORD-1 (6), ORD-2 (3), ORD-4 (2), ORD-0 (1)
        assert_eq!(rows[0].reinforcement_count, 11);
        assert_eq!(rows[1].reinforcement_count, 6);
        assert_eq!(rows[2].reinforcement_count, 3);
        assert_eq!(rows[3].reinforcement_count, 2);
        assert_eq!(rows[4].reinforcement_count, 1);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn reinforce_does_not_affect_resolved_status() {
        let conn = open_memory().unwrap();
        let id = insert_chain(&conn, 109, "bug", "REINF-RS", "x").unwrap();
        resolve_chain(&conn, id, 110).unwrap();
        reinforce_chain(&conn, "REINF-RS", 111).unwrap();
        let row = find_by_label(&conn, "REINF-RS").unwrap().unwrap();
        // resolved_session must remain 110 — reinforce only touches count + last_session
        assert_eq!(row.resolved_session, Some(110));
        assert_eq!(row.reinforcement_count, 2);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn session_zero_is_valid() {
        let conn = open_memory().unwrap();
        let id = insert_chain(&conn, 0, "bug", "S0-1", "session zero").unwrap();
        assert!(id > 0);
        let row = find_by_label(&conn, "S0-1").unwrap().unwrap();
        assert_eq!(row.origin_session, 0);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn resolve_chain_on_empty_db_returns_false() {
        let conn = open_memory().unwrap();
        let found = resolve_chain(&conn, 1, 100).unwrap();
        assert!(!found);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn find_unresolved_large_limit_returns_all() {
        let conn = open_memory().unwrap();
        for i in 0..7_u32 {
            insert_chain(&conn, 109, "bug", &format!("BIG-{i}"), "x").unwrap();
        }
        let rows = find_unresolved(&conn, 100).unwrap();
        assert_eq!(rows.len(), 7);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn auto_resolve_threshold_zero_resolves_all() {
        let conn = open_memory().unwrap();
        insert_chain(&conn, 109, "trap", "TH0-A", "x").unwrap();
        insert_chain(&conn, 109, "pattern", "TH0-B", "y").unwrap();
        // gap ≥ 0 is always true for trap/pattern types
        let count = auto_resolve_stale(&conn, 109, 0).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn count_and_find_unresolved_agree() {
        let conn = open_memory().unwrap();
        for i in 0..6_u32 {
            insert_chain(&conn, 109, "bug", &format!("AGR-{i}"), "x").unwrap();
        }
        let cnt = count_unresolved(&conn).unwrap();
        let rows = find_unresolved(&conn, 100).unwrap();
        assert_eq!(cnt as usize, rows.len());
    }
}
