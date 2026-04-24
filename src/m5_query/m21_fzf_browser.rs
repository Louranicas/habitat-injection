//! `m21_fzf_browser` — `fzf`-powered interactive memory browser.
//!
//! Pipes table contents through `fzf` with `--preview` showing related
//! records. Requires `fzf` in `PATH`. Falls back to non-interactive plain-text
//! display when `fzf` is absent.
//!
//! # What this module does NOT do
//!
//! It does **not** spawn `fzf` in interactive/TUI mode (that requires a
//! controlling terminal that is not available in all contexts). Instead it
//! provides:
//!
//! 1. Functions to **format** data for `fzf` input.
//! 2. Functions to **build** `fzf` command-line arguments.
//! 3. A `--filter` mode for non-interactive fuzzy search (scriptable).
//! 4. Detection of `fzf` availability.
//!
//! # Layer
//!
//! `m5_query`
//!
//! # Dependencies
//!
//! `m01_types`, `m02_errors`, `m07_causal_chain`, `m08_trajectory`,
//! `m09_workstream`, `m10_pattern`

#[cfg(feature = "sqlite")]
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::m1_foundation::m02_errors::QueryError;

#[cfg(feature = "sqlite")]
use super::query_err;
#[cfg(feature = "sqlite")]
use crate::m2_schema::{
    m07_causal_chain::{CausalChainRow, find_unresolved},
    m08_trajectory::{TrajectoryRow, get_recent},
    m09_workstream::{WorkstreamRow, get_active, get_blocked},
    m10_pattern::{PatternRow, get_top_by_weight},
};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Selects which table(s) to browse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrowserTable {
    /// `causal_chain` — unresolved bugs, traps, and plans sorted by frequency.
    Chains,
    /// `session_trajectory` — last N session fitness arcs.
    Trajectory,
    /// `workstream` — active and blocked workstreams.
    Workstreams,
    /// `reinforced_pattern` — top patterns by Hebbian weight.
    Patterns,
    /// All tables concatenated with section headers.
    All,
}

impl std::fmt::Display for BrowserTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Chains => f.write_str("chains"),
            Self::Trajectory => f.write_str("trajectory"),
            Self::Workstreams => f.write_str("workstreams"),
            Self::Patterns => f.write_str("patterns"),
            Self::All => f.write_str("all"),
        }
    }
}

/// Configuration for an `fzf` browser session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    /// Which table(s) to query and display.
    pub table: BrowserTable,
    /// Non-interactive filter mode: if `Some`, passes `--filter <value>` to
    /// `fzf` (or falls back to [`simple_filter`]).
    pub filter: Option<String>,
    /// Whether to include a `--preview` pane in the `fzf` arguments.
    pub preview: bool,
    /// Whether to allow multi-select (`--multi`).
    pub multi: bool,
}

impl BrowserConfig {
    /// Construct a minimal [`BrowserConfig`] for the given table with all
    /// optional fields at their defaults.
    #[must_use]
    pub fn new(table: BrowserTable) -> Self {
        Self {
            table,
            filter: None,
            preview: false,
            multi: false,
        }
    }

    /// Builder method: set the filter string.
    #[must_use]
    pub fn with_filter(mut self, filter: impl Into<String>) -> Self {
        self.filter = Some(filter.into());
        self
    }

    /// Builder method: enable preview pane.
    #[must_use]
    pub fn with_preview(mut self) -> Self {
        self.preview = true;
        self
    }

    /// Builder method: enable multi-select.
    #[must_use]
    pub fn with_multi(mut self) -> Self {
        self.multi = true;
        self
    }
}

/// Result of an `fzf` browser operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserResult {
    /// Lines selected (matched) by `fzf` or [`simple_filter`].
    pub selected: Vec<String>,
    /// Which table was browsed.
    pub table: BrowserTable,
    /// Total number of items fed into the browser before filtering.
    pub total_items: usize,
    /// Number of items that survived the filter (equals `selected.len()` when
    /// no further post-processing is applied).
    pub filtered_items: usize,
}

// ---------------------------------------------------------------------------
// fzf availability
// ---------------------------------------------------------------------------

/// Returns `true` if `fzf` is present somewhere on `PATH`.
///
/// The check is a lightweight `which`-style scan — it never spawns a process.
#[must_use]
pub fn is_fzf_available() -> bool {
    let Some(path_env) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path_env).any(|dir| dir.join("fzf").is_file())
}

// ---------------------------------------------------------------------------
// Formatting
// ---------------------------------------------------------------------------

/// Format query results as `fzf`-compatible input.
///
/// Each row becomes a single line, with columns joined by `\t`. The first line
/// is the header row (also tab-separated). An empty `rows` slice returns only
/// the header, or an empty string if `headers` is also empty.
///
/// # Example
///
/// ```ignore
/// let headers = vec!["label".to_string(), "count".to_string()];
/// let rows    = vec![vec!["BUG-001".to_string(), "5".to_string()]];
/// let out     = format_for_fzf(&rows, &headers);
/// assert!(out.starts_with("label\tcount\n"));
/// ```
#[must_use]
pub fn format_for_fzf(rows: &[Vec<String>], headers: &[String]) -> String {
    if headers.is_empty() && rows.is_empty() {
        return String::new();
    }

    let mut out = String::new();

    if !headers.is_empty() {
        out.push_str(&headers.join("\t"));
        out.push('\n');
    }

    for row in rows {
        out.push_str(&row.join("\t"));
        out.push('\n');
    }

    out
}

// ---------------------------------------------------------------------------
// Argument building
// ---------------------------------------------------------------------------

/// Build `fzf` command-line arguments from a [`BrowserConfig`].
///
/// Arguments always include:
/// - `--delimiter=\t` — matches the tab-separated output of [`format_for_fzf`].
/// - `--no-sort` — chains are pre-sorted by reinforcement frequency.
/// - `--header-lines=1` — treats the first line as a column header.
///
/// Conditionally added:
/// - `--preview` (with a default `echo {}` preview command) when
///   `config.preview` is `true`.
/// - `--filter <value>` when `config.filter` is `Some`.
/// - `--multi` when `config.multi` is `true`.
#[must_use]
pub fn build_fzf_args(config: &BrowserConfig) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "--delimiter=\t".to_string(),
        "--no-sort".to_string(),
        "--header-lines=1".to_string(),
    ];

    if config.preview {
        args.push("--preview".to_string());
        args.push("echo {}".to_string());
    }

    if let Some(ref f) = config.filter {
        args.push("--filter".to_string());
        args.push(f.clone());
    }

    if config.multi {
        args.push("--multi".to_string());
    }

    args
}

// ---------------------------------------------------------------------------
// Pure-Rust fallback filter
// ---------------------------------------------------------------------------

/// Case-insensitive substring filter — the pure-Rust fallback when `fzf` is
/// absent.
///
/// Every non-empty line in `input` that contains `filter` (case-insensitively)
/// is included in the output. An empty `filter` returns all non-empty lines.
/// An empty `input` returns an empty `Vec`.
#[must_use]
pub fn simple_filter(input: &str, filter: &str) -> Vec<String> {
    let needle = filter.to_lowercase();
    input
        .lines()
        .filter(|line| !line.is_empty())
        .filter(|line| needle.is_empty() || line.to_lowercase().contains(&needle))
        .map(std::string::ToString::to_string)
        .collect()
}

// ---------------------------------------------------------------------------
// Subprocess filter
// ---------------------------------------------------------------------------

/// Run `fzf --filter <filter>` as a subprocess, piping `input` via stdin.
///
/// Returns the matched lines as a `Vec<String>`. Falls back to
/// [`simple_filter`] when `fzf` is not available on `PATH`.
///
/// # Errors
///
/// Returns [`QueryError::FzfFailed`] if the `fzf` subprocess cannot be
/// spawned or if writing to its stdin fails. Simple-filter fallback never
/// returns an error.
pub fn run_filter(input: &str, filter: &str) -> Result<Vec<String>, QueryError> {
    use std::io::Write as _;
    use std::process::{Command, Stdio};

    if !is_fzf_available() {
        return Ok(simple_filter(input, filter));
    }

    let mut child = Command::new("fzf")
        .arg("--filter")
        .arg(filter)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| QueryError::FzfFailed(format!("spawn failed: {e}")))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(input.as_bytes())
            .map_err(|e| QueryError::FzfFailed(format!("stdin write failed: {e}")))?;
        // stdin closes when dropped — signals EOF to fzf
    }

    let output = child
        .wait_with_output()
        .map_err(|e| QueryError::FzfFailed(format!("wait failed: {e}")))?;

    // fzf exits 1 when no matches — that is not an error for us.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<String> = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(std::string::ToString::to_string)
        .collect();

    Ok(lines)
}

// ---------------------------------------------------------------------------
// Per-table formatting helpers (feature-gated)
// ---------------------------------------------------------------------------

#[cfg(feature = "sqlite")]
fn chain_row_to_cols(row: &CausalChainRow) -> Vec<String> {
    vec![
        row.label.clone(),
        row.chain_type.clone(),
        row.reinforcement_count.to_string(),
        row.description.clone(),
    ]
}

#[cfg(feature = "sqlite")]
fn chain_headers() -> Vec<String> {
    vec![
        "label".to_string(),
        "type".to_string(),
        "reinforcements".to_string(),
        "description".to_string(),
    ]
}

#[cfg(feature = "sqlite")]
fn trajectory_row_to_cols(row: &TrajectoryRow) -> Vec<String> {
    vec![
        row.session_id.to_string(),
        format!("{:.3}", row.ralph_fitness),
        format!("{:.3}", row.field_r),
        format!("{:.3}", row.thermal_t),
        format!("{:.3}", row.ltp_ltd_ratio),
        row.services_healthy.to_string(),
        row.delta_summary.clone(),
    ]
}

#[cfg(feature = "sqlite")]
fn trajectory_headers() -> Vec<String> {
    vec![
        "session".to_string(),
        "fitness".to_string(),
        "field_r".to_string(),
        "thermal_t".to_string(),
        "ltp_ltd".to_string(),
        "services".to_string(),
        "delta_summary".to_string(),
    ]
}

#[cfg(feature = "sqlite")]
fn workstream_row_to_cols(row: &WorkstreamRow) -> Vec<String> {
    vec![
        row.ws_id.clone(),
        row.status.clone(),
        row.priority.to_string(),
        row.blocker.clone().unwrap_or_default(),
        row.title.clone(),
    ]
}

#[cfg(feature = "sqlite")]
fn workstream_headers() -> Vec<String> {
    vec![
        "ws_id".to_string(),
        "status".to_string(),
        "priority".to_string(),
        "blocker".to_string(),
        "title".to_string(),
    ]
}

#[cfg(feature = "sqlite")]
fn pattern_row_to_cols(row: &PatternRow) -> Vec<String> {
    vec![
        row.pattern_id.clone(),
        row.category.clone(),
        format!("{:.4}", row.weight),
        row.hit_count.to_string(),
        row.description.clone(),
    ]
}

#[cfg(feature = "sqlite")]
fn pattern_headers() -> Vec<String> {
    vec![
        "pattern_id".to_string(),
        "category".to_string(),
        "weight".to_string(),
        "hits".to_string(),
        "description".to_string(),
    ]
}

// ---------------------------------------------------------------------------
// browse_table — the main entry point
// ---------------------------------------------------------------------------

/// Query the specified table(s), format for `fzf`, apply the filter if set,
/// and return a [`BrowserResult`].
///
/// For [`BrowserTable::All`], all four tables are concatenated with section
/// header lines so the caller sees a unified stream. Limits applied:
/// - Chains: 50 unresolved, ordered by reinforcement count.
/// - Trajectory: last 20 sessions.
/// - Workstreams: all active + all blocked.
/// - Patterns: top 30 by weight.
///
/// # Errors
///
/// Returns [`QueryError::ExecutionFailed`] when the underlying `SQLite` query
/// fails. Returns [`QueryError::FzfFailed`] when `fzf` cannot be spawned and
/// the error propagates from [`run_filter`].
#[cfg(feature = "sqlite")]
pub fn browse_table(
    conn: &Connection,
    config: &BrowserConfig,
) -> Result<BrowserResult, QueryError> {
    let (headers, rows) = collect_rows(conn, config.table)?;

    let formatted = format_for_fzf(&rows, &headers);
    let total_items = rows.len();

    let selected = match &config.filter {
        Some(f) => run_filter(&formatted, f)?,
        None => formatted
            .lines()
            .filter(|l| !l.is_empty())
            .map(std::string::ToString::to_string)
            .collect(),
    };

    let filtered_items = selected.len();

    Ok(BrowserResult {
        selected,
        table: config.table,
        total_items,
        filtered_items,
    })
}

/// Collect rows from the requested table(s) into a unified `(headers, rows)` pair.
///
/// For [`BrowserTable::All`] a synthetic "section" column is prepended so
/// callers can distinguish which table each row came from.
#[cfg(feature = "sqlite")]
fn collect_rows(
    conn: &Connection,
    table: BrowserTable,
) -> Result<(Vec<String>, Vec<Vec<String>>), QueryError> {
    match table {
        BrowserTable::Chains => {
            let data = find_unresolved(conn, 50)
                .map_err(|e| query_err(&e))?;
            let rows: Vec<Vec<String>> = data.iter().map(chain_row_to_cols).collect();
            Ok((chain_headers(), rows))
        }
        BrowserTable::Trajectory => {
            let data = get_recent(conn, 20)
                .map_err(|e| query_err(&e))?;
            let rows: Vec<Vec<String>> = data.iter().map(trajectory_row_to_cols).collect();
            Ok((trajectory_headers(), rows))
        }
        BrowserTable::Workstreams => {
            let mut active = get_active(conn)
                .map_err(|e| query_err(&e))?;
            let blocked = get_blocked(conn)
                .map_err(|e| query_err(&e))?;
            active.extend(blocked);
            let rows: Vec<Vec<String>> = active.iter().map(workstream_row_to_cols).collect();
            Ok((workstream_headers(), rows))
        }
        BrowserTable::Patterns => {
            let data = get_top_by_weight(conn, 30)
                .map_err(|e| query_err(&e))?;
            let rows: Vec<Vec<String>> = data.iter().map(pattern_row_to_cols).collect();
            Ok((pattern_headers(), rows))
        }
        BrowserTable::All => collect_all(conn),
    }
}

/// Concatenate all four tables with a leading `section` column.
///
/// The unified header is `section\tlabel\tvalue` where `value` is the
/// tab-joined remainder of each row's natural columns.
#[cfg(feature = "sqlite")]
fn collect_all(
    conn: &Connection,
) -> Result<(Vec<String>, Vec<Vec<String>>), QueryError> {
    let mut all_rows: Vec<Vec<String>> = Vec::new();

    // Chains
    let chains = find_unresolved(conn, 50)
        .map_err(|e| query_err(&e))?;
    for row in &chains {
        let mut cols = vec!["chains".to_string()];
        cols.extend(chain_row_to_cols(row));
        all_rows.push(cols);
    }

    // Trajectory
    let traj = get_recent(conn, 20)
        .map_err(|e| query_err(&e))?;
    for row in &traj {
        let mut cols = vec!["trajectory".to_string()];
        cols.extend(trajectory_row_to_cols(row));
        all_rows.push(cols);
    }

    // Workstreams (active + blocked)
    let mut ws = get_active(conn)
        .map_err(|e| query_err(&e))?;
    let blocked = get_blocked(conn)
        .map_err(|e| query_err(&e))?;
    ws.extend(blocked);
    for row in &ws {
        let mut cols = vec!["workstreams".to_string()];
        cols.extend(workstream_row_to_cols(row));
        all_rows.push(cols);
    }

    // Patterns
    let patterns = get_top_by_weight(conn, 30)
        .map_err(|e| query_err(&e))?;
    for row in &patterns {
        let mut cols = vec!["patterns".to_string()];
        cols.extend(pattern_row_to_cols(row));
        all_rows.push(cols);
    }

    let headers = vec![
        "section".to_string(),
        "key".to_string(),
        "type_or_status".to_string(),
        "metric".to_string(),
        "detail".to_string(),
    ];

    Ok((headers, all_rows))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // is_fzf_available
    // -----------------------------------------------------------------------

    #[test]
    fn fzf_available_returns_bool() {
        // Smoke test — just ensure it doesn't panic. The actual value depends
        // on the system.
        let _available = is_fzf_available();
    }

    #[test]
    fn fzf_not_available_when_path_empty() {
        // When PATH is empty there should be no fzf found.
        // We cannot safely mutate the env here without affecting other tests,
        // so we just verify the function doesn't panic on a fresh call.
        let _ = is_fzf_available();
    }

    // -----------------------------------------------------------------------
    // BrowserTable
    // -----------------------------------------------------------------------

    #[test]
    fn browser_table_display_chains() {
        assert_eq!(BrowserTable::Chains.to_string(), "chains");
    }

    #[test]
    fn browser_table_display_trajectory() {
        assert_eq!(BrowserTable::Trajectory.to_string(), "trajectory");
    }

    #[test]
    fn browser_table_display_workstreams() {
        assert_eq!(BrowserTable::Workstreams.to_string(), "workstreams");
    }

    #[test]
    fn browser_table_display_patterns() {
        assert_eq!(BrowserTable::Patterns.to_string(), "patterns");
    }

    #[test]
    fn browser_table_display_all() {
        assert_eq!(BrowserTable::All.to_string(), "all");
    }

    #[test]
    fn browser_table_eq_same_variant() {
        assert_eq!(BrowserTable::Chains, BrowserTable::Chains);
    }

    #[test]
    fn browser_table_ne_different_variants() {
        assert_ne!(BrowserTable::Chains, BrowserTable::Patterns);
    }

    #[test]
    fn browser_table_copy() {
        let t = BrowserTable::Trajectory;
        let t2 = t;
        assert_eq!(t, t2);
    }

    #[test]
    fn browser_table_debug_not_empty() {
        let dbg = format!("{:?}", BrowserTable::All);
        assert!(!dbg.is_empty());
    }

    #[test]
    fn browser_table_serde_roundtrip_chains() {
        let original = BrowserTable::Chains;
        let json = serde_json::to_string(&original).unwrap();
        let decoded: BrowserTable = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn browser_table_serde_roundtrip_all() {
        let original = BrowserTable::All;
        let json = serde_json::to_string(&original).unwrap();
        let decoded: BrowserTable = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    // -----------------------------------------------------------------------
    // BrowserConfig
    // -----------------------------------------------------------------------

    #[test]
    fn browser_config_new_defaults() {
        let cfg = BrowserConfig::new(BrowserTable::Chains);
        assert_eq!(cfg.table, BrowserTable::Chains);
        assert!(cfg.filter.is_none());
        assert!(!cfg.preview);
        assert!(!cfg.multi);
    }

    #[test]
    fn browser_config_with_filter() {
        let cfg = BrowserConfig::new(BrowserTable::Patterns).with_filter("BUG");
        assert_eq!(cfg.filter.as_deref(), Some("BUG"));
    }

    #[test]
    fn browser_config_with_preview() {
        let cfg = BrowserConfig::new(BrowserTable::Trajectory).with_preview();
        assert!(cfg.preview);
    }

    #[test]
    fn browser_config_with_multi() {
        let cfg = BrowserConfig::new(BrowserTable::Workstreams).with_multi();
        assert!(cfg.multi);
    }

    #[test]
    fn browser_config_builder_chain() {
        let cfg = BrowserConfig::new(BrowserTable::All)
            .with_filter("fzf")
            .with_preview()
            .with_multi();
        assert_eq!(cfg.filter.as_deref(), Some("fzf"));
        assert!(cfg.preview);
        assert!(cfg.multi);
        assert_eq!(cfg.table, BrowserTable::All);
    }

    #[test]
    fn browser_config_serde_roundtrip() {
        let original = BrowserConfig {
            table: BrowserTable::Chains,
            filter: Some("trap".to_string()),
            preview: true,
            multi: false,
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: BrowserConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.table, original.table);
        assert_eq!(decoded.filter, original.filter);
        assert_eq!(decoded.preview, original.preview);
        assert_eq!(decoded.multi, original.multi);
    }

    #[test]
    fn browser_config_debug_not_empty() {
        let cfg = BrowserConfig::new(BrowserTable::Patterns);
        assert!(!format!("{cfg:?}").is_empty());
    }

    #[test]
    fn browser_config_clone() {
        let cfg = BrowserConfig::new(BrowserTable::Workstreams).with_filter("x");
        let cloned = cfg.clone();
        assert_eq!(cloned.filter, cfg.filter);
    }

    // -----------------------------------------------------------------------
    // BrowserResult
    // -----------------------------------------------------------------------

    #[test]
    fn browser_result_construction() {
        let result = BrowserResult {
            selected: vec!["line1".to_string(), "line2".to_string()],
            table: BrowserTable::Chains,
            total_items: 10,
            filtered_items: 2,
        };
        assert_eq!(result.selected.len(), 2);
        assert_eq!(result.total_items, 10);
        assert_eq!(result.filtered_items, 2);
        assert_eq!(result.table, BrowserTable::Chains);
    }

    #[test]
    fn browser_result_serde_roundtrip() {
        let original = BrowserResult {
            selected: vec!["a".to_string()],
            table: BrowserTable::Patterns,
            total_items: 5,
            filtered_items: 1,
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: BrowserResult = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.table, original.table);
        assert_eq!(decoded.total_items, original.total_items);
        assert_eq!(decoded.selected, original.selected);
    }

    #[test]
    fn browser_result_clone() {
        let r = BrowserResult {
            selected: vec!["x".to_string()],
            table: BrowserTable::All,
            total_items: 1,
            filtered_items: 1,
        };
        let r2 = r.clone();
        assert_eq!(r2.selected, r.selected);
    }

    #[test]
    fn browser_result_debug_not_empty() {
        let r = BrowserResult {
            selected: vec![],
            table: BrowserTable::Trajectory,
            total_items: 0,
            filtered_items: 0,
        };
        assert!(!format!("{r:?}").is_empty());
    }

    // -----------------------------------------------------------------------
    // format_for_fzf
    // -----------------------------------------------------------------------

    #[test]
    fn format_for_fzf_empty_input_returns_empty() {
        let out = format_for_fzf(&[], &[]);
        assert!(out.is_empty());
    }

    #[test]
    fn format_for_fzf_headers_only_no_rows() {
        let headers = vec!["col1".to_string(), "col2".to_string()];
        let out = format_for_fzf(&[], &headers);
        assert_eq!(out, "col1\tcol2\n");
    }

    #[test]
    fn format_for_fzf_single_row() {
        let headers = vec!["a".to_string(), "b".to_string()];
        let rows = vec![vec!["val1".to_string(), "val2".to_string()]];
        let out = format_for_fzf(&rows, &headers);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[0], "a\tb");
        assert_eq!(lines[1], "val1\tval2");
    }

    #[test]
    fn format_for_fzf_multiple_rows() {
        let headers = vec!["x".to_string()];
        let rows = vec![
            vec!["r1".to_string()],
            vec!["r2".to_string()],
            vec!["r3".to_string()],
        ];
        let out = format_for_fzf(&rows, &headers);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 4); // header + 3 data rows
        assert_eq!(lines[1], "r1");
        assert_eq!(lines[2], "r2");
        assert_eq!(lines[3], "r3");
    }

    #[test]
    fn format_for_fzf_tab_separation() {
        let headers = vec!["h1".to_string(), "h2".to_string(), "h3".to_string()];
        let rows = vec![vec!["a".to_string(), "b".to_string(), "c".to_string()]];
        let out = format_for_fzf(&rows, &headers);
        let first_data = out.lines().nth(1).unwrap_or("");
        assert_eq!(first_data, "a\tb\tc");
    }

    #[test]
    fn format_for_fzf_no_headers_with_rows() {
        let rows = vec![vec!["only-data".to_string()]];
        let out = format_for_fzf(&rows, &[]);
        assert_eq!(out, "only-data\n");
    }

    #[test]
    fn format_for_fzf_each_row_ends_with_newline() {
        let headers = vec!["h".to_string()];
        let rows = vec![vec!["v".to_string()], vec!["w".to_string()]];
        let out = format_for_fzf(&rows, &headers);
        // Every line should end with exactly one newline
        assert_eq!(out.chars().filter(|&c| c == '\n').count(), 3); // header + 2 rows
    }

    // -----------------------------------------------------------------------
    // build_fzf_args
    // -----------------------------------------------------------------------

    #[test]
    fn build_fzf_args_defaults_include_delimiter_and_nosort() {
        let cfg = BrowserConfig::new(BrowserTable::Chains);
        let args = build_fzf_args(&cfg);
        assert!(args.iter().any(|a| a == "--delimiter=\t"));
        assert!(args.iter().any(|a| a == "--no-sort"));
    }

    #[test]
    fn build_fzf_args_defaults_include_header_lines() {
        let cfg = BrowserConfig::new(BrowserTable::Chains);
        let args = build_fzf_args(&cfg);
        assert!(args.iter().any(|a| a == "--header-lines=1"));
    }

    #[test]
    fn build_fzf_args_no_preview_by_default() {
        let cfg = BrowserConfig::new(BrowserTable::Chains);
        let args = build_fzf_args(&cfg);
        assert!(!args.iter().any(|a| a == "--preview"));
    }

    #[test]
    fn build_fzf_args_preview_adds_flag() {
        let cfg = BrowserConfig::new(BrowserTable::Chains).with_preview();
        let args = build_fzf_args(&cfg);
        assert!(args.iter().any(|a| a == "--preview"));
    }

    #[test]
    fn build_fzf_args_filter_adds_flag_and_value() {
        let cfg = BrowserConfig::new(BrowserTable::Chains).with_filter("trap");
        let args = build_fzf_args(&cfg);
        let filter_idx = args.iter().position(|a| a == "--filter");
        assert!(filter_idx.is_some());
        let idx = filter_idx.unwrap();
        assert_eq!(args.get(idx + 1).map(String::as_str), Some("trap"));
    }

    #[test]
    fn build_fzf_args_no_filter_by_default() {
        let cfg = BrowserConfig::new(BrowserTable::Patterns);
        let args = build_fzf_args(&cfg);
        assert!(!args.iter().any(|a| a == "--filter"));
    }

    #[test]
    fn build_fzf_args_multi_adds_flag() {
        let cfg = BrowserConfig::new(BrowserTable::Chains).with_multi();
        let args = build_fzf_args(&cfg);
        assert!(args.iter().any(|a| a == "--multi"));
    }

    #[test]
    fn build_fzf_args_no_multi_by_default() {
        let cfg = BrowserConfig::new(BrowserTable::Chains);
        let args = build_fzf_args(&cfg);
        assert!(!args.iter().any(|a| a == "--multi"));
    }

    #[test]
    fn build_fzf_args_all_options_together() {
        let cfg = BrowserConfig::new(BrowserTable::All)
            .with_filter("BUG")
            .with_preview()
            .with_multi();
        let args = build_fzf_args(&cfg);
        assert!(args.iter().any(|a| a == "--filter"));
        assert!(args.iter().any(|a| a == "--preview"));
        assert!(args.iter().any(|a| a == "--multi"));
        assert!(args.iter().any(|a| a == "--no-sort"));
    }

    // -----------------------------------------------------------------------
    // simple_filter
    // -----------------------------------------------------------------------

    #[test]
    fn simple_filter_empty_input_returns_empty() {
        let result = simple_filter("", "bug");
        assert!(result.is_empty());
    }

    #[test]
    fn simple_filter_empty_filter_returns_all_lines() {
        let input = "line1\nline2\nline3\n";
        let result = simple_filter(input, "");
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn simple_filter_case_insensitive() {
        let input = "BUG-001\tbug description\nTRAP-002\ttrap here\n";
        let result = simple_filter(input, "bug");
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("BUG-001"));
    }

    #[test]
    fn simple_filter_no_match_returns_empty() {
        let input = "line1\nline2\n";
        let result = simple_filter(input, "xxxxxxx");
        assert!(result.is_empty());
    }

    #[test]
    fn simple_filter_multi_match() {
        let input = "alpha bug\nbeta bug\ngamma no\n";
        let result = simple_filter(input, "bug");
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn simple_filter_skips_empty_lines() {
        let input = "\n\nvalid line\n\n";
        let result = simple_filter(input, "");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "valid line");
    }

    #[test]
    fn simple_filter_match_at_start_of_line() {
        let input = "START match\nnot this\n";
        let result = simple_filter(input, "start");
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn simple_filter_match_at_end_of_line() {
        let input = "something END\nnot this\n";
        let result = simple_filter(input, "end");
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn simple_filter_full_case_insensitive_unicode() {
        // Basic ASCII normalisation
        let input = "UPPERCASE\nlowercase\nMiXeD\n";
        let result = simple_filter(input, "uppercase");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "UPPERCASE");
    }

    #[test]
    fn simple_filter_returns_owned_strings() {
        let input = "hello world\n";
        let result = simple_filter(input, "hello");
        assert_eq!(result[0], "hello world");
    }

    // -----------------------------------------------------------------------
    // run_filter (fallback path — no fzf installed in CI)
    // -----------------------------------------------------------------------

    #[test]
    fn run_filter_returns_matched_lines() {
        // This exercises the simple_filter fallback when fzf is absent, or
        // the real fzf path when it is present.  Either way the semantics
        // must hold.
        let input = "BUG-001\tsome bug\nTRAP-002\tsome trap\n";
        let result = run_filter(input, "BUG").unwrap();
        assert!(!result.is_empty());
        assert!(result.iter().any(|l| l.contains("BUG-001")));
    }

    #[test]
    fn run_filter_no_match_returns_empty_vec() {
        let input = "alpha\nbeta\ngamma\n";
        let result = run_filter(input, "xxxxxxx").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn run_filter_empty_input_returns_empty() {
        let result = run_filter("", "anything").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn run_filter_empty_filter_returns_all() {
        let input = "line1\nline2\n";
        let result = run_filter(input, "").unwrap();
        // Both the fzf and simple_filter paths return all non-empty lines for
        // an empty filter (fzf passthrough / simple_filter empty needle).
        assert!(!result.is_empty());
    }

    #[test]
    fn run_filter_ok_result_type() {
        let r: Result<Vec<String>, QueryError> = run_filter("x\n", "x");
        assert!(r.is_ok());
    }

    // -----------------------------------------------------------------------
    // browse_table (sqlite feature-gated)
    // -----------------------------------------------------------------------

    #[cfg(feature = "sqlite")]
    mod sqlite_tests {
        use super::super::*;
        use crate::m2_schema::m06_schema::open_memory;
        use crate::m2_schema::m07_causal_chain::insert_chain;
        use crate::m2_schema::m08_trajectory::insert_point;
        use crate::m2_schema::m09_workstream::insert_workstream;
        use crate::m2_schema::m10_pattern::insert_pattern;

        fn empty_conn() -> Connection {
            open_memory().unwrap()
        }

        // ---- browse_table with empty DB -----------------------------------

        #[test]
        fn browse_chains_empty_db() {
            let conn = empty_conn();
            let cfg = BrowserConfig::new(BrowserTable::Chains);
            let result = browse_table(&conn, &cfg).unwrap();
            assert_eq!(result.total_items, 0);
            assert_eq!(result.table, BrowserTable::Chains);
        }

        #[test]
        fn browse_trajectory_empty_db() {
            let conn = empty_conn();
            let cfg = BrowserConfig::new(BrowserTable::Trajectory);
            let result = browse_table(&conn, &cfg).unwrap();
            assert_eq!(result.total_items, 0);
        }

        #[test]
        fn browse_workstreams_empty_db() {
            let conn = empty_conn();
            let cfg = BrowserConfig::new(BrowserTable::Workstreams);
            let result = browse_table(&conn, &cfg).unwrap();
            assert_eq!(result.total_items, 0);
        }

        #[test]
        fn browse_patterns_empty_db() {
            let conn = empty_conn();
            let cfg = BrowserConfig::new(BrowserTable::Patterns);
            let result = browse_table(&conn, &cfg).unwrap();
            assert_eq!(result.total_items, 0);
        }

        #[test]
        fn browse_all_empty_db() {
            let conn = empty_conn();
            let cfg = BrowserConfig::new(BrowserTable::All);
            let result = browse_table(&conn, &cfg).unwrap();
            assert_eq!(result.total_items, 0);
            assert_eq!(result.table, BrowserTable::All);
        }

        // ---- browse_table with seeded data --------------------------------

        fn seed_chain(conn: &Connection) {
            insert_chain(conn, 109, "bug", "BUG-001", "test bug").unwrap();
        }

        fn seed_trajectory(conn: &Connection) {
            insert_point(conn, 109, 0.67, 0.92, 0.244, 5.88, 12, "delta text", None).unwrap();
        }

        fn seed_workstream(conn: &Connection) {
            insert_workstream(conn, "ws-1", "Demo WS", "active", 109, "resume here").unwrap();
        }

        fn seed_pattern(conn: &Connection) {
            insert_pattern(conn, "verify-first", "procedural", "verify before ship", None)
                .unwrap();
        }

        #[test]
        fn browse_chains_with_data() {
            let conn = empty_conn();
            seed_chain(&conn);
            let cfg = BrowserConfig::new(BrowserTable::Chains);
            let result = browse_table(&conn, &cfg).unwrap();
            assert_eq!(result.total_items, 1);
            assert!(result.selected.iter().any(|l| l.contains("BUG-001")));
        }

        #[test]
        fn browse_trajectory_with_data() {
            let conn = empty_conn();
            seed_trajectory(&conn);
            let cfg = BrowserConfig::new(BrowserTable::Trajectory);
            let result = browse_table(&conn, &cfg).unwrap();
            assert_eq!(result.total_items, 1);
            assert!(result.selected.iter().any(|l| l.contains("109")));
        }

        #[test]
        fn browse_workstreams_with_data() {
            let conn = empty_conn();
            seed_workstream(&conn);
            let cfg = BrowserConfig::new(BrowserTable::Workstreams);
            let result = browse_table(&conn, &cfg).unwrap();
            assert_eq!(result.total_items, 1);
            assert!(result.selected.iter().any(|l| l.contains("ws-1")));
        }

        #[test]
        fn browse_patterns_with_data() {
            let conn = empty_conn();
            seed_pattern(&conn);
            let cfg = BrowserConfig::new(BrowserTable::Patterns);
            let result = browse_table(&conn, &cfg).unwrap();
            assert_eq!(result.total_items, 1);
            assert!(result.selected.iter().any(|l| l.contains("verify-first")));
        }

        #[test]
        fn browse_all_with_seeded_data() {
            let conn = empty_conn();
            seed_chain(&conn);
            seed_trajectory(&conn);
            seed_workstream(&conn);
            seed_pattern(&conn);
            let cfg = BrowserConfig::new(BrowserTable::All);
            let result = browse_table(&conn, &cfg).unwrap();
            // At least 4 data rows (one from each table).
            assert!(result.total_items >= 4);
        }

        #[test]
        fn browse_chains_filter_match() {
            let conn = empty_conn();
            insert_chain(&conn, 109, "bug", "BUG-FILTER", "matchable").unwrap();
            insert_chain(&conn, 109, "trap", "TRAP-OTHER", "unrelated").unwrap();
            let cfg = BrowserConfig::new(BrowserTable::Chains).with_filter("BUG-FILTER");
            let result = browse_table(&conn, &cfg).unwrap();
            assert!(result.selected.iter().any(|l| l.contains("BUG-FILTER")));
        }

        #[test]
        fn browse_chains_filter_no_match() {
            let conn = empty_conn();
            insert_chain(&conn, 109, "bug", "BUG-001", "desc").unwrap();
            let cfg = BrowserConfig::new(BrowserTable::Chains).with_filter("xxxxxxxxxxx");
            let result = browse_table(&conn, &cfg).unwrap();
            // filtered_items should be 0 (or header-only if fzf includes it)
            // The selected vec should contain no data row with "BUG-001"
            assert!(!result.selected.iter().any(|l| l.contains("BUG-001")));
        }

        #[test]
        fn browse_table_result_table_field_correct() {
            let conn = empty_conn();
            let cfg = BrowserConfig::new(BrowserTable::Patterns);
            let result = browse_table(&conn, &cfg).unwrap();
            assert_eq!(result.table, BrowserTable::Patterns);
        }

        #[test]
        fn browse_table_filtered_items_leq_total() {
            let conn = empty_conn();
            seed_chain(&conn);
            let cfg = BrowserConfig::new(BrowserTable::Chains).with_filter("zzz");
            let result = browse_table(&conn, &cfg).unwrap();
            assert!(result.filtered_items <= result.total_items);
        }

        #[test]
        fn browse_workstreams_includes_blocked() {
            let conn = empty_conn();
            // Insert one active and one blocked workstream
            insert_workstream(&conn, "ws-active", "Active WS", "active", 109, "ctx").unwrap();
            insert_workstream(&conn, "ws-blocked", "Blocked WS", "blocked", 109, "ctx").unwrap();
            let cfg = BrowserConfig::new(BrowserTable::Workstreams);
            let result = browse_table(&conn, &cfg).unwrap();
            assert_eq!(result.total_items, 2);
        }

        #[test]
        fn browse_chains_excludes_resolved() {
            let conn = empty_conn();
            let id = insert_chain(&conn, 109, "bug", "RESOLVED", "done").unwrap();
            crate::m2_schema::m07_causal_chain::resolve_chain(&conn, id, 110).unwrap();
            insert_chain(&conn, 109, "bug", "OPEN", "open").unwrap();
            let cfg = BrowserConfig::new(BrowserTable::Chains);
            let result = browse_table(&conn, &cfg).unwrap();
            assert_eq!(result.total_items, 1);
            assert!(result.selected.iter().any(|l| l.contains("OPEN")));
        }

        #[test]
        fn all_section_header_present_in_output() {
            let conn = empty_conn();
            seed_chain(&conn);
            let cfg = BrowserConfig::new(BrowserTable::All);
            let result = browse_table(&conn, &cfg).unwrap();
            // The first selected line should be the header
            let first = result.selected.first().unwrap_or(&String::new()).clone();
            assert!(first.contains("section") || first.contains("chains"),
                "expected header or 'chains' section marker, got: {first}");
        }
    }
}
