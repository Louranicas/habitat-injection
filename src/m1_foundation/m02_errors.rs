//! `m02_errors` — Error taxonomy for the habitat-injection system.
//!
//! Six domain-specific error enums (`InjectionError`, `ConsolidationError`,
//! `SchemaError`, `QueryError`, `MigrationError`, `ConfigError`), all
//! `#[non_exhaustive]` for forward-compatible extension. A unified
//! [`HabitatError`] wraps them via `#[from]` for ergonomic `?` propagation.
//!
//! ## Layer
//!
//! `m1_foundation`
//!
//! ## Dependencies
//!
//! - [`crate::m1_foundation::m01_types::SessionId`] — used in error context fields
//!
//! ## Invariants
//!
//! - All error enums are `Send + Sync` for cross-task propagation.
//! - All error enums are `#[non_exhaustive]` — downstream `match` must include `_`.
//! - [`HabitatError::kind()`] is exhaustive over all variants.

use std::fmt;
use std::path::PathBuf;

use crate::m1_foundation::m01_types::SessionId;

/// Top-level error — wraps all domain errors for cross-layer use.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum HabitatError {
    #[error(transparent)]
    Injection(#[from] InjectionError),

    #[error(transparent)]
    Consolidation(#[from] ConsolidationError),

    #[error(transparent)]
    Schema(#[from] SchemaError),

    #[error(transparent)]
    Query(#[from] QueryError),

    #[error(transparent)]
    Migration(#[from] MigrationError),

    #[error(transparent)]
    Config(#[from] ConfigError),
}

// ---------------------------------------------------------------------------
// InjectionError
// ---------------------------------------------------------------------------

/// Errors during context-window injection (L3).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum InjectionError {
    /// Token budget exhausted before rendering all sections.
    #[error("token budget exhausted: {used} of {budget} used, {section} skipped")]
    BudgetExhausted {
        budget: u32,
        used: u32,
        section: String,
    },

    /// Database query failed during injection.
    #[error("injection query failed: {0}")]
    QueryFailed(String),

    /// All fallback tiers exhausted (`SQLite` → atuin KV → static).
    #[error("all fallback tiers exhausted: sqlite={sqlite_err}, atuin={atuin_err}")]
    AllFallbacksExhausted {
        sqlite_err: String,
        atuin_err: String,
    },

    /// Consent filter blocked injection for a sphere.
    #[error("consent filter blocked injection for sphere {sphere_id}")]
    ConsentBlocked { sphere_id: String },

    /// Payload exceeds maximum size.
    #[error("payload exceeds max size: {size} bytes > {max} bytes")]
    PayloadTooLarge { size: usize, max: usize },

    /// Injection timed out.
    #[error("injection timed out after {elapsed_ms}ms (budget: {budget_ms}ms)")]
    Timeout { elapsed_ms: u64, budget_ms: u64 },
}

// ---------------------------------------------------------------------------
// ConsolidationError
// ---------------------------------------------------------------------------

/// Errors during post-session consolidation (L4).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ConsolidationError {
    /// Checkpoint ingestion failed.
    #[error("checkpoint ingest failed for '{label}': {reason}")]
    CheckpointIngestFailed { label: String, reason: String },

    /// Trajectory capture failed.
    #[error("trajectory capture failed for session {session}: {reason}")]
    TrajectoryCaptureFailed { session: SessionId, reason: String },

    /// Workstream update failed.
    #[error("workstream update failed: {0}")]
    WorkstreamUpdateFailed(String),

    /// Causal chain reinforcement failed.
    #[error("chain reinforcement failed for chain {chain_id}: {reason}")]
    ChainReinforcementFailed { chain_id: u64, reason: String },

    /// Hebbian decay cycle failed.
    #[error("hebbian decay failed: {0}")]
    DecayFailed(String),

    /// Pattern reinforcement failed.
    #[error("pattern reinforce failed for '{pattern_id}': {reason}")]
    ReinforceFailed { pattern_id: String, reason: String },

    /// Cache rebuild failed.
    #[error("injection cache rebuild failed: {0}")]
    CacheRebuildFailed(String),

    /// Atuin KV write failed.
    #[error("atuin cache write failed for key {key}: {reason}")]
    AtuinWriteFailed { key: String, reason: String },

    /// Auto-resolve found no eligible chains.
    #[error("auto-resolve found no chains inactive for {threshold} sessions")]
    NoStaleChains { threshold: u32 },
}

// ---------------------------------------------------------------------------
// SchemaError
// ---------------------------------------------------------------------------

/// Errors in schema creation and migration (L2).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SchemaError {
    /// Database file could not be opened or created.
    #[error("database open failed at {path}: {reason}")]
    DatabaseOpenFailed { path: PathBuf, reason: String },

    /// Schema migration failed.
    #[error("migration {version} failed: {reason}")]
    MigrationFailed { version: u32, reason: String },

    /// Table creation failed.
    #[error("table creation failed for {table}: {reason}")]
    TableCreationFailed { table: String, reason: String },

    /// Index creation failed.
    #[error("index creation failed for {index} on {table}: {reason}")]
    IndexCreationFailed {
        table: String,
        index: String,
        reason: String,
    },

    /// Schema version mismatch.
    #[error("schema version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: u32, found: u32 },

    /// `SQLite` error passthrough.
    #[error("sqlite error: {0}")]
    Sqlite(String),
}

// ---------------------------------------------------------------------------
// QueryError
// ---------------------------------------------------------------------------

/// Errors during query execution (L5).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum QueryError {
    /// SQL syntax or execution error.
    #[error("query execution failed: {0}")]
    ExecutionFailed(String),

    /// No rows returned when at least one expected.
    #[error("query returned no results for: {query}")]
    NoResults { query: String },

    /// Row parsing failed.
    #[error("row parse failed at column {column}: {reason}")]
    ParseFailed { column: String, reason: String },

    /// Query timed out.
    #[error("query timed out after {elapsed_ms}ms")]
    Timeout { elapsed_ms: u64 },

    /// FZF subprocess failed.
    #[error("fzf subprocess failed: {0}")]
    FzfFailed(String),

    /// Raw SQL passthrough is disabled in this context.
    #[error("raw SQL passthrough is not allowed in this context")]
    RawSqlDisallowed,
}

// ---------------------------------------------------------------------------
// MigrationError
// ---------------------------------------------------------------------------

/// Errors during STDB migration (L6).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum MigrationError {
    /// STDB connection failed.
    #[error("STDB connection failed at {endpoint}: {reason}")]
    ConnectionFailed { endpoint: String, reason: String },

    /// Source data read failed.
    #[error("source read failed from {origin}: {reason}")]
    SourceReadFailed { origin: String, reason: String },

    /// Row count mismatch after migration.
    #[error("row count mismatch for {table}: source={source_count}, target={target_count}")]
    RowCountMismatch {
        table: String,
        source_count: u64,
        target_count: u64,
    },

    /// Weight checksum mismatch after migration.
    #[error("weight checksum mismatch for {table}: source={source_sum:.6}, target={target_sum:.6}")]
    ChecksumMismatch {
        table: String,
        source_sum: f64,
        target_sum: f64,
    },

    /// Dual-write phase transition failed.
    #[error("dual-write transition failed: {0}")]
    DualWriteTransitionFailed(String),

    /// STDB reducer call failed.
    #[error("reducer {reducer} failed: {reason}")]
    ReducerFailed { reducer: String, reason: String },
}

// ---------------------------------------------------------------------------
// ConfigError
// ---------------------------------------------------------------------------

/// Errors in configuration loading (L1).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// Config file not found.
    #[error("config file not found at {path}")]
    FileNotFound { path: PathBuf },

    /// Config parse failed.
    #[error("config parse failed: {0}")]
    ParseFailed(String),

    /// Environment variable override invalid.
    #[error("invalid env override for {key}: {reason}")]
    InvalidEnvOverride { key: String, reason: String },

    /// Required field missing.
    #[error("required config field missing: {field}")]
    MissingField { field: String },
}

// ---------------------------------------------------------------------------
// Display helper — human-readable context for error chains
// ---------------------------------------------------------------------------

/// Formats an error chain for tracing/logging output.
pub fn format_error_chain(err: &dyn std::error::Error) -> String {
    let mut chain = String::new();
    chain.push_str(&err.to_string());
    let mut source = err.source();
    while let Some(cause) = source {
        chain.push_str(" → ");
        chain.push_str(&cause.to_string());
        source = cause.source();
    }
    chain
}

/// Convenience result type using [`HabitatError`].
pub type Result<T> = std::result::Result<T, HabitatError>;

// ---------------------------------------------------------------------------
// ErrorKind — coarse classification for metrics/reporting
// ---------------------------------------------------------------------------

/// Coarse error classification for metrics and reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    Injection,
    Consolidation,
    Schema,
    Query,
    Migration,
    Config,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Injection => f.write_str("injection"),
            Self::Consolidation => f.write_str("consolidation"),
            Self::Schema => f.write_str("schema"),
            Self::Query => f.write_str("query"),
            Self::Migration => f.write_str("migration"),
            Self::Config => f.write_str("config"),
        }
    }
}

impl HabitatError {
    /// Returns the coarse [`ErrorKind`] for metrics/reporting.
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        match self {
            Self::Injection(_) => ErrorKind::Injection,
            Self::Consolidation(_) => ErrorKind::Consolidation,
            Self::Schema(_) => ErrorKind::Schema,
            Self::Query(_) => ErrorKind::Query,
            Self::Migration(_) => ErrorKind::Migration,
            Self::Config(_) => ErrorKind::Config,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- HabitatError --

    #[test]
    fn habitat_error_from_injection() {
        let err = InjectionError::BudgetExhausted {
            budget: 1100,
            used: 1100,
            section: "trajectory".into(),
        };
        let habitat: HabitatError = err.into();
        assert_eq!(habitat.kind(), ErrorKind::Injection);
    }

    #[test]
    fn habitat_error_from_consolidation() {
        let err = ConsolidationError::DecayFailed("test".into());
        let habitat: HabitatError = err.into();
        assert_eq!(habitat.kind(), ErrorKind::Consolidation);
    }

    #[test]
    fn habitat_error_from_schema() {
        let err = SchemaError::Sqlite("test".into());
        let habitat: HabitatError = err.into();
        assert_eq!(habitat.kind(), ErrorKind::Schema);
    }

    #[test]
    fn habitat_error_from_query() {
        let err = QueryError::ExecutionFailed("test".into());
        let habitat: HabitatError = err.into();
        assert_eq!(habitat.kind(), ErrorKind::Query);
    }

    #[test]
    fn habitat_error_from_migration() {
        let err = MigrationError::ConnectionFailed {
            endpoint: "localhost:3000".into(),
            reason: "refused".into(),
        };
        let habitat: HabitatError = err.into();
        assert_eq!(habitat.kind(), ErrorKind::Migration);
    }

    #[test]
    fn habitat_error_from_config() {
        let err = ConfigError::MissingField {
            field: "db_path".into(),
        };
        let habitat: HabitatError = err.into();
        assert_eq!(habitat.kind(), ErrorKind::Config);
    }

    // -- InjectionError --

    #[test]
    fn injection_budget_exhausted_display() {
        let err = InjectionError::BudgetExhausted {
            budget: 1100,
            used: 1100,
            section: "workstreams".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("1100"));
        assert!(msg.contains("workstreams"));
    }

    #[test]
    fn injection_query_failed_display() {
        let err = InjectionError::QueryFailed("no such table".into());
        assert!(err.to_string().contains("no such table"));
    }

    #[test]
    fn injection_all_fallbacks_display() {
        let err = InjectionError::AllFallbacksExhausted {
            sqlite_err: "locked".into(),
            atuin_err: "timeout".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("locked"));
        assert!(msg.contains("timeout"));
    }

    #[test]
    fn injection_consent_blocked_display() {
        let err = InjectionError::ConsentBlocked {
            sphere_id: "sphere-7".into(),
        };
        assert!(err.to_string().contains("sphere-7"));
    }

    #[test]
    fn injection_payload_too_large_display() {
        let err = InjectionError::PayloadTooLarge {
            size: 20_000,
            max: 15_360,
        };
        let msg = err.to_string();
        assert!(msg.contains("20000"));
        assert!(msg.contains("15360"));
    }

    #[test]
    fn injection_timeout_display() {
        let err = InjectionError::Timeout {
            elapsed_ms: 150,
            budget_ms: 100,
        };
        let msg = err.to_string();
        assert!(msg.contains("150"));
        assert!(msg.contains("100"));
    }

    // -- ConsolidationError --

    #[test]
    fn consolidation_checkpoint_ingest_display() {
        let err = ConsolidationError::CheckpointIngestFailed {
            label: "s109-close".into(),
            reason: "duplicate label".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("s109-close"));
        assert!(msg.contains("duplicate label"));
    }

    #[test]
    fn consolidation_reinforce_failed_display() {
        let err = ConsolidationError::ReinforceFailed {
            pattern_id: "verify-before-ship".into(),
            reason: "db locked".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("verify-before-ship"));
        assert!(msg.contains("db locked"));
    }

    #[test]
    fn consolidation_trajectory_display() {
        let err = ConsolidationError::TrajectoryCaptureFailed {
            session: SessionId::new(109),
            reason: "db locked".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("S109"));
        assert!(msg.contains("db locked"));
    }

    #[test]
    fn consolidation_workstream_display() {
        let err = ConsolidationError::WorkstreamUpdateFailed("not found".into());
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn consolidation_chain_display() {
        let err = ConsolidationError::ChainReinforcementFailed {
            chain_id: 42,
            reason: "concurrent write".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("42"));
        assert!(msg.contains("concurrent write"));
    }

    #[test]
    fn consolidation_decay_display() {
        let err = ConsolidationError::DecayFailed("timeout".into());
        assert!(err.to_string().contains("timeout"));
    }

    #[test]
    fn consolidation_cache_rebuild_display() {
        let err = ConsolidationError::CacheRebuildFailed("disk full".into());
        assert!(err.to_string().contains("disk full"));
    }

    #[test]
    fn consolidation_atuin_display() {
        let err = ConsolidationError::AtuinWriteFailed {
            key: "session.109".into(),
            reason: "permission denied".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("session.109"));
        assert!(msg.contains("permission denied"));
    }

    #[test]
    fn consolidation_no_stale_chains_display() {
        let err = ConsolidationError::NoStaleChains { threshold: 10 };
        assert!(err.to_string().contains("10"));
    }

    // -- SchemaError --

    #[test]
    fn schema_db_open_display() {
        let err = SchemaError::DatabaseOpenFailed {
            path: PathBuf::from("/tmp/test.db"),
            reason: "permission denied".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("/tmp/test.db"));
        assert!(msg.contains("permission denied"));
    }

    #[test]
    fn schema_migration_display() {
        let err = SchemaError::MigrationFailed {
            version: 3,
            reason: "syntax error".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("3"));
        assert!(msg.contains("syntax error"));
    }

    #[test]
    fn schema_table_creation_display() {
        let err = SchemaError::TableCreationFailed {
            table: "causal_chain".into(),
            reason: "already exists".into(),
        };
        assert!(err.to_string().contains("causal_chain"));
    }

    #[test]
    fn schema_index_creation_display() {
        let err = SchemaError::IndexCreationFailed {
            table: "causal_chain".into(),
            index: "idx_label".into(),
            reason: "duplicate".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("idx_label"));
        assert!(msg.contains("causal_chain"));
    }

    #[test]
    fn schema_version_mismatch_display() {
        let err = SchemaError::VersionMismatch {
            expected: 5,
            found: 3,
        };
        let msg = err.to_string();
        assert!(msg.contains("5"));
        assert!(msg.contains("3"));
    }

    #[test]
    fn schema_sqlite_display() {
        let err = SchemaError::Sqlite("database is locked".into());
        assert!(err.to_string().contains("database is locked"));
    }

    // -- QueryError --

    #[test]
    fn query_execution_display() {
        let err = QueryError::ExecutionFailed("near \"SELEC\": syntax error".into());
        assert!(err.to_string().contains("SELEC"));
    }

    #[test]
    fn query_no_results_display() {
        let err = QueryError::NoResults {
            query: "SELECT * FROM causal_chain WHERE id = 999".into(),
        };
        assert!(err.to_string().contains("999"));
    }

    #[test]
    fn query_parse_display() {
        let err = QueryError::ParseFailed {
            column: "weight".into(),
            reason: "not a float".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("weight"));
        assert!(msg.contains("not a float"));
    }

    #[test]
    fn query_timeout_display() {
        let err = QueryError::Timeout { elapsed_ms: 5000 };
        assert!(err.to_string().contains("5000"));
    }

    #[test]
    fn query_fzf_display() {
        let err = QueryError::FzfFailed("not installed".into());
        assert!(err.to_string().contains("not installed"));
    }

    #[test]
    fn query_raw_sql_disallowed_display() {
        let err = QueryError::RawSqlDisallowed;
        assert!(err.to_string().contains("not allowed"));
    }

    // -- MigrationError --

    #[test]
    fn migration_connection_display() {
        let err = MigrationError::ConnectionFailed {
            endpoint: "localhost:3000".into(),
            reason: "connection refused".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("localhost:3000"));
        assert!(msg.contains("connection refused"));
    }

    #[test]
    fn migration_source_read_display() {
        let err = MigrationError::SourceReadFailed {
            origin: "povm:8125".into(),
            reason: "timeout".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("povm:8125"));
        assert!(msg.contains("timeout"));
    }

    #[test]
    fn migration_row_count_display() {
        let err = MigrationError::RowCountMismatch {
            table: "knowledge_edge".into(),
            source_count: 3554,
            target_count: 3550,
        };
        let msg = err.to_string();
        assert!(msg.contains("3554"));
        assert!(msg.contains("3550"));
    }

    #[test]
    fn migration_checksum_display() {
        let err = MigrationError::ChecksumMismatch {
            table: "knowledge_edge".into(),
            source_sum: 1234.567_89,
            target_sum: 1234.567_80,
        };
        assert!(err.to_string().contains("knowledge_edge"));
    }

    #[test]
    fn migration_dual_write_display() {
        let err = MigrationError::DualWriteTransitionFailed("phase 2 aborted".into());
        assert!(err.to_string().contains("phase 2"));
    }

    #[test]
    fn migration_reducer_display() {
        let err = MigrationError::ReducerFailed {
            reducer: "ingest_event".into(),
            reason: "serialization error".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("ingest_event"));
        assert!(msg.contains("serialization error"));
    }

    // -- ConfigError --

    #[test]
    fn config_file_not_found_display() {
        let err = ConfigError::FileNotFound {
            path: PathBuf::from("/etc/habitat/injection.toml"),
        };
        assert!(err.to_string().contains("injection.toml"));
    }

    #[test]
    fn config_parse_display() {
        let err = ConfigError::ParseFailed("invalid TOML".into());
        assert!(err.to_string().contains("invalid TOML"));
    }

    #[test]
    fn config_invalid_env_display() {
        let err = ConfigError::InvalidEnvOverride {
            key: "HABITAT_DECAY_RATE".into(),
            reason: "not a number".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("HABITAT_DECAY_RATE"));
        assert!(msg.contains("not a number"));
    }

    #[test]
    fn config_missing_field_display() {
        let err = ConfigError::MissingField {
            field: "db_path".into(),
        };
        assert!(err.to_string().contains("db_path"));
    }

    // -- ErrorKind --

    #[test]
    fn error_kind_display() {
        assert_eq!(ErrorKind::Injection.to_string(), "injection");
        assert_eq!(ErrorKind::Consolidation.to_string(), "consolidation");
        assert_eq!(ErrorKind::Schema.to_string(), "schema");
        assert_eq!(ErrorKind::Query.to_string(), "query");
        assert_eq!(ErrorKind::Migration.to_string(), "migration");
        assert_eq!(ErrorKind::Config.to_string(), "config");
    }

    // -- format_error_chain --

    #[test]
    fn format_error_chain_single() {
        let err = QueryError::ExecutionFailed("bad sql".into());
        let chain = format_error_chain(&err);
        assert_eq!(chain, "query execution failed: bad sql");
    }

    #[test]
    fn format_error_chain_nested() {
        let inner = InjectionError::QueryFailed("db locked".into());
        let outer: HabitatError = inner.into();
        let chain = format_error_chain(&outer);
        assert!(chain.contains("db locked"));
    }

    // -- Result type alias --

    #[test]
    fn result_type_works() {
        let ok: Result<u32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: Result<u32> = Err(SchemaError::Sqlite("test".into()).into());
        assert!(err.is_err());
    }

    // -- Thread safety --

    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    #[test]
    fn habitat_error_is_send() {
        assert_send::<HabitatError>();
    }

    #[test]
    fn habitat_error_is_sync() {
        assert_sync::<HabitatError>();
    }

    #[test]
    fn injection_error_is_send_sync() {
        assert_send::<InjectionError>();
        assert_sync::<InjectionError>();
    }

    #[test]
    fn consolidation_error_is_send_sync() {
        assert_send::<ConsolidationError>();
        assert_sync::<ConsolidationError>();
    }

    #[test]
    fn schema_error_is_send_sync() {
        assert_send::<SchemaError>();
        assert_sync::<SchemaError>();
    }

    #[test]
    fn query_error_is_send_sync() {
        assert_send::<QueryError>();
        assert_sync::<QueryError>();
    }

    #[test]
    fn migration_error_is_send_sync() {
        assert_send::<MigrationError>();
        assert_sync::<MigrationError>();
    }

    #[test]
    fn config_error_is_send_sync() {
        assert_send::<ConfigError>();
        assert_sync::<ConfigError>();
    }

    // -- Error transparency --

    #[test]
    fn habitat_error_transparent_display_matches_inner() {
        let inner = InjectionError::QueryFailed("inner query".into());
        let inner_msg = inner.to_string();
        let outer: HabitatError = inner.into();
        assert_eq!(outer.to_string(), inner_msg);
    }

    #[test]
    fn habitat_error_transparent_schema_display() {
        let inner = SchemaError::Sqlite("locked".into());
        let inner_msg = inner.to_string();
        let outer: HabitatError = inner.into();
        assert_eq!(outer.to_string(), inner_msg);
    }

    // -- ErrorKind identity --

    #[test]
    fn error_kind_eq() {
        assert_eq!(ErrorKind::Injection, ErrorKind::Injection);
        assert_ne!(ErrorKind::Injection, ErrorKind::Query);
    }

    #[test]
    fn error_kind_hash_consistent() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ErrorKind::Schema);
        set.insert(ErrorKind::Schema);
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn error_kind_clone() {
        let k = ErrorKind::Migration;
        let k2 = k;
        assert_eq!(k, k2);
    }

    #[test]
    fn error_kind_debug() {
        let dbg = format!("{:?}", ErrorKind::Config);
        assert_eq!(dbg, "Config");
    }

    // -- format_error_chain edge cases --

    #[test]
    fn format_error_chain_preserves_full_message() {
        let err = InjectionError::BudgetExhausted {
            budget: 500,
            used: 600,
            section: "chains".into(),
        };
        let chain = format_error_chain(&err);
        assert!(chain.contains("500"));
        assert!(chain.contains("600"));
        assert!(chain.contains("chains"));
    }

    #[test]
    fn format_error_chain_with_habitat_wrapper() {
        let inner = ConsolidationError::DecayFailed("rate negative".into());
        let outer: HabitatError = inner.into();
        let chain = format_error_chain(&outer);
        assert!(chain.contains("rate negative"));
    }

    // -- Display does not panic on empty strings --

    #[test]
    fn consolidation_checkpoint_ingest_empty_fields() {
        let err = ConsolidationError::CheckpointIngestFailed {
            label: String::new(),
            reason: String::new(),
        };
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn consolidation_reinforce_failed_empty_fields() {
        let err = ConsolidationError::ReinforceFailed {
            pattern_id: String::new(),
            reason: String::new(),
        };
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn injection_empty_section() {
        let err = InjectionError::BudgetExhausted {
            budget: 0,
            used: 0,
            section: String::new(),
        };
        let msg = err.to_string();
        assert!(!msg.is_empty());
    }

    #[test]
    fn schema_empty_reason() {
        let err = SchemaError::MigrationFailed {
            version: 0,
            reason: String::new(),
        };
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn query_empty_query_string() {
        let err = QueryError::NoResults {
            query: String::new(),
        };
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn migration_empty_reducer() {
        let err = MigrationError::ReducerFailed {
            reducer: String::new(),
            reason: String::new(),
        };
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn config_empty_field() {
        let err = ConfigError::MissingField {
            field: String::new(),
        };
        assert!(!err.to_string().is_empty());
    }

    // -- Cross-variant kind mapping exhaustiveness --

    #[test]
    fn all_error_kinds_reachable() {
        let variants: Vec<HabitatError> = vec![
            InjectionError::QueryFailed("t".into()).into(),
            ConsolidationError::DecayFailed("t".into()).into(),
            SchemaError::Sqlite("t".into()).into(),
            QueryError::ExecutionFailed("t".into()).into(),
            MigrationError::DualWriteTransitionFailed("t".into()).into(),
            ConfigError::MissingField { field: "t".into() }.into(),
        ];
        let kinds: Vec<ErrorKind> = variants.iter().map(HabitatError::kind).collect();
        assert_eq!(kinds.len(), 6);
        let unique: std::collections::HashSet<ErrorKind> = kinds.into_iter().collect();
        assert_eq!(unique.len(), 6);
    }

    // -- Debug formatting --

    #[test]
    fn habitat_error_debug_not_empty() {
        let err: HabitatError = QueryError::Timeout { elapsed_ms: 42 }.into();
        let dbg = format!("{err:?}");
        assert!(!dbg.is_empty());
        assert!(dbg.contains("42"));
    }

    #[test]
    fn injection_error_debug_contains_variant() {
        let err = InjectionError::ConsentBlocked {
            sphere_id: "s1".into(),
        };
        let dbg = format!("{err:?}");
        assert!(dbg.contains("ConsentBlocked"));
    }
}
