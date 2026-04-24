//! `m10b_checkpoint` — CRUD for the `session_checkpoint` table.
//!
//! Ingests `/save-session` checkpoint data. The `session_checkpoint` table is
//! the bridge between `/save-session`'s 7-surface persistence and the injection
//! database. It captures:
//!
//! - Frontmatter: `label`, `timestamp`, `pane_id`, `tab`, `cwd`, git state,
//!   services, watcher, persona
//! - `accomplished`: concrete artefacts (files, commits, deploys)
//! - `in_progress`: half-shipped work
//! - `blocked` / deferred: with reasons
//! - `key_findings`: insights, measurements, discoveries
//! - `resume_instructions`: concrete first-moves for next session
//! - `conversation_anchors`: `file:line` refs, POVM pathway IDs, RM IDs,
//!   wikilinks
//!
//! JSON columns (`accomplished_json`, `in_progress_json`, `blocked_json`,
//! `key_findings_json`) store `Vec<String>` serialised with `serde_json`.
//! All reads deserialise them back to `Vec<String>`.
//!
//! Layer: `m2_schema`
//! Dependencies: `m01_types` (indirectly), `m02_errors`, `m06_schema`

#[cfg(feature = "sqlite")]
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use crate::m1_foundation::m02_errors::SchemaError;

#[cfg(feature = "sqlite")]
use super::sqlite_err;

// ---------------------------------------------------------------------------
// Consent level
// ---------------------------------------------------------------------------

/// Consent level stored in the `consent` column.
///
/// `SQLite` `CHECK` constraint enforces only these three values.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Consent {
    /// Emit this checkpoint during context injection (default).
    #[default]
    Emit,
    /// Store only — do not inject into context.
    Store,
    /// Do not store or inject.
    Forget,
}

impl Consent {
    /// Returns the string representation used in the database.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Emit => "Emit",
            Self::Store => "Store",
            Self::Forget => "Forget",
        }
    }
}

impl std::fmt::Display for Consent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Consent {
    type Err = SchemaError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Emit" => Ok(Self::Emit),
            "Store" => Ok(Self::Store),
            "Forget" => Ok(Self::Forget),
            other => Err(SchemaError::Sqlite(format!(
                "invalid consent value: {other:?}"
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// CheckpointRow — full row returned by SELECT
// ---------------------------------------------------------------------------

/// A fully hydrated row from the `session_checkpoint` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointRow {
    /// Primary key.
    pub id: i64,
    /// Unique human-readable label, e.g. `"s109-close"`.
    pub label: String,
    /// Session number (e.g. `109`). Optional.
    pub session_number: Option<u32>,
    /// ISO-8601 UTC timestamp string.
    pub timestamp_utc: String,
    /// Zellij pane identifier. Optional.
    pub pane_id: Option<String>,
    /// Zellij tab name. Optional.
    pub tab: Option<String>,
    /// Human-readable session name. Optional.
    pub session_name: Option<String>,
    /// Current working directory. Optional.
    pub cwd: Option<String>,
    /// Git commit SHA (short or full). Optional.
    pub git_sha: Option<String>,
    /// Git branch name. Optional.
    pub git_branch: Option<String>,
    /// Number of dirty files in the working tree.
    pub git_dirty_files: i64,
    /// Subject of the last git commit. Optional.
    pub git_last_commit: Option<String>,
    /// Services alive at checkpoint time.
    pub services_alive: i64,
    /// Total expected services (default `12`).
    pub services_total: i64,
    /// Comma-separated or JSON list of alive ports. Optional.
    pub services_alive_ports: Option<String>,
    /// Whether the Watcher persona was ready (`1`) or not (`0`).
    pub watcher_ready: i64,
    /// Reason the Watcher was not ready, if any.
    pub watcher_reason: Option<String>,
    /// Active persona name. Optional.
    pub persona: Option<String>,
    /// Scope constraint for this checkpoint. Optional.
    pub scope_constraint: Option<String>,
    /// Concrete artefacts accomplished in this session.
    pub accomplished: Vec<String>,
    /// Work in progress at checkpoint time.
    pub in_progress: Vec<String>,
    /// Blocked or deferred items.
    pub blocked: Vec<String>,
    /// Key findings, insights, and measurements.
    pub key_findings: Vec<String>,
    /// Raw markdown resume instructions for the next session.
    pub resume_instructions: String,
    /// Raw markdown conversation anchors. Optional.
    pub conversation_anchors: Option<String>,
    /// Path to the source checkpoint `.md` file.
    pub source_file: String,
    /// Consent level controlling injection and storage.
    pub consent: String,
}

// ---------------------------------------------------------------------------
// CheckpointInsert — builder-style input struct
// ---------------------------------------------------------------------------

/// Input struct for [`insert_checkpoint`].
///
/// Construct with [`CheckpointInsert::new`] for required fields, then set
/// optional fields via direct field assignment before passing to
/// [`insert_checkpoint`].
#[derive(Debug, Clone, Default)]
pub struct CheckpointInsert {
    // --- required ---
    /// Unique label, e.g. `"s109-close"`.
    pub label: String,
    /// ISO-8601 UTC timestamp string.
    pub timestamp_utc: String,
    /// Number of services alive.
    pub services_alive: i64,
    /// Accomplished items (serialised to JSON).
    pub accomplished: Vec<String>,
    /// In-progress items (serialised to JSON).
    pub in_progress: Vec<String>,
    /// Blocked items (serialised to JSON).
    pub blocked: Vec<String>,
    /// Key findings (serialised to JSON).
    pub key_findings: Vec<String>,
    /// Raw markdown resume instructions.
    pub resume_instructions: String,
    /// Path to source `.md` file.
    pub source_file: String,

    // --- optional ---
    /// Session number, if known.
    pub session_number: Option<u32>,
    /// Zellij pane ID.
    pub pane_id: Option<String>,
    /// Zellij tab name.
    pub tab: Option<String>,
    /// Human-readable session name.
    pub session_name: Option<String>,
    /// Current working directory.
    pub cwd: Option<String>,
    /// Git commit SHA.
    pub git_sha: Option<String>,
    /// Git branch name.
    pub git_branch: Option<String>,
    /// Dirty file count in the working tree.
    pub git_dirty_files: Option<i64>,
    /// Subject of the last git commit.
    pub git_last_commit: Option<String>,
    /// `services_total` override (default `12`).
    pub services_total: Option<i64>,
    /// Alive ports string.
    pub services_alive_ports: Option<String>,
    /// Whether the Watcher is ready (`true` → `1`).
    pub watcher_ready: Option<bool>,
    /// Reason the Watcher is not ready.
    pub watcher_reason: Option<String>,
    /// Active persona name.
    pub persona: Option<String>,
    /// Scope constraint.
    pub scope_constraint: Option<String>,
    /// Conversation anchors markdown.
    pub conversation_anchors: Option<String>,
    /// Consent level (default `"Emit"`).
    pub consent: Option<Consent>,
}

impl CheckpointInsert {
    /// Construct with all required fields.
    ///
    /// Optional fields are `None` / default by [`Default`].
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    #[cfg(feature = "sqlite")]
    pub fn new(
        label: impl Into<String>,
        timestamp_utc: impl Into<String>,
        services_alive: i64,
        accomplished: Vec<String>,
        in_progress: Vec<String>,
        blocked: Vec<String>,
        key_findings: Vec<String>,
        resume_instructions: impl Into<String>,
        source_file: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            timestamp_utc: timestamp_utc.into(),
            services_alive,
            accomplished,
            in_progress,
            blocked,
            key_findings,
            resume_instructions: resume_instructions.into(),
            source_file: source_file.into(),
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[cfg(feature = "sqlite")]
fn ser_vec(v: &[String]) -> Result<String, SchemaError> {
    serde_json::to_string(v).map_err(|e| sqlite_err(format_args!("json serialise: {e}")))
}

#[cfg(feature = "sqlite")]
fn de_vec(s: &str) -> Result<Vec<String>, SchemaError> {
    serde_json::from_str(s).map_err(|e| sqlite_err(format_args!("json deserialise: {e}")))
}

/// Extract a full [`CheckpointRow`] from a `rusqlite::Row`.
#[cfg(feature = "sqlite")]
fn row_from_rusqlite(row: &rusqlite::Row<'_>) -> Result<CheckpointRow, rusqlite::Error> {
    let accomplished_json: String = row.get(19)?;
    let in_progress_json: String = row.get(20)?;
    let blocked_json: String = row.get(21)?;
    let key_findings_json: String = row.get(22)?;

    let accomplished =
        de_vec(&accomplished_json).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(SqlDeError(e.to_string()))))?;
    let in_progress =
        de_vec(&in_progress_json).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(SqlDeError(e.to_string()))))?;
    let blocked =
        de_vec(&blocked_json).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(SqlDeError(e.to_string()))))?;
    let key_findings =
        de_vec(&key_findings_json).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(SqlDeError(e.to_string()))))?;

    Ok(CheckpointRow {
        id: row.get(0)?,
        label: row.get(1)?,
        session_number: row.get(2)?,
        timestamp_utc: row.get(3)?,
        pane_id: row.get(4)?,
        tab: row.get(5)?,
        session_name: row.get(6)?,
        cwd: row.get(7)?,
        git_sha: row.get(8)?,
        git_branch: row.get(9)?,
        git_dirty_files: row.get::<_, Option<i64>>(10)?.unwrap_or(0),
        git_last_commit: row.get(11)?,
        services_alive: row.get(12)?,
        services_total: row.get::<_, Option<i64>>(13)?.unwrap_or(12),
        services_alive_ports: row.get(14)?,
        watcher_ready: row.get::<_, Option<i64>>(15)?.unwrap_or(0),
        watcher_reason: row.get(16)?,
        persona: row.get(17)?,
        scope_constraint: row.get(18)?,
        accomplished,
        in_progress,
        blocked,
        key_findings,
        resume_instructions: row.get(23)?,
        conversation_anchors: row.get(24)?,
        source_file: row.get(25)?,
        consent: row.get(26)?,
    })
}

/// Tiny error wrapper so we can box a string into `rusqlite::Error`.
#[derive(Debug)]
#[cfg(feature = "sqlite")]
struct SqlDeError(String);

#[cfg(feature = "sqlite")]
impl std::fmt::Display for SqlDeError {
    #[cfg(feature = "sqlite")]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(feature = "sqlite")]
impl std::error::Error for SqlDeError {}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Insert a new checkpoint row. Returns the new row `id`.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on constraint violation (e.g. duplicate
/// `label`) or database error. Returns [`SchemaError::Sqlite`] on JSON
/// serialisation failure.
#[cfg(feature = "sqlite")]
pub fn insert_checkpoint(
    conn: &Connection,
    cp: &CheckpointInsert,
) -> Result<i64, SchemaError> {
    let accomplished_json = ser_vec(&cp.accomplished)?;
    let in_progress_json = ser_vec(&cp.in_progress)?;
    let blocked_json = ser_vec(&cp.blocked)?;
    let key_findings_json = ser_vec(&cp.key_findings)?;

    let consent = cp
        .consent
        .as_ref()
        .map_or("Emit", Consent::as_str);
    let watcher_ready: i64 = cp.watcher_ready.map_or(0, i64::from);
    let services_total: i64 = cp.services_total.unwrap_or(12);

    conn.execute(
        "INSERT INTO session_checkpoint (
             label, session_number, timestamp_utc,
             pane_id, tab, session_name, cwd,
             git_sha, git_branch, git_dirty_files, git_last_commit,
             services_alive, services_total, services_alive_ports,
             watcher_ready, watcher_reason, persona, scope_constraint,
             accomplished_json, in_progress_json, blocked_json, key_findings_json,
             resume_instructions, conversation_anchors, source_file, consent
         ) VALUES (
             ?1, ?2, ?3,
             ?4, ?5, ?6, ?7,
             ?8, ?9, ?10, ?11,
             ?12, ?13, ?14,
             ?15, ?16, ?17, ?18,
             ?19, ?20, ?21, ?22,
             ?23, ?24, ?25, ?26
         )",
        params![
            cp.label,
            cp.session_number,
            cp.timestamp_utc,
            cp.pane_id,
            cp.tab,
            cp.session_name,
            cp.cwd,
            cp.git_sha,
            cp.git_branch,
            cp.git_dirty_files,
            cp.git_last_commit,
            cp.services_alive,
            services_total,
            cp.services_alive_ports,
            watcher_ready,
            cp.watcher_reason,
            cp.persona,
            cp.scope_constraint,
            accomplished_json,
            in_progress_json,
            blocked_json,
            key_findings_json,
            cp.resume_instructions,
            cp.conversation_anchors,
            cp.source_file,
            consent,
        ],
    )
    .map_err(|e| sqlite_err(&e))?;

    Ok(conn.last_insert_rowid())
}

/// Retrieve a checkpoint by its unique `label`. Returns `None` if not found.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on query or deserialisation failure.
#[cfg(feature = "sqlite")]
pub fn get_by_label(
    conn: &Connection,
    label: &str,
) -> Result<Option<CheckpointRow>, SchemaError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, label, session_number, timestamp_utc,
                    pane_id, tab, session_name, cwd,
                    git_sha, git_branch, git_dirty_files, git_last_commit,
                    services_alive, services_total, services_alive_ports,
                    watcher_ready, watcher_reason, persona, scope_constraint,
                    accomplished_json, in_progress_json, blocked_json, key_findings_json,
                    resume_instructions, conversation_anchors, source_file, consent
             FROM session_checkpoint
             WHERE label = ?1",
        )
        .map_err(|e| sqlite_err(&e))?;

    let mut rows = stmt
        .query_map(params![label], row_from_rusqlite)
        .map_err(|e| sqlite_err(&e))?;

    match rows.next() {
        None => Ok(None),
        Some(result) => result.map(Some).map_err(|e| sqlite_err(&e)),
    }
}

/// Retrieve all checkpoints for a given `session_number`, ordered by
/// `timestamp_utc DESC`.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on query or deserialisation failure.
#[cfg(feature = "sqlite")]
pub fn get_by_session(
    conn: &Connection,
    session_number: u32,
) -> Result<Vec<CheckpointRow>, SchemaError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, label, session_number, timestamp_utc,
                    pane_id, tab, session_name, cwd,
                    git_sha, git_branch, git_dirty_files, git_last_commit,
                    services_alive, services_total, services_alive_ports,
                    watcher_ready, watcher_reason, persona, scope_constraint,
                    accomplished_json, in_progress_json, blocked_json, key_findings_json,
                    resume_instructions, conversation_anchors, source_file, consent
             FROM session_checkpoint
             WHERE session_number = ?1
             ORDER BY timestamp_utc DESC",
        )
        .map_err(|e| sqlite_err(&e))?;

    let rows = stmt
        .query_map(params![session_number], row_from_rusqlite)
        .map_err(|e| sqlite_err(&e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| sqlite_err(&e))?;

    Ok(rows)
}

/// Retrieve the `limit` most recent checkpoints by `timestamp_utc DESC`.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on query or deserialisation failure.
#[cfg(feature = "sqlite")]
pub fn get_recent(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<CheckpointRow>, SchemaError> {
    let limit_i64 = i64::try_from(limit)
        .map_err(|e| sqlite_err(format_args!("limit out of range: {e}")))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, label, session_number, timestamp_utc,
                    pane_id, tab, session_name, cwd,
                    git_sha, git_branch, git_dirty_files, git_last_commit,
                    services_alive, services_total, services_alive_ports,
                    watcher_ready, watcher_reason, persona, scope_constraint,
                    accomplished_json, in_progress_json, blocked_json, key_findings_json,
                    resume_instructions, conversation_anchors, source_file, consent
             FROM session_checkpoint
             ORDER BY timestamp_utc DESC
             LIMIT ?1",
        )
        .map_err(|e| sqlite_err(&e))?;

    let rows = stmt
        .query_map(params![limit_i64], row_from_rusqlite)
        .map_err(|e| sqlite_err(&e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| sqlite_err(&e))?;

    Ok(rows)
}

/// Retrieve the single most recent checkpoint by `timestamp_utc DESC`.
/// Returns `None` if the table is empty.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on query or deserialisation failure.
#[cfg(feature = "sqlite")]
pub fn get_latest(conn: &Connection) -> Result<Option<CheckpointRow>, SchemaError> {
    let rows = get_recent(conn, 1)?;
    Ok(rows.into_iter().next())
}

/// Count all rows in `session_checkpoint`.
///
/// # Errors
///
/// Returns [`SchemaError::Sqlite`] on query failure.
#[cfg(feature = "sqlite")]
pub fn count(conn: &Connection) -> Result<u64, SchemaError> {
    conn.query_row("SELECT COUNT(*) FROM session_checkpoint", [], |r| r.get::<_, i64>(0))
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

    // --- helpers -----------------------------------------------------------

#[cfg(feature = "sqlite")]
    fn db() -> Connection {
        open_memory().expect("open_memory")
    }

    #[cfg(feature = "sqlite")]
    fn minimal(label: &str) -> CheckpointInsert {
        CheckpointInsert::new(
            label,
            "2026-04-24T12:00:00Z",
            11,
            vec!["shipped m10b".to_string()],
            vec!["m11 next".to_string()],
            vec![],
            vec!["JSON roundtrip works".to_string()],
            "cd memory-injection && cargo test",
            "/tmp/s109.md",
        )
    }

    // --- insert_checkpoint (minimal) ---------------------------------------

    #[test]
    #[cfg(feature = "sqlite")]
    fn insert_minimal_returns_positive_id() {
        let conn = db();
        let id = insert_checkpoint(&conn, &minimal("s109-min")).expect("insert");
        assert!(id > 0);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn insert_minimal_count_becomes_one() {
        let conn = db();
        insert_checkpoint(&conn, &minimal("s109-cnt")).expect("insert");
        assert_eq!(count(&conn).expect("count"), 1);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn insert_multiple_count_increments() {
        let conn = db();
        for i in 0..5u32 {
            insert_checkpoint(&conn, &minimal(&format!("lbl-{i}"))).expect("insert");
        }
        assert_eq!(count(&conn).expect("count"), 5);
    }

    // --- insert_checkpoint (full / all optional) ---------------------------

    #[test]
    #[cfg(feature = "sqlite")]
    fn insert_full_optional_fields() {
        let conn = db();
        let mut cp = minimal("s109-full");
        cp.session_number = Some(109);
        cp.pane_id = Some("pane-3".into());
        cp.tab = Some("Tab 1 Orchestrator".into());
        cp.session_name = Some("SpaceTimeDB Memory Injection".into());
        cp.cwd = Some("/home/louranicas/claude-code-workspace/memory-injection".into());
        cp.git_sha = Some("abc1234".into());
        cp.git_branch = Some("main".into());
        cp.git_dirty_files = Some(3);
        cp.git_last_commit = Some("feat: implement m10b checkpoint CRUD".into());
        cp.services_total = Some(12);
        cp.services_alive_ports =
            Some("8080,8081,8090,8105,8110,8120,8125,8130,8132,8133,9001,10001".into());
        cp.watcher_ready = Some(true);
        cp.watcher_reason = None;
        cp.persona = Some("The Watcher".into());
        cp.scope_constraint = Some("m2_schema only".into());
        cp.conversation_anchors = Some("[[Session 109]]".into());
        cp.consent = Some(Consent::Emit);

        let id = insert_checkpoint(&conn, &cp).expect("insert full");
        assert!(id > 0);

        let row = get_by_label(&conn, "s109-full")
            .expect("get")
            .expect("some");
        assert_eq!(row.session_number, Some(109));
        assert_eq!(row.pane_id.as_deref(), Some("pane-3"));
        assert_eq!(row.tab.as_deref(), Some("Tab 1 Orchestrator"));
        assert_eq!(row.git_dirty_files, 3);
        assert_eq!(row.watcher_ready, 1);
        assert_eq!(row.persona.as_deref(), Some("The Watcher"));
        assert_eq!(row.consent, "Emit");
    }

    // --- default values ---------------------------------------------------

    #[test]
    #[cfg(feature = "sqlite")]
    fn default_services_total_is_12() {
        let conn = db();
        insert_checkpoint(&conn, &minimal("def-total")).expect("insert");
        let row = get_by_label(&conn, "def-total")
            .expect("get")
            .expect("some");
        assert_eq!(row.services_total, 12);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn default_consent_is_emit() {
        let conn = db();
        insert_checkpoint(&conn, &minimal("def-consent")).expect("insert");
        let row = get_by_label(&conn, "def-consent")
            .expect("get")
            .expect("some");
        assert_eq!(row.consent, "Emit");
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn default_git_dirty_files_is_zero() {
        let conn = db();
        insert_checkpoint(&conn, &minimal("def-dirty")).expect("insert");
        let row = get_by_label(&conn, "def-dirty")
            .expect("get")
            .expect("some");
        assert_eq!(row.git_dirty_files, 0);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn default_watcher_ready_is_zero() {
        let conn = db();
        insert_checkpoint(&conn, &minimal("def-watcher")).expect("insert");
        let row = get_by_label(&conn, "def-watcher")
            .expect("get")
            .expect("some");
        assert_eq!(row.watcher_ready, 0);
    }

    // --- JSON roundtrip for array columns ---------------------------------

    #[test]
    #[cfg(feature = "sqlite")]
    fn accomplished_json_roundtrip() {
        let conn = db();
        let mut cp = minimal("json-acc");
        cp.accomplished = vec!["item A".into(), "item B".into(), "item C".into()];
        insert_checkpoint(&conn, &cp).expect("insert");
        let row = get_by_label(&conn, "json-acc").expect("get").expect("some");
        assert_eq!(row.accomplished, vec!["item A", "item B", "item C"]);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn in_progress_json_roundtrip() {
        let conn = db();
        let mut cp = minimal("json-ip");
        cp.in_progress = vec!["work-X".into(), "work-Y".into()];
        insert_checkpoint(&conn, &cp).expect("insert");
        let row = get_by_label(&conn, "json-ip").expect("get").expect("some");
        assert_eq!(row.in_progress, vec!["work-X", "work-Y"]);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn blocked_json_roundtrip() {
        let conn = db();
        let mut cp = minimal("json-bl");
        cp.blocked = vec!["BUG-055 systemd".into()];
        insert_checkpoint(&conn, &cp).expect("insert");
        let row = get_by_label(&conn, "json-bl").expect("get").expect("some");
        assert_eq!(row.blocked, vec!["BUG-055 systemd"]);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn key_findings_json_roundtrip() {
        let conn = db();
        let mut cp = minimal("json-kf");
        cp.key_findings = vec!["field_r=0.876".into(), "LTP/LTD=4.2".into()];
        insert_checkpoint(&conn, &cp).expect("insert");
        let row = get_by_label(&conn, "json-kf").expect("get").expect("some");
        assert_eq!(row.key_findings, vec!["field_r=0.876", "LTP/LTD=4.2"]);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn empty_json_arrays_roundtrip() {
        let conn = db();
        let mut cp = minimal("json-empty");
        cp.accomplished = vec![];
        cp.in_progress = vec![];
        cp.blocked = vec![];
        cp.key_findings = vec![];
        insert_checkpoint(&conn, &cp).expect("insert");
        let row = get_by_label(&conn, "json-empty")
            .expect("get")
            .expect("some");
        assert!(row.accomplished.is_empty());
        assert!(row.in_progress.is_empty());
        assert!(row.blocked.is_empty());
        assert!(row.key_findings.is_empty());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn json_entries_with_special_characters() {
        let conn = db();
        let mut cp = minimal("json-special");
        cp.accomplished = vec![
            r#"wrote "quoted" text"#.into(),
            "line\nnewline".into(),
            "unicode: \u{1F600}".into(),
        ];
        insert_checkpoint(&conn, &cp).expect("insert");
        let row = get_by_label(&conn, "json-special")
            .expect("get")
            .expect("some");
        assert_eq!(row.accomplished[0], r#"wrote "quoted" text"#);
        assert_eq!(row.accomplished[1], "line\nnewline");
    }

    // --- get_by_label -----------------------------------------------------

    #[test]
    #[cfg(feature = "sqlite")]
    fn get_by_label_returns_none_for_missing() {
        let conn = db();
        let result = get_by_label(&conn, "no-such-label").expect("get");
        assert!(result.is_none());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn get_by_label_returns_correct_row() {
        let conn = db();
        let mut cp = minimal("find-me");
        cp.session_number = Some(42);
        insert_checkpoint(&conn, &cp).expect("insert");
        let row = get_by_label(&conn, "find-me").expect("get").expect("some");
        assert_eq!(row.label, "find-me");
        assert_eq!(row.session_number, Some(42));
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn get_by_label_case_sensitive() {
        let conn = db();
        insert_checkpoint(&conn, &minimal("CaseSensitive")).expect("insert");
        assert!(get_by_label(&conn, "casesensitive").expect("get").is_none());
        assert!(get_by_label(&conn, "CaseSensitive").expect("get").is_some());
    }

    // --- label uniqueness -------------------------------------------------

    #[test]
    #[cfg(feature = "sqlite")]
    fn duplicate_label_errors() {
        let conn = db();
        insert_checkpoint(&conn, &minimal("dup-label")).expect("first insert");
        let result = insert_checkpoint(&conn, &minimal("dup-label"));
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn different_labels_both_succeed() {
        let conn = db();
        insert_checkpoint(&conn, &minimal("label-a")).expect("a");
        insert_checkpoint(&conn, &minimal("label-b")).expect("b");
        assert_eq!(count(&conn).expect("count"), 2);
    }

    // --- get_by_session ---------------------------------------------------

    #[test]
    #[cfg(feature = "sqlite")]
    fn get_by_session_returns_matching_rows() {
        let conn = db();
        for i in 0..3u32 {
            let mut cp = minimal(&format!("s109-{i}"));
            cp.session_number = Some(109);
            insert_checkpoint(&conn, &cp).expect("insert");
        }
        let mut other = minimal("s108-0");
        other.session_number = Some(108);
        insert_checkpoint(&conn, &other).expect("insert other");

        let rows = get_by_session(&conn, 109).expect("get");
        assert_eq!(rows.len(), 3);
        for row in &rows {
            assert_eq!(row.session_number, Some(109));
        }
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn get_by_session_returns_empty_for_missing_session() {
        let conn = db();
        insert_checkpoint(&conn, &minimal("s109-only")).expect("insert");
        let rows = get_by_session(&conn, 999).expect("get");
        assert!(rows.is_empty());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn get_by_session_ordered_by_timestamp_desc() {
        let conn = db();
        let timestamps = [
            ("s109-a", "2026-04-24T10:00:00Z"),
            ("s109-b", "2026-04-24T12:00:00Z"),
            ("s109-c", "2026-04-24T11:00:00Z"),
        ];
        for (label, ts) in &timestamps {
            let mut cp = CheckpointInsert::new(
                *label,
                *ts,
                11,
                vec![],
                vec![],
                vec![],
                vec![],
                "resume",
                "/tmp/f.md",
            );
            cp.session_number = Some(109);
            insert_checkpoint(&conn, &cp).expect("insert");
        }
        let rows = get_by_session(&conn, 109).expect("get");
        assert_eq!(rows[0].label, "s109-b");
        assert_eq!(rows[1].label, "s109-c");
        assert_eq!(rows[2].label, "s109-a");
    }

    // --- get_recent -------------------------------------------------------

    #[test]
    #[cfg(feature = "sqlite")]
    fn get_recent_returns_latest_first() {
        let conn = db();
        let labels_ts = [
            ("r-a", "2026-01-01T00:00:00Z"),
            ("r-b", "2026-03-01T00:00:00Z"),
            ("r-c", "2026-02-01T00:00:00Z"),
        ];
        for (label, ts) in &labels_ts {
            let cp = CheckpointInsert::new(*label, *ts, 11, vec![], vec![], vec![], vec![], "r", "/f");
            insert_checkpoint(&conn, &cp).expect("insert");
        }
        let rows = get_recent(&conn, 3).expect("get");
        assert_eq!(rows[0].label, "r-b");
        assert_eq!(rows[1].label, "r-c");
        assert_eq!(rows[2].label, "r-a");
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn get_recent_limit_honoured() {
        let conn = db();
        for i in 0..10u32 {
            insert_checkpoint(&conn, &minimal(&format!("lim-{i:02}"))).expect("insert");
        }
        let rows = get_recent(&conn, 3).expect("get");
        assert_eq!(rows.len(), 3);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn get_recent_zero_limit_returns_empty() {
        let conn = db();
        insert_checkpoint(&conn, &minimal("lim0")).expect("insert");
        let rows = get_recent(&conn, 0).expect("get");
        assert!(rows.is_empty());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn get_recent_on_empty_table_returns_empty() {
        let conn = db();
        let rows = get_recent(&conn, 5).expect("get");
        assert!(rows.is_empty());
    }

    // --- get_latest -------------------------------------------------------

    #[test]
    #[cfg(feature = "sqlite")]
    fn get_latest_returns_most_recent() {
        let conn = db();
        let labels_ts = [
            ("lat-a", "2026-01-01T00:00:00Z"),
            ("lat-b", "2026-04-24T00:00:00Z"),
            ("lat-c", "2026-02-15T00:00:00Z"),
        ];
        for (label, ts) in &labels_ts {
            let cp = CheckpointInsert::new(*label, *ts, 11, vec![], vec![], vec![], vec![], "r", "/f");
            insert_checkpoint(&conn, &cp).expect("insert");
        }
        let row = get_latest(&conn).expect("get").expect("some");
        assert_eq!(row.label, "lat-b");
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn get_latest_on_empty_table_returns_none() {
        let conn = db();
        assert!(get_latest(&conn).expect("get").is_none());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn get_latest_single_entry() {
        let conn = db();
        insert_checkpoint(&conn, &minimal("solo")).expect("insert");
        let row = get_latest(&conn).expect("get").expect("some");
        assert_eq!(row.label, "solo");
    }

    // --- count ------------------------------------------------------------

    #[test]
    #[cfg(feature = "sqlite")]
    fn count_empty_table_is_zero() {
        let conn = db();
        assert_eq!(count(&conn).expect("count"), 0);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn count_after_ten_inserts_is_ten() {
        let conn = db();
        for i in 0..10u32 {
            insert_checkpoint(&conn, &minimal(&format!("cnt-{i}"))).expect("insert");
        }
        assert_eq!(count(&conn).expect("count"), 10);
    }

    // --- consent values ---------------------------------------------------

    #[test]
    #[cfg(feature = "sqlite")]
    fn consent_store_stored_correctly() {
        let conn = db();
        let mut cp = minimal("consent-store");
        cp.consent = Some(Consent::Store);
        insert_checkpoint(&conn, &cp).expect("insert");
        let row = get_by_label(&conn, "consent-store")
            .expect("get")
            .expect("some");
        assert_eq!(row.consent, "Store");
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn consent_forget_stored_correctly() {
        let conn = db();
        let mut cp = minimal("consent-forget");
        cp.consent = Some(Consent::Forget);
        insert_checkpoint(&conn, &cp).expect("insert");
        let row = get_by_label(&conn, "consent-forget")
            .expect("get")
            .expect("some");
        assert_eq!(row.consent, "Forget");
    }

    // --- row field correctness --------------------------------------------

    #[test]
    #[cfg(feature = "sqlite")]
    fn resume_instructions_preserved() {
        let conn = db();
        let mut cp = minimal("resume-check");
        cp.resume_instructions =
            "## Next session\n1. Run cargo test\n2. Check `count()` returns 50".into();
        insert_checkpoint(&conn, &cp).expect("insert");
        let row = get_by_label(&conn, "resume-check")
            .expect("get")
            .expect("some");
        assert!(row.resume_instructions.contains("cargo test"));
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn source_file_preserved() {
        let conn = db();
        let mut cp = minimal("src-check");
        cp.source_file = "/home/user/projects/shared-context/s109.md".into();
        insert_checkpoint(&conn, &cp).expect("insert");
        let row = get_by_label(&conn, "src-check").expect("get").expect("some");
        assert_eq!(row.source_file, "/home/user/projects/shared-context/s109.md");
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn services_alive_preserved() {
        let conn = db();
        let mut cp = minimal("svc-alive");
        cp.services_alive = 9;
        insert_checkpoint(&conn, &cp).expect("insert");
        let row = get_by_label(&conn, "svc-alive").expect("get").expect("some");
        assert_eq!(row.services_alive, 9);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn services_total_override_respected() {
        let conn = db();
        let mut cp = minimal("svc-total");
        cp.services_total = Some(8);
        insert_checkpoint(&conn, &cp).expect("insert");
        let row = get_by_label(&conn, "svc-total").expect("get").expect("some");
        assert_eq!(row.services_total, 8);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn timestamp_utc_preserved() {
        let conn = db();
        let ts = "2026-04-24T13:37:42Z";
        let cp = CheckpointInsert::new("ts-check", ts, 11, vec![], vec![], vec![], vec![], "r", "/f");
        insert_checkpoint(&conn, &cp).expect("insert");
        let row = get_by_label(&conn, "ts-check").expect("get").expect("some");
        assert_eq!(row.timestamp_utc, ts);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn watcher_ready_true_stored_as_one() {
        let conn = db();
        let mut cp = minimal("watcher-t");
        cp.watcher_ready = Some(true);
        insert_checkpoint(&conn, &cp).expect("insert");
        let row = get_by_label(&conn, "watcher-t").expect("get").expect("some");
        assert_eq!(row.watcher_ready, 1);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn watcher_ready_false_stored_as_zero() {
        let conn = db();
        let mut cp = minimal("watcher-f");
        cp.watcher_ready = Some(false);
        insert_checkpoint(&conn, &cp).expect("insert");
        let row = get_by_label(&conn, "watcher-f").expect("get").expect("some");
        assert_eq!(row.watcher_ready, 0);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn optional_fields_are_none_when_unset() {
        let conn = db();
        insert_checkpoint(&conn, &minimal("optionals")).expect("insert");
        let row = get_by_label(&conn, "optionals").expect("get").expect("some");
        assert!(row.session_number.is_none());
        assert!(row.pane_id.is_none());
        assert!(row.tab.is_none());
        assert!(row.session_name.is_none());
        assert!(row.cwd.is_none());
        assert!(row.git_sha.is_none());
        assert!(row.git_branch.is_none());
        assert!(row.git_last_commit.is_none());
        assert!(row.services_alive_ports.is_none());
        assert!(row.watcher_reason.is_none());
        assert!(row.persona.is_none());
        assert!(row.scope_constraint.is_none());
        assert!(row.conversation_anchors.is_none());
    }

    // --- Consent type tests -----------------------------------------------

    #[test]
    #[cfg(feature = "sqlite")]
    fn consent_default_is_emit() {
        assert_eq!(Consent::default(), Consent::Emit);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn consent_as_str() {
        assert_eq!(Consent::Emit.as_str(), "Emit");
        assert_eq!(Consent::Store.as_str(), "Store");
        assert_eq!(Consent::Forget.as_str(), "Forget");
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn consent_from_str_valid() {
        use std::str::FromStr;
        assert_eq!(Consent::from_str("Emit").expect("emit"), Consent::Emit);
        assert_eq!(Consent::from_str("Store").expect("store"), Consent::Store);
        assert_eq!(Consent::from_str("Forget").expect("forget"), Consent::Forget);
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn consent_from_str_invalid_errors() {
        use std::str::FromStr;
        assert!(Consent::from_str("invalid").is_err());
        assert!(Consent::from_str("emit").is_err()); // case-sensitive
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn consent_display() {
        assert_eq!(Consent::Emit.to_string(), "Emit");
        assert_eq!(Consent::Store.to_string(), "Store");
        assert_eq!(Consent::Forget.to_string(), "Forget");
    }

    // --- CheckpointInsert::new() constructor ------------------------------

    #[test]
    #[cfg(feature = "sqlite")]
    fn new_constructor_sets_required_fields() {
        let cp = CheckpointInsert::new(
            "lbl",
            "2026-01-01",
            10,
            vec!["a".into()],
            vec!["b".into()],
            vec!["c".into()],
            vec!["d".into()],
            "resume",
            "/src.md",
        );
        assert_eq!(cp.label, "lbl");
        assert_eq!(cp.timestamp_utc, "2026-01-01");
        assert_eq!(cp.services_alive, 10);
        assert_eq!(cp.accomplished, vec!["a"]);
        assert_eq!(cp.in_progress, vec!["b"]);
        assert_eq!(cp.blocked, vec!["c"]);
        assert_eq!(cp.key_findings, vec!["d"]);
        assert_eq!(cp.resume_instructions, "resume");
        assert_eq!(cp.source_file, "/src.md");
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn new_constructor_optional_fields_default_to_none() {
        let cp = CheckpointInsert::new("lbl", "ts", 1, vec![], vec![], vec![], vec![], "r", "/f");
        assert!(cp.session_number.is_none());
        assert!(cp.pane_id.is_none());
        assert!(cp.consent.is_none());
    }

    // --- IDs are auto-increment and increasing ----------------------------

    #[test]
    #[cfg(feature = "sqlite")]
    fn ids_are_increasing() {
        let conn = db();
        let id1 = insert_checkpoint(&conn, &minimal("id-a")).expect("a");
        let id2 = insert_checkpoint(&conn, &minimal("id-b")).expect("b");
        let id3 = insert_checkpoint(&conn, &minimal("id-c")).expect("c");
        assert!(id1 < id2);
        assert!(id2 < id3);
    }

    // --- large vec serialisation ------------------------------------------

    #[test]
    #[cfg(feature = "sqlite")]
    fn large_accomplished_array_roundtrip() {
        let conn = db();
        let items: Vec<String> = (0..100).map(|i| format!("item {i}")).collect();
        let mut cp = minimal("large-arr");
        cp.accomplished = items.clone();
        insert_checkpoint(&conn, &cp).expect("insert");
        let row = get_by_label(&conn, "large-arr").expect("get").expect("some");
        assert_eq!(row.accomplished.len(), 100);
        assert_eq!(row.accomplished, items);
    }

    // --- get_by_session with no session_number set ------------------------

    #[test]
    #[cfg(feature = "sqlite")]
    fn get_by_session_ignores_rows_without_session_number() {
        let conn = db();
        // Insert row without session_number
        insert_checkpoint(&conn, &minimal("no-session")).expect("insert");
        // Should not appear for any session
        let rows = get_by_session(&conn, 109).expect("get");
        assert!(rows.is_empty());
    }
}
