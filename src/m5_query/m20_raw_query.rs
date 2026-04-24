//! `m20_raw_query` — Raw `SQL` passthrough: takes arbitrary `SQL` string, executes against
//! `injection.db`, returns formatted results (header + separator + rows).
//!
//! Safety: only `SELECT`, `EXPLAIN`, and `PRAGMA` statements are permitted.
//! Write statements (`INSERT`, `UPDATE`, `DELETE`, `DROP`, `ALTER`, `CREATE`) are rejected
//! with [`QueryError::RawSqlDisallowed`] before reaching the database.
//!
//! Layer: `m5_query`
//! Dependencies: `m02_errors`, `m06_schema`

use std::fmt::Write as _;
#[cfg(feature = "sqlite")]
use std::time::Instant;

#[cfg(feature = "sqlite")]
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::m1_foundation::m02_errors::QueryError;

#[cfg(feature = "sqlite")]
use super::query_err;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Structured output from a raw `SQL` query execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryOutput {
    /// Column names in result order.
    pub columns: Vec<String>,
    /// Each row is a `Vec` of stringified column values.
    pub rows: Vec<Vec<String>>,
    /// Total number of data rows (not counting the header).
    pub row_count: usize,
    /// Wall-clock execution time in milliseconds.
    pub elapsed_ms: u64,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Fast pre-filter: rejects statements whose first keyword is a known write
/// operation before they reach the `SQLite` parser.
///
/// This is a heuristic — it cannot catch all write patterns (e.g.
/// `WITH ... INSERT`). The authoritative check happens in [`execute_raw`]
/// via `Statement::readonly()` after `prepare`.
///
/// # Errors
///
/// Returns [`QueryError::RawSqlDisallowed`] if the first keyword is a write
/// operation or unrecognised.
pub fn validate_read_only(sql: &str) -> Result<(), QueryError> {
    let first = sql
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_uppercase();

    match first.as_str() {
        "SELECT" | "EXPLAIN" | "PRAGMA" | "WITH" => Ok(()),
        _ => Err(QueryError::RawSqlDisallowed),
    }
}

// ---------------------------------------------------------------------------
// Core execution
// ---------------------------------------------------------------------------

/// Executes arbitrary `SQL` against an open [`Connection`] and returns [`QueryOutput`].
///
/// Two-layer safety:
/// 1. [`validate_read_only`] rejects obvious write keywords before parsing.
/// 2. After `prepare`, `Statement::readonly()` (wrapping `SQLite`'s
///    `sqlite3_stmt_readonly`) authoritatively rejects any compiled statement
///    that would mutate the database — including `WITH ... INSERT`.
///
/// Multi-statement input (`;`-separated) is handled by `SQLite` — only the
/// *first* statement executes because `prepare` stops at the first boundary.
///
/// # Errors
///
/// - [`QueryError::RawSqlDisallowed`] — statement is not read-only.
/// - [`QueryError::ExecutionFailed`] — `rusqlite` reports an error.
#[cfg(feature = "sqlite")]
pub fn execute_raw(conn: &Connection, sql: &str) -> Result<QueryOutput, QueryError> {
    validate_read_only(sql)?;

    let start = Instant::now();

    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| query_err(&e))?;

    if !stmt.readonly() {
        return Err(QueryError::RawSqlDisallowed);
    }

    let column_names: Vec<String> = stmt.column_names().iter().map(|s| (*s).to_owned()).collect();

    let column_count = column_names.len();

    let rows_raw = stmt
        .query_map([], |row| {
            let mut cells = Vec::with_capacity(column_count);
            for i in 0..column_count {
                let value: rusqlite::types::Value = row.get(i)?;
                let cell = match value {
                    rusqlite::types::Value::Null => String::from("NULL"),
                    rusqlite::types::Value::Integer(n) => n.to_string(),
                    rusqlite::types::Value::Real(f) => f.to_string(),
                    rusqlite::types::Value::Text(s) => s,
                    rusqlite::types::Value::Blob(b) => format!("<blob {} bytes>", b.len()),
                };
                cells.push(cell);
            }
            Ok(cells)
        })
        .map_err(|e| query_err(&e))?;

    let mut rows: Vec<Vec<String>> = Vec::new();
    for row_result in rows_raw {
        rows.push(row_result.map_err(|e| query_err(&e))?);
    }

    let elapsed_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
    let row_count = rows.len();

    Ok(QueryOutput {
        columns: column_names,
        rows,
        row_count,
        elapsed_ms,
    })
}

// ---------------------------------------------------------------------------
// Formatting
// ---------------------------------------------------------------------------

/// Formats a [`QueryOutput`] into an aligned plain-text table.
///
/// Column widths are computed as `max(header_len, max_value_len)`. The header
/// and data rows are separated by a line of `─` (U+2500) characters. An
/// optional footer line shows the row count and elapsed time.
///
/// # Example output
///
/// ```text
/// id  label              reinforcement_count
/// ──  ─────              ───────────────────
/// 1   convergence_trap   7
/// (1 rows, 2ms)
/// ```
#[must_use]
pub fn format_results(output: &QueryOutput) -> String {
    if output.columns.is_empty() {
        return format!(
            "(0 rows, {}ms)",
            output.elapsed_ms
        );
    }

    // Compute per-column widths.
    let mut widths: Vec<usize> = output.columns.iter().map(String::len).collect();

    for row in &output.rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    let mut buf = String::new();

    // Header row.
    let header_parts: Vec<String> = output
        .columns
        .iter()
        .enumerate()
        .map(|(i, col)| format!("{:<width$}", col, width = widths[i]))
        .collect();
    buf.push_str(&header_parts.join("  "));
    buf.push('\n');

    // Separator row.
    let sep_parts: Vec<String> = widths
        .iter()
        .map(|&w| "─".repeat(w))
        .collect();
    buf.push_str(&sep_parts.join("  "));
    buf.push('\n');

    // Data rows.
    for row in &output.rows {
        let row_parts: Vec<String> = widths
            .iter()
            .enumerate()
            .map(|(i, &w)| {
                let cell = row.get(i).map_or("", String::as_str);
                format!("{cell:<w$}")
            })
            .collect();
        buf.push_str(&row_parts.join("  "));
        buf.push('\n');
    }

    // Footer.
    // `String`'s `fmt::Write` impl is infallible; the `Result` is always `Ok`.
    let _ = write!(buf, "({} rows, {}ms)", output.row_count, output.elapsed_ms);

    buf
}

// ---------------------------------------------------------------------------
// Convenience: execute + format in one call
// ---------------------------------------------------------------------------

/// Executes `sql` and returns a formatted table string.
///
/// This is a convenience wrapper around [`execute_raw`] and [`format_results`].
///
/// # Errors
///
/// Returns the same errors as [`execute_raw`].
#[cfg(feature = "sqlite")]
pub fn execute_raw_formatted(conn: &Connection, sql: &str) -> Result<String, QueryError> {
    let output = execute_raw(conn, sql)?;
    Ok(format_results(&output))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Test helpers
    // -----------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    fn mem_db() -> Connection {
        use crate::m2_schema::m06_schema::open_memory;
        open_memory().expect("open_memory failed in test")
    }

    #[cfg(feature = "sqlite")]
    fn seed_causal_chain(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO causal_chain (origin_session, chain_type, label, description, reinforcement_count)
             VALUES
               (109, 'trap', 'convergence_trap', 'RALPH oscillation', 7),
               (108, 'bug',  'povm_write_only',  'Reads return stale', 3);",
        )
        .expect("seed failed");
    }

    // -----------------------------------------------------------------------
    // validate_read_only — allow list
    // -----------------------------------------------------------------------

    #[test]
    fn validate_select_ok() {
        assert!(validate_read_only("SELECT 1").is_ok());
    }

    #[test]
    fn validate_select_mixed_case_ok() {
        assert!(validate_read_only("select * from causal_chain").is_ok());
    }

    #[test]
    fn validate_select_upper_ok() {
        assert!(validate_read_only("SELECT * FROM causal_chain WHERE id = 1").is_ok());
    }

    #[test]
    fn validate_explain_ok() {
        assert!(validate_read_only("EXPLAIN SELECT 1").is_ok());
    }

    #[test]
    fn validate_pragma_ok() {
        assert!(validate_read_only("PRAGMA table_info(causal_chain)").is_ok());
    }

    #[test]
    fn validate_with_cte_ok() {
        assert!(validate_read_only("WITH cte AS (SELECT 1) SELECT * FROM cte").is_ok());
    }

    #[test]
    fn validate_select_with_leading_whitespace_ok() {
        assert!(validate_read_only("   SELECT 1").is_ok());
    }

    #[test]
    fn validate_select_with_tab_ok() {
        assert!(validate_read_only("\tSELECT id FROM causal_chain").is_ok());
    }

    // -----------------------------------------------------------------------
    // validate_read_only — deny list
    // -----------------------------------------------------------------------

    #[test]
    fn validate_insert_rejected() {
        assert!(validate_read_only("INSERT INTO causal_chain VALUES (1)").is_err());
    }

    #[test]
    fn validate_update_rejected() {
        assert!(validate_read_only("UPDATE causal_chain SET label = 'x'").is_err());
    }

    #[test]
    fn validate_delete_rejected() {
        assert!(validate_read_only("DELETE FROM causal_chain").is_err());
    }

    #[test]
    fn validate_drop_rejected() {
        assert!(validate_read_only("DROP TABLE causal_chain").is_err());
    }

    #[test]
    fn validate_alter_rejected() {
        assert!(validate_read_only("ALTER TABLE causal_chain ADD COLUMN x TEXT").is_err());
    }

    #[test]
    fn validate_create_rejected() {
        assert!(validate_read_only("CREATE TABLE foo (id INTEGER)").is_err());
    }

    #[test]
    fn validate_replace_rejected() {
        assert!(validate_read_only("REPLACE INTO causal_chain VALUES (1, 1, NULL, 'bug', 'x', 'y', 1, NULL, 'Emit')").is_err());
    }

    #[test]
    fn validate_attach_rejected() {
        assert!(validate_read_only("ATTACH DATABASE ':memory:' AS tmp").is_err());
    }

    #[test]
    fn validate_vacuum_rejected() {
        assert!(validate_read_only("VACUUM").is_err());
    }

    #[test]
    fn validate_empty_rejected() {
        assert!(validate_read_only("").is_err());
    }

    #[test]
    fn validate_whitespace_only_rejected() {
        assert!(validate_read_only("   ").is_err());
    }

    #[test]
    fn validate_insert_lowercase_rejected() {
        assert!(validate_read_only("insert into foo values (1)").is_err());
    }

    #[test]
    fn validate_drop_lowercase_rejected() {
        assert!(validate_read_only("drop table causal_chain").is_err());
    }

    #[test]
    fn validate_delete_mixed_case_rejected() {
        assert!(validate_read_only("Delete FROM causal_chain").is_err());
    }

    // -----------------------------------------------------------------------
    // validate_read_only — error variant
    // -----------------------------------------------------------------------

    #[test]
    fn validate_rejected_returns_raw_sql_disallowed() {
        let err = validate_read_only("DELETE FROM causal_chain").unwrap_err();
        assert!(matches!(err, QueryError::RawSqlDisallowed));
    }

    // -----------------------------------------------------------------------
    // execute_raw — basic SELECT
    // -----------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_raw_simple_select() {
        let conn = mem_db();
        seed_causal_chain(&conn);
        let output = execute_raw(&conn, "SELECT label FROM causal_chain ORDER BY id").unwrap();
        assert_eq!(output.columns, vec!["label"]);
        assert_eq!(output.row_count, 2);
        assert_eq!(output.rows[0][0], "convergence_trap");
        assert_eq!(output.rows[1][0], "povm_write_only");
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_raw_select_with_where() {
        let conn = mem_db();
        seed_causal_chain(&conn);
        let output = execute_raw(
            &conn,
            "SELECT label FROM causal_chain WHERE reinforcement_count > 5",
        )
        .unwrap();
        assert_eq!(output.row_count, 1);
        assert_eq!(output.rows[0][0], "convergence_trap");
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_raw_select_aggregate() {
        let conn = mem_db();
        seed_causal_chain(&conn);
        let output =
            execute_raw(&conn, "SELECT COUNT(*) AS cnt FROM causal_chain").unwrap();
        assert_eq!(output.row_count, 1);
        assert_eq!(output.rows[0][0], "2");
        assert_eq!(output.columns[0], "cnt");
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_raw_empty_result() {
        let conn = mem_db();
        let output = execute_raw(
            &conn,
            "SELECT * FROM causal_chain WHERE id = 999999",
        )
        .unwrap();
        assert_eq!(output.row_count, 0);
        assert!(output.rows.is_empty());
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_raw_single_row() {
        let conn = mem_db();
        seed_causal_chain(&conn);
        let output = execute_raw(
            &conn,
            "SELECT label FROM causal_chain WHERE reinforcement_count = 3",
        )
        .unwrap();
        assert_eq!(output.row_count, 1);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_raw_multiple_rows() {
        let conn = mem_db();
        seed_causal_chain(&conn);
        let output = execute_raw(&conn, "SELECT id, label FROM causal_chain ORDER BY id").unwrap();
        assert_eq!(output.row_count, 2);
        assert_eq!(output.columns.len(), 2);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_raw_elapsed_ms_present() {
        let conn = mem_db();
        let output = execute_raw(&conn, "SELECT 1").unwrap();
        // elapsed_ms is a u64; just verify the field exists and does not
        // overflow (any value is valid for a fast query).
        let _ = output.elapsed_ms;
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_raw_columns_match_projection() {
        let conn = mem_db();
        seed_causal_chain(&conn);
        let output = execute_raw(
            &conn,
            "SELECT label, reinforcement_count, description FROM causal_chain ORDER BY id LIMIT 1",
        )
        .unwrap();
        assert_eq!(output.columns, vec!["label", "reinforcement_count", "description"]);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_raw_select_with_join() {
        // Joining causal_chain to itself via a subquery to verify JOIN support.
        let conn = mem_db();
        seed_causal_chain(&conn);
        let output = execute_raw(
            &conn,
            "SELECT a.label FROM causal_chain a JOIN causal_chain b ON a.id = b.id ORDER BY a.id",
        )
        .unwrap();
        assert_eq!(output.row_count, 2);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_raw_null_value_rendered() {
        let conn = mem_db();
        // resolved_session starts as NULL.
        seed_causal_chain(&conn);
        let output = execute_raw(
            &conn,
            "SELECT resolved_session FROM causal_chain ORDER BY id LIMIT 1",
        )
        .unwrap();
        assert_eq!(output.rows[0][0], "NULL");
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_raw_integer_rendered_as_string() {
        let conn = mem_db();
        seed_causal_chain(&conn);
        let output =
            execute_raw(&conn, "SELECT reinforcement_count FROM causal_chain ORDER BY id LIMIT 1")
                .unwrap();
        assert_eq!(output.rows[0][0], "7");
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_raw_explain_permitted() {
        let conn = mem_db();
        let result = execute_raw(&conn, "EXPLAIN SELECT 1");
        assert!(result.is_ok());
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_raw_pragma_permitted() {
        let conn = mem_db();
        let output = execute_raw(&conn, "PRAGMA foreign_keys").unwrap();
        assert_eq!(output.row_count, 1);
    }

    // -----------------------------------------------------------------------
    // execute_raw — rejection
    // -----------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_raw_rejects_insert() {
        let conn = mem_db();
        let err = execute_raw(
            &conn,
            "INSERT INTO causal_chain (origin_session, chain_type, label, description) VALUES (1, 'bug', 'x', 'y')",
        )
        .unwrap_err();
        assert!(matches!(err, QueryError::RawSqlDisallowed));
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_raw_rejects_delete() {
        let conn = mem_db();
        let err = execute_raw(&conn, "DELETE FROM causal_chain").unwrap_err();
        assert!(matches!(err, QueryError::RawSqlDisallowed));
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_raw_rejects_drop() {
        let conn = mem_db();
        let err = execute_raw(&conn, "DROP TABLE causal_chain").unwrap_err();
        assert!(matches!(err, QueryError::RawSqlDisallowed));
    }

    // -----------------------------------------------------------------------
    // SQL injection safety: multi-statement — only first runs
    // -----------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_raw_sql_injection_only_first_statement_runs() {
        // rusqlite::prepare() stops at the first `;`, so the DROP never executes.
        let conn = mem_db();
        seed_causal_chain(&conn);
        // The trailing DROP TABLE is not a SELECT, but because `prepare` only
        // compiles the first statement, the whole string is still a SELECT.
        let output = execute_raw(
            &conn,
            "SELECT * FROM causal_chain; DROP TABLE causal_chain",
        )
        .unwrap();
        // The first statement (SELECT) ran — table still exists.
        assert_eq!(output.row_count, 2);
        // Table is still intact.
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM causal_chain", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }

    // -----------------------------------------------------------------------
    // execute_raw — bad SQL returns ExecutionFailed
    // -----------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_raw_syntax_error_returns_execution_failed() {
        let conn = mem_db();
        let err = execute_raw(&conn, "SELECT * FROM nonexistent_table_xyz").unwrap_err();
        assert!(matches!(err, QueryError::ExecutionFailed(_)));
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_raw_malformed_sql_execution_failed() {
        let conn = mem_db();
        let err = execute_raw(&conn, "SELECT FROM FROM").unwrap_err();
        assert!(matches!(err, QueryError::ExecutionFailed(_)));
    }

    // -----------------------------------------------------------------------
    // format_results — structure
    // -----------------------------------------------------------------------

    #[test]
    fn format_results_empty_output() {
        let output = QueryOutput {
            columns: vec![],
            rows: vec![],
            row_count: 0,
            elapsed_ms: 1,
        };
        let s = format_results(&output);
        assert!(s.contains("0 rows"));
    }

    #[test]
    fn format_results_contains_header() {
        let output = QueryOutput {
            columns: vec!["id".into(), "label".into()],
            rows: vec![vec!["1".into(), "convergence_trap".into()]],
            row_count: 1,
            elapsed_ms: 3,
        };
        let s = format_results(&output);
        assert!(s.contains("id"));
        assert!(s.contains("label"));
    }

    #[test]
    fn format_results_contains_separator_line() {
        let output = QueryOutput {
            columns: vec!["col".into()],
            rows: vec![vec!["val".into()]],
            row_count: 1,
            elapsed_ms: 0,
        };
        let s = format_results(&output);
        // Separator uses U+2500 (─)
        assert!(s.contains('─'));
    }

    #[test]
    fn format_results_row_count_in_footer() {
        let output = QueryOutput {
            columns: vec!["x".into()],
            rows: vec![vec!["a".into()], vec!["b".into()]],
            row_count: 2,
            elapsed_ms: 5,
        };
        let s = format_results(&output);
        assert!(s.contains("2 rows"));
        assert!(s.contains("5ms"));
    }

    #[test]
    fn format_results_data_rows_present() {
        let output = QueryOutput {
            columns: vec!["label".into()],
            rows: vec![vec!["convergence_trap".into()], vec!["povm_write_only".into()]],
            row_count: 2,
            elapsed_ms: 2,
        };
        let s = format_results(&output);
        assert!(s.contains("convergence_trap"));
        assert!(s.contains("povm_write_only"));
    }

    #[test]
    fn format_results_column_width_expands_for_long_value() {
        // value "very_long_value_here" is longer than header "col" → column width = value length.
        let output = QueryOutput {
            columns: vec!["col".into()],
            rows: vec![vec!["very_long_value_here".into()]],
            row_count: 1,
            elapsed_ms: 0,
        };
        let s = format_results(&output);
        // Both the separator and the header should be at least 20 chars wide.
        // Check that the separator line contains enough ─ characters.
        let sep_len = s
            .lines()
            .nth(1)
            .unwrap_or("")
            .chars()
            .filter(|&c| c == '─')
            .count();
        assert!(sep_len >= "very_long_value_here".len());
    }

    #[test]
    fn format_results_column_width_stays_at_header_when_values_shorter() {
        let output = QueryOutput {
            columns: vec!["long_header_column".into()],
            rows: vec![vec!["x".into()]],
            row_count: 1,
            elapsed_ms: 0,
        };
        let s = format_results(&output);
        // Header line should contain the full header.
        assert!(s.contains("long_header_column"));
    }

    #[test]
    fn format_results_multi_column_alignment() {
        let output = QueryOutput {
            columns: vec!["id".into(), "label".into(), "cnt".into()],
            rows: vec![
                vec!["1".into(), "convergence_trap".into(), "7".into()],
                vec!["2".into(), "povm_write_only".into(), "3".into()],
            ],
            row_count: 2,
            elapsed_ms: 4,
        };
        let s = format_results(&output);
        // All three column names appear in the first line.
        let first_line = s.lines().next().unwrap_or("");
        assert!(first_line.contains("id"));
        assert!(first_line.contains("label"));
        assert!(first_line.contains("cnt"));
    }

    #[test]
    fn format_results_zero_elapsed() {
        let output = QueryOutput {
            columns: vec!["x".into()],
            rows: vec![vec!["1".into()]],
            row_count: 1,
            elapsed_ms: 0,
        };
        let s = format_results(&output);
        assert!(s.contains("0ms"));
    }

    #[test]
    fn format_results_no_rows_shows_zero_footer() {
        let output = QueryOutput {
            columns: vec!["id".into()],
            rows: vec![],
            row_count: 0,
            elapsed_ms: 1,
        };
        let s = format_results(&output);
        assert!(s.contains("0 rows"));
    }

    #[test]
    fn format_results_null_value_appears_in_row() {
        let output = QueryOutput {
            columns: vec!["resolved_session".into()],
            rows: vec![vec!["NULL".into()]],
            row_count: 1,
            elapsed_ms: 0,
        };
        let s = format_results(&output);
        assert!(s.contains("NULL"));
    }

    #[test]
    fn format_results_line_count() {
        // 2 data rows → header line + separator line + 2 data lines + footer = 5 lines.
        let output = QueryOutput {
            columns: vec!["x".into()],
            rows: vec![vec!["a".into()], vec!["b".into()]],
            row_count: 2,
            elapsed_ms: 0,
        };
        let s = format_results(&output);
        // Count newline-separated non-empty lines.
        let line_count = s.lines().count();
        assert_eq!(line_count, 5);
    }

    // -----------------------------------------------------------------------
    // QueryOutput construction
    // -----------------------------------------------------------------------

    #[test]
    fn query_output_construction() {
        let output = QueryOutput {
            columns: vec!["a".into(), "b".into()],
            rows: vec![vec!["1".into(), "2".into()]],
            row_count: 1,
            elapsed_ms: 42,
        };
        assert_eq!(output.columns.len(), 2);
        assert_eq!(output.row_count, 1);
        assert_eq!(output.elapsed_ms, 42);
    }

    // -----------------------------------------------------------------------
    // Serde roundtrip
    // -----------------------------------------------------------------------

    #[test]
    fn query_output_serde_roundtrip() {
        let original = QueryOutput {
            columns: vec!["id".into(), "label".into()],
            rows: vec![
                vec!["1".into(), "convergence_trap".into()],
                vec!["2".into(), "povm_write_only".into()],
            ],
            row_count: 2,
            elapsed_ms: 7,
        };
        let json = serde_json::to_string(&original).expect("serialize failed");
        let restored: QueryOutput = serde_json::from_str(&json).expect("deserialize failed");
        assert_eq!(restored.columns, original.columns);
        assert_eq!(restored.rows, original.rows);
        assert_eq!(restored.row_count, original.row_count);
        assert_eq!(restored.elapsed_ms, original.elapsed_ms);
    }

    #[test]
    fn query_output_serde_empty() {
        let original = QueryOutput {
            columns: vec![],
            rows: vec![],
            row_count: 0,
            elapsed_ms: 0,
        };
        let json = serde_json::to_string(&original).expect("serialize failed");
        let restored: QueryOutput = serde_json::from_str(&json).expect("deserialize failed");
        assert_eq!(restored.row_count, 0);
        assert!(restored.columns.is_empty());
    }

    #[test]
    fn query_output_clone() {
        let output = QueryOutput {
            columns: vec!["x".into()],
            rows: vec![vec!["1".into()]],
            row_count: 1,
            elapsed_ms: 1,
        };
        let cloned = output.clone();
        assert_eq!(cloned.columns, output.columns);
        assert_eq!(cloned.rows, output.rows);
    }

    #[test]
    fn query_output_debug_not_empty() {
        let output = QueryOutput {
            columns: vec!["x".into()],
            rows: vec![],
            row_count: 0,
            elapsed_ms: 0,
        };
        let dbg = format!("{output:?}");
        assert!(!dbg.is_empty());
    }

    // -----------------------------------------------------------------------
    // execute_raw_formatted
    // -----------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_raw_formatted_returns_string() {
        let conn = mem_db();
        seed_causal_chain(&conn);
        let s = execute_raw_formatted(
            &conn,
            "SELECT label, reinforcement_count FROM causal_chain ORDER BY id",
        )
        .unwrap();
        assert!(s.contains("label"));
        assert!(s.contains("convergence_trap"));
        assert!(s.contains("2 rows"));
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_raw_formatted_rejects_write() {
        let conn = mem_db();
        let err =
            execute_raw_formatted(&conn, "DELETE FROM causal_chain").unwrap_err();
        assert!(matches!(err, QueryError::RawSqlDisallowed));
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_raw_formatted_empty_result() {
        let conn = mem_db();
        let s =
            execute_raw_formatted(&conn, "SELECT * FROM causal_chain WHERE id = -1").unwrap();
        assert!(s.contains("0 rows"));
    }

    // -----------------------------------------------------------------------
    // Whitespace handling
    // -----------------------------------------------------------------------

    #[test]
    fn validate_leading_newline_select_ok() {
        assert!(validate_read_only("\n\nSELECT 1").is_ok());
    }

    #[test]
    fn validate_carriage_return_select_ok() {
        assert!(validate_read_only("\r\nSELECT 1").is_ok());
    }

    // -----------------------------------------------------------------------
    // Row count field accuracy
    // -----------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_raw_row_count_matches_rows_len() {
        let conn = mem_db();
        seed_causal_chain(&conn);
        let output = execute_raw(&conn, "SELECT * FROM causal_chain").unwrap();
        assert_eq!(output.row_count, output.rows.len());
    }

    // -----------------------------------------------------------------------
    // WITH + write — authoritative readonly() check
    // -----------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_raw_rejects_with_insert() {
        let conn = mem_db();
        let err = execute_raw(
            &conn,
            "WITH cte AS (SELECT 109, 'bug', 'injected', 'pwned') INSERT INTO causal_chain (origin_session, chain_type, label, description) SELECT * FROM cte",
        )
        .unwrap_err();
        assert!(matches!(err, QueryError::RawSqlDisallowed));
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_raw_rejects_with_update() {
        let conn = mem_db();
        seed_causal_chain(&conn);
        let err = execute_raw(
            &conn,
            "WITH vals(lbl) AS (VALUES ('convergence_trap')) UPDATE causal_chain SET description = 'pwned' WHERE label IN (SELECT lbl FROM vals)",
        )
        .unwrap_err();
        assert!(matches!(err, QueryError::RawSqlDisallowed));
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_raw_rejects_with_delete() {
        let conn = mem_db();
        seed_causal_chain(&conn);
        let err = execute_raw(
            &conn,
            "WITH targets AS (SELECT id FROM causal_chain) DELETE FROM causal_chain WHERE id IN (SELECT id FROM targets)",
        )
        .unwrap_err();
        assert!(matches!(err, QueryError::RawSqlDisallowed));
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_raw_allows_with_select() {
        let conn = mem_db();
        seed_causal_chain(&conn);
        let output = execute_raw(
            &conn,
            "WITH cte AS (SELECT label FROM causal_chain) SELECT * FROM cte",
        )
        .unwrap();
        assert_eq!(output.row_count, 2);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn execute_raw_with_insert_does_not_modify_data() {
        let conn = mem_db();
        seed_causal_chain(&conn);
        let before: i64 = conn
            .query_row("SELECT COUNT(*) FROM causal_chain", [], |r| r.get(0))
            .unwrap();
        let _ = execute_raw(
            &conn,
            "WITH cte AS (SELECT 1, 'bug', 'x', 'y') INSERT INTO causal_chain (origin_session, chain_type, label, description) SELECT * FROM cte",
        );
        let after: i64 = conn
            .query_row("SELECT COUNT(*) FROM causal_chain", [], |r| r.get(0))
            .unwrap();
        assert_eq!(before, after, "data must not be modified by rejected statement");
    }

    #[test]
    fn validate_with_cte_select_still_allowed() {
        assert!(validate_read_only("WITH cte AS (SELECT 1) SELECT * FROM cte").is_ok());
    }
}
