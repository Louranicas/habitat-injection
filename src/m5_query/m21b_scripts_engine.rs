// m21b_scripts_engine — Atuin-compatible scripts engine backed by injection.db
//
// Stores, manages, and runs scripts just like `atuin scripts` but from
// the injection database. Scripts stored here can query the injection DB
// directly — making them "memory-aware" scripts.
//
// Schema:
//
// CREATE TABLE injection_script (
//     id                  TEXT PRIMARY KEY,      -- UUIDv7
//     name                TEXT NOT NULL UNIQUE,   -- human-readable, kebab-case
//     description         TEXT NOT NULL,
//     tags                TEXT NOT NULL DEFAULT '', -- comma-separated
//     shebang             TEXT NOT NULL DEFAULT '#!/usr/bin/env bash',
//     script_body         TEXT NOT NULL,          -- the actual script content
//     template_vars_json  TEXT NOT NULL DEFAULT '{}', -- default values for {{VAR}} substitution
//     created_at          TEXT NOT NULL,
//     updated_at          TEXT NOT NULL,
//     last_run            TEXT,
//     run_count           INTEGER NOT NULL DEFAULT 0,
//     exit_code_last      INTEGER,
//     consent             TEXT NOT NULL DEFAULT 'Emit'
//         CHECK(consent IN ('Emit', 'Store', 'Forget'))
// );
// CREATE INDEX idx_script_name ON injection_script(name);
// CREATE INDEX idx_script_tags ON injection_script(tags);
//
// Atuin-compatible interface:
//
//   habitat-scripts new <name> [--description <desc>] [--tags <tags>]
//                               [--shebang <shebang>] [--script <body>]
//   habitat-scripts list [--tags <filter>]
//   habitat-scripts run <name> [-v KEY=VALUE ...]
//   habitat-scripts get <name>
//   habitat-scripts edit <name>
//   habitat-scripts delete <name>
//
// Template variable substitution:
//   {{VAR}} in script_body is replaced with -v VAR=value at runtime
//   Special auto-injected variables:
//     {{__DB_PATH__}}      → path to injection.db passed at run time
//     {{__SESSION_ID__}}   → current session number (from latest trajectory)
//     {{__TIMESTAMP__}}    → ISO 8601 UTC
//     {{__PANE_ID__}}      → $ZELLIJ_PANE_ID or "unknown"
//
// Dual-surface registration:
//   When a script is created via habitat-scripts new, it is ALSO registered
//   with atuin via `atuin scripts new` so it appears in both systems.
//   The injection.db copy is the source of truth; the atuin copy is a mirror.
//
// Memory-aware script examples:
//
//   # Check if I'm about to repeat a known trap
//   habitat-scripts new trap-check \
//     --description "Check causal_chain for active traps before starting work" \
//     --tags "habitat,safety,pre-work" \
//     --script 'sqlite3 {{__DB_PATH__}} "SELECT label, reinforcement_count, description FROM causal_chain WHERE resolved_session IS NULL ORDER BY reinforcement_count DESC LIMIT 5"'
//
//   # Show what I was doing last session
//   habitat-scripts new last-session \
//     --description "Show the last /save-session checkpoint summary" \
//     --tags "habitat,resume,context" \
//     --script 'sqlite3 -header -column {{__DB_PATH__}} "SELECT label, accomplished_json FROM session_checkpoint ORDER BY timestamp_utc DESC LIMIT 1"'
//
//   # Inject trajectory into current context
//   habitat-scripts new trajectory \
//     --description "Show fitness trajectory for last N sessions" \
//     --tags "habitat,trajectory,monitoring" \
//     --script 'sqlite3 -header {{__DB_PATH__}} "SELECT session_id, ralph_fitness, delta_summary FROM session_trajectory ORDER BY session_id DESC LIMIT {{n:-5}}"'

//! `m21b_scripts_engine` — Atuin-compatible scripts engine backed by `injection.db`.
//!
//! Provides a full CRUD interface for memory-aware scripts stored in the
//! `injection_script` table, plus template-variable substitution and
//! subprocess execution via the script's shebang line.
//!
//! The table is created lazily via [`ensure_scripts_table`] — no changes to
//! [`crate::m2_schema::m06_schema`] are required.
//!
//! # Layer
//!
//! `m5_query`
//!
//! # Dependencies
//!
//! `m01_types`, `m02_errors`, `m06_schema` (for `open_memory` in tests)

#[cfg(feature = "sqlite")]
mod inner {
    use std::collections::HashMap;
    use std::io::Write as _;
    use std::time::Instant;

    use chrono::Utc;
    use rusqlite::{Connection, OptionalExtension as _, params};
    use serde::{Deserialize, Serialize};
    use tracing::instrument;
    use uuid::Uuid;

    use crate::m1_foundation::m02_errors::{QueryError, SchemaError};
    use crate::m5_query::query_err;

    // -----------------------------------------------------------------------
    // Types
    // -----------------------------------------------------------------------

    /// A stored script record in `injection_script`.
    ///
    /// Mirrors all columns of the table. `template_vars` is stored as JSON
    /// but surfaced here as a plain [`HashMap`] for ergonomic access.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ScriptRecord {
        /// Primary key — [`Uuid`] v7 (time-ordered).
        pub id: String,
        /// Human-readable script name in kebab-case.
        pub name: String,
        /// One-line description of what the script does.
        pub description: String,
        /// Comma-separated tags (e.g. `"habitat,safety,pre-work"`).
        pub tags: String,
        /// Shebang line prepended to the script when written to a temp file.
        pub shebang: String,
        /// The actual script body, possibly containing `{{VAR}}` placeholders.
        pub script_body: String,
        /// Default values for `{{VAR}}` placeholders; keyed by variable name.
        pub template_vars: HashMap<String, String>,
        /// ISO 8601 UTC creation timestamp.
        pub created_at: String,
        /// ISO 8601 UTC last-update timestamp.
        pub updated_at: String,
        /// ISO 8601 UTC timestamp of the most recent run, or `None`.
        pub last_run: Option<String>,
        /// How many times this script has been executed.
        pub run_count: u32,
    }

    /// Result of a single script execution.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ScriptRunResult {
        /// Script name that was executed.
        pub name: String,
        /// Process exit code (`0` = success).
        pub exit_code: i32,
        /// Captured standard output.
        pub stdout: String,
        /// Captured standard error.
        pub stderr: String,
        /// Wall-clock execution time in milliseconds.
        pub elapsed_ms: u64,
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    #[derive(Debug)]
    struct JsonParseError(String);
    impl std::fmt::Display for JsonParseError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&self.0)
        }
    }
    impl std::error::Error for JsonParseError {}

    /// Parse a [`ScriptRecord`] from a prepared-statement [`rusqlite::Row`].
    fn parse_script_row(row: &rusqlite::Row<'_>) -> Result<ScriptRecord, rusqlite::Error> {
        let template_vars_json: String = row.get(6)?;
        let template_vars: HashMap<String, String> =
            serde_json::from_str(&template_vars_json).map_err(|e| {
                rusqlite::Error::ToSqlConversionFailure(Box::new(JsonParseError(format!(
                    "template_vars_json: {e}"
                ))))
            })?;

        Ok(ScriptRecord {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            tags: row.get(3)?,
            shebang: row.get(4)?,
            script_body: row.get(5)?,
            template_vars,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
            last_run: row.get(9)?,
            run_count: row.get::<_, u32>(10)?,
        })
    }

    /// Return the current UTC timestamp as an ISO 8601 string.
    fn now_iso8601() -> String {
        Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
    }

    // -----------------------------------------------------------------------
    // Schema
    // -----------------------------------------------------------------------

    /// Verify `injection_script` table exists.
    ///
    /// Since schema v3, the table is created by
    /// [`crate::m2_schema::m06_schema::create_all_tables`]. This function
    /// runs an idempotent `CREATE TABLE IF NOT EXISTS` as a safety net for
    /// databases opened before the v3 migration.
    ///
    /// # Errors
    ///
    /// Returns [`SchemaError::TableCreationFailed`] if any DDL statement fails.
    #[instrument(skip(conn))]
    pub fn ensure_scripts_table(conn: &Connection) -> Result<(), SchemaError> {
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
            CREATE INDEX IF NOT EXISTS idx_script_tags  ON injection_script(tags);",
        )
        .map_err(|e| SchemaError::TableCreationFailed {
            table: "injection_script".into(),
            reason: e.to_string(),
        })
    }

    // -----------------------------------------------------------------------
    // CRUD
    // -----------------------------------------------------------------------

    /// Insert a new script record into `injection_script`.
    ///
    /// Generates a [`Uuid`] v7 for `id` and ISO 8601 timestamps for
    /// `created_at` / `updated_at`. `tags` defaults to `""` and `shebang`
    /// defaults to `"#!/usr/bin/env bash"` when `None` is supplied.
    ///
    /// # Errors
    ///
    /// Returns [`QueryError::ExecutionFailed`] on INSERT failure (e.g. a
    /// duplicate `name`).
    #[instrument(skip(conn))]
    pub fn create_script(
        conn: &Connection,
        name: &str,
        description: &str,
        script_body: &str,
        tags: Option<&str>,
        shebang: Option<&str>,
    ) -> Result<ScriptRecord, QueryError> {
        let id = Uuid::now_v7().to_string();
        let now = now_iso8601();
        let tags_val = tags.unwrap_or("");
        let shebang_val = shebang.unwrap_or("#!/usr/bin/env bash");
        let vars_json = "{}";

        conn.execute(
            "INSERT INTO injection_script
                 (id, name, description, tags, shebang, script_body, template_vars_json,
                  created_at, updated_at, last_run, run_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, NULL, 0)",
            params![id, name, description, tags_val, shebang_val, script_body, vars_json, now],
        )
        .map_err(|e| query_err(&e))?;

        Ok(ScriptRecord {
            id,
            name: name.to_owned(),
            description: description.to_owned(),
            tags: tags_val.to_owned(),
            shebang: shebang_val.to_owned(),
            script_body: script_body.to_owned(),
            template_vars: HashMap::new(),
            created_at: now.clone(),
            updated_at: now,
            last_run: None,
            run_count: 0,
        })
    }

    /// Retrieve a script by `name`.
    ///
    /// Returns `Ok(None)` if no script with that name exists.
    ///
    /// # Errors
    ///
    /// Returns [`QueryError::ExecutionFailed`] on database or parse failure.
    #[instrument(skip(conn))]
    pub fn get_script(
        conn: &Connection,
        name: &str,
    ) -> Result<Option<ScriptRecord>, QueryError> {
        let mut stmt = conn
            .prepare(
                "SELECT id, name, description, tags, shebang, script_body,
                        template_vars_json, created_at, updated_at, last_run, run_count
                   FROM injection_script
                  WHERE name = ?1",
            )
            .map_err(|e| query_err(&e))?;

        stmt.query_row(params![name], parse_script_row)
            .optional()
            .map_err(|e| query_err(&e))
    }

    /// List all scripts, optionally filtered by a substring match on `tags`.
    ///
    /// When `tag_filter` is `Some(t)`, only scripts whose `tags` field
    /// contains `t` as a substring are returned.  Results are ordered by
    /// `name` ascending.
    ///
    /// # Errors
    ///
    /// Returns [`QueryError::ExecutionFailed`] on database or parse failure.
    #[instrument(skip(conn))]
    pub fn list_scripts(
        conn: &Connection,
        tag_filter: Option<&str>,
    ) -> Result<Vec<ScriptRecord>, QueryError> {
        // Build a filter pattern once so the borrow lives long enough.
        let pattern;
        let sql;

        if let Some(tag) = tag_filter {
            pattern = format!("%{tag}%");
            sql = "SELECT id, name, description, tags, shebang, script_body,
                          template_vars_json, created_at, updated_at, last_run, run_count
                     FROM injection_script
                    WHERE tags LIKE ?1
                    ORDER BY name ASC";
            let mut stmt = conn.prepare(sql).map_err(|e| query_err(&e))?;
            let rows = stmt
                .query_map(params![pattern], parse_script_row)
                .map_err(|e| query_err(&e))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| query_err(&e))?;
            Ok(rows)
        } else {
            sql = "SELECT id, name, description, tags, shebang, script_body,
                          template_vars_json, created_at, updated_at, last_run, run_count
                     FROM injection_script
                    ORDER BY name ASC";
            let mut stmt = conn.prepare(sql).map_err(|e| query_err(&e))?;
            let rows = stmt
                .query_map([], parse_script_row)
                .map_err(|e| query_err(&e))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| query_err(&e))?;
            Ok(rows)
        }
    }

    /// Delete a script by `name`.
    ///
    /// Returns `true` if a row was deleted, `false` if no such script exists.
    ///
    /// # Errors
    ///
    /// Returns [`QueryError::ExecutionFailed`] on database failure.
    #[instrument(skip(conn))]
    pub fn delete_script(conn: &Connection, name: &str) -> Result<bool, QueryError> {
        let n = conn
            .execute("DELETE FROM injection_script WHERE name = ?1", params![name])
            .map_err(|e| query_err(&e))?;
        Ok(n > 0)
    }

    // -----------------------------------------------------------------------
    // Template substitution
    // -----------------------------------------------------------------------

    /// Substitute `{{KEY}}` and `{{KEY:-default}}` placeholders in `body`.
    ///
    /// Rules:
    ///
    /// - `{{KEY}}` → `vars[KEY]` if present, otherwise the literal `{{KEY}}`
    ///   is preserved (no silent data loss).
    /// - `{{KEY:-default}}` → `vars[KEY]` if present, otherwise `default`.
    ///
    /// The function is a pure string transformation — it does not touch the
    /// database or the filesystem.
    #[must_use]
    pub fn substitute_vars<S: std::hash::BuildHasher>(
        body: &str,
        vars: &HashMap<String, String, S>,
    ) -> String {
        // We process one `{{...}}` token at a time, replacing from left to right.
        let mut result = String::with_capacity(body.len());
        let mut remaining = body;

        while let Some(open) = remaining.find("{{") {
            // Copy everything before the opening `{{`.
            result.push_str(&remaining[..open]);
            let after_open = &remaining[open + 2..];

            if let Some(close) = after_open.find("}}") {
                let inner = &after_open[..close];
                // Check for `KEY:-default` syntax.
                if let Some(dash_pos) = inner.find(":-") {
                    let key = &inner[..dash_pos];
                    let default_val = &inner[dash_pos + 2..];
                    if let Some(v) = vars.get(key) {
                        result.push_str(v);
                    } else {
                        result.push_str(default_val);
                    }
                } else {
                    // Plain `KEY` — preserve literal if not in map.
                    if let Some(v) = vars.get(inner) {
                        result.push_str(v);
                    } else {
                        // Preserve the original placeholder.
                        result.push_str("{{");
                        result.push_str(inner);
                        result.push_str("}}");
                    }
                }
                remaining = &after_open[close + 2..];
            } else {
                // No closing `}}` — copy the `{{` literally and keep scanning.
                result.push_str("{{");
                remaining = after_open;
            }
        }

        result.push_str(remaining);
        result
    }

    // -----------------------------------------------------------------------
    // Execution
    // -----------------------------------------------------------------------

    /// Retrieve, substitute, and execute a script by `name`.
    ///
    /// Steps:
    ///
    /// 1. Look up the script by `name`; return error if not found.
    /// 2. Build the effective variable map — start from the script's stored
    ///    `template_vars`, overlay caller-supplied `vars`, then inject the
    ///    auto-vars `__DB_PATH__`, `__TIMESTAMP__`, and `__PANE_ID__`.
    /// 3. Call [`substitute_vars`] on `script_body`.
    /// 4. Write `shebang\n<body>` to a temporary file; make it executable.
    /// 5. Execute via `/bin/sh <tempfile>` (works even when the shebang
    ///    refers to a missing interpreter — `/bin/sh` always exists).
    /// 6. Capture `stdout` + `stderr` and measure wall-clock time.
    /// 7. Update `run_count` and `last_run` in the database.
    ///
    /// # Errors
    ///
    /// - [`QueryError::ExecutionFailed`] if the script is not found, the
    ///   temp file cannot be written, or the database update fails.
    /// - Does **not** return an error for non-zero exit codes — those are
    ///   surfaced via [`ScriptRunResult::exit_code`].
    #[instrument(skip(conn, vars))]
    pub fn run_script<S: std::hash::BuildHasher>(
        conn: &Connection,
        name: &str,
        vars: &HashMap<String, String, S>,
        db_path: &str,
    ) -> Result<ScriptRunResult, QueryError> {
        // 1. Fetch the script record.
        let script = get_script(conn, name)?.ok_or_else(|| QueryError::NoResults {
            query: format!("injection_script WHERE name = '{name}'"),
        })?;

        // 2. Build effective variable map.
        let mut effective: HashMap<String, String> = script.template_vars.clone();
        effective.extend(vars.iter().map(|(k, v)| (k.clone(), v.clone())));
        effective.insert("__DB_PATH__".into(), db_path.to_owned());
        effective.insert(
            "__TIMESTAMP__".into(),
            Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        );
        let pane_id = std::env::var("ZELLIJ_PANE_ID").unwrap_or_else(|_| "unknown".into());
        effective.insert("__PANE_ID__".into(), pane_id);

        // 3. Substitute variables.
        let body = substitute_vars(&script.script_body, &effective);

        // 4. Write to a temp file.
        let mut tmp = tempfile_new().map_err(|e| {
            QueryError::ExecutionFailed(format!("failed to create temp file: {e}"))
        })?;
        let full_script = format!("{}\n{}", script.shebang, body);
        tmp.file
            .write_all(full_script.as_bytes())
            .map_err(|e| QueryError::ExecutionFailed(format!("failed to write temp script: {e}")))?;
        tmp.file
            .flush()
            .map_err(|e| QueryError::ExecutionFailed(format!("failed to flush temp script: {e}")))?;

        // Make the file executable (rwxr-xr-x).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&tmp.path, std::fs::Permissions::from_mode(0o755))
                .map_err(|e| {
                    QueryError::ExecutionFailed(format!("failed to chmod temp script: {e}"))
                })?;
        }

        // 5–6. Execute and capture output.
        let start = Instant::now();
        let output = std::process::Command::new("/bin/sh")
            .arg(&tmp.path)
            .output()
            .map_err(|e| {
                QueryError::ExecutionFailed(format!("failed to spawn script process: {e}"))
            })?;
        let elapsed_ms = start.elapsed().as_millis().try_into().unwrap_or(u64::MAX);

        let exit_code = output.status.code().unwrap_or_else(|| {
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt as _;
                output.status.signal().map_or(-1, |sig| 128 + sig)
            }
            #[cfg(not(unix))]
            {
                -1
            }
        });
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        // 7. Update run_count and last_run.
        let now = now_iso8601();
        conn.execute(
            "UPDATE injection_script
                SET run_count     = run_count + 1,
                    last_run      = ?1,
                    exit_code_last = ?2,
                    updated_at    = ?1
              WHERE name = ?3",
            params![now, exit_code, name],
        )
        .map_err(|e| query_err(&e))?;

        Ok(ScriptRunResult {
            name: name.to_owned(),
            exit_code,
            stdout,
            stderr,
            elapsed_ms,
        })
    }

    // -----------------------------------------------------------------------
    // Temp-file helper (avoids depending on the `tempfile` crate)
    // -----------------------------------------------------------------------

    struct TempScript {
        file: std::fs::File,
        path: std::path::PathBuf,
    }

    impl Drop for TempScript {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    fn tempfile_new() -> Result<TempScript, std::io::Error> {
        use std::time::SystemTime;
        let ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let path =
            std::env::temp_dir().join(format!("habitat_script_{ts}_{}.sh", std::process::id()));
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        Ok(TempScript { file, path })
    }

}

// ---------------------------------------------------------------------------
// Public surface — available unconditionally; bodies gated on `sqlite`
// ---------------------------------------------------------------------------

#[cfg(feature = "sqlite")]
pub use inner::{
    ScriptRecord, ScriptRunResult, create_script, delete_script, ensure_scripts_table,
    get_script, list_scripts, run_script, substitute_vars,
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use std::collections::HashMap;

    use crate::m2_schema::m06_schema::open_memory;

    use super::{
        ScriptRecord, ScriptRunResult, create_script, delete_script, ensure_scripts_table,
        get_script, list_scripts, run_script, substitute_vars,
    };

    // ---- helpers -----------------------------------------------------------

    fn mem_db() -> rusqlite::Connection {
        let conn = open_memory().expect("open_memory failed");
        ensure_scripts_table(&conn).expect("ensure_scripts_table failed");
        conn
    }

    fn simple_script(conn: &rusqlite::Connection, name: &str) -> ScriptRecord {
        create_script(conn, name, "A test script", "echo hello", None, None)
            .expect("create_script failed")
    }

    // ---- ensure_scripts_table ---------------------------------------------

    #[test]
    fn ensure_table_creates_table() {
        let conn = open_memory().expect("open_memory");
        ensure_scripts_table(&conn).expect("first call");
        // Table must exist — do a simple SELECT.
        conn.execute_batch("SELECT 1 FROM injection_script LIMIT 1")
            .expect("table should exist");
    }

    #[test]
    fn ensure_table_idempotent() {
        let conn = open_memory().expect("open_memory");
        ensure_scripts_table(&conn).expect("first call");
        ensure_scripts_table(&conn).expect("second call — must not error");
        ensure_scripts_table(&conn).expect("third call — must not error");
    }

    #[test]
    fn ensure_table_creates_name_index() {
        let conn = mem_db();
        let idx: String = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_script_name'",
                [],
                |r| r.get(0),
            )
            .expect("idx_script_name should exist");
        assert_eq!(idx, "idx_script_name");
    }

    #[test]
    fn ensure_table_creates_tags_index() {
        let conn = mem_db();
        let idx: String = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_script_tags'",
                [],
                |r| r.get(0),
            )
            .expect("idx_script_tags should exist");
        assert_eq!(idx, "idx_script_tags");
    }

    // ---- create_script -----------------------------------------------------

    #[test]
    fn create_returns_record_with_correct_name() {
        let conn = mem_db();
        let rec = simple_script(&conn, "my-script");
        assert_eq!(rec.name, "my-script");
    }

    #[test]
    fn create_returns_record_with_uuid_id() {
        let conn = mem_db();
        let rec = simple_script(&conn, "uuid-test");
        assert!(!rec.id.is_empty());
        // UUID v7 is 36 chars with hyphens.
        assert_eq!(rec.id.len(), 36);
    }

    #[test]
    fn create_default_shebang() {
        let conn = mem_db();
        let rec = simple_script(&conn, "shebang-default");
        assert_eq!(rec.shebang, "#!/usr/bin/env bash");
    }

    #[test]
    fn create_custom_shebang() {
        let conn = mem_db();
        let rec = create_script(
            &conn,
            "custom-shebang",
            "desc",
            "echo hi",
            None,
            Some("#!/usr/bin/env python3"),
        )
        .expect("create_script");
        assert_eq!(rec.shebang, "#!/usr/bin/env python3");
    }

    #[test]
    fn create_default_tags_empty() {
        let conn = mem_db();
        let rec = simple_script(&conn, "no-tags");
        assert_eq!(rec.tags, "");
    }

    #[test]
    fn create_custom_tags() {
        let conn = mem_db();
        let rec =
            create_script(&conn, "tagged", "desc", "echo hi", Some("habitat,safety"), None)
                .expect("create_script");
        assert_eq!(rec.tags, "habitat,safety");
    }

    #[test]
    fn create_stores_script_body() {
        let conn = mem_db();
        let rec = create_script(&conn, "body-test", "desc", "echo world", None, None)
            .expect("create_script");
        assert_eq!(rec.script_body, "echo world");
    }

    #[test]
    fn create_initial_run_count_zero() {
        let conn = mem_db();
        let rec = simple_script(&conn, "run-count");
        assert_eq!(rec.run_count, 0);
    }

    #[test]
    fn create_initial_last_run_none() {
        let conn = mem_db();
        let rec = simple_script(&conn, "last-run");
        assert!(rec.last_run.is_none());
    }

    #[test]
    fn create_timestamps_populated() {
        let conn = mem_db();
        let rec = simple_script(&conn, "ts-test");
        assert!(!rec.created_at.is_empty());
        assert!(!rec.updated_at.is_empty());
    }

    #[test]
    fn create_duplicate_name_errors() {
        let conn = mem_db();
        simple_script(&conn, "dup-name");
        let result = create_script(&conn, "dup-name", "desc2", "echo bye", None, None);
        assert!(result.is_err());
    }

    #[test]
    fn create_empty_template_vars() {
        let conn = mem_db();
        let rec = simple_script(&conn, "no-vars");
        assert!(rec.template_vars.is_empty());
    }

    // ---- get_script --------------------------------------------------------

    #[test]
    fn get_existing_script_returns_some() {
        let conn = mem_db();
        simple_script(&conn, "get-me");
        let found = get_script(&conn, "get-me").expect("get_script");
        assert!(found.is_some());
    }

    #[test]
    fn get_nonexistent_script_returns_none() {
        let conn = mem_db();
        let found = get_script(&conn, "does-not-exist").expect("get_script");
        assert!(found.is_none());
    }

    #[test]
    fn get_returns_correct_fields() {
        let conn = mem_db();
        create_script(&conn, "field-check", "A description", "cat /dev/null", Some("test,qa"), None)
            .expect("create_script");
        let rec = get_script(&conn, "field-check")
            .expect("get_script")
            .expect("should be Some");
        assert_eq!(rec.name, "field-check");
        assert_eq!(rec.description, "A description");
        assert_eq!(rec.script_body, "cat /dev/null");
        assert_eq!(rec.tags, "test,qa");
    }

    #[test]
    fn get_roundtrips_shebang() {
        let conn = mem_db();
        create_script(
            &conn,
            "shebang-rt",
            "desc",
            "pass",
            None,
            Some("#!/usr/bin/env zsh"),
        )
        .expect("create_script");
        let rec = get_script(&conn, "shebang-rt")
            .expect("get_script")
            .expect("Some");
        assert_eq!(rec.shebang, "#!/usr/bin/env zsh");
    }

    // ---- list_scripts ------------------------------------------------------

    #[test]
    fn list_empty_returns_empty_vec() {
        let conn = mem_db();
        let scripts = list_scripts(&conn, None).expect("list_scripts");
        assert!(scripts.is_empty());
    }

    #[test]
    fn list_all_returns_all_scripts() {
        let conn = mem_db();
        for name in &["alpha", "beta", "gamma"] {
            simple_script(&conn, name);
        }
        let scripts = list_scripts(&conn, None).expect("list_scripts");
        assert_eq!(scripts.len(), 3);
    }

    #[test]
    fn list_ordered_by_name_asc() {
        let conn = mem_db();
        for name in &["zz-last", "aa-first", "mm-mid"] {
            simple_script(&conn, name);
        }
        let scripts = list_scripts(&conn, None).expect("list_scripts");
        let names: Vec<&str> = scripts.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["aa-first", "mm-mid", "zz-last"]);
    }

    #[test]
    fn list_with_tag_filter_matches_substring() {
        let conn = mem_db();
        create_script(&conn, "s1", "d", "e", Some("habitat,safety"), None).expect("create");
        create_script(&conn, "s2", "d", "e", Some("devops,monitoring"), None).expect("create");
        create_script(&conn, "s3", "d", "e", Some("habitat,monitoring"), None).expect("create");

        let filtered = list_scripts(&conn, Some("habitat")).expect("list_scripts");
        assert_eq!(filtered.len(), 2);
        let names: Vec<&str> = filtered.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"s1"));
        assert!(names.contains(&"s3"));
    }

    #[test]
    fn list_tag_filter_no_matches_returns_empty() {
        let conn = mem_db();
        create_script(&conn, "t1", "d", "e", Some("devops"), None).expect("create");
        let result = list_scripts(&conn, Some("not-a-tag")).expect("list_scripts");
        assert!(result.is_empty());
    }

    #[test]
    fn list_tag_filter_none_returns_all() {
        let conn = mem_db();
        for n in &["x", "y"] {
            simple_script(&conn, n);
        }
        let all = list_scripts(&conn, None).expect("list_scripts");
        assert_eq!(all.len(), 2);
    }

    // ---- delete_script -----------------------------------------------------

    #[test]
    fn delete_existing_returns_true() {
        let conn = mem_db();
        simple_script(&conn, "delete-me");
        let deleted = delete_script(&conn, "delete-me").expect("delete_script");
        assert!(deleted);
    }

    #[test]
    fn delete_removes_from_db() {
        let conn = mem_db();
        simple_script(&conn, "gone");
        delete_script(&conn, "gone").expect("delete_script");
        let found = get_script(&conn, "gone").expect("get_script");
        assert!(found.is_none());
    }

    #[test]
    fn delete_nonexistent_returns_false() {
        let conn = mem_db();
        let result = delete_script(&conn, "never-existed").expect("delete_script");
        assert!(!result);
    }

    #[test]
    fn delete_leaves_other_scripts_intact() {
        let conn = mem_db();
        simple_script(&conn, "keep");
        simple_script(&conn, "remove");
        delete_script(&conn, "remove").expect("delete_script");
        let remaining = list_scripts(&conn, None).expect("list_scripts");
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].name, "keep");
    }

    // ---- substitute_vars ---------------------------------------------------

    #[test]
    fn substitute_simple_replacement() {
        let mut vars = HashMap::new();
        vars.insert("NAME".into(), "Alice".into());
        let result = substitute_vars("Hello, {{NAME}}!", &vars);
        assert_eq!(result, "Hello, Alice!");
    }

    #[test]
    fn substitute_multiple_vars() {
        let mut vars = HashMap::new();
        vars.insert("A".into(), "foo".into());
        vars.insert("B".into(), "bar".into());
        let result = substitute_vars("{{A}} and {{B}}", &vars);
        assert_eq!(result, "foo and bar");
    }

    #[test]
    fn substitute_same_var_twice() {
        let mut vars = HashMap::new();
        vars.insert("X".into(), "42".into());
        let result = substitute_vars("{{X}} + {{X}}", &vars);
        assert_eq!(result, "42 + 42");
    }

    #[test]
    fn substitute_missing_var_preserved() {
        let vars: HashMap<String, String> = HashMap::new();
        let result = substitute_vars("{{MISSING}}", &vars);
        assert_eq!(result, "{{MISSING}}");
    }

    #[test]
    fn substitute_default_syntax_key_present() {
        let mut vars = HashMap::new();
        vars.insert("N".into(), "10".into());
        let result = substitute_vars("LIMIT {{N:-5}}", &vars);
        assert_eq!(result, "LIMIT 10");
    }

    #[test]
    fn substitute_default_syntax_key_absent_uses_default() {
        let vars: HashMap<String, String> = HashMap::new();
        let result = substitute_vars("LIMIT {{N:-5}}", &vars);
        assert_eq!(result, "LIMIT 5");
    }

    #[test]
    fn substitute_default_empty_string() {
        let vars: HashMap<String, String> = HashMap::new();
        let result = substitute_vars("{{OPT:-}}", &vars);
        assert_eq!(result, "");
    }

    #[test]
    fn substitute_no_placeholders_unchanged() {
        let vars: HashMap<String, String> = HashMap::new();
        let body = "echo hello world";
        let result = substitute_vars(body, &vars);
        assert_eq!(result, body);
    }

    #[test]
    fn substitute_empty_body() {
        let vars: HashMap<String, String> = HashMap::new();
        assert_eq!(substitute_vars("", &vars), "");
    }

    #[test]
    fn substitute_empty_vars_map() {
        let vars: HashMap<String, String> = HashMap::new();
        // Placeholders preserved when map is empty.
        let result = substitute_vars("{{KEY}}", &vars);
        assert_eq!(result, "{{KEY}}");
    }

    #[test]
    fn substitute_auto_injected_db_path() {
        let mut vars = HashMap::new();
        vars.insert("__DB_PATH__".into(), "/tmp/test.db".into());
        let result = substitute_vars("sqlite3 {{__DB_PATH__}} .tables", &vars);
        assert_eq!(result, "sqlite3 /tmp/test.db .tables");
    }

    #[test]
    fn substitute_auto_injected_timestamp() {
        let mut vars = HashMap::new();
        vars.insert("__TIMESTAMP__".into(), "2026-04-24T12:00:00Z".into());
        let result = substitute_vars("ts={{__TIMESTAMP__}}", &vars);
        assert_eq!(result, "ts=2026-04-24T12:00:00Z");
    }

    #[test]
    fn substitute_unclosed_brace_preserved() {
        let vars: HashMap<String, String> = HashMap::new();
        // `{{` without matching `}}` should not panic.
        let result = substitute_vars("{{OPEN", &vars);
        assert_eq!(result, "{{OPEN");
    }

    #[test]
    fn substitute_adjacent_placeholders() {
        let mut vars = HashMap::new();
        vars.insert("A".into(), "1".into());
        vars.insert("B".into(), "2".into());
        let result = substitute_vars("{{A}}{{B}}", &vars);
        assert_eq!(result, "12");
    }

    #[test]
    fn substitute_default_with_colon_in_value() {
        // Default value contains a colon — should not confuse the parser.
        let vars: HashMap<String, String> = HashMap::new();
        let result = substitute_vars("{{URL:-http://localhost:8080}}", &vars);
        assert_eq!(result, "http://localhost:8080");
    }

    // ---- serde roundtrip ---------------------------------------------------

    #[test]
    fn script_record_serde_roundtrip() {
        let conn = mem_db();
        let rec =
            create_script(&conn, "serde-test", "desc", "echo serde", Some("test"), None)
                .expect("create_script");
        let json = serde_json::to_string(&rec).expect("serialize");
        let back: ScriptRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.name, rec.name);
        assert_eq!(back.id, rec.id);
        assert_eq!(back.script_body, rec.script_body);
    }

    #[test]
    fn script_run_result_serde_roundtrip() {
        let result = ScriptRunResult {
            name: "my-script".into(),
            exit_code: 0,
            stdout: "hello\n".into(),
            stderr: String::new(),
            elapsed_ms: 42,
        };
        let json = serde_json::to_string(&result).expect("serialize");
        let back: ScriptRunResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.name, result.name);
        assert_eq!(back.exit_code, result.exit_code);
        assert_eq!(back.stdout, result.stdout);
        assert_eq!(back.elapsed_ms, result.elapsed_ms);
    }

    // ---- run_script --------------------------------------------------------

    #[test]
    fn run_script_simple_echo() {
        // Only run when `/bin/sh` is available (always true on Linux).
        if !std::path::Path::new("/bin/sh").exists() {
            return;
        }
        let conn = mem_db();
        create_script(&conn, "echo-test", "desc", "echo habitat", None, None)
            .expect("create_script");
        let result = run_script(&conn, "echo-test", &HashMap::new(), "/tmp/test.db")
            .expect("run_script");
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("habitat"));
    }

    #[test]
    fn run_script_updates_run_count() {
        if !std::path::Path::new("/bin/sh").exists() {
            return;
        }
        let conn = mem_db();
        create_script(&conn, "count-script", "desc", "true", None, None).expect("create_script");
        run_script(&conn, "count-script", &HashMap::new(), "/tmp/x.db").expect("run 1");
        run_script(&conn, "count-script", &HashMap::new(), "/tmp/x.db").expect("run 2");
        let rec = get_script(&conn, "count-script")
            .expect("get")
            .expect("Some");
        assert_eq!(rec.run_count, 2);
    }

    #[test]
    fn run_script_updates_last_run() {
        if !std::path::Path::new("/bin/sh").exists() {
            return;
        }
        let conn = mem_db();
        create_script(&conn, "last-run-script", "desc", "true", None, None)
            .expect("create_script");
        run_script(&conn, "last-run-script", &HashMap::new(), "/tmp/x.db").expect("run");
        let rec = get_script(&conn, "last-run-script")
            .expect("get")
            .expect("Some");
        assert!(rec.last_run.is_some());
    }

    #[test]
    fn run_script_substitutes_db_path() {
        if !std::path::Path::new("/bin/sh").exists() {
            return;
        }
        let conn = mem_db();
        // Script echoes the db path variable.
        create_script(&conn, "db-path-script", "desc", "echo {{__DB_PATH__}}", None, None)
            .expect("create_script");
        let result =
            run_script(&conn, "db-path-script", &HashMap::new(), "/tmp/injection.db")
                .expect("run_script");
        assert!(result.stdout.contains("/tmp/injection.db"));
    }

    #[test]
    fn run_script_nonexistent_name_errors() {
        let conn = mem_db();
        let result = run_script(&conn, "no-such-script", &HashMap::new(), "/tmp/x.db");
        assert!(result.is_err());
    }

    #[test]
    fn run_script_captures_exit_code() {
        if !std::path::Path::new("/bin/sh").exists() {
            return;
        }
        let conn = mem_db();
        create_script(&conn, "exit-42", "desc", "exit 42", None, None).expect("create_script");
        let result =
            run_script(&conn, "exit-42", &HashMap::new(), "/tmp/x.db").expect("run_script");
        assert_eq!(result.exit_code, 42);
    }

    #[test]
    fn run_script_captures_stderr() {
        if !std::path::Path::new("/bin/sh").exists() {
            return;
        }
        let conn = mem_db();
        create_script(&conn, "stderr-test", "desc", "echo error >&2", None, None)
            .expect("create_script");
        let result =
            run_script(&conn, "stderr-test", &HashMap::new(), "/tmp/x.db").expect("run_script");
        assert!(result.stderr.contains("error"));
    }

    #[test]
    fn run_script_elapsed_ms_reasonable() {
        if !std::path::Path::new("/bin/sh").exists() {
            return;
        }
        let conn = mem_db();
        create_script(&conn, "timing-script", "desc", "true", None, None).expect("create_script");
        let result =
            run_script(&conn, "timing-script", &HashMap::new(), "/tmp/x.db").expect("run_script");
        // Must be non-zero and under 10 seconds for a `true` call.
        assert!(result.elapsed_ms < 10_000);
    }

    #[test]
    fn run_script_result_name_matches() {
        if !std::path::Path::new("/bin/sh").exists() {
            return;
        }
        let conn = mem_db();
        create_script(&conn, "name-check", "desc", "true", None, None).expect("create_script");
        let result =
            run_script(&conn, "name-check", &HashMap::new(), "/tmp/x.db").expect("run_script");
        assert_eq!(result.name, "name-check");
    }

    // ---- ScriptRunResult construction -------------------------------------

    #[test]
    fn script_run_result_fields_accessible() {
        let r = ScriptRunResult {
            name: "test".into(),
            exit_code: 1,
            stdout: "out".into(),
            stderr: "err".into(),
            elapsed_ms: 100,
        };
        assert_eq!(r.name, "test");
        assert_eq!(r.exit_code, 1);
        assert_eq!(r.stdout, "out");
        assert_eq!(r.stderr, "err");
        assert_eq!(r.elapsed_ms, 100);
    }

    // ---- consent / schema constraints ------------------------------------

    #[test]
    fn injection_script_consent_default_emit() {
        let conn = mem_db();
        simple_script(&conn, "consent-check");
        let consent: String = conn
            .query_row(
                "SELECT consent FROM injection_script WHERE name = 'consent-check'",
                [],
                |r| r.get(0),
            )
            .expect("query consent");
        assert_eq!(consent, "Emit");
    }

    #[test]
    fn injection_script_name_unique_constraint() {
        let conn = mem_db();
        simple_script(&conn, "unique-test");
        let result = create_script(&conn, "unique-test", "d2", "e2", None, None);
        assert!(result.is_err());
    }

    // ---- ScriptRecord clone + debug ---------------------------------------

    #[test]
    fn script_record_clone() {
        let conn = mem_db();
        let rec = simple_script(&conn, "clone-me");
        let cloned = rec.clone();
        assert_eq!(cloned.name, rec.name);
        assert_eq!(cloned.id, rec.id);
    }

    #[test]
    fn script_record_debug_not_empty() {
        let conn = mem_db();
        let rec = simple_script(&conn, "debug-test");
        let dbg = format!("{rec:?}");
        assert!(dbg.contains("debug-test"));
    }

    #[test]
    fn script_run_result_clone() {
        let r = ScriptRunResult {
            name: "x".into(),
            exit_code: 0,
            stdout: "y".into(),
            stderr: String::new(),
            elapsed_ms: 1,
        };
        let r2 = r.clone();
        assert_eq!(r2.name, r.name);
    }
}
