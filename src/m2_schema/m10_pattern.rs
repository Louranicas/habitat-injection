//! `m10_pattern` — CRUD for `reinforced_pattern` table.
//!
//! Operations: [`insert_pattern`], [`reinforce`] (`weight += 0.1*(1-weight)`,
//! `hit_count`++), [`decay_all`] (`weight *= rate` where `last_fired_session`
//! IS NOT NULL), [`prune_weak`] (`DELETE` where `weight < threshold`),
//! [`get_top_by_weight`], [`get_by_id`], [`get_by_category`], [`count`].
//!
//! Layer: `m2_schema`
//! Dependencies: `m01_types`, `m02_errors`, `m06_schema`
//! Memory Scientist + Performance Engineer

#[cfg(feature = "sqlite")]
use rusqlite::{Connection, OptionalExtension as _, params};
use serde::{Deserialize, Serialize};

#[cfg(feature = "sqlite")]
use crate::m1_foundation::m02_errors::SchemaError;

#[cfg(feature = "sqlite")]
use super::sqlite_err;

// ---------------------------------------------------------------------------
// PatternRow
// ---------------------------------------------------------------------------

/// A single row from the `reinforced_pattern` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternRow {
    /// Unique pattern identifier (e.g., `"verify-before-ship"`).
    pub pattern_id: String,
    /// Category — one of `procedural`, `semantic`, `trap`, `feedback`.
    pub category: String,
    /// Human-readable description of the pattern.
    pub description: String,
    /// Optional counter-example or anti-pattern description.
    pub anti_pattern: Option<String>,
    /// Hebbian weight in `[0.0, 1.0)`. Asymptotes toward `1.0` via `reinforce`.
    pub weight: f64,
    /// Number of times this pattern has been fired (natural + auto combined).
    pub hit_count: u32,
    /// Session number when the pattern was last fired, or `None` if never fired
    /// after initial insertion.
    pub last_fired_session: Option<u32>,
    /// Firings from context-matched session activity only. Buoy qualifies on this.
    #[serde(default)]
    pub natural_hit_count: u32,
    /// Comma-separated keywords for context matching against atuin history.
    #[serde(default)]
    pub keywords: String,
    /// Consent level — one of `Emit`, `Store`, `Forget`.
    pub consent: String,
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Parse a `PatternRow` from a `rusqlite::Row`.
#[cfg(feature = "sqlite")]
fn row_to_pattern(row: &rusqlite::Row<'_>) -> rusqlite::Result<PatternRow> {
    Ok(PatternRow {
        pattern_id: row.get(0)?,
        category: row.get(1)?,
        description: row.get(2)?,
        anti_pattern: row.get(3)?,
        weight: row.get(4)?,
        hit_count: row.get::<_, u32>(5)?,
        last_fired_session: row.get::<_, Option<u32>>(6)?,
        natural_hit_count: row.get::<_, u32>(7)?,
        keywords: row.get::<_, String>(8)?,
        consent: row.get(9)?,
    })
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Insert a new pattern.
///
/// The row is created with the default `weight` of `0.5` and `hit_count` of `1`.
/// `last_fired_session` is left `NULL` until the first [`reinforce`] call.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] if the insert fails (e.g., duplicate
/// `pattern_id` or invalid `category`).
#[cfg(feature = "sqlite")]
pub fn insert_pattern(
    conn: &Connection,
    pattern_id: &str,
    category: &str,
    description: &str,
    anti_pattern: Option<&str>,
) -> Result<(), SchemaError> {
    conn.execute(
        "INSERT INTO reinforced_pattern (pattern_id, category, description, anti_pattern)
         VALUES (?1, ?2, ?3, ?4)",
        params![pattern_id, category, description, anti_pattern],
    )
    .map_err(|e| sqlite_err(&e))?;
    Ok(())
}

/// Naturally reinforce a pattern from context-matched session activity.
///
/// Increments both `hit_count` and `natural_hit_count`. Buoy qualifies
/// on `natural_hit_count` only, so this is the path that earns buoy
/// protection.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on database error.
#[cfg(feature = "sqlite")]
pub fn natural_reinforce(
    conn: &Connection,
    pattern_id: &str,
    session: u32,
) -> Result<bool, SchemaError> {
    natural_reinforce_weighted(conn, pattern_id, session, 1.0)
}

/// Naturally reinforce with session intensity weighting.
///
/// `intensity` is in `[0.0, 1.0]` — derived from `min(1.0, tool_uses / BASELINE)`.
/// The weight update is `weight += REINFORCE_RATE * intensity * (1 - weight)`.
/// A 10-minute session with 5 tool calls gets weaker reinforcement than an
/// 8-hour session with 500 calls.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on database error.
#[cfg(feature = "sqlite")]
pub fn natural_reinforce_weighted(
    conn: &Connection,
    pattern_id: &str,
    session: u32,
    intensity: f64,
) -> Result<bool, SchemaError> {
    let rate = 0.1 * intensity.clamp(0.0, 1.0);
    let updated = conn
        .execute(
            "UPDATE reinforced_pattern
             SET weight              = weight + ?3 * (1.0 - weight),
                 hit_count           = hit_count + 1,
                 natural_hit_count   = natural_hit_count + 1,
                 last_fired_session  = ?2,
                 updated_at          = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE pattern_id = ?1",
            params![pattern_id, session, rate],
        )
        .map_err(|e| sqlite_err(&e))?;
    Ok(updated > 0)
}

/// Auto-reinforce a pattern (blanket fire). Increments `hit_count` only —
/// does NOT increment `natural_hit_count`. Buoy will not qualify on this.
///
/// Returns `true` if the `pattern_id` was found and updated, `false` if no
/// row existed.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on database error.
#[cfg(feature = "sqlite")]
pub fn reinforce(
    conn: &Connection,
    pattern_id: &str,
    session: u32,
) -> Result<bool, SchemaError> {
    let updated = conn
        .execute(
            "UPDATE reinforced_pattern
             SET weight             = weight + 0.1 * (1.0 - weight),
                 hit_count          = hit_count + 1,
                 last_fired_session = ?2,
                 updated_at         = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE pattern_id = ?1",
            params![pattern_id, session],
        )
        .map_err(|e| sqlite_err(&e))?;
    Ok(updated > 0)
}

/// Apply Hebbian decay to all patterns that have been fired at least once
/// (`last_fired_session IS NOT NULL`): `weight *= rate`.
///
/// Unfired patterns (seeded but never reinforced) decay with a floor at
/// `SEEDED_PATTERN_FLOOR` to prevent extinction before being tested.
///
/// Returns the number of rows updated.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on database error.
#[cfg(feature = "sqlite")]
pub fn decay_all(conn: &Connection, rate: f64) -> Result<u32, SchemaError> {
    use crate::m1_foundation::m05_constants::SEEDED_PATTERN_FLOOR;

    let fired = conn
        .execute(
            "UPDATE reinforced_pattern
             SET weight     = weight * ?1,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE last_fired_session IS NOT NULL",
            params![rate],
        )
        .map_err(|e| sqlite_err(&e))?;

    let unfired = conn
        .execute(
            "UPDATE reinforced_pattern
             SET weight     = MAX(?1, weight * ?2),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE last_fired_session IS NULL AND weight > ?1",
            params![SEEDED_PATTERN_FLOOR, rate],
        )
        .map_err(|e| sqlite_err(&e))?;

    Ok(u32::try_from(fired + unfired).unwrap_or(u32::MAX))
}

/// Delete all patterns whose `weight` is strictly below `threshold`.
///
/// Returns the number of rows deleted.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on database error.
#[cfg(feature = "sqlite")]
pub fn prune_weak(conn: &Connection, threshold: f64) -> Result<u32, SchemaError> {
    let deleted = conn
        .execute(
            "DELETE FROM reinforced_pattern WHERE weight < ?1",
            params![threshold],
        )
        .map_err(|e| sqlite_err(&e))?;
    Ok(u32::try_from(deleted).unwrap_or(u32::MAX))
}

/// Return up to `limit` patterns ordered by `weight DESC`.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on prepare or query failure.
#[cfg(feature = "sqlite")]
pub fn get_top_by_weight(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<PatternRow>, SchemaError> {
    let mut stmt = conn
        .prepare(
            "SELECT pattern_id, category, description, anti_pattern,
                    weight, hit_count, last_fired_session, natural_hit_count, keywords, consent
             FROM reinforced_pattern
             ORDER BY weight DESC
             LIMIT ?1",
        )
        .map_err(|e| sqlite_err(&e))?;

    let limit_i64 = i64::try_from(limit).unwrap_or(i64::MAX);
    let rows = stmt
        .query_map(params![limit_i64], row_to_pattern)
        .map_err(|e| sqlite_err(&e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| sqlite_err(&e))?;

    Ok(rows)
}

/// Return the pattern with the given `pattern_id`, or `None` if not found.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on database error.
#[cfg(feature = "sqlite")]
pub fn get_by_id(
    conn: &Connection,
    pattern_id: &str,
) -> Result<Option<PatternRow>, SchemaError> {
    let mut stmt = conn
        .prepare(
            "SELECT pattern_id, category, description, anti_pattern,
                    weight, hit_count, last_fired_session, natural_hit_count, keywords, consent
             FROM reinforced_pattern
             WHERE pattern_id = ?1",
        )
        .map_err(|e| sqlite_err(&e))?;

    stmt.query_row(params![pattern_id], row_to_pattern)
        .optional()
        .map_err(|e| sqlite_err(&e))
}

/// Return all patterns in the given `category` (one of `procedural`, `semantic`,
/// `trap`, `feedback`), ordered by `weight DESC`.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on database error.
#[cfg(feature = "sqlite")]
pub fn get_by_category(
    conn: &Connection,
    category: &str,
) -> Result<Vec<PatternRow>, SchemaError> {
    let mut stmt = conn
        .prepare(
            "SELECT pattern_id, category, description, anti_pattern,
                    weight, hit_count, last_fired_session, natural_hit_count, keywords, consent
             FROM reinforced_pattern
             WHERE category = ?1
             ORDER BY weight DESC",
        )
        .map_err(|e| sqlite_err(&e))?;

    let rows = stmt
        .query_map(params![category], row_to_pattern)
        .map_err(|e| sqlite_err(&e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| sqlite_err(&e))?;

    Ok(rows)
}

/// Apply buoy maintenance pulse to all qualified pathways.
///
/// A pathway qualifies when `hit_count >= threshold` (it has been naturally
/// reinforced enough times to prove its value). The buoy pulse is weaker than
/// full reinforcement: `weight += buoy_rate * (1 - weight)`.
///
/// Permanent: once qualified, always buoyed. No re-qualification needed.
///
/// Returns the number of rows updated.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on database error.
#[cfg(feature = "sqlite")]
pub fn buoy_qualified(
    conn: &Connection,
    buoy_rate: f64,
    hit_threshold: u32,
    current_session: u32,
    ttl_sessions: u32,
) -> Result<u32, SchemaError> {
    let updated = conn
        .execute(
            "UPDATE reinforced_pattern
             SET weight     = weight + ?1 * (1.0 - weight),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE natural_hit_count >= ?2
               AND last_fired_session IS NOT NULL
               AND (?3 - last_fired_session) < ?4",
            params![buoy_rate, hit_threshold, current_session, ttl_sessions],
        )
        .map_err(|e| sqlite_err(&e))?;
    Ok(u32::try_from(updated).unwrap_or(u32::MAX))
}

/// Return all pattern IDs grouped by category.
///
/// Returns a flat list of `(pattern_id, category)` pairs for every row.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on database error.
#[cfg(feature = "sqlite")]
pub fn list_all_ids(conn: &Connection) -> Result<Vec<(String, String, String)>, SchemaError> {
    let mut stmt = conn
        .prepare("SELECT pattern_id, category, keywords FROM reinforced_pattern")
        .map_err(|e| sqlite_err(&e))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| sqlite_err(&e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| sqlite_err(&e))?;
    Ok(rows)
}

/// Return the total number of rows in `reinforced_pattern`.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on database error.
#[cfg(feature = "sqlite")]
pub fn count(conn: &Connection) -> Result<u64, SchemaError> {
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM reinforced_pattern",
            [],
            |row| row.get(0),
        )
        .map_err(|e| sqlite_err(&e))?;
    Ok(n.cast_unsigned())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use crate::m2_schema::m06_schema::open_memory;

#[cfg(feature = "sqlite")]
    fn mem() -> Connection {
        open_memory().unwrap()
    }

    // ---- insert_pattern ----

    #[test]
    #[cfg(feature = "sqlite")]
    fn insert_minimal_pattern() {
        let conn = mem();
        insert_pattern(&conn, "p1", "procedural", "Always verify before deploying", None).unwrap();
        assert_eq!(count(&conn).unwrap(), 1);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn insert_with_anti_pattern() {
        let conn = mem();
        insert_pattern(
            &conn,
            "p1",
            "trap",
            "Use read-only forensics",
            Some("Never spawn background processes manually"),
        )
        .unwrap();
        let row = get_by_id(&conn, "p1").unwrap().unwrap();
        assert_eq!(
            row.anti_pattern.as_deref(),
            Some("Never spawn background processes manually")
        );
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn insert_sets_default_weight() {
        let conn = mem();
        insert_pattern(&conn, "p1", "semantic", "desc", None).unwrap();
        let row = get_by_id(&conn, "p1").unwrap().unwrap();
        assert!((row.weight - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn insert_sets_default_hit_count() {
        let conn = mem();
        insert_pattern(&conn, "p1", "feedback", "desc", None).unwrap();
        let row = get_by_id(&conn, "p1").unwrap().unwrap();
        assert_eq!(row.hit_count, 1);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn insert_last_fired_session_null_by_default() {
        let conn = mem();
        insert_pattern(&conn, "p1", "procedural", "desc", None).unwrap();
        let row = get_by_id(&conn, "p1").unwrap().unwrap();
        assert!(row.last_fired_session.is_none());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn insert_default_consent_is_emit() {
        let conn = mem();
        insert_pattern(&conn, "p1", "trap", "desc", None).unwrap();
        let row = get_by_id(&conn, "p1").unwrap().unwrap();
        assert_eq!(row.consent, "Emit");
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn insert_duplicate_pattern_id_fails() {
        let conn = mem();
        insert_pattern(&conn, "p1", "trap", "desc", None).unwrap();
        let result = insert_pattern(&conn, "p1", "semantic", "other", None);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn insert_invalid_category_fails() {
        let conn = mem();
        let result = insert_pattern(&conn, "p1", "bogus", "desc", None);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn insert_all_valid_categories() {
        let conn = mem();
        for (i, cat) in ["procedural", "semantic", "trap", "feedback"]
            .iter()
            .enumerate()
        {
            insert_pattern(&conn, &format!("p{i}"), cat, "desc", None).unwrap();
        }
        assert_eq!(count(&conn).unwrap(), 4);
    }

    // ---- reinforce ----

    #[test]
    #[cfg(feature = "sqlite")]
    fn reinforce_returns_true_when_found() {
        let conn = mem();
        insert_pattern(&conn, "p1", "procedural", "desc", None).unwrap();
        assert!(reinforce(&conn, "p1", 110).unwrap());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn reinforce_returns_false_when_not_found() {
        let conn = mem();
        assert!(!reinforce(&conn, "nonexistent", 110).unwrap());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn reinforce_updates_last_fired_session() {
        let conn = mem();
        insert_pattern(&conn, "p1", "procedural", "desc", None).unwrap();
        reinforce(&conn, "p1", 42).unwrap();
        let row = get_by_id(&conn, "p1").unwrap().unwrap();
        assert_eq!(row.last_fired_session, Some(42));
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn reinforce_increments_hit_count() {
        let conn = mem();
        insert_pattern(&conn, "p1", "semantic", "desc", None).unwrap();
        reinforce(&conn, "p1", 110).unwrap();
        reinforce(&conn, "p1", 111).unwrap();
        let row = get_by_id(&conn, "p1").unwrap().unwrap();
        assert_eq!(row.hit_count, 3); // starts at 1
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn reinforce_math_correct_single_step() {
        let conn = mem();
        insert_pattern(&conn, "p1", "trap", "desc", None).unwrap();
        // initial weight = 0.5
        reinforce(&conn, "p1", 1).unwrap();
        // expected: 0.5 + 0.1 * (1.0 - 0.5) = 0.55
        let row = get_by_id(&conn, "p1").unwrap().unwrap();
        assert!((row.weight - 0.55).abs() < 1e-9);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn reinforce_math_correct_second_step() {
        let conn = mem();
        insert_pattern(&conn, "p1", "trap", "desc", None).unwrap();
        reinforce(&conn, "p1", 1).unwrap(); // 0.55
        reinforce(&conn, "p1", 2).unwrap(); // 0.55 + 0.1*(1-0.55) = 0.595
        let row = get_by_id(&conn, "p1").unwrap().unwrap();
        assert!((row.weight - 0.595).abs() < 1e-9);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn reinforce_approaches_one_asymptotically() {
        let conn = mem();
        insert_pattern(&conn, "p1", "feedback", "desc", None).unwrap();
        for s in 0_u32..200 {
            reinforce(&conn, "p1", s).unwrap();
        }
        let row = get_by_id(&conn, "p1").unwrap().unwrap();
        // After many reinforcements weight must be very close to 1 but < 1
        assert!(row.weight > 0.999);
        assert!(row.weight < 1.0);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn reinforce_never_reaches_or_exceeds_one() {
        let conn = mem();
        insert_pattern(&conn, "p1", "procedural", "desc", None).unwrap();
        for s in 0_u32..1000 {
            reinforce(&conn, "p1", s).unwrap();
        }
        let row = get_by_id(&conn, "p1").unwrap().unwrap();
        assert!(row.weight < 1.0);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn reinforce_updates_session_to_latest() {
        let conn = mem();
        insert_pattern(&conn, "p1", "semantic", "desc", None).unwrap();
        reinforce(&conn, "p1", 100).unwrap();
        reinforce(&conn, "p1", 105).unwrap();
        let row = get_by_id(&conn, "p1").unwrap().unwrap();
        assert_eq!(row.last_fired_session, Some(105));
    }

    // ---- decay_all ----

    #[test]
    #[cfg(feature = "sqlite")]
    fn decay_all_returns_zero_when_no_fired_patterns_below_floor() {
        use crate::m1_foundation::m05_constants::SEEDED_PATTERN_FLOOR;
        let conn = mem();
        insert_pattern(&conn, "p1", "trap", "desc", None).unwrap();
        // Unfired at 0.5 is above floor (0.3) — decays but clamps at floor
        let updated = decay_all(&conn, 0.95).unwrap();
        assert_eq!(updated, 1);
        let p = get_by_id(&conn, "p1").unwrap().unwrap();
        assert!(p.weight >= SEEDED_PATTERN_FLOOR);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn decay_all_unfired_clamps_at_floor() {
        use crate::m1_foundation::m05_constants::SEEDED_PATTERN_FLOOR;
        let conn = mem();
        insert_pattern(&conn, "p1", "trap", "desc", None).unwrap();
        // Repeatedly decay — unfired pattern should never go below floor
        for _ in 0..200 {
            decay_all(&conn, 0.95).unwrap();
        }
        let p = get_by_id(&conn, "p1").unwrap().unwrap();
        assert!((p.weight - SEEDED_PATTERN_FLOOR).abs() < 1e-9);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn decay_all_fired_and_unfired_counted() {
        let conn = mem();
        insert_pattern(&conn, "p1", "trap", "desc", None).unwrap(); // unfired
        insert_pattern(&conn, "p2", "semantic", "desc2", None).unwrap(); // unfired
        reinforce(&conn, "p2", 109).unwrap(); // now fired
        let updated = decay_all(&conn, 0.95).unwrap();
        assert_eq!(updated, 2);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn decay_all_math_correct() {
        let conn = mem();
        insert_pattern(&conn, "p1", "trap", "desc", None).unwrap();
        reinforce(&conn, "p1", 109).unwrap(); // weight = 0.55
        decay_all(&conn, 0.95).unwrap();
        let row = get_by_id(&conn, "p1").unwrap().unwrap();
        let expected = 0.55 * 0.95;
        assert!((row.weight - expected).abs() < 1e-9);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn decay_all_returns_correct_count() {
        let conn = mem();
        for i in 0..5 {
            insert_pattern(&conn, &format!("p{i}"), "trap", "desc", None).unwrap();
            reinforce(&conn, &format!("p{i}"), 109).unwrap();
        }
        insert_pattern(&conn, "unfired", "semantic", "desc", None).unwrap();
        // 5 fired + 1 unfired above floor = 6
        let updated = decay_all(&conn, 0.9).unwrap();
        assert_eq!(updated, 6);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn decay_all_repeated_application_reduces_weight() {
        let conn = mem();
        insert_pattern(&conn, "p1", "trap", "desc", None).unwrap();
        reinforce(&conn, "p1", 1).unwrap();
        for _ in 0..50 {
            decay_all(&conn, 0.95).unwrap();
        }
        let row = get_by_id(&conn, "p1").unwrap().unwrap();
        // 0.55 * 0.95^50 ~ 0.0427
        assert!(row.weight < 0.1);
    }

    // ---- prune_weak ----

    #[test]
    #[cfg(feature = "sqlite")]
    fn prune_weak_returns_zero_when_nothing_below_threshold() {
        let conn = mem();
        insert_pattern(&conn, "p1", "trap", "desc", None).unwrap(); // weight=0.5
        let deleted = prune_weak(&conn, 0.1).unwrap();
        assert_eq!(deleted, 0);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn prune_weak_deletes_below_threshold() {
        let conn = mem();
        insert_pattern(&conn, "p1", "trap", "desc", None).unwrap();
        reinforce(&conn, "p1", 1).unwrap(); // weight → 0.55
        decay_all(&conn, 0.0).unwrap(); // weight → 0.0
        let deleted = prune_weak(&conn, 0.1).unwrap();
        assert_eq!(deleted, 1);
        assert_eq!(count(&conn).unwrap(), 0);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn prune_weak_does_not_delete_at_threshold() {
        let conn = mem();
        // Insert with explicit weight = threshold via raw SQL
        conn.execute(
            "INSERT INTO reinforced_pattern (pattern_id, category, description, weight)
             VALUES ('p1', 'trap', 'desc', 0.05)",
            [],
        )
        .unwrap();
        // weight = 0.05 is not < 0.05, so should not be pruned
        let deleted = prune_weak(&conn, 0.05).unwrap();
        assert_eq!(deleted, 0);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn prune_weak_deletes_correct_count() {
        let conn = mem();
        // 3 patterns with low weights, 2 with high
        for (i, w) in [0.01, 0.02, 0.03, 0.6, 0.9].iter().enumerate() {
            conn.execute(
                "INSERT INTO reinforced_pattern (pattern_id, category, description, weight)
                 VALUES (?1, 'trap', 'desc', ?2)",
                params![format!("p{i}"), w],
            )
            .unwrap();
        }
        let deleted = prune_weak(&conn, 0.05).unwrap();
        assert_eq!(deleted, 3);
        assert_eq!(count(&conn).unwrap(), 2);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn decay_then_prune_removes_weak_patterns() {
        let conn = mem();
        insert_pattern(&conn, "strong", "procedural", "desc", None).unwrap();
        insert_pattern(&conn, "weak", "trap", "desc", None).unwrap();
        // Reinforce both
        reinforce(&conn, "strong", 1).unwrap();
        reinforce(&conn, "weak", 1).unwrap();
        // Reinforce strong many more times
        for s in 2_u32..50 {
            reinforce(&conn, "strong", s).unwrap();
        }
        // Decay many times
        for _ in 0..100 {
            decay_all(&conn, 0.9).unwrap();
        }
        // prune below 0.05
        prune_weak(&conn, 0.05).unwrap();
        // strong should survive, weak should be gone
        assert!(count(&conn).unwrap() <= 1);
    }

    // ---- get_top_by_weight ----

    #[test]
    #[cfg(feature = "sqlite")]
    fn get_top_by_weight_empty_table() {
        let conn = mem();
        let rows = get_top_by_weight(&conn, 5).unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn get_top_by_weight_returns_ordered_desc() {
        let conn = mem();
        for (i, w) in [0.3, 0.9, 0.1, 0.7].iter().enumerate() {
            conn.execute(
                "INSERT INTO reinforced_pattern (pattern_id, category, description, weight)
                 VALUES (?1, 'trap', 'desc', ?2)",
                params![format!("p{i}"), w],
            )
            .unwrap();
        }
        let rows = get_top_by_weight(&conn, 4).unwrap();
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].pattern_id, "p1"); // weight=0.9
        assert_eq!(rows[1].pattern_id, "p3"); // weight=0.7
        assert_eq!(rows[2].pattern_id, "p0"); // weight=0.3
        assert_eq!(rows[3].pattern_id, "p2"); // weight=0.1
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn get_top_by_weight_limit_respected() {
        let conn = mem();
        for i in 0..10 {
            insert_pattern(&conn, &format!("p{i}"), "trap", "desc", None).unwrap();
        }
        let rows = get_top_by_weight(&conn, 3).unwrap();
        assert_eq!(rows.len(), 3);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn get_top_by_weight_zero_limit() {
        let conn = mem();
        insert_pattern(&conn, "p1", "trap", "desc", None).unwrap();
        let rows = get_top_by_weight(&conn, 0).unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn get_top_by_weight_all_fields_populated() {
        let conn = mem();
        insert_pattern(&conn, "p1", "procedural", "desc", Some("anti")).unwrap();
        reinforce(&conn, "p1", 109).unwrap();
        let rows = get_top_by_weight(&conn, 1).unwrap();
        let row = &rows[0];
        assert_eq!(row.pattern_id, "p1");
        assert_eq!(row.category, "procedural");
        assert_eq!(row.description, "desc");
        assert_eq!(row.anti_pattern.as_deref(), Some("anti"));
        assert!(row.weight > 0.5);
        assert_eq!(row.hit_count, 2);
        assert_eq!(row.last_fired_session, Some(109));
        assert_eq!(row.consent, "Emit");
    }

    // ---- get_by_id ----

    #[test]
    #[cfg(feature = "sqlite")]
    fn get_by_id_returns_none_when_missing() {
        let conn = mem();
        let row = get_by_id(&conn, "nonexistent").unwrap();
        assert!(row.is_none());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn get_by_id_returns_correct_row() {
        let conn = mem();
        insert_pattern(&conn, "p1", "feedback", "my feedback pattern", None).unwrap();
        let row = get_by_id(&conn, "p1").unwrap().unwrap();
        assert_eq!(row.pattern_id, "p1");
        assert_eq!(row.category, "feedback");
        assert_eq!(row.description, "my feedback pattern");
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn get_by_id_does_not_return_other_rows() {
        let conn = mem();
        insert_pattern(&conn, "p1", "trap", "desc1", None).unwrap();
        insert_pattern(&conn, "p2", "trap", "desc2", None).unwrap();
        let row = get_by_id(&conn, "p1").unwrap().unwrap();
        assert_eq!(row.pattern_id, "p1");
        assert_eq!(row.description, "desc1");
    }

    // ---- get_by_category ----

    #[test]
    #[cfg(feature = "sqlite")]
    fn get_by_category_empty_result() {
        let conn = mem();
        insert_pattern(&conn, "p1", "trap", "desc", None).unwrap();
        let rows = get_by_category(&conn, "procedural").unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn get_by_category_returns_only_matching() {
        let conn = mem();
        insert_pattern(&conn, "p1", "trap", "desc1", None).unwrap();
        insert_pattern(&conn, "p2", "trap", "desc2", None).unwrap();
        insert_pattern(&conn, "p3", "semantic", "desc3", None).unwrap();
        let rows = get_by_category(&conn, "trap").unwrap();
        assert_eq!(rows.len(), 2);
        for row in &rows {
            assert_eq!(row.category, "trap");
        }
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn get_by_category_ordered_by_weight_desc() {
        let conn = mem();
        conn.execute(
            "INSERT INTO reinforced_pattern (pattern_id, category, description, weight)
             VALUES ('low', 'procedural', 'desc', 0.2)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO reinforced_pattern (pattern_id, category, description, weight)
             VALUES ('high', 'procedural', 'desc', 0.8)",
            [],
        )
        .unwrap();
        let rows = get_by_category(&conn, "procedural").unwrap();
        assert_eq!(rows[0].pattern_id, "high");
        assert_eq!(rows[1].pattern_id, "low");
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn get_by_category_all_four_valid() {
        let conn = mem();
        for (i, cat) in ["procedural", "semantic", "trap", "feedback"]
            .iter()
            .enumerate()
        {
            insert_pattern(&conn, &format!("p{i}"), cat, "desc", None).unwrap();
        }
        for cat in &["procedural", "semantic", "trap", "feedback"] {
            let rows = get_by_category(&conn, cat).unwrap();
            assert_eq!(rows.len(), 1, "category={cat}");
        }
    }

    // ---- count ----

    #[test]
    #[cfg(feature = "sqlite")]
    fn count_empty_table() {
        let conn = mem();
        assert_eq!(count(&conn).unwrap(), 0);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn count_increments_on_insert() {
        let conn = mem();
        insert_pattern(&conn, "p1", "trap", "desc", None).unwrap();
        assert_eq!(count(&conn).unwrap(), 1);
        insert_pattern(&conn, "p2", "semantic", "desc", None).unwrap();
        assert_eq!(count(&conn).unwrap(), 2);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn count_decrements_on_prune() {
        let conn = mem();
        for i in 0..5 {
            insert_pattern(&conn, &format!("p{i}"), "trap", "desc", None).unwrap();
        }
        assert_eq!(count(&conn).unwrap(), 5);
        // Force weight to 0
        conn.execute(
            "UPDATE reinforced_pattern SET last_fired_session = 1",
            [],
        )
        .unwrap();
        decay_all(&conn, 0.0).unwrap();
        prune_weak(&conn, 0.01).unwrap();
        assert_eq!(count(&conn).unwrap(), 0);
    }

    // ---- integration / cross-function ----

    #[test]
    #[cfg(feature = "sqlite")]
    fn full_lifecycle_insert_reinforce_decay_prune() {
        // Mathematical analysis (all weights start at 0.5 per schema default):
        //
        // "strong": 30 reinforce steps → weight ≈ 1 - 0.9^31 * 0.5 ≈ 0.977
        // "weak"  : 1  reinforce step  → weight = 0.55
        //
        // After 17 decay steps at 0.85:
        //   strong: 0.977 * 0.85^17 ≈ 0.977 * 0.0631 ≈ 0.062  (>= 0.05 → survives)
        //   weak  : 0.55  * 0.85^17 ≈ 0.55  * 0.0631 ≈ 0.035  (<  0.05 → pruned)
        let conn = mem();
        insert_pattern(&conn, "strong", "procedural", "desc", None).unwrap();
        insert_pattern(&conn, "weak", "trap", "desc", None).unwrap();

        // Reinforce strong 30 times and weak once
        for s in 0_u32..30 {
            reinforce(&conn, "strong", s).unwrap();
        }
        reinforce(&conn, "weak", 1).unwrap();

        // Apply 17 rounds of decay at 0.85
        for _ in 0..17 {
            decay_all(&conn, 0.85).unwrap();
        }

        // Prune below 0.05 — weak should be pruned, strong should survive
        let pruned = prune_weak(&conn, 0.05).unwrap();
        assert_eq!(pruned, 1, "weak pattern should be the only one pruned");
        let remaining = count(&conn).unwrap();
        assert_eq!(remaining, 1, "strong pattern should survive");
        let survivor = get_by_id(&conn, "strong").unwrap();
        assert!(survivor.is_some(), "strong must be the survivor");
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn reinforce_then_get_top_shows_updated_weight() {
        let conn = mem();
        insert_pattern(&conn, "p1", "procedural", "desc", None).unwrap();
        insert_pattern(&conn, "p2", "procedural", "desc", None).unwrap();
        // Reinforce p2 more
        for s in 0_u32..5 {
            reinforce(&conn, "p2", s).unwrap();
        }
        let top = get_top_by_weight(&conn, 1).unwrap();
        assert_eq!(top[0].pattern_id, "p2");
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn category_filter_after_reinforce() {
        let conn = mem();
        insert_pattern(&conn, "t1", "trap", "desc", None).unwrap();
        insert_pattern(&conn, "t2", "trap", "desc", None).unwrap();
        insert_pattern(&conn, "s1", "semantic", "desc", None).unwrap();
        reinforce(&conn, "t2", 109).unwrap();
        let traps = get_by_category(&conn, "trap").unwrap();
        assert_eq!(traps.len(), 2);
        assert_eq!(traps[0].pattern_id, "t2"); // higher weight after reinforce
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn consent_values_survive_round_trip() {
        let conn = mem();
        for (i, consent) in ["Emit", "Store", "Forget"].iter().enumerate() {
            conn.execute(
                "INSERT INTO reinforced_pattern (pattern_id, category, description, consent)
                 VALUES (?1, 'trap', 'desc', ?2)",
                params![format!("p{i}"), consent],
            )
            .unwrap();
        }
        for (i, expected_consent) in ["Emit", "Store", "Forget"].iter().enumerate() {
            let row = get_by_id(&conn, &format!("p{i}")).unwrap().unwrap();
            assert_eq!(row.consent, *expected_consent);
        }
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn invalid_consent_fails() {
        let conn = mem();
        let result = conn.execute(
            "INSERT INTO reinforced_pattern (pattern_id, category, description, consent)
             VALUES ('x', 'trap', 'desc', 'BadConsent')",
            [],
        );
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn pattern_row_derives_clone() {
        let conn = mem();
        insert_pattern(&conn, "p1", "procedural", "desc", None).unwrap();
        let row = get_by_id(&conn, "p1").unwrap().unwrap();
        let cloned = row.clone();
        assert_eq!(row.pattern_id, cloned.pattern_id);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn pattern_row_serde_roundtrip() {
        let conn = mem();
        insert_pattern(&conn, "p1", "semantic", "desc", Some("anti")).unwrap();
        reinforce(&conn, "p1", 109).unwrap();
        let row = get_by_id(&conn, "p1").unwrap().unwrap();
        let json = serde_json::to_string(&row).unwrap();
        let deserialized: PatternRow = serde_json::from_str(&json).unwrap();
        assert_eq!(row.pattern_id, deserialized.pattern_id);
        assert!((row.weight - deserialized.weight).abs() < f64::EPSILON);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn multiple_reinforcements_hit_count_exact() {
        let conn = mem();
        insert_pattern(&conn, "p1", "trap", "desc", None).unwrap();
        // hit_count starts at 1; reinforce 9 more times → 10
        for s in 0_u32..9 {
            reinforce(&conn, "p1", s).unwrap();
        }
        let row = get_by_id(&conn, "p1").unwrap().unwrap();
        assert_eq!(row.hit_count, 10);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn decay_rate_zero_sets_weight_to_zero() {
        let conn = mem();
        insert_pattern(&conn, "p1", "trap", "desc", None).unwrap();
        reinforce(&conn, "p1", 1).unwrap();
        decay_all(&conn, 0.0).unwrap();
        let row = get_by_id(&conn, "p1").unwrap().unwrap();
        assert!(row.weight.abs() < f64::EPSILON);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn decay_rate_one_leaves_weight_unchanged() {
        let conn = mem();
        insert_pattern(&conn, "p1", "trap", "desc", None).unwrap();
        reinforce(&conn, "p1", 1).unwrap();
        let before = get_by_id(&conn, "p1").unwrap().unwrap().weight;
        decay_all(&conn, 1.0).unwrap();
        let after = get_by_id(&conn, "p1").unwrap().unwrap().weight;
        assert!((before - after).abs() < f64::EPSILON);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn prune_threshold_zero_deletes_nothing() {
        let conn = mem();
        insert_pattern(&conn, "p1", "trap", "desc", None).unwrap();
        // weight=0.5, which is NOT < 0.0
        let deleted = prune_weak(&conn, 0.0).unwrap();
        assert_eq!(deleted, 0);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn prune_threshold_one_deletes_all() {
        let conn = mem();
        for i in 0..4 {
            insert_pattern(&conn, &format!("p{i}"), "trap", "desc", None).unwrap();
        }
        // All weights are 0.5, which is < 1.0
        let deleted = prune_weak(&conn, 1.0).unwrap();
        assert_eq!(deleted, 4);
        assert_eq!(count(&conn).unwrap(), 0);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn asymptotic_convergence_50_steps() {
        // After N steps: w_n = 1 - 0.9^(n+1) * (1 - w0)
        // Verify the formula holds numerically.
        let conn = mem();
        insert_pattern(&conn, "p1", "procedural", "desc", None).unwrap();
        // initial weight = 0.5; after 1 step: 0.5 + 0.1*(1-0.5) = 0.55
        let mut expected = 0.5_f64;
        for s in 0_u32..50 {
            expected += 0.1 * (1.0 - expected);
            reinforce(&conn, "p1", s).unwrap();
        }
        let row = get_by_id(&conn, "p1").unwrap().unwrap();
        assert!((row.weight - expected).abs() < 1e-9);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn three_tier_separation_holds_over_100_sessions() {
        use crate::m1_foundation::m05_constants::{BUOY_RATE, BUOY_THRESHOLD, BUOY_TTL_SESSIONS, DECAY_RATE, SEEDED_PATTERN_FLOOR};

        let conn = mem();
        for i in 0..5_u32 {
            insert_pattern(&conn, &format!("active-{i}"), "procedural", "active pattern", None).unwrap();
            insert_pattern(&conn, &format!("buoyed-{i}"), "procedural", "buoyed pattern", None).unwrap();
            insert_pattern(&conn, &format!("floor-{i}"), "procedural", "floor pattern", None).unwrap();
        }

        for session in 1..=100_u32 {
            decay_all(&conn, DECAY_RATE).unwrap();
            buoy_qualified(&conn, BUOY_RATE, BUOY_THRESHOLD, session, BUOY_TTL_SESSIONS).unwrap();

            if session % 5 == 0 {
                for i in 0..5_u32 {
                    natural_reinforce(&conn, &format!("active-{i}"), session).unwrap();
                }
            }
            if session <= 20 && session % 5 == 0 {
                for i in 0..5_u32 {
                    natural_reinforce(&conn, &format!("buoyed-{i}"), session).unwrap();
                }
            }
        }

        let avg = |prefix: &str| -> f64 {
            let mut sum = 0.0;
            let mut count = 0_u32;
            for i in 0..5_u32 {
                let row = get_by_id(&conn, &format!("{prefix}{i}")).unwrap().unwrap();
                sum += row.weight;
                count += 1;
            }
            sum / f64::from(count)
        };

        let active_avg = avg("active-");
        let buoyed_avg = avg("buoyed-");
        let floor_avg = avg("floor-");

        assert!(active_avg > 0.65, "active tier should be >0.65, was {active_avg}");
        assert!(buoyed_avg > 0.4 && buoyed_avg < 0.65, "buoyed tier should be 0.4-0.65, was {buoyed_avg}");
        assert!((floor_avg - SEEDED_PATTERN_FLOOR).abs() < 0.05, "floor should be ~0.3, was {floor_avg}");
        assert!(active_avg - buoyed_avg > 0.15, "active-buoyed gap too small: {}", active_avg - buoyed_avg);
        assert!(buoyed_avg - floor_avg > 0.1, "buoyed-floor gap too small: {}", buoyed_avg - floor_avg);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn natural_reinforce_increments_both_counters() {
        let conn = mem();
        insert_pattern(&conn, "p1", "feedback", "desc", None).unwrap();
        natural_reinforce(&conn, "p1", 100).unwrap();
        natural_reinforce(&conn, "p1", 101).unwrap();
        let row = get_by_id(&conn, "p1").unwrap().unwrap();
        assert_eq!(row.hit_count, 3);
        assert_eq!(row.natural_hit_count, 2);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn auto_reinforce_does_not_increment_natural() {
        let conn = mem();
        insert_pattern(&conn, "p1", "feedback", "desc", None).unwrap();
        reinforce(&conn, "p1", 100).unwrap();
        reinforce(&conn, "p1", 101).unwrap();
        let row = get_by_id(&conn, "p1").unwrap().unwrap();
        assert_eq!(row.hit_count, 3);
        assert_eq!(row.natural_hit_count, 0);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn buoy_qualification_uses_natural_count() {
        use crate::m1_foundation::m05_constants::{BUOY_RATE, BUOY_THRESHOLD};
        let conn = mem();
        insert_pattern(&conn, "auto-only", "procedural", "desc", None).unwrap();
        insert_pattern(&conn, "natural", "procedural", "desc", None).unwrap();

        for s in 0..10_u32 {
            reinforce(&conn, "auto-only", s).unwrap();
        }
        for s in 0..3_u32 {
            natural_reinforce(&conn, "natural", s).unwrap();
        }

        let w_auto_before = get_by_id(&conn, "auto-only").unwrap().unwrap().weight;
        let w_nat_before = get_by_id(&conn, "natural").unwrap().unwrap().weight;

        buoy_qualified(&conn, BUOY_RATE, BUOY_THRESHOLD, 10, 100).unwrap();

        let w_auto_after = get_by_id(&conn, "auto-only").unwrap().unwrap().weight;
        let w_nat_after = get_by_id(&conn, "natural").unwrap().unwrap().weight;

        assert!((w_auto_after - w_auto_before).abs() < 1e-10, "auto-only should NOT be buoyed");
        assert!(w_nat_after > w_nat_before, "natural should be buoyed");
    }

    #[test]
    fn buoy_equilibrium_converges_to_half() {
        use crate::m1_foundation::m05_constants::{BUOY_RATE, DECAY_RATE};
        let equilibrium = BUOY_RATE / (1.0 - DECAY_RATE + BUOY_RATE);
        assert!((equilibrium - 0.5).abs() < 0.01, "equilibrium should be ~0.5, was {equilibrium}");
    }
}
