//! `m15_checkpoint_ingest` — `/save-session` → injection DB bridge.
//!
//! The PRIMARY write path for consolidation. Accepts pre-parsed
//! [`CheckpointData`] from the caller (file I/O is the caller's
//! responsibility), writes a row to `session_checkpoint` via
//! [`crate::m2_schema::m10b_checkpoint`], then auto-discovers and
//! creates/reinforces [`crate::m2_schema::m07_causal_chain`] entries for
//! any `BUG-NNN` references or known trap names found in the bullet lists.
//!
//! ## Scope
//!
//! This module writes to two tables:
//! 1. `session_checkpoint` — the full structured record.
//! 2. `causal_chain` — auto-discovered `BUG-NNN` and trap references.
//!
//! Trajectory, workstream, and pattern harvesting from checkpoint data are
//! orchestrated by the consolidation pipeline caller (the CLI binary),
//! which chains `m15b_trajectory_capture` and `m16_hebbian_engine` after
//! this module completes.
//!
//! ## Layer
//!
//! `m4_consolidation`
//!
//! ## Dependencies
//!
//! - [`crate::m1_foundation::m02_errors::ConsolidationError`]
//! - [`crate::m2_schema::m10b_checkpoint`] — `CheckpointInsert`, `insert_checkpoint`
//! - [`crate::m2_schema::m07_causal_chain`] — `insert_chain`, `reinforce_chain`, `find_by_label`
//!
//! ## Invariants
//!
//! - `BUG-NNN` extraction rejects false positives (`DEBUG`, `BUGFIX`, `XBUG-`).
//! - Known traps are matched case-insensitively as substrings.
//! - Duplicate labels (bug + trap overlap) are deduplicated before DB writes.
//! - Existing chains are reinforced, not duplicated.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::m1_foundation::m02_errors::{ConsolidationError, SchemaError};
use crate::m2_schema::m07_causal_chain::{find_by_label, insert_chain, reinforce_chain};
use crate::m2_schema::m10b_checkpoint::{insert_checkpoint, CheckpointInsert};

// ---------------------------------------------------------------------------
// Known traps constant
// ---------------------------------------------------------------------------

/// Canonical list of known habitat trap names from the T7 `TrapState` note.
///
/// These are matched case-insensitively as substrings of any bullet text.
const KNOWN_TRAPS: &[&str] = &[
    "cp-alias",
    "pkill-exit-144",
    "rm-tsv-only",
    "povm-hydrate-broken",
    "bridge-url-prefix",
    "pswarm-port-10002",
    "synthex-api-health",
    "me-port-8180",
    "zellij-wasm-no-http",
    "pv2-ipc-socket",
    "synthex-v2-no-v3",
    "povm-pathways-plural",
    "unwrap-in-wasm",
    "timer-5s-minimum",
    "focus-next-pane",
    "synthex-ws-collision",
    "orac-breakers-cascade",
    "pv2-governance-gated",
];

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Input data for checkpoint ingestion — pre-parsed by caller.
///
/// File I/O is the caller's responsibility; this struct holds only the parsed
/// values ready for persistence.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckpointData {
    /// Unique human-readable label, e.g. `"s109-close"`.
    pub label: String,
    /// Session number, if known.
    pub session_number: Option<u32>,
    /// ISO-8601 UTC timestamp string.
    pub timestamp_utc: String,
    /// Number of services alive at checkpoint time.
    pub services_alive: u32,
    /// Concrete artefacts accomplished this session.
    pub accomplished: Vec<String>,
    /// Work in progress at checkpoint time.
    pub in_progress: Vec<String>,
    /// Blocked or deferred items.
    pub blocked: Vec<String>,
    /// Key findings, insights, and measurements.
    pub key_findings: Vec<String>,
    /// Raw markdown resume instructions for the next session.
    pub resume_instructions: String,
    /// Path to the source checkpoint `.md` file.
    pub source_file: String,
    // Optional metadata fields
    /// Zellij pane identifier.
    pub pane_id: Option<String>,
    /// Zellij tab name.
    pub tab: Option<String>,
    /// Active persona name.
    pub persona: Option<String>,
    /// Git commit SHA.
    pub git_sha: Option<String>,
    /// Git branch name.
    pub git_branch: Option<String>,
}

/// Result of a checkpoint ingestion operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IngestResult {
    /// Row ID of the inserted `session_checkpoint` row.
    pub checkpoint_id: i64,
    /// Number of new `causal_chain` entries created.
    pub chains_created: u32,
    /// Number of existing `causal_chain` entries reinforced.
    pub chains_reinforced: u32,
    /// `BUG-NNN` references discovered (deduplicated, upper-cased).
    pub bugs_found: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Ingest a pre-parsed checkpoint and populate the injection DB tables.
///
/// Steps:
/// 1. Build a [`CheckpointInsert`] from `data` and write it to
///    `session_checkpoint`.
/// 2. Collect all bullets from `accomplished`, `in_progress`, `blocked`, and
///    `key_findings`.
/// 3. Extract `BUG-NNN` references with [`extract_bug_references`].
/// 4. Extract known-trap references with [`extract_trap_references`].
/// 5. Deduplicate the union of both sets.
/// 6. For each label: if a matching `causal_chain` row already exists,
///    reinforce it; otherwise insert a new `"bug"` (for `BUG-NNN`) or
///    `"trap"` entry.
///
/// All writes (to `session_checkpoint` and `causal_chain`) are executed inside
/// a single `SQLite` transaction opened with [`Connection::unchecked_transaction`].
/// If any step fails the transaction is rolled back, leaving the database
/// unchanged.
///
/// Returns an [`IngestResult`] summarising what happened.
///
/// # Errors
///
/// Returns [`ConsolidationError`] if writing to the database fails or if the
/// transaction cannot be started or committed.
pub fn ingest_checkpoint(
    conn: &Connection,
    data: &CheckpointData,
) -> Result<IngestResult, ConsolidationError> {
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| ConsolidationError::CheckpointIngestFailed {
            label: data.label.clone(),
            reason: e.to_string(),
        })?;

    let checkpoint_id = write_checkpoint(&tx, data)?;

    let all_bullets: Vec<String> = data
        .accomplished
        .iter()
        .chain(data.in_progress.iter())
        .chain(data.blocked.iter())
        .chain(data.key_findings.iter())
        .cloned()
        .collect();

    let bugs = extract_bug_references(&all_bullets);
    let traps = extract_trap_references(&all_bullets, KNOWN_TRAPS);

    // Build a deduplicated list of (label, chain_type) pairs.
    // BUG-NNN entries use type "bug"; trap names use type "trap".
    // Order: bugs first, then traps; skip traps already in bug list.
    let mut entries: Vec<(String, &str)> = bugs
        .iter()
        .map(|b| (b.clone(), "bug"))
        .collect();

    for trap in &traps {
        // Only add trap if the label is not already present (as a BUG label).
        if !entries.iter().any(|(l, _)| l == trap) {
            entries.push((trap.clone(), "trap"));
        }
    }

    let session = data.session_number.unwrap_or(0);
    let mut chains_created: u32 = 0;
    let mut chains_reinforced: u32 = 0;

    for (label, chain_type) in &entries {
        let existing =
            find_by_label(&tx, label).map_err(|ref e| schema_to_consolidation(e, label))?;

        if existing.is_some() {
            reinforce_chain(&tx, label, session)
                .map_err(|ref e| schema_to_consolidation(e, label))?;
            chains_reinforced += 1;
        } else {
            let description = auto_description(label, chain_type, data.session_number);
            insert_chain(&tx, session, chain_type, label, &description)
                .map_err(|ref e| schema_to_consolidation(e, label))?;
            chains_created += 1;
        }
    }

    tx.commit()
        .map_err(|e| ConsolidationError::CheckpointIngestFailed {
            label: data.label.clone(),
            reason: e.to_string(),
        })?;

    Ok(IngestResult {
        checkpoint_id,
        chains_created,
        chains_reinforced,
        bugs_found: bugs,
    })
}

/// Extract `BUG-NNN[a-z]?` references from a slice of bullet strings.
///
/// The pattern matches the literal prefix `BUG-` followed by one or more
/// decimal digits and an optional lowercase letter suffix (e.g. `BUG-064i`,
/// `BUG-001`). Matches are upper-cased, sorted, and deduplicated before
/// being returned.
///
/// False-positive rejection: `DEBUG` and `BUGFIX` are explicitly excluded
/// because the regex requires the word boundary `BUG-` (with a hyphen).
///
/// # Examples
///
/// ```
/// # use habitat_injection::m4_consolidation::m15_checkpoint_ingest::extract_bug_references;
/// let bullets = vec!["fixed BUG-042 and BUG-001".to_string(), "see BUG-042 again".to_string()];
/// let bugs = extract_bug_references(&bullets);
/// assert_eq!(bugs, vec!["BUG-001", "BUG-042"]);
/// ```
#[must_use]
pub fn extract_bug_references(bullets: &[String]) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for bullet in bullets {
        for (i, _) in bullet.char_indices() {
            // Look for the literal prefix "BUG-" at position `i`.
            if bullet[i..].starts_with("BUG-") {
                // Verify it's not part of a longer prefix like "DEBUG" by
                // checking that the character before 'B' is not an ASCII letter
                // or digit.
                let preceded_by_alnum = i > 0
                    && bullet[..i]
                        .chars()
                        .next_back()
                        .is_some_and(|c| c.is_ascii_alphanumeric());
                if preceded_by_alnum {
                    // skip — part of a longer word
                    continue;
                }
                let rest = &bullet[i + 4..]; // after "BUG-"
                let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
                if digits.is_empty() {
                    continue;
                }
                let after_digits = &rest[digits.len()..];
                let suffix: String = after_digits
                    .chars()
                    .take(1)
                    .filter(char::is_ascii_lowercase)
                    .collect();
                let label = format!("BUG-{digits}{suffix}");
                if !found.contains(&label) {
                    found.push(label);
                }
            }
        }
    }
    found.sort();
    found
}

/// Scan `bullets` for any of the `known_traps` (case-insensitive substring
/// match).
///
/// Returns the matching trap names (from `known_traps`), sorted and
/// deduplicated.
///
/// # Examples
///
/// ```
/// # use habitat_injection::m4_consolidation::m15_checkpoint_ingest::extract_trap_references;
/// let bullets = vec!["hit cp-alias again".to_string(), "CP-ALIAS second hit".to_string()];
/// let traps = extract_trap_references(&bullets, &["cp-alias", "pv2-ipc-socket"]);
/// assert_eq!(traps, vec!["cp-alias"]);
/// ```
#[must_use]
pub fn extract_trap_references(bullets: &[String], known_traps: &[&str]) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for trap in known_traps {
        let trap_lower = trap.to_lowercase();
        let matched = bullets
            .iter()
            .any(|b| b.to_lowercase().contains(&trap_lower));
        if matched && !found.contains(&(*trap).to_string()) {
            found.push((*trap).to_string());
        }
    }
    found.sort();
    found
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Build and persist a [`CheckpointInsert`] from [`CheckpointData`].
fn write_checkpoint(conn: &Connection, data: &CheckpointData) -> Result<i64, ConsolidationError> {
    let mut cp = CheckpointInsert::new(
        data.label.clone(),
        data.timestamp_utc.clone(),
        i64::from(data.services_alive),
        data.accomplished.clone(),
        data.in_progress.clone(),
        data.blocked.clone(),
        data.key_findings.clone(),
        data.resume_instructions.clone(),
        data.source_file.clone(),
    );
    cp.session_number = data.session_number;
    cp.pane_id.clone_from(&data.pane_id);
    cp.tab.clone_from(&data.tab);
    cp.persona.clone_from(&data.persona);
    cp.git_sha.clone_from(&data.git_sha);
    cp.git_branch.clone_from(&data.git_branch);

    insert_checkpoint(conn, &cp).map_err(|e| ConsolidationError::CheckpointIngestFailed {
        label: data.label.clone(),
        reason: e.to_string(),
    })
}

/// Generate a short description for a newly-auto-created chain.
fn auto_description(label: &str, chain_type: &str, session_number: Option<u32>) -> String {
    let session_tag = session_number
        .map(|n| format!(" (first seen S{n})"))
        .unwrap_or_default();
    format!("Auto-created {chain_type} '{label}' from /save-session checkpoint{session_tag}.")
}

/// Map a [`SchemaError`] to a [`ConsolidationError`].
fn schema_to_consolidation(e: &SchemaError, label: &str) -> ConsolidationError {
    ConsolidationError::CheckpointIngestFailed {
        label: label.to_owned(),
        reason: format!("chain operation failed: {e}"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::m2_schema::m06_schema::open_memory;
    use crate::m2_schema::m07_causal_chain::{find_by_label, insert_chain};

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn minimal_data(label: &str) -> CheckpointData {
        CheckpointData {
            label: label.to_string(),
            session_number: Some(109),
            timestamp_utc: "2026-04-24T12:00:00Z".to_string(),
            services_alive: 11,
            accomplished: vec!["shipped m15".to_string()],
            in_progress: vec![],
            blocked: vec![],
            key_findings: vec![],
            resume_instructions: "continue from m16".to_string(),
            source_file: "/tmp/s109.md".to_string(),
            pane_id: None,
            tab: None,
            persona: None,
            git_sha: None,
            git_branch: None,
        }
    }

    // -----------------------------------------------------------------------
    // ingest_checkpoint — minimal data
    // -----------------------------------------------------------------------

    #[test]
    fn ingest_minimal_returns_positive_checkpoint_id() {
        let conn = open_memory().expect("open_memory");
        let result = ingest_checkpoint(&conn, &minimal_data("min-1")).expect("ingest");
        assert!(result.checkpoint_id > 0);
    }

    #[test]
    fn ingest_minimal_no_bugs_no_chains() {
        let conn = open_memory().expect("open_memory");
        let result = ingest_checkpoint(&conn, &minimal_data("min-2")).expect("ingest");
        assert_eq!(result.chains_created, 0);
        assert_eq!(result.chains_reinforced, 0);
        assert!(result.bugs_found.is_empty());
    }

    #[test]
    fn ingest_minimal_checkpoint_id_increments_per_call() {
        let conn = open_memory().expect("open_memory");
        let r1 = ingest_checkpoint(&conn, &minimal_data("seq-1")).expect("ingest 1");
        let r2 = ingest_checkpoint(&conn, &minimal_data("seq-2")).expect("ingest 2");
        assert!(r2.checkpoint_id > r1.checkpoint_id);
    }

    // -----------------------------------------------------------------------
    // ingest_checkpoint — BUG references
    // -----------------------------------------------------------------------

    #[test]
    fn ingest_with_bug_in_accomplished_creates_chain() {
        let conn = open_memory().expect("open_memory");
        let mut data = minimal_data("bug-acc");
        data.accomplished = vec!["fixed BUG-042 at last".to_string()];
        let result = ingest_checkpoint(&conn, &data).expect("ingest");
        assert_eq!(result.chains_created, 1);
        assert_eq!(result.bugs_found, vec!["BUG-042"]);
        assert!(find_by_label(&conn, "BUG-042").expect("find").is_some());
    }

    #[test]
    fn ingest_with_bug_in_blocked_creates_chain() {
        let conn = open_memory().expect("open_memory");
        let mut data = minimal_data("bug-blk");
        data.blocked = vec!["blocked on BUG-055 systemd".to_string()];
        let result = ingest_checkpoint(&conn, &data).expect("ingest");
        assert_eq!(result.chains_created, 1);
        assert!(result.bugs_found.contains(&"BUG-055".to_string()));
    }

    #[test]
    fn ingest_with_bug_in_key_findings_creates_chain() {
        let conn = open_memory().expect("open_memory");
        let mut data = minimal_data("bug-kf");
        data.key_findings = vec!["BUG-001 is the root cause".to_string()];
        let result = ingest_checkpoint(&conn, &data).expect("ingest");
        assert!(result.bugs_found.contains(&"BUG-001".to_string()));
    }

    #[test]
    fn ingest_existing_bug_reinforces_not_creates() {
        let conn = open_memory().expect("open_memory");
        // Pre-insert the chain so it already exists.
        insert_chain(&conn, 108, "bug", "BUG-007", "pre-existing").expect("insert");

        let mut data = minimal_data("bug-reinf");
        data.accomplished = vec!["triggered BUG-007 again".to_string()];
        let result = ingest_checkpoint(&conn, &data).expect("ingest");
        assert_eq!(result.chains_created, 0);
        assert_eq!(result.chains_reinforced, 1);

        let row = find_by_label(&conn, "BUG-007")
            .expect("find")
            .expect("some");
        // original count 1 + reinforce 1 = 2
        assert_eq!(row.reinforcement_count, 2);
    }

    #[test]
    fn ingest_bug_with_suffix_created_correctly() {
        let conn = open_memory().expect("open_memory");
        let mut data = minimal_data("bug-sfx");
        data.accomplished = vec!["resolved BUG-064i once and for all".to_string()];
        let result = ingest_checkpoint(&conn, &data).expect("ingest");
        assert!(result.bugs_found.contains(&"BUG-064i".to_string()));
        assert!(find_by_label(&conn, "BUG-064i").expect("find").is_some());
    }

    #[test]
    fn ingest_multiple_bugs_all_created() {
        let conn = open_memory().expect("open_memory");
        let mut data = minimal_data("multi-bug");
        data.accomplished = vec!["BUG-001 done".to_string()];
        data.blocked = vec!["still open: BUG-099 and BUG-100".to_string()];
        let result = ingest_checkpoint(&conn, &data).expect("ingest");
        assert_eq!(result.chains_created, 3);
        assert_eq!(result.bugs_found.len(), 3);
    }

    #[test]
    fn ingest_duplicate_bug_in_multiple_bullets_deduplicates() {
        let conn = open_memory().expect("open_memory");
        let mut data = minimal_data("dup-bug");
        data.accomplished = vec!["BUG-010 patched".to_string()];
        data.blocked = vec!["BUG-010 still flaky in CI".to_string()];
        data.key_findings = vec!["root cause of BUG-010 found".to_string()];
        let result = ingest_checkpoint(&conn, &data).expect("ingest");
        // Only one chain created (one unique label).
        assert_eq!(result.chains_created, 1);
        assert_eq!(result.bugs_found.len(), 1);
    }

    // -----------------------------------------------------------------------
    // ingest_checkpoint — trap references
    // -----------------------------------------------------------------------

    #[test]
    fn ingest_with_known_trap_creates_chain() {
        let conn = open_memory().expect("open_memory");
        let mut data = minimal_data("trap-cp");
        data.accomplished = vec!["hit the cp-alias trap again".to_string()];
        let result = ingest_checkpoint(&conn, &data).expect("ingest");
        assert_eq!(result.chains_created, 1);
        assert!(find_by_label(&conn, "cp-alias").expect("find").is_some());
    }

    #[test]
    fn ingest_trap_case_insensitive() {
        let conn = open_memory().expect("open_memory");
        let mut data = minimal_data("trap-case");
        // uppercase variant of a known trap
        data.key_findings = vec!["CP-ALIAS struck again".to_string()];
        let result = ingest_checkpoint(&conn, &data).expect("ingest");
        assert_eq!(result.chains_created, 1);
    }

    #[test]
    fn ingest_existing_trap_reinforces() {
        let conn = open_memory().expect("open_memory");
        insert_chain(&conn, 100, "trap", "pkill-exit-144", "pre-existing trap").expect("insert");
        let mut data = minimal_data("trap-reinf");
        data.accomplished = vec!["pkill-exit-144 bit us again".to_string()];
        let result = ingest_checkpoint(&conn, &data).expect("ingest");
        assert_eq!(result.chains_reinforced, 1);
        assert_eq!(result.chains_created, 0);
    }

    // -----------------------------------------------------------------------
    // IngestResult accuracy
    // -----------------------------------------------------------------------

    #[test]
    fn ingest_result_chains_created_plus_reinforced_matches_total() {
        let conn = open_memory().expect("open_memory");
        // Pre-insert one chain that will be reinforced.
        insert_chain(&conn, 100, "bug", "BUG-001", "existing").expect("insert");
        let mut data = minimal_data("acc-check");
        data.accomplished = vec!["BUG-001 fixed, BUG-002 opened".to_string()];
        let result = ingest_checkpoint(&conn, &data).expect("ingest");
        assert_eq!(result.chains_created, 1);
        assert_eq!(result.chains_reinforced, 1);
    }

    #[test]
    fn ingest_result_is_default_constructable() {
        let _default = IngestResult::default();
    }

    #[test]
    fn ingest_result_serialises_to_json() {
        let conn = open_memory().expect("open_memory");
        let result = ingest_checkpoint(&conn, &minimal_data("json-res")).expect("ingest");
        let json = serde_json::to_string(&result).expect("serialize");
        assert!(json.contains("checkpoint_id"));
    }

    #[test]
    fn ingest_result_roundtrips_json() {
        let result = IngestResult {
            checkpoint_id: 7,
            chains_created: 2,
            chains_reinforced: 1,
            bugs_found: vec!["BUG-001".to_string(), "BUG-002".to_string()],
        };
        let json = serde_json::to_string(&result).expect("serialize");
        let decoded: IngestResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.checkpoint_id, 7);
        assert_eq!(decoded.chains_created, 2);
        assert_eq!(decoded.bugs_found.len(), 2);
    }

    // -----------------------------------------------------------------------
    // extract_bug_references — core cases
    // -----------------------------------------------------------------------

    #[test]
    fn extract_bug_basic_single() {
        let bugs = extract_bug_references(&["see BUG-001".to_string()]);
        assert_eq!(bugs, vec!["BUG-001"]);
    }

    #[test]
    fn extract_bug_with_suffix() {
        let bugs = extract_bug_references(&["BUG-064i".to_string()]);
        assert_eq!(bugs, vec!["BUG-064i"]);
    }

    #[test]
    fn extract_bug_three_digits() {
        let bugs = extract_bug_references(&["BUG-999 is open".to_string()]);
        assert_eq!(bugs, vec!["BUG-999"]);
    }

    #[test]
    fn extract_bug_at_start_of_bullet() {
        let bugs = extract_bug_references(&["BUG-001 caused the crash".to_string()]);
        assert_eq!(bugs, vec!["BUG-001"]);
    }

    #[test]
    fn extract_bug_at_end_of_bullet() {
        let bugs = extract_bug_references(&["crash due to BUG-007".to_string()]);
        assert_eq!(bugs, vec!["BUG-007"]);
    }

    #[test]
    fn extract_bug_multiple_in_one_bullet() {
        let bugs = extract_bug_references(&["fixed BUG-001 and BUG-002 today".to_string()]);
        assert_eq!(bugs, vec!["BUG-001", "BUG-002"]);
    }

    #[test]
    fn extract_bug_deduplicated_within_same_bullet() {
        let bugs = extract_bug_references(&["BUG-001 causes BUG-001 to loop".to_string()]);
        assert_eq!(bugs, vec!["BUG-001"]);
    }

    #[test]
    fn extract_bug_deduplicated_across_bullets() {
        let bullets = vec!["BUG-042 observed".to_string(), "BUG-042 persists".to_string()];
        let bugs = extract_bug_references(&bullets);
        assert_eq!(bugs, vec!["BUG-042"]);
    }

    #[test]
    fn extract_bug_sorted_ascending() {
        let bullets = vec!["BUG-099 and BUG-001 and BUG-050".to_string()];
        let bugs = extract_bug_references(&bullets);
        assert_eq!(bugs, vec!["BUG-001", "BUG-050", "BUG-099"]);
    }

    #[test]
    fn extract_bug_no_false_positive_debug() {
        // "DEBUG" must not match — it does not contain the literal "BUG-" (with hyphen).
        let bugs = extract_bug_references(&["DEBUG this value".to_string()]);
        assert!(bugs.is_empty());
    }

    #[test]
    fn extract_bug_no_false_positive_bugfix() {
        // "BUGFIX" must not match — no hyphen after "BUG".
        let bugs = extract_bug_references(&["BUGFIX for issue 42".to_string()]);
        assert!(bugs.is_empty());
    }

    #[test]
    fn extract_bug_no_false_positive_bug_without_digits() {
        // "BUG-" with no digits following must not match.
        let bugs = extract_bug_references(&["encountered a BUG- issue".to_string()]);
        assert!(bugs.is_empty());
    }

    #[test]
    fn extract_bug_no_false_positive_xbug() {
        // A word like "XBUG-001" must not match — 'X' is alphanumeric, so it
        // is preceded by an alnum char and should be rejected.
        let bugs = extract_bug_references(&["XBUG-001 found".to_string()]);
        assert!(bugs.is_empty());
    }

    #[test]
    fn extract_bug_empty_bullets_returns_empty() {
        let bugs = extract_bug_references(&[]);
        assert!(bugs.is_empty());
    }

    #[test]
    fn extract_bug_empty_string_returns_empty() {
        let bugs = extract_bug_references(&[String::new()]);
        assert!(bugs.is_empty());
    }

    #[test]
    fn extract_bug_all_empty_strings_returns_empty() {
        let bugs =
            extract_bug_references(&[String::new(), String::new(), "no refs".to_string()]);
        assert!(bugs.is_empty());
    }

    #[test]
    fn extract_bug_suffix_only_one_lowercase_char() {
        // "BUG-001ab" — only 'a' is taken as suffix; 'b' is not part of the
        // label.
        let bugs = extract_bug_references(&["BUG-001ab".to_string()]);
        assert_eq!(bugs, vec!["BUG-001a"]);
    }

    #[test]
    fn extract_bug_suffix_uppercase_is_not_suffix() {
        // "BUG-001A" — 'A' is not lowercase, so no suffix is captured.
        let bugs = extract_bug_references(&["BUG-001A".to_string()]);
        assert_eq!(bugs, vec!["BUG-001"]);
    }

    #[test]
    fn extract_bug_large_number() {
        let bugs = extract_bug_references(&["BUG-12345".to_string()]);
        assert_eq!(bugs, vec!["BUG-12345"]);
    }

    #[test]
    fn extract_bug_hyphen_separated_from_word() {
        // e.g. "pre-BUG-007-post" — the hyphen before 'B' is NOT alphanumeric,
        // so `BUG-007` IS extracted (hyphen is a valid separator).
        let bugs = extract_bug_references(&["pre-BUG-007-post".to_string()]);
        assert_eq!(bugs, vec!["BUG-007"]);
    }

    #[test]
    fn extract_bug_preceded_by_space_matches() {
        let bugs = extract_bug_references(&[" BUG-042 ".to_string()]);
        assert_eq!(bugs, vec!["BUG-042"]);
    }

    #[test]
    fn extract_bug_preceded_by_paren_matches() {
        let bugs = extract_bug_references(&["(BUG-042)".to_string()]);
        assert_eq!(bugs, vec!["BUG-042"]);
    }

    // -----------------------------------------------------------------------
    // extract_trap_references — core cases
    // -----------------------------------------------------------------------

    #[test]
    fn extract_trap_exact_match() {
        let traps = extract_trap_references(&["cp-alias caused this".to_string()], KNOWN_TRAPS);
        assert!(traps.contains(&"cp-alias".to_string()));
    }

    #[test]
    fn extract_trap_case_insensitive() {
        let traps = extract_trap_references(&["CP-ALIAS was triggered".to_string()], KNOWN_TRAPS);
        assert!(traps.contains(&"cp-alias".to_string()));
    }

    #[test]
    fn extract_trap_mixed_case() {
        let traps =
            extract_trap_references(&["Pkill-Exit-144 killed the proc".to_string()], KNOWN_TRAPS);
        assert!(traps.contains(&"pkill-exit-144".to_string()));
    }

    #[test]
    fn extract_trap_no_match_returns_empty() {
        let traps =
            extract_trap_references(&["nothing interesting here".to_string()], KNOWN_TRAPS);
        assert!(traps.is_empty());
    }

    #[test]
    fn extract_trap_multiple_traps_in_one_bullet() {
        let traps = extract_trap_references(
            &["hit cp-alias and focus-next-pane".to_string()],
            KNOWN_TRAPS,
        );
        assert!(traps.contains(&"cp-alias".to_string()));
        assert!(traps.contains(&"focus-next-pane".to_string()));
    }

    #[test]
    fn extract_trap_deduplicated_across_bullets() {
        let bullets = vec![
            "cp-alias issue again".to_string(),
            "still hitting CP-ALIAS".to_string(),
        ];
        let traps = extract_trap_references(&bullets, KNOWN_TRAPS);
        let cp_count = traps.iter().filter(|t| *t == "cp-alias").count();
        assert_eq!(cp_count, 1);
    }

    #[test]
    fn extract_trap_sorted_ascending() {
        let bullets = vec!["zellij-wasm-no-http and cp-alias and bridge-url-prefix".to_string()];
        let traps = extract_trap_references(&bullets, KNOWN_TRAPS);
        // Verify sorted order.
        let mut sorted = traps.clone();
        sorted.sort();
        assert_eq!(traps, sorted);
    }

    #[test]
    fn extract_trap_empty_bullets_returns_empty() {
        let traps = extract_trap_references(&[], KNOWN_TRAPS);
        assert!(traps.is_empty());
    }

    #[test]
    fn extract_trap_empty_known_traps_returns_empty() {
        let traps = extract_trap_references(&["cp-alias".to_string()], &[]);
        assert!(traps.is_empty());
    }

    #[test]
    fn extract_trap_substring_match() {
        // "cp-alias" is a substring of "the cp-alias bug"
        let traps =
            extract_trap_references(&["the cp-alias bug appeared".to_string()], KNOWN_TRAPS);
        assert!(traps.contains(&"cp-alias".to_string()));
    }

    #[test]
    fn extract_trap_all_18_known_traps_matchable() {
        // Construct one bullet per trap.
        let bullets: Vec<String> = KNOWN_TRAPS.iter().map(|t| t.to_string()).collect();
        let traps = extract_trap_references(&bullets, KNOWN_TRAPS);
        assert_eq!(traps.len(), KNOWN_TRAPS.len());
    }

    // -----------------------------------------------------------------------
    // CheckpointData serde roundtrip
    // -----------------------------------------------------------------------

    #[test]
    fn checkpoint_data_serde_roundtrip() {
        let data = minimal_data("serde-rt");
        let json = serde_json::to_string(&data).expect("serialize");
        let decoded: CheckpointData = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.label, data.label);
        assert_eq!(decoded.session_number, data.session_number);
        assert_eq!(decoded.services_alive, data.services_alive);
    }

    #[test]
    fn checkpoint_data_default_is_valid() {
        let d = CheckpointData::default();
        assert!(d.label.is_empty());
        assert!(d.session_number.is_none());
        assert_eq!(d.services_alive, 0);
    }

    // -----------------------------------------------------------------------
    // Optional fields flow through to the checkpoint row
    // -----------------------------------------------------------------------

    #[test]
    fn optional_fields_persisted() {
        let conn = open_memory().expect("open_memory");
        let mut data = minimal_data("opts");
        data.pane_id = Some("pane-7".to_string());
        data.tab = Some("Tab 1".to_string());
        data.persona = Some("The Watcher".to_string());
        data.git_sha = Some("abc1234".to_string());
        data.git_branch = Some("main".to_string());
        let result = ingest_checkpoint(&conn, &data).expect("ingest");
        assert!(result.checkpoint_id > 0);
    }

    // -----------------------------------------------------------------------
    // Session number propagated to new chain origin_session
    // -----------------------------------------------------------------------

    #[test]
    fn new_bug_chain_uses_session_number_as_origin() {
        let conn = open_memory().expect("open_memory");
        let mut data = minimal_data("origin-sess");
        data.session_number = Some(42);
        data.accomplished = vec!["BUG-099 surfaced".to_string()];
        ingest_checkpoint(&conn, &data).expect("ingest");
        let row = find_by_label(&conn, "BUG-099")
            .expect("find")
            .expect("some");
        assert_eq!(row.origin_session, 42);
    }

    #[test]
    fn new_bug_chain_without_session_number_uses_zero() {
        let conn = open_memory().expect("open_memory");
        let mut data = minimal_data("no-sess-num");
        data.session_number = None;
        data.accomplished = vec!["BUG-200 observed".to_string()];
        ingest_checkpoint(&conn, &data).expect("ingest");
        let row = find_by_label(&conn, "BUG-200")
            .expect("find")
            .expect("some");
        assert_eq!(row.origin_session, 0);
    }

    // -----------------------------------------------------------------------
    // Bug chain type is "bug"; trap chain type is "trap"
    // -----------------------------------------------------------------------

    #[test]
    fn new_bug_chain_has_type_bug() {
        let conn = open_memory().expect("open_memory");
        let mut data = minimal_data("type-bug");
        data.accomplished = vec!["BUG-777 fixed".to_string()];
        ingest_checkpoint(&conn, &data).expect("ingest");
        let row = find_by_label(&conn, "BUG-777")
            .expect("find")
            .expect("some");
        assert_eq!(row.chain_type, "bug");
    }

    #[test]
    fn new_trap_chain_has_type_trap() {
        let conn = open_memory().expect("open_memory");
        let mut data = minimal_data("type-trap");
        data.accomplished = vec!["focus-next-pane caught us".to_string()];
        ingest_checkpoint(&conn, &data).expect("ingest");
        let row = find_by_label(&conn, "focus-next-pane")
            .expect("find")
            .expect("some");
        assert_eq!(row.chain_type, "trap");
    }

    // -----------------------------------------------------------------------
    // Bug label present as trap label does not double-count
    // -----------------------------------------------------------------------

    #[test]
    fn bug_and_trap_overlap_not_double_counted() {
        // If a bullet mentions both a BUG-NNN and a known trap, they are
        // independent labels — no duplication between the sets.
        let conn = open_memory().expect("open_memory");
        let mut data = minimal_data("overlap");
        data.accomplished = vec!["BUG-001 and cp-alias both triggered".to_string()];
        let result = ingest_checkpoint(&conn, &data).expect("ingest");
        // Two distinct labels → two chains.
        assert_eq!(result.chains_created, 2);
    }

    // -----------------------------------------------------------------------
    // Reinforce updates last_reinforced_session
    // -----------------------------------------------------------------------

    #[test]
    fn reinforced_chain_last_session_updated() {
        let conn = open_memory().expect("open_memory");
        insert_chain(&conn, 100, "bug", "BUG-011", "existing").expect("insert");

        let mut data = minimal_data("reinf-sess");
        data.session_number = Some(109);
        data.accomplished = vec!["BUG-011 re-triggered".to_string()];
        ingest_checkpoint(&conn, &data).expect("ingest");

        let row = find_by_label(&conn, "BUG-011")
            .expect("find")
            .expect("some");
        assert_eq!(row.last_reinforced_session, Some(109));
    }
}
