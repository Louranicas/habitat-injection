//! `m24_migration` — `SQLite` → STDB migration plan, source enumeration,
//! checksum computation, and verification logic.
//!
//! This module does **not** connect to `SpaceTimeDB`. It reads from `SQLite`
//! and produces [`MigrationPlan`] values, pre/post [`MigrationChecksum`]
//! records for verification, and [`MigrationStatus`] tracking. The actual
//! STDB write operations are Phase 2 runtime code in `m23_ingester`.
//!
//! # Architecture
//!
//! ```text
//! 16 sources → 6 STDB tables
//! ───────────────────────────────────────────────
//! Phase A  Core tables + ingester
//!   service_tracking.db::learned_patterns        → knowledge_node
//!   service_tracking.db::orchestration_graph     → knowledge_edge
//!   service_tracking.db::service_events          → service_event
//! Phase B  Knowledge graph migration
//!   service_tracking.db::cross_agent_learnings   → knowledge_edge
//!   hebbian_pulse.db::neural_pathways            → knowledge_node
//!   hebbian_pulse.db::hebbian_pathways           → knowledge_edge
//! Phase C  Watcher + causal chains
//!   hebbian_pulse.db::decay_audit_log            → service_event
//!   POVM :8125 /pathways                         → knowledge_node
//!   system_synergy.db                            → knowledge_edge
//! Phase D  Cross-service integration
//!   V3 workflow_state.db                         → session_snapshot
//!   synthex-v2 gradient_snapshot.db              → service_event
//!   synthex-v2 bridge_health.db                  → service_event
//! Phase E  Bootstrap revolution
//!   causal_chain (injection DB)                  → knowledge_node
//!   session_trajectory (injection DB)            → session_snapshot
//!   workstream (injection DB)                    → knowledge_node
//!   RM :8130 heartbeat                           → service_event
//! ```
//!
//! # Layer
//!
//! `m6_stdb`
//!
//! # Dependencies
//!
//! `m01_types`, `m02_errors`

use serde::{Deserialize, Serialize};

use crate::m1_foundation::m02_errors::MigrationError;

// ---------------------------------------------------------------------------
// SourceType
// ---------------------------------------------------------------------------

/// Where the migration source data lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SourceType {
    /// A table inside a local `SQLite` database file.
    SqliteTable,
    /// A remote HTTP endpoint (polled via GET).
    HttpEndpoint,
}

// ---------------------------------------------------------------------------
// MigrationPhase
// ---------------------------------------------------------------------------

/// Ordered migration phase — determines which sources are processed first.
///
/// Phases are ordered `A < B < C < D < E`; the `Ord` derive encodes that
/// ordering via declaration position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[non_exhaustive]
pub enum MigrationPhase {
    /// Phase A — core tables + ingester (service tracking).
    A,
    /// Phase B — knowledge graph migration (hebbian + cross-agent).
    B,
    /// Phase C — watcher + causal chains (decay audit + POVM).
    C,
    /// Phase D — cross-service integration (V3 workflow + synthex-v2).
    D,
    /// Phase E — bootstrap revolution (injection DB + RM heartbeat).
    E,
}

impl MigrationPhase {
    /// Returns a human-readable display name such as `"Phase A"`.
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::A => "Phase A",
            Self::B => "Phase B",
            Self::C => "Phase C",
            Self::D => "Phase D",
            Self::E => "Phase E",
        }
    }

    /// Returns all phases in ascending order.
    #[must_use]
    pub fn all() -> Vec<Self> {
        vec![Self::A, Self::B, Self::C, Self::D, Self::E]
    }
}

// ---------------------------------------------------------------------------
// MigrationSource
// ---------------------------------------------------------------------------

/// A single migration source — one table or endpoint to copy into STDB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationSource {
    /// Short, unique identifier for this source (e.g. `"learned_patterns"`).
    pub name: String,
    /// Where the data resides.
    pub source_type: SourceType,
    /// Name of the target STDB table (e.g. `"knowledge_node"`).
    pub target_table: String,
    /// Estimated row count used for progress reporting.
    pub estimated_rows: u64,
    /// Migration phase in which this source is processed.
    pub phase: MigrationPhase,
}

// ---------------------------------------------------------------------------
// MigrationChecksum
// ---------------------------------------------------------------------------

/// Verification checksum captured before and after a migration step.
///
/// Row counts must match exactly; weight aggregates are compared within a
/// caller-supplied tolerance (see [`verify_checksums`]).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MigrationChecksum {
    /// Name of the source this checksum belongs to.
    pub source_name: String,
    /// Exact row count at capture time.
    pub row_count: u64,
    /// Sum of the weight column, if one was requested.
    pub weight_sum: Option<f64>,
    /// Average of the weight column, if one was requested.
    pub weight_avg: Option<f64>,
}

// ---------------------------------------------------------------------------
// MigrationStepStatus
// ---------------------------------------------------------------------------

/// Per-source migration step state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum MigrationStepStatus {
    /// Not yet started.
    Pending,
    /// Currently being migrated.
    InProgress,
    /// Finished successfully and checksums verified.
    Completed,
    /// Encountered an unrecoverable error.
    Failed,
    /// Intentionally skipped (e.g. source was empty).
    Skipped,
}

// ---------------------------------------------------------------------------
// MigrationSourceStatus
// ---------------------------------------------------------------------------

/// Runtime tracking for a single migration source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationSourceStatus {
    /// The source definition from the plan.
    pub source: MigrationSource,
    /// Checksum captured before migration started.
    pub pre_checksum: Option<MigrationChecksum>,
    /// Checksum captured after migration completed.
    pub post_checksum: Option<MigrationChecksum>,
    /// How many rows were actually written to STDB.
    pub rows_migrated: u64,
    /// Current step status.
    pub status: MigrationStepStatus,
    /// Error message, present only when `status == Failed`.
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// MigrationStatus
// ---------------------------------------------------------------------------

/// Full migration run status — aggregates across all sources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationStatus {
    /// Per-source tracking.
    pub sources: Vec<MigrationSourceStatus>,
    /// Phase being executed in this run.
    pub phase: MigrationPhase,
    /// RFC-3339 timestamp of when the run started.
    pub started_at: String,
    /// RFC-3339 timestamp of when the run completed, or `None` if still running.
    pub completed_at: Option<String>,
    /// Sum of `rows_migrated` across all completed sources.
    pub total_rows_migrated: u64,
    /// Non-fatal error messages accumulated during the run.
    pub errors: Vec<String>,
}

impl MigrationStatus {
    /// Returns `true` when every source has reached [`MigrationStepStatus::Completed`]
    /// or [`MigrationStepStatus::Skipped`].
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.sources.iter().all(|s| {
            matches!(
                s.status,
                MigrationStepStatus::Completed | MigrationStepStatus::Skipped
            )
        })
    }

    /// Returns `true` when any source has [`MigrationStepStatus::Failed`] or
    /// when the `errors` vector is non-empty.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
            || self
                .sources
                .iter()
                .any(|s| s.status == MigrationStepStatus::Failed)
    }
}

// ---------------------------------------------------------------------------
// MigrationPlan
// ---------------------------------------------------------------------------

/// Complete migration plan: all 16 sources grouped across 5 phases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    /// All migration sources, in the recommended execution order.
    pub sources: Vec<MigrationSource>,
    /// Sum of `estimated_rows` across all sources.
    pub total_estimated_rows: u64,
    /// Distinct phases present in this plan, in ascending order.
    pub phases: Vec<MigrationPhase>,
}

impl MigrationPlan {
    /// Returns all sources that belong to `phase`.
    #[must_use]
    pub fn sources_for_phase(&self, phase: MigrationPhase) -> Vec<&MigrationSource> {
        self.sources.iter().filter(|s| s.phase == phase).collect()
    }

    /// Returns the sum of `estimated_rows` for all sources in `phase`.
    #[must_use]
    pub fn total_for_phase(&self, phase: MigrationPhase) -> u64 {
        self.sources
            .iter()
            .filter(|s| s.phase == phase)
            .map(|s| s.estimated_rows)
            .sum()
    }
}

// ---------------------------------------------------------------------------
// build_migration_plan
// ---------------------------------------------------------------------------

/// Phase A sources — core `service_tracking.db` tables.
fn sources_phase_a() -> [MigrationSource; 3] {
    [
        MigrationSource {
            name: "learned_patterns".into(),
            source_type: SourceType::SqliteTable,
            target_table: "knowledge_node".into(),
            estimated_rows: 500,
            phase: MigrationPhase::A,
        },
        MigrationSource {
            name: "orchestration_graph".into(),
            source_type: SourceType::SqliteTable,
            target_table: "knowledge_edge".into(),
            estimated_rows: 800,
            phase: MigrationPhase::A,
        },
        MigrationSource {
            name: "service_events".into(),
            source_type: SourceType::SqliteTable,
            target_table: "service_event".into(),
            estimated_rows: 2000,
            phase: MigrationPhase::A,
        },
    ]
}

/// Phase B sources — knowledge graph migration (`hebbian_pulse.db` + cross-agent).
fn sources_phase_b() -> [MigrationSource; 3] {
    [
        MigrationSource {
            name: "cross_agent_learnings".into(),
            source_type: SourceType::SqliteTable,
            target_table: "knowledge_edge".into(),
            estimated_rows: 1200,
            phase: MigrationPhase::B,
        },
        MigrationSource {
            name: "neural_pathways".into(),
            source_type: SourceType::SqliteTable,
            target_table: "knowledge_node".into(),
            estimated_rows: 400,
            phase: MigrationPhase::B,
        },
        MigrationSource {
            name: "hebbian_pathways".into(),
            source_type: SourceType::SqliteTable,
            target_table: "knowledge_edge".into(),
            estimated_rows: 1600,
            phase: MigrationPhase::B,
        },
    ]
}

/// Phase C sources — watcher, causal chains, decay audit, POVM.
fn sources_phase_c() -> [MigrationSource; 3] {
    [
        MigrationSource {
            name: "decay_audit_log".into(),
            source_type: SourceType::SqliteTable,
            target_table: "service_event".into(),
            estimated_rows: 3000,
            phase: MigrationPhase::C,
        },
        MigrationSource {
            name: "povm_pathways".into(),
            source_type: SourceType::HttpEndpoint,
            target_table: "knowledge_node".into(),
            estimated_rows: 3554,
            phase: MigrationPhase::C,
        },
        MigrationSource {
            name: "system_synergy".into(),
            source_type: SourceType::SqliteTable,
            target_table: "knowledge_edge".into(),
            estimated_rows: 600,
            phase: MigrationPhase::C,
        },
    ]
}

/// Phase D sources — cross-service integration (V3 + synthex-v2).
fn sources_phase_d() -> [MigrationSource; 3] {
    [
        MigrationSource {
            name: "v3_workflow_state".into(),
            source_type: SourceType::SqliteTable,
            target_table: "session_snapshot".into(),
            estimated_rows: 200,
            phase: MigrationPhase::D,
        },
        MigrationSource {
            name: "gradient_snapshot".into(),
            source_type: SourceType::SqliteTable,
            target_table: "service_event".into(),
            estimated_rows: 900,
            phase: MigrationPhase::D,
        },
        MigrationSource {
            name: "bridge_health".into(),
            source_type: SourceType::SqliteTable,
            target_table: "service_event".into(),
            estimated_rows: 500,
            phase: MigrationPhase::D,
        },
    ]
}

/// Phase E sources — bootstrap revolution (injection DB + RM heartbeat).
fn sources_phase_e() -> [MigrationSource; 4] {
    [
        MigrationSource {
            name: "causal_chain".into(),
            source_type: SourceType::SqliteTable,
            target_table: "knowledge_node".into(),
            estimated_rows: 150,
            phase: MigrationPhase::E,
        },
        MigrationSource {
            name: "session_trajectory".into(),
            source_type: SourceType::SqliteTable,
            target_table: "session_snapshot".into(),
            estimated_rows: 110,
            phase: MigrationPhase::E,
        },
        MigrationSource {
            name: "workstream".into(),
            source_type: SourceType::SqliteTable,
            target_table: "knowledge_node".into(),
            estimated_rows: 80,
            phase: MigrationPhase::E,
        },
        MigrationSource {
            name: "rm_heartbeat".into(),
            source_type: SourceType::HttpEndpoint,
            target_table: "service_event".into(),
            estimated_rows: 1,
            phase: MigrationPhase::E,
        },
    ]
}

/// Returns all 16 canonical migration sources in phase (A → E) order.
fn canonical_sources() -> Vec<MigrationSource> {
    let mut out = Vec::with_capacity(16);
    out.extend(sources_phase_a());
    out.extend(sources_phase_b());
    out.extend(sources_phase_c());
    out.extend(sources_phase_d());
    out.extend(sources_phase_e());
    out
}

/// Build the canonical migration plan containing all 16 sources.
///
/// Sources are ordered by phase (A → E) and then by their logical dependency
/// within each phase. Estimated row counts reflect live observations from the
/// habitat at plan-authoring time and will drift — they are used only for
/// progress reporting, not correctness checks.
#[must_use]
pub fn build_migration_plan() -> MigrationPlan {
    let sources = canonical_sources();
    let total_estimated_rows = sources.iter().map(|s| s.estimated_rows).sum();

    // Collect the distinct phases present, in ascending order.
    let mut phases: Vec<MigrationPhase> = sources.iter().map(|s| s.phase).collect();
    phases.sort_unstable();
    phases.dedup();

    MigrationPlan {
        sources,
        total_estimated_rows,
        phases,
    }
}

// ---------------------------------------------------------------------------
// compute_checksum_from_sqlite  (sqlite feature-gated)
// ---------------------------------------------------------------------------

/// Compute a [`MigrationChecksum`] for `table` using the given connection.
///
/// Executes `COUNT(*)` and, if `weight_column` is `Some`, also `SUM` and
/// `AVG` over that column. The column name is checked against a simple
/// allowlist (`[a-zA-Z0-9_]`) before being interpolated into the query to
/// prevent SQL injection.
///
/// # Errors
///
/// Returns [`MigrationError::SourceReadFailed`] if:
/// - `table` or `weight_column` contains characters outside `[a-zA-Z0-9_]`.
/// - Any SQL query fails.
#[cfg(feature = "sqlite")]
pub fn compute_checksum_from_sqlite(
    conn: &rusqlite::Connection,
    table: &str,
    weight_column: Option<&str>,
) -> Result<MigrationChecksum, MigrationError> {
    // Validate identifiers to prevent SQL injection.
    validate_identifier(table)?;
    if let Some(col) = weight_column {
        validate_identifier(col)?;
    }

    // COUNT(*) — always.
    let row_count: u64 = conn
        .query_row(
            &format!("SELECT COUNT(*) FROM \"{table}\""),
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(i64::unsigned_abs)
        .map_err(|e| MigrationError::SourceReadFailed {
            origin: table.to_owned(),
            reason: e.to_string(),
        })?;

    // Optional SUM / AVG.
    let (weight_sum, weight_avg) = match weight_column {
        Some(col) if row_count > 0 => {
            let sum: f64 = conn
                .query_row(
                    &format!("SELECT COALESCE(SUM(\"{col}\"), 0.0) FROM \"{table}\""),
                    [],
                    |row| row.get::<_, f64>(0),
                )
                .map_err(|e| MigrationError::SourceReadFailed {
                    origin: table.to_owned(),
                    reason: e.to_string(),
                })?;
            let avg: f64 = conn
                .query_row(
                    &format!("SELECT COALESCE(AVG(\"{col}\"), 0.0) FROM \"{table}\""),
                    [],
                    |row| row.get::<_, f64>(0),
                )
                .map_err(|e| MigrationError::SourceReadFailed {
                    origin: table.to_owned(),
                    reason: e.to_string(),
                })?;
            (Some(sum), Some(avg))
        }
        Some(_) => {
            // Table is empty — aggregates are definitionally zero.
            (Some(0.0_f64), Some(0.0_f64))
        }
        None => (None, None),
    };

    Ok(MigrationChecksum {
        source_name: table.to_owned(),
        row_count,
        weight_sum,
        weight_avg,
    })
}

/// Validate that `ident` contains only `[a-zA-Z0-9_]`.
///
/// # Errors
///
/// Returns [`MigrationError::SourceReadFailed`] when invalid characters are
/// found.
fn validate_identifier(ident: &str) -> Result<(), MigrationError> {
    if ident.is_empty()
        || !ident
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(MigrationError::SourceReadFailed {
            origin: ident.to_owned(),
            reason: format!(
                "identifier '{ident}' contains invalid characters; only [a-zA-Z0-9_] allowed"
            ),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// verify_checksums
// ---------------------------------------------------------------------------

/// Verify that `pre` and `post` checksums are consistent.
///
/// Rules:
/// - `row_count` must match **exactly**.
/// - `weight_sum` and `weight_avg`, when both are `Some`, must differ by at
///   most `tolerance` (absolute).
/// - If one side has a weight aggregate and the other does not, the check is
///   skipped (the columns may legitimately differ between source and target).
///
/// # Errors
///
/// Returns [`MigrationError::RowCountMismatch`] when row counts differ, or
/// [`MigrationError::ChecksumMismatch`] when a weight aggregate exceeds
/// `tolerance`.
#[must_use = "checksum verification result must be checked — ignoring it skips data integrity validation"]
pub fn verify_checksums(
    pre: &MigrationChecksum,
    post: &MigrationChecksum,
    tolerance: f64,
) -> Result<(), MigrationError> {
    // Row count must be exact.
    if pre.row_count != post.row_count {
        return Err(MigrationError::RowCountMismatch {
            table: pre.source_name.clone(),
            source_count: pre.row_count,
            target_count: post.row_count,
        });
    }

    // Weight sum — only compare when both sides have a value.
    if let (Some(pre_sum), Some(post_sum)) = (pre.weight_sum, post.weight_sum)
        && (pre_sum - post_sum).abs() > tolerance
    {
        return Err(MigrationError::ChecksumMismatch {
            table: pre.source_name.clone(),
            source_sum: pre_sum,
            target_sum: post_sum,
        });
    }

    // Weight avg — same rule.
    if let (Some(pre_avg), Some(post_avg)) = (pre.weight_avg, post.weight_avg)
        && (pre_avg - post_avg).abs() > tolerance
    {
        return Err(MigrationError::ChecksumMismatch {
            table: pre.source_name.clone(),
            source_sum: pre_avg,
            target_sum: post_avg,
        });
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── MigrationPhase ordering ───────────────────────────────────────────

    #[test]
    fn phase_ordering_a_lt_b() {
        assert!(MigrationPhase::A < MigrationPhase::B);
    }

    #[test]
    fn phase_ordering_b_lt_c() {
        assert!(MigrationPhase::B < MigrationPhase::C);
    }

    #[test]
    fn phase_ordering_c_lt_d() {
        assert!(MigrationPhase::C < MigrationPhase::D);
    }

    #[test]
    fn phase_ordering_d_lt_e() {
        assert!(MigrationPhase::D < MigrationPhase::E);
    }

    #[test]
    fn phase_ordering_a_lt_e() {
        assert!(MigrationPhase::A < MigrationPhase::E);
    }

    #[test]
    fn phase_ordering_eq() {
        assert_eq!(MigrationPhase::C, MigrationPhase::C);
    }

    #[test]
    fn phase_all_ascending() {
        let phases = MigrationPhase::all();
        assert_eq!(phases.len(), 5);
        for w in phases.windows(2) {
            assert!(w[0] < w[1]);
        }
    }

    // ── MigrationPhase::display_name ─────────────────────────────────────

    #[test]
    fn phase_display_name_a() {
        assert_eq!(MigrationPhase::A.display_name(), "Phase A");
    }

    #[test]
    fn phase_display_name_b() {
        assert_eq!(MigrationPhase::B.display_name(), "Phase B");
    }

    #[test]
    fn phase_display_name_c() {
        assert_eq!(MigrationPhase::C.display_name(), "Phase C");
    }

    #[test]
    fn phase_display_name_d() {
        assert_eq!(MigrationPhase::D.display_name(), "Phase D");
    }

    #[test]
    fn phase_display_name_e() {
        assert_eq!(MigrationPhase::E.display_name(), "Phase E");
    }

    // ── build_migration_plan ─────────────────────────────────────────────

    #[test]
    fn plan_has_sixteen_sources() {
        let plan = build_migration_plan();
        assert_eq!(plan.sources.len(), 16);
    }

    #[test]
    fn plan_has_five_phases() {
        let plan = build_migration_plan();
        assert_eq!(plan.phases.len(), 5);
    }

    #[test]
    fn plan_phases_are_ascending() {
        let plan = build_migration_plan();
        for w in plan.phases.windows(2) {
            assert!(w[0] < w[1]);
        }
    }

    #[test]
    fn plan_total_estimated_rows_correct() {
        let plan = build_migration_plan();
        let manual_sum: u64 = plan.sources.iter().map(|s| s.estimated_rows).sum();
        assert_eq!(plan.total_estimated_rows, manual_sum);
    }

    #[test]
    fn plan_total_estimated_rows_nonzero() {
        let plan = build_migration_plan();
        assert!(plan.total_estimated_rows > 0);
    }

    #[test]
    fn plan_all_sources_have_nonempty_name() {
        let plan = build_migration_plan();
        for src in &plan.sources {
            assert!(!src.name.is_empty(), "source name must not be empty");
        }
    }

    #[test]
    fn plan_all_sources_have_nonempty_target_table() {
        let plan = build_migration_plan();
        for src in &plan.sources {
            assert!(
                !src.target_table.is_empty(),
                "target_table must not be empty for {}",
                src.name
            );
        }
    }

    #[test]
    fn plan_source_names_unique() {
        let plan = build_migration_plan();
        let mut names: Vec<&str> = plan.sources.iter().map(|s| s.name.as_str()).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len(), "source names must be unique");
    }

    // ── MigrationPlan::sources_for_phase ─────────────────────────────────

    #[test]
    fn sources_for_phase_a_count() {
        let plan = build_migration_plan();
        let phase_a = plan.sources_for_phase(MigrationPhase::A);
        assert_eq!(phase_a.len(), 3);
    }

    #[test]
    fn sources_for_phase_b_count() {
        let plan = build_migration_plan();
        assert_eq!(plan.sources_for_phase(MigrationPhase::B).len(), 3);
    }

    #[test]
    fn sources_for_phase_c_count() {
        let plan = build_migration_plan();
        assert_eq!(plan.sources_for_phase(MigrationPhase::C).len(), 3);
    }

    #[test]
    fn sources_for_phase_d_count() {
        let plan = build_migration_plan();
        assert_eq!(plan.sources_for_phase(MigrationPhase::D).len(), 3);
    }

    #[test]
    fn sources_for_phase_e_count() {
        let plan = build_migration_plan();
        assert_eq!(plan.sources_for_phase(MigrationPhase::E).len(), 4);
    }

    #[test]
    fn sources_for_phase_all_phases_sum_to_total() {
        let plan = build_migration_plan();
        let sum: usize = MigrationPhase::all()
            .iter()
            .map(|&p| plan.sources_for_phase(p).len())
            .sum();
        assert_eq!(sum, plan.sources.len());
    }

    #[test]
    fn sources_for_phase_a_names_correct() {
        let plan = build_migration_plan();
        let names: Vec<&str> = plan
            .sources_for_phase(MigrationPhase::A)
            .into_iter()
            .map(|s| s.name.as_str())
            .collect();
        assert!(names.contains(&"learned_patterns"));
        assert!(names.contains(&"orchestration_graph"));
        assert!(names.contains(&"service_events"));
    }

    #[test]
    fn sources_for_phase_e_includes_rm_heartbeat() {
        let plan = build_migration_plan();
        let phase_e = plan.sources_for_phase(MigrationPhase::E);
        let has_rm = phase_e.iter().any(|s| s.name == "rm_heartbeat");
        assert!(has_rm, "Phase E must include rm_heartbeat");
    }

    // ── MigrationPlan::total_for_phase ───────────────────────────────────

    #[test]
    fn total_for_phase_a_matches_manual() {
        let plan = build_migration_plan();
        let expected: u64 = plan
            .sources_for_phase(MigrationPhase::A)
            .iter()
            .map(|s| s.estimated_rows)
            .sum();
        assert_eq!(plan.total_for_phase(MigrationPhase::A), expected);
    }

    #[test]
    fn total_for_phase_c_includes_povm_rows() {
        let plan = build_migration_plan();
        // POVM has 3554 estimated rows — Phase C total must be ≥ that.
        assert!(plan.total_for_phase(MigrationPhase::C) >= 3554);
    }

    #[test]
    fn total_for_phase_sums_to_grand_total() {
        let plan = build_migration_plan();
        let per_phase_sum: u64 = MigrationPhase::all()
            .iter()
            .map(|&p| plan.total_for_phase(p))
            .sum();
        assert_eq!(per_phase_sum, plan.total_estimated_rows);
    }

    // ── SourceType ────────────────────────────────────────────────────────

    #[test]
    fn source_type_eq() {
        assert_eq!(SourceType::SqliteTable, SourceType::SqliteTable);
        assert_eq!(SourceType::HttpEndpoint, SourceType::HttpEndpoint);
        assert_ne!(SourceType::SqliteTable, SourceType::HttpEndpoint);
    }

    #[test]
    fn source_type_copy() {
        let t = SourceType::SqliteTable;
        let t2 = t;
        assert_eq!(t, t2);
    }

    #[test]
    fn http_endpoint_sources_are_povm_and_rm() {
        let plan = build_migration_plan();
        let http_names: Vec<&str> = plan
            .sources
            .iter()
            .filter(|s| s.source_type == SourceType::HttpEndpoint)
            .map(|s| s.name.as_str())
            .collect();
        assert_eq!(http_names.len(), 2);
        assert!(http_names.contains(&"povm_pathways"));
        assert!(http_names.contains(&"rm_heartbeat"));
    }

    // ── MigrationStatus ───────────────────────────────────────────────────

    fn make_source_status(status: MigrationStepStatus) -> MigrationSourceStatus {
        MigrationSourceStatus {
            source: MigrationSource {
                name: "test_src".into(),
                source_type: SourceType::SqliteTable,
                target_table: "test_table".into(),
                estimated_rows: 10,
                phase: MigrationPhase::A,
            },
            pre_checksum: None,
            post_checksum: None,
            rows_migrated: 0,
            status,
            error: None,
        }
    }

    #[test]
    fn migration_status_is_complete_all_completed() {
        let status = MigrationStatus {
            sources: vec![
                make_source_status(MigrationStepStatus::Completed),
                make_source_status(MigrationStepStatus::Completed),
            ],
            phase: MigrationPhase::A,
            started_at: "2026-04-24T00:00:00Z".into(),
            completed_at: None,
            total_rows_migrated: 0,
            errors: vec![],
        };
        assert!(status.is_complete());
    }

    #[test]
    fn migration_status_is_complete_mixed_completed_skipped() {
        let status = MigrationStatus {
            sources: vec![
                make_source_status(MigrationStepStatus::Completed),
                make_source_status(MigrationStepStatus::Skipped),
            ],
            phase: MigrationPhase::A,
            started_at: "2026-04-24T00:00:00Z".into(),
            completed_at: None,
            total_rows_migrated: 0,
            errors: vec![],
        };
        assert!(status.is_complete());
    }

    #[test]
    fn migration_status_not_complete_when_pending() {
        let status = MigrationStatus {
            sources: vec![
                make_source_status(MigrationStepStatus::Completed),
                make_source_status(MigrationStepStatus::Pending),
            ],
            phase: MigrationPhase::A,
            started_at: "2026-04-24T00:00:00Z".into(),
            completed_at: None,
            total_rows_migrated: 0,
            errors: vec![],
        };
        assert!(!status.is_complete());
    }

    #[test]
    fn migration_status_not_complete_when_in_progress() {
        let status = MigrationStatus {
            sources: vec![make_source_status(MigrationStepStatus::InProgress)],
            phase: MigrationPhase::B,
            started_at: "2026-04-24T00:00:00Z".into(),
            completed_at: None,
            total_rows_migrated: 0,
            errors: vec![],
        };
        assert!(!status.is_complete());
    }

    #[test]
    fn migration_status_not_complete_when_failed() {
        let status = MigrationStatus {
            sources: vec![make_source_status(MigrationStepStatus::Failed)],
            phase: MigrationPhase::C,
            started_at: "2026-04-24T00:00:00Z".into(),
            completed_at: None,
            total_rows_migrated: 0,
            errors: vec![],
        };
        assert!(!status.is_complete());
    }

    #[test]
    fn migration_status_has_errors_via_error_vec() {
        let status = MigrationStatus {
            sources: vec![make_source_status(MigrationStepStatus::Completed)],
            phase: MigrationPhase::A,
            started_at: "2026-04-24T00:00:00Z".into(),
            completed_at: None,
            total_rows_migrated: 0,
            errors: vec!["checksum skew".into()],
        };
        assert!(status.has_errors());
    }

    #[test]
    fn migration_status_has_errors_via_failed_source() {
        let status = MigrationStatus {
            sources: vec![make_source_status(MigrationStepStatus::Failed)],
            phase: MigrationPhase::D,
            started_at: "2026-04-24T00:00:00Z".into(),
            completed_at: None,
            total_rows_migrated: 0,
            errors: vec![],
        };
        assert!(status.has_errors());
    }

    #[test]
    fn migration_status_no_errors_clean() {
        let status = MigrationStatus {
            sources: vec![
                make_source_status(MigrationStepStatus::Completed),
                make_source_status(MigrationStepStatus::Skipped),
            ],
            phase: MigrationPhase::E,
            started_at: "2026-04-24T00:00:00Z".into(),
            completed_at: Some("2026-04-24T01:00:00Z".into()),
            total_rows_migrated: 100,
            errors: vec![],
        };
        assert!(!status.has_errors());
        assert!(status.is_complete());
    }

    #[test]
    fn migration_status_empty_sources_is_complete() {
        // Vacuously true — no sources, all trivially done.
        let status = MigrationStatus {
            sources: vec![],
            phase: MigrationPhase::A,
            started_at: "2026-04-24T00:00:00Z".into(),
            completed_at: None,
            total_rows_migrated: 0,
            errors: vec![],
        };
        assert!(status.is_complete());
    }

    // ── verify_checksums ──────────────────────────────────────────────────

    fn checksum(name: &str, rows: u64, sum: Option<f64>, avg: Option<f64>) -> MigrationChecksum {
        MigrationChecksum {
            source_name: name.to_owned(),
            row_count: rows,
            weight_sum: sum,
            weight_avg: avg,
        }
    }

    #[test]
    fn verify_checksums_exact_match_no_weights() {
        let pre = checksum("tbl", 100, None, None);
        let post = checksum("tbl", 100, None, None);
        assert!(verify_checksums(&pre, &post, 0.01).is_ok());
    }

    #[test]
    fn verify_checksums_exact_match_with_weights() {
        let pre = checksum("tbl", 50, Some(123.456), Some(2.469_12));
        let post = checksum("tbl", 50, Some(123.456), Some(2.469_12));
        assert!(verify_checksums(&pre, &post, 0.01).is_ok());
    }

    #[test]
    fn verify_checksums_within_tolerance() {
        let pre = checksum("tbl", 50, Some(100.0), Some(2.0));
        let post = checksum("tbl", 50, Some(100.005), Some(2.000_5));
        assert!(verify_checksums(&pre, &post, 0.01).is_ok());
    }

    #[test]
    fn verify_checksums_row_count_mismatch_fails() {
        let pre = checksum("tbl", 100, None, None);
        let post = checksum("tbl", 99, None, None);
        let err = verify_checksums(&pre, &post, 0.01).unwrap_err();
        match err {
            MigrationError::RowCountMismatch {
                source_count,
                target_count,
                ..
            } => {
                assert_eq!(source_count, 100);
                assert_eq!(target_count, 99);
            }
            other => panic!("expected RowCountMismatch, got {other:?}"),
        }
    }

    #[test]
    fn verify_checksums_weight_sum_out_of_tolerance_fails() {
        let pre = checksum("tbl", 10, Some(100.0), None);
        let post = checksum("tbl", 10, Some(100.05), None);
        let err = verify_checksums(&pre, &post, 0.01).unwrap_err();
        assert!(matches!(err, MigrationError::ChecksumMismatch { .. }));
    }

    #[test]
    fn verify_checksums_weight_avg_out_of_tolerance_fails() {
        let pre = checksum("tbl", 5, None, Some(1.0));
        let post = checksum("tbl", 5, None, Some(1.05));
        let err = verify_checksums(&pre, &post, 0.01).unwrap_err();
        assert!(matches!(err, MigrationError::ChecksumMismatch { .. }));
    }

    #[test]
    fn verify_checksums_pre_has_weights_post_none_skips() {
        // If post has no weight column, skip the weight comparison.
        let pre = checksum("tbl", 20, Some(999.0), Some(49.95));
        let post = checksum("tbl", 20, None, None);
        assert!(verify_checksums(&pre, &post, 0.01).is_ok());
    }

    #[test]
    fn verify_checksums_post_has_weights_pre_none_skips() {
        let pre = checksum("tbl", 20, None, None);
        let post = checksum("tbl", 20, Some(999.0), Some(49.95));
        assert!(verify_checksums(&pre, &post, 0.01).is_ok());
    }

    #[test]
    fn verify_checksums_zero_tolerance_exact_only() {
        let pre = checksum("tbl", 10, Some(5.0), None);
        let post = checksum("tbl", 10, Some(5.000_001), None);
        // 1e-6 > 0.0, so fails with tolerance=0.
        let err = verify_checksums(&pre, &post, 0.0).unwrap_err();
        assert!(matches!(err, MigrationError::ChecksumMismatch { .. }));
    }

    #[test]
    fn verify_checksums_large_tolerance_accepts_drift() {
        let pre = checksum("tbl", 1000, Some(1_000_000.0), Some(1000.0));
        let post = checksum("tbl", 1000, Some(1_000_099.0), Some(1000.099));
        assert!(verify_checksums(&pre, &post, 100.0).is_ok());
    }

    #[test]
    fn verify_checksums_row_count_zero_passes() {
        let pre = checksum("empty_tbl", 0, Some(0.0), Some(0.0));
        let post = checksum("empty_tbl", 0, Some(0.0), Some(0.0));
        assert!(verify_checksums(&pre, &post, 0.01).is_ok());
    }

    // ── SQLite checksum tests ─────────────────────────────────────────────

    #[cfg(feature = "sqlite")]
    mod sqlite_tests {
        use crate::m2_schema::m06_schema::open_memory;

        use super::*;

        #[test]
        fn checksum_empty_table_no_weight() {
            let conn = open_memory().unwrap();
            // causal_chain exists from schema but will be empty.
            let cs =
                compute_checksum_from_sqlite(&conn, "causal_chain", None).unwrap();
            assert_eq!(cs.row_count, 0);
            assert!(cs.weight_sum.is_none());
            assert!(cs.weight_avg.is_none());
        }

        #[test]
        fn checksum_empty_table_with_weight_column_returns_zero_aggregates() {
            let conn = open_memory().unwrap();
            // reinforced_pattern has a `weight` column.
            let cs = compute_checksum_from_sqlite(
                &conn,
                "reinforced_pattern",
                Some("weight"),
            )
            .unwrap();
            assert_eq!(cs.row_count, 0);
            assert_eq!(cs.weight_sum, Some(0.0));
            assert_eq!(cs.weight_avg, Some(0.0));
        }

        #[test]
        fn checksum_populated_table_counts_rows() {
            let conn = open_memory().unwrap();
            conn.execute_batch(
                "INSERT INTO causal_chain
                    (origin_session, chain_type, label, description)
                 VALUES (109, 'bug', 'BUG-001', 'test');
                 INSERT INTO causal_chain
                    (origin_session, chain_type, label, description)
                 VALUES (109, 'bug', 'BUG-002', 'test2');",
            )
            .unwrap();
            let cs =
                compute_checksum_from_sqlite(&conn, "causal_chain", None).unwrap();
            assert_eq!(cs.row_count, 2);
        }

        #[test]
        fn checksum_with_weight_column_sum_and_avg() {
            let conn = open_memory().unwrap();
            conn.execute_batch(
                "INSERT INTO reinforced_pattern
                    (pattern_id, category, description, weight)
                 VALUES ('p1', 'procedural', 'desc1', 0.6);
                 INSERT INTO reinforced_pattern
                    (pattern_id, category, description, weight)
                 VALUES ('p2', 'procedural', 'desc2', 0.4);",
            )
            .unwrap();
            let cs = compute_checksum_from_sqlite(
                &conn,
                "reinforced_pattern",
                Some("weight"),
            )
            .unwrap();
            assert_eq!(cs.row_count, 2);
            let sum = cs.weight_sum.unwrap();
            let avg = cs.weight_avg.unwrap();
            assert!((sum - 1.0).abs() < 1e-9, "sum={sum}");
            assert!((avg - 0.5).abs() < 1e-9, "avg={avg}");
        }

        #[test]
        fn checksum_invalid_table_name_with_space_fails() {
            let conn = open_memory().unwrap();
            let err =
                compute_checksum_from_sqlite(&conn, "bad table", None).unwrap_err();
            assert!(matches!(err, MigrationError::SourceReadFailed { .. }));
        }

        #[test]
        fn checksum_invalid_table_name_with_dash_fails() {
            let conn = open_memory().unwrap();
            let err =
                compute_checksum_from_sqlite(&conn, "bad-table", None).unwrap_err();
            assert!(matches!(err, MigrationError::SourceReadFailed { .. }));
        }

        #[test]
        fn checksum_empty_identifier_fails() {
            let conn = open_memory().unwrap();
            let err = compute_checksum_from_sqlite(&conn, "", None).unwrap_err();
            assert!(matches!(err, MigrationError::SourceReadFailed { .. }));
        }

        #[test]
        fn checksum_invalid_weight_column_fails() {
            let conn = open_memory().unwrap();
            let err = compute_checksum_from_sqlite(
                &conn,
                "causal_chain",
                Some("bad col"),
            )
            .unwrap_err();
            assert!(matches!(err, MigrationError::SourceReadFailed { .. }));
        }

        #[test]
        fn checksum_nonexistent_table_fails() {
            let conn = open_memory().unwrap();
            let err =
                compute_checksum_from_sqlite(&conn, "does_not_exist", None).unwrap_err();
            assert!(matches!(err, MigrationError::SourceReadFailed { .. }));
        }

        #[test]
        fn checksum_source_name_equals_table_name() {
            let conn = open_memory().unwrap();
            let cs =
                compute_checksum_from_sqlite(&conn, "causal_chain", None).unwrap();
            assert_eq!(cs.source_name, "causal_chain");
        }

        #[test]
        fn checksum_multiple_rows_no_weight() {
            let conn = open_memory().unwrap();
            for i in 0..5_u32 {
                conn.execute(
                    "INSERT INTO causal_chain
                        (origin_session, chain_type, label, description)
                     VALUES (?1, 'plan', ?2, 'auto')",
                    rusqlite::params![
                        i + 100,
                        format!("PLAN-{i:03}")
                    ],
                )
                .unwrap();
            }
            let cs =
                compute_checksum_from_sqlite(&conn, "causal_chain", None).unwrap();
            assert_eq!(cs.row_count, 5);
            assert!(cs.weight_sum.is_none());
        }

        #[test]
        fn verify_after_double_checksum_passes() {
            let conn = open_memory().unwrap();
            conn.execute_batch(
                "INSERT INTO reinforced_pattern
                    (pattern_id, category, description, weight)
                 VALUES ('x', 'procedural', 'd', 0.7);",
            )
            .unwrap();
            let pre = compute_checksum_from_sqlite(
                &conn,
                "reinforced_pattern",
                Some("weight"),
            )
            .unwrap();
            // Simulate no writes in between — post == pre.
            let post = compute_checksum_from_sqlite(
                &conn,
                "reinforced_pattern",
                Some("weight"),
            )
            .unwrap();
            assert!(verify_checksums(&pre, &post, 0.001).is_ok());
        }
    }

    // ── Serde roundtrips ──────────────────────────────────────────────────

    #[test]
    fn migration_source_serde_roundtrip() {
        let src = MigrationSource {
            name: "learned_patterns".into(),
            source_type: SourceType::SqliteTable,
            target_table: "knowledge_node".into(),
            estimated_rows: 500,
            phase: MigrationPhase::A,
        };
        let json = serde_json::to_string(&src).unwrap();
        let decoded: MigrationSource = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.name, src.name);
        assert_eq!(decoded.estimated_rows, src.estimated_rows);
        assert_eq!(decoded.phase, MigrationPhase::A);
    }

    #[test]
    fn migration_checksum_serde_roundtrip() {
        let cs = MigrationChecksum {
            source_name: "knowledge_node".into(),
            row_count: 3554,
            weight_sum: Some(1234.567),
            weight_avg: Some(0.347),
        };
        let json = serde_json::to_string(&cs).unwrap();
        let decoded: MigrationChecksum = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, cs);
    }

    #[test]
    fn migration_checksum_serde_none_weights() {
        let cs = MigrationChecksum {
            source_name: "service_event".into(),
            row_count: 2000,
            weight_sum: None,
            weight_avg: None,
        };
        let json = serde_json::to_string(&cs).unwrap();
        let decoded: MigrationChecksum = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, cs);
    }

    #[test]
    fn migration_step_status_serde_roundtrip() {
        for status in [
            MigrationStepStatus::Pending,
            MigrationStepStatus::InProgress,
            MigrationStepStatus::Completed,
            MigrationStepStatus::Failed,
            MigrationStepStatus::Skipped,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let decoded: MigrationStepStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, status);
        }
    }

    #[test]
    fn migration_phase_serde_roundtrip() {
        for phase in MigrationPhase::all() {
            let json = serde_json::to_string(&phase).unwrap();
            let decoded: MigrationPhase = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, phase);
        }
    }

    #[test]
    fn migration_plan_serde_roundtrip() {
        let plan = build_migration_plan();
        let json = serde_json::to_string(&plan).unwrap();
        let decoded: MigrationPlan = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.sources.len(), plan.sources.len());
        assert_eq!(decoded.total_estimated_rows, plan.total_estimated_rows);
        assert_eq!(decoded.phases.len(), plan.phases.len());
    }

    #[test]
    fn migration_status_serde_roundtrip() {
        let status = MigrationStatus {
            sources: vec![make_source_status(MigrationStepStatus::Completed)],
            phase: MigrationPhase::B,
            started_at: "2026-04-24T00:00:00Z".into(),
            completed_at: Some("2026-04-24T01:00:00Z".into()),
            total_rows_migrated: 42,
            errors: vec!["non-fatal drift".into()],
        };
        let json = serde_json::to_string(&status).unwrap();
        let decoded: MigrationStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.total_rows_migrated, 42);
        assert_eq!(decoded.errors.len(), 1);
    }

    #[test]
    fn source_type_serde_roundtrip() {
        let t = SourceType::HttpEndpoint;
        let json = serde_json::to_string(&t).unwrap();
        let decoded: SourceType = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, t);
    }

    // ── Debug impls ────────────────────────────────────────────────────────

    #[test]
    fn migration_source_debug_not_empty() {
        let src = build_migration_plan().sources[0].clone();
        let dbg = format!("{src:?}");
        assert!(!dbg.is_empty());
        assert!(dbg.contains("learned_patterns"));
    }

    #[test]
    fn migration_plan_debug_not_empty() {
        let plan = build_migration_plan();
        let dbg = format!("{plan:?}");
        assert!(!dbg.is_empty());
    }

    #[test]
    fn migration_status_debug_not_empty() {
        let status = MigrationStatus {
            sources: vec![],
            phase: MigrationPhase::A,
            started_at: "2026-04-24T00:00:00Z".into(),
            completed_at: None,
            total_rows_migrated: 0,
            errors: vec![],
        };
        assert!(!format!("{status:?}").is_empty());
    }
}
