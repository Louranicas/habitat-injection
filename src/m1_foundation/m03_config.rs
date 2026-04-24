//! `m03_config` — Configuration for the habitat-injection system.
//!
//! Loads from TOML file with environment variable overlay.
//! Defaults are sane — runs without a config file.
//!
//! ## Layer
//!
//! `m1_foundation`
//!
//! ## Dependencies
//!
//! - [`crate::m1_foundation::m02_errors::ConfigError`] — error type for load/validate
//! - [`crate::m1_foundation::m05_constants`] — all default values
//!
//! ## Invariants
//!
//! - `Config::default()` always passes `validate()`.
//! - `decay_rate`, `reinforce_rate`, `prune_threshold` must be in `(0.0, 1.0)`.
//! - `envelope_days < delete_days` and `gradient_hourly_days < gradient_daily_days`.
//! - Environment variable overrides silently ignore unparseable values.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::m1_foundation::m02_errors::ConfigError;
use crate::m1_foundation::m05_constants;

/// Top-level configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub injection: InjectionConfig,
    pub consolidation: ConsolidationConfig,
    pub retention: RetentionConfig,
    pub services: ServicesConfig,
}

/// Database path and connection settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Path to the `SQLite` database file.
    pub path: PathBuf,
    /// Enable WAL mode for concurrent reads during injection.
    pub wal_mode: bool,
    /// Busy timeout in milliseconds.
    pub busy_timeout_ms: u32,
}

/// Injection engine settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectionConfig {
    /// Token budget for bootstrap payload.
    pub token_budget: u32,
    /// Maximum payload size in bytes.
    pub max_payload_bytes: usize,
    /// Maximum injection latency in milliseconds.
    pub max_latency_ms: u64,
    /// Maximum causal chains per injection.
    pub max_chains: usize,
    /// Maximum patterns per injection.
    pub max_patterns: usize,
    /// Maximum trajectory points per injection.
    pub max_trajectory_points: usize,
    /// Maximum workstreams per injection.
    pub max_workstreams: usize,
}

/// Consolidation engine settings (decay, reinforcement).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationConfig {
    /// Hebbian decay factor for unfired patterns.
    pub decay_rate: f64,
    /// Reinforcement rate for fired patterns.
    pub reinforce_rate: f64,
    /// Sessions of inactivity before auto-resolve.
    pub auto_resolve_sessions: u32,
    /// Cache rebuild interval in seconds.
    pub cache_rebuild_secs: u64,
    /// Weight threshold for pruning.
    pub prune_threshold: f64,
    /// Decay scheduler interval in seconds.
    pub decay_interval_secs: u64,
}

/// Data retention policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionConfig {
    /// Days before event payloads are stripped to envelopes.
    pub envelope_days: u32,
    /// Days before events are fully deleted.
    pub delete_days: u32,
    /// Days before gradient snapshots downsample to hourly.
    pub gradient_hourly_days: u32,
    /// Days before gradient snapshots downsample to daily.
    pub gradient_daily_days: u32,
    /// Compaction interval in seconds.
    pub compaction_interval_secs: u64,
}

/// Service endpoints for polling / ingestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicesConfig {
    pub stdb_port: u16,
    pub ingester_health_port: u16,
    pub orac_poll_secs: u64,
    pub synthex_poll_secs: u64,
    pub povm_sync_secs: u64,
    pub gradient_capture_secs: u64,
}


impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: default_db_path(),
            wal_mode: true,
            busy_timeout_ms: 5000,
        }
    }
}

impl Default for InjectionConfig {
    fn default() -> Self {
        Self {
            token_budget: m05_constants::DEFAULT_BUDGET.as_u32(),
            max_payload_bytes: m05_constants::MAX_PAYLOAD_BYTES,
            max_latency_ms: m05_constants::MAX_INJECTION_LATENCY_MS,
            max_chains: m05_constants::MAX_CHAINS_INJECTED,
            max_patterns: m05_constants::MAX_PATTERNS_INJECTED,
            max_trajectory_points: m05_constants::MAX_TRAJECTORY_POINTS,
            max_workstreams: m05_constants::MAX_WORKSTREAMS,
        }
    }
}

impl Default for ConsolidationConfig {
    fn default() -> Self {
        Self {
            decay_rate: m05_constants::DECAY_RATE,
            reinforce_rate: m05_constants::REINFORCE_RATE,
            auto_resolve_sessions: m05_constants::AUTO_RESOLVE_SESSIONS,
            cache_rebuild_secs: m05_constants::CACHE_REBUILD_SECS,
            prune_threshold: m05_constants::PRUNE_THRESHOLD,
            decay_interval_secs: m05_constants::DECAY_INTERVAL_SECS,
        }
    }
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            envelope_days: m05_constants::RETENTION_ENVELOPE_DAYS,
            delete_days: m05_constants::RETENTION_DELETE_DAYS,
            gradient_hourly_days: m05_constants::GRADIENT_DOWNSAMPLE_HOURLY_DAYS,
            gradient_daily_days: m05_constants::GRADIENT_DOWNSAMPLE_DAILY_DAYS,
            compaction_interval_secs: m05_constants::COMPACTION_INTERVAL_SECS,
        }
    }
}

impl Default for ServicesConfig {
    fn default() -> Self {
        Self {
            stdb_port: m05_constants::STDB_PORT,
            ingester_health_port: m05_constants::INGESTER_HEALTH_PORT,
            orac_poll_secs: m05_constants::ORAC_POLL_INTERVAL_SECS,
            synthex_poll_secs: m05_constants::SYNTHEX_POLL_INTERVAL_SECS,
            povm_sync_secs: m05_constants::POVM_SYNC_INTERVAL_SECS,
            gradient_capture_secs: m05_constants::GRADIENT_CAPTURE_INTERVAL_SECS,
        }
    }
}

impl Config {
    /// Load config from a TOML file, falling back to defaults for missing fields.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::FileNotFound` if `path` does not exist.
    /// Returns `ConfigError::ParseFailed` on malformed TOML.
    pub fn from_file(path: &Path) -> std::result::Result<Self, ConfigError> {
        if !path.exists() {
            return Err(ConfigError::FileNotFound {
                path: path.to_path_buf(),
            });
        }
        let contents = std::fs::read_to_string(path).map_err(|e| ConfigError::ParseFailed(e.to_string()))?;
        let config: Self =
            toml::from_str(&contents).map_err(|e| ConfigError::ParseFailed(e.to_string()))?;
        Ok(config)
    }

    /// Load config from a TOML file if it exists, otherwise use defaults.
    /// Then apply environment variable overrides.
    ///
    /// If the file exists but cannot be parsed, a warning is printed to stderr
    /// and the default configuration is used. (`tracing` may not be initialized
    /// at config-load time, so `eprintln!` is used directly.)
    #[must_use]
    pub fn load(path: Option<&Path>) -> Self {
        let mut config = match path {
            Some(p) if p.exists() => match Self::from_file(p) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("habitat-injection: config load failed ({e}), using defaults");
                    Self::default()
                }
            },
            _ => Self::default(),
        };
        config.apply_env_overrides();
        config
    }

    /// Apply environment variable overrides.
    ///
    /// Supported variables:
    /// - `HABITAT_DB_PATH` → `database.path`
    /// - `HABITAT_TOKEN_BUDGET` → `injection.token_budget`
    /// - `HABITAT_DECAY_RATE` → `consolidation.decay_rate`
    /// - `HABITAT_REINFORCE_RATE` → `consolidation.reinforce_rate`
    /// - `HABITAT_STDB_PORT` → `services.stdb_port`
    pub fn apply_env_overrides(&mut self) {
        if let Ok(val) = std::env::var("HABITAT_DB_PATH") {
            self.database.path = PathBuf::from(val);
        }
        if let Some(n) = std::env::var("HABITAT_TOKEN_BUDGET").ok().and_then(|v| v.parse::<u32>().ok()) {
            self.injection.token_budget = n;
        }
        if let Some(n) = std::env::var("HABITAT_DECAY_RATE")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|&v| v > 0.0 && v < 1.0)
        {
            self.consolidation.decay_rate = n;
        }
        if let Some(n) = std::env::var("HABITAT_REINFORCE_RATE")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|&v| v > 0.0 && v < 1.0)
        {
            self.consolidation.reinforce_rate = n;
        }
        if let Some(n) = std::env::var("HABITAT_STDB_PORT").ok().and_then(|v| v.parse::<u16>().ok()) {
            self.services.stdb_port = n;
        }
    }

    /// Validate all config values are within sane ranges.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::InvalidEnvOverride` for out-of-range values.
    pub fn validate(&self) -> std::result::Result<(), ConfigError> {
        if self.consolidation.decay_rate <= 0.0 || self.consolidation.decay_rate >= 1.0 {
            return Err(ConfigError::InvalidEnvOverride {
                key: "decay_rate".into(),
                reason: format!("must be in (0, 1), got {}", self.consolidation.decay_rate),
            });
        }
        if self.consolidation.reinforce_rate <= 0.0 || self.consolidation.reinforce_rate >= 1.0 {
            return Err(ConfigError::InvalidEnvOverride {
                key: "reinforce_rate".into(),
                reason: format!(
                    "must be in (0, 1), got {}",
                    self.consolidation.reinforce_rate
                ),
            });
        }
        if self.consolidation.prune_threshold <= 0.0 || self.consolidation.prune_threshold >= 1.0 {
            return Err(ConfigError::InvalidEnvOverride {
                key: "prune_threshold".into(),
                reason: format!(
                    "must be in (0, 1), got {}",
                    self.consolidation.prune_threshold
                ),
            });
        }
        if self.injection.token_budget == 0 {
            return Err(ConfigError::InvalidEnvOverride {
                key: "token_budget".into(),
                reason: "must be > 0".into(),
            });
        }
        if self.retention.envelope_days >= self.retention.delete_days {
            return Err(ConfigError::InvalidEnvOverride {
                key: "retention".into(),
                reason: format!(
                    "envelope_days ({}) must be < delete_days ({})",
                    self.retention.envelope_days, self.retention.delete_days
                ),
            });
        }
        if self.retention.gradient_hourly_days >= self.retention.gradient_daily_days {
            return Err(ConfigError::InvalidEnvOverride {
                key: "retention".into(),
                reason: format!(
                    "gradient_hourly_days ({}) must be < gradient_daily_days ({})",
                    self.retention.gradient_hourly_days, self.retention.gradient_daily_days
                ),
            });
        }
        Ok(())
    }

    /// Serialize config to TOML string.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::ParseFailed` if serialization fails.
    pub fn to_toml(&self) -> std::result::Result<String, ConfigError> {
        toml::to_string_pretty(self).map_err(|e| ConfigError::ParseFailed(e.to_string()))
    }
}

/// Resolve the default database path: `$HOME/.local/share/habitat/injection.db`.
fn default_db_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(m05_constants::DEFAULT_DB_RELATIVE_PATH)
    } else {
        PathBuf::from(m05_constants::DEFAULT_DB_RELATIVE_PATH)
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;

    #[test]
    fn default_config_is_valid() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn default_db_path_ends_with_db() {
        let config = Config::default();
        assert!(
            config
                .database
                .path
                .to_string_lossy()
                .ends_with("injection.db")
        );
    }

    #[test]
    fn default_budget_matches_constant() {
        let config = Config::default();
        assert_eq!(
            config.injection.token_budget,
            m05_constants::DEFAULT_BUDGET.as_u32()
        );
    }

    #[test]
    fn default_decay_matches_constant() {
        let config = Config::default();
        assert!((config.consolidation.decay_rate - m05_constants::DECAY_RATE).abs() < f64::EPSILON);
    }

    #[test]
    fn default_reinforce_matches_constant() {
        let config = Config::default();
        assert!(
            (config.consolidation.reinforce_rate - m05_constants::REINFORCE_RATE).abs()
                < f64::EPSILON
        );
    }

    #[test]
    fn default_wal_enabled() {
        let config = Config::default();
        assert!(config.database.wal_mode);
    }

    #[test]
    fn validate_rejects_zero_budget() {
        let mut config = Config::default();
        config.injection.token_budget = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_rejects_decay_out_of_range() {
        let mut config = Config::default();
        config.consolidation.decay_rate = 1.0;
        assert!(config.validate().is_err());

        config.consolidation.decay_rate = 0.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_rejects_reinforce_out_of_range() {
        let mut config = Config::default();
        config.consolidation.reinforce_rate = 1.5;
        assert!(config.validate().is_err());

        config.consolidation.reinforce_rate = -0.1;
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_rejects_prune_out_of_range() {
        let mut config = Config::default();
        config.consolidation.prune_threshold = 0.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_rejects_bad_retention_ordering() {
        let mut config = Config::default();
        config.retention.envelope_days = 100;
        config.retention.delete_days = 50;
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_rejects_bad_gradient_ordering() {
        let mut config = Config::default();
        config.retention.gradient_hourly_days = 30;
        config.retention.gradient_daily_days = 7;
        assert!(config.validate().is_err());
    }

    #[test]
    fn from_file_missing() {
        let result = Config::from_file(Path::new("/nonexistent/path.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn load_with_no_file_returns_defaults() {
        let config = Config::load(None);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn load_with_missing_file_returns_defaults() {
        let config = Config::load(Some(Path::new("/nonexistent/path.toml")));
        assert!(config.validate().is_ok());
    }

    #[test]
    fn to_toml_roundtrip() {
        let config = Config::default();
        let toml_str = config.to_toml().unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.injection.token_budget, config.injection.token_budget);
        assert!(
            (parsed.consolidation.decay_rate - config.consolidation.decay_rate).abs()
                < f64::EPSILON
        );
    }

    #[test]
    fn to_toml_contains_sections() {
        let config = Config::default();
        let toml_str = config.to_toml().unwrap();
        assert!(toml_str.contains("[database]"));
        assert!(toml_str.contains("[injection]"));
        assert!(toml_str.contains("[consolidation]"));
        assert!(toml_str.contains("[retention]"));
        assert!(toml_str.contains("[services]"));
    }

    #[test]
    fn serde_roundtrip_json() {
        let config = Config::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.injection.token_budget, config.injection.token_budget);
    }

    #[test]
    #[serial]
    fn env_override_db_path() {
        let mut config = Config::default();
        unsafe { std::env::set_var("HABITAT_DB_PATH", "/tmp/test.db") };
        config.apply_env_overrides();
        assert_eq!(config.database.path, PathBuf::from("/tmp/test.db"));
        unsafe { std::env::remove_var("HABITAT_DB_PATH") };
    }

    #[test]
    #[serial]
    fn env_override_token_budget() {
        let mut config = Config::default();
        unsafe { std::env::set_var("HABITAT_TOKEN_BUDGET", "2000") };
        config.apply_env_overrides();
        assert_eq!(config.injection.token_budget, 2000);
        unsafe { std::env::remove_var("HABITAT_TOKEN_BUDGET") };
    }

    #[test]
    #[serial]
    fn env_override_decay_rate() {
        let mut config = Config::default();
        unsafe { std::env::set_var("HABITAT_DECAY_RATE", "0.9") };
        config.apply_env_overrides();
        assert!((config.consolidation.decay_rate - 0.9).abs() < f64::EPSILON);
        unsafe { std::env::remove_var("HABITAT_DECAY_RATE") };
    }

    #[test]
    #[serial]
    fn env_override_invalid_ignored() {
        let mut config = Config::default();
        let original_budget = config.injection.token_budget;
        unsafe { std::env::set_var("HABITAT_TOKEN_BUDGET", "not_a_number") };
        config.apply_env_overrides();
        assert_eq!(config.injection.token_budget, original_budget);
        unsafe { std::env::remove_var("HABITAT_TOKEN_BUDGET") };
    }

    #[test]
    fn default_services_ports() {
        let config = Config::default();
        assert_eq!(config.services.stdb_port, 3000);
        assert_eq!(config.services.ingester_health_port, 3001);
    }

    #[test]
    fn default_services_intervals() {
        let config = Config::default();
        assert_eq!(config.services.orac_poll_secs, 30);
        assert_eq!(config.services.synthex_poll_secs, 60);
        assert_eq!(config.services.povm_sync_secs, 300);
    }

    #[test]
    fn database_config_from_toml() {
        let toml_str = r#"
path = "/tmp/custom.db"
wal_mode = false
busy_timeout_ms = 1000
"#;
        let db: DatabaseConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(db.path, PathBuf::from("/tmp/custom.db"));
        assert!(!db.wal_mode);
        assert_eq!(db.busy_timeout_ms, 1000);
    }

    #[test]
    fn config_clone() {
        let config = Config::default();
        let cloned = config.clone();
        assert_eq!(
            cloned.injection.token_budget,
            config.injection.token_budget
        );
    }

    #[test]
    fn config_debug() {
        let config = Config::default();
        let debug = format!("{config:?}");
        assert!(debug.contains("DatabaseConfig"));
        assert!(debug.contains("InjectionConfig"));
    }

    #[test]
    fn default_busy_timeout() {
        let config = Config::default();
        assert_eq!(config.database.busy_timeout_ms, 5000);
    }

    #[test]
    fn retention_defaults_match_constants() {
        let config = Config::default();
        assert_eq!(
            config.retention.envelope_days,
            m05_constants::RETENTION_ENVELOPE_DAYS
        );
        assert_eq!(
            config.retention.delete_days,
            m05_constants::RETENTION_DELETE_DAYS
        );
        assert_eq!(
            config.retention.gradient_hourly_days,
            m05_constants::GRADIENT_DOWNSAMPLE_HOURLY_DAYS
        );
        assert_eq!(
            config.retention.gradient_daily_days,
            m05_constants::GRADIENT_DOWNSAMPLE_DAILY_DAYS
        );
    }

    // -- Validation boundary tests --

    #[test]
    fn validate_accepts_boundary_decay_rate() {
        let mut config = Config::default();
        config.consolidation.decay_rate = 0.001;
        assert!(config.validate().is_ok());
        config.consolidation.decay_rate = 0.999;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_accepts_boundary_reinforce_rate() {
        let mut config = Config::default();
        config.consolidation.reinforce_rate = 0.001;
        assert!(config.validate().is_ok());
        config.consolidation.reinforce_rate = 0.999;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_accepts_boundary_prune_threshold() {
        let mut config = Config::default();
        config.consolidation.prune_threshold = 0.001;
        assert!(config.validate().is_ok());
        config.consolidation.prune_threshold = 0.999;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_accepts_minimum_budget() {
        let mut config = Config::default();
        config.injection.token_budget = 1;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_accepts_equal_days_boundary() {
        let mut config = Config::default();
        config.retention.envelope_days = 89;
        config.retention.delete_days = 90;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_rejects_equal_retention_days() {
        let mut config = Config::default();
        config.retention.envelope_days = 30;
        config.retention.delete_days = 30;
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_rejects_equal_gradient_days() {
        let mut config = Config::default();
        config.retention.gradient_hourly_days = 7;
        config.retention.gradient_daily_days = 7;
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_rejects_negative_decay_rate() {
        let mut config = Config::default();
        config.consolidation.decay_rate = -0.5;
        assert!(config.validate().is_err());
    }

    // -- TOML serialization edge cases --

    #[test]
    fn to_toml_contains_all_fields() {
        let config = Config::default();
        let toml_str = config.to_toml().unwrap();
        assert!(toml_str.contains("token_budget"));
        assert!(toml_str.contains("decay_rate"));
        assert!(toml_str.contains("reinforce_rate"));
        assert!(toml_str.contains("prune_threshold"));
        assert!(toml_str.contains("wal_mode"));
        assert!(toml_str.contains("stdb_port"));
    }

    #[test]
    fn toml_roundtrip_preserves_all_values() {
        let mut config = Config::default();
        config.injection.token_budget = 2200;
        config.consolidation.decay_rate = 0.88;
        config.consolidation.reinforce_rate = 0.2;
        config.retention.envelope_days = 14;
        config.services.stdb_port = 4000;
        let toml_str = config.to_toml().unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.injection.token_budget, 2200);
        assert!((parsed.consolidation.decay_rate - 0.88).abs() < f64::EPSILON);
        assert!((parsed.consolidation.reinforce_rate - 0.2).abs() < f64::EPSILON);
        assert_eq!(parsed.retention.envelope_days, 14);
        assert_eq!(parsed.services.stdb_port, 4000);
    }

    #[test]
    fn json_roundtrip_preserves_all_values() {
        let mut config = Config::default();
        config.database.wal_mode = false;
        config.injection.max_chains = 10;
        let json = serde_json::to_string(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert!(!parsed.database.wal_mode);
        assert_eq!(parsed.injection.max_chains, 10);
    }

    // -- Sub-config defaults --

    #[test]
    fn injection_config_default_max_fields() {
        let ic = InjectionConfig::default();
        assert_eq!(ic.max_chains, m05_constants::MAX_CHAINS_INJECTED);
        assert_eq!(ic.max_patterns, m05_constants::MAX_PATTERNS_INJECTED);
        assert_eq!(
            ic.max_trajectory_points,
            m05_constants::MAX_TRAJECTORY_POINTS
        );
        assert_eq!(ic.max_workstreams, m05_constants::MAX_WORKSTREAMS);
    }

    #[test]
    fn consolidation_config_default_intervals() {
        let cc = ConsolidationConfig::default();
        assert_eq!(cc.cache_rebuild_secs, m05_constants::CACHE_REBUILD_SECS);
        assert_eq!(cc.decay_interval_secs, m05_constants::DECAY_INTERVAL_SECS);
        assert_eq!(cc.auto_resolve_sessions, m05_constants::AUTO_RESOLVE_SESSIONS);
    }

    #[test]
    fn retention_config_default_compaction() {
        let rc = RetentionConfig::default();
        assert_eq!(
            rc.compaction_interval_secs,
            m05_constants::COMPACTION_INTERVAL_SECS
        );
    }

    #[test]
    fn services_config_default_gradient() {
        let sc = ServicesConfig::default();
        assert_eq!(
            sc.gradient_capture_secs,
            m05_constants::GRADIENT_CAPTURE_INTERVAL_SECS
        );
    }

    // -- File loading --

    #[test]
    fn from_file_with_valid_toml() {
        let dir = std::env::temp_dir().join("habitat_test_config");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test_config.toml");
        let config = Config::default();
        let toml_str = config.to_toml().unwrap();
        std::fs::write(&path, &toml_str).unwrap();

        let loaded = Config::from_file(&path).unwrap();
        assert_eq!(loaded.injection.token_budget, config.injection.token_budget);

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn from_file_with_invalid_toml() {
        let dir = std::env::temp_dir().join("habitat_test_config_bad");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("bad_config.toml");
        std::fs::write(&path, "this is not valid toml {{{").unwrap();

        let result = Config::from_file(&path);
        assert!(result.is_err());

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn load_with_valid_file_overrides_defaults() {
        let dir = std::env::temp_dir().join("habitat_test_config_load");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("load_config.toml");
        let mut config = Config::default();
        config.injection.token_budget = 9999;
        let toml_str = config.to_toml().unwrap();
        std::fs::write(&path, &toml_str).unwrap();

        let loaded = Config::load(Some(&path));
        assert_eq!(loaded.injection.token_budget, 9999);

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    // -- Validate error messages --

    #[test]
    fn validate_error_contains_field_name() {
        let mut config = Config::default();
        config.consolidation.decay_rate = 0.0;
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("decay_rate"));
    }

    #[test]
    fn validate_error_contains_actual_value() {
        let mut config = Config::default();
        config.consolidation.reinforce_rate = 1.5;
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("1.5"));
    }

    // -- Sub-config independence --

    #[test]
    fn database_config_serde_roundtrip() {
        let db = DatabaseConfig::default();
        let json = serde_json::to_string(&db).unwrap();
        let parsed: DatabaseConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.busy_timeout_ms, db.busy_timeout_ms);
        assert_eq!(parsed.wal_mode, db.wal_mode);
    }

    #[test]
    fn injection_config_serde_roundtrip() {
        let ic = InjectionConfig::default();
        let json = serde_json::to_string(&ic).unwrap();
        let parsed: InjectionConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.token_budget, ic.token_budget);
        assert_eq!(parsed.max_latency_ms, ic.max_latency_ms);
    }

    #[test]
    fn consolidation_config_serde_roundtrip() {
        let cc = ConsolidationConfig::default();
        let json = serde_json::to_string(&cc).unwrap();
        let parsed: ConsolidationConfig = serde_json::from_str(&json).unwrap();
        assert!((parsed.decay_rate - cc.decay_rate).abs() < f64::EPSILON);
    }

    #[test]
    fn retention_config_serde_roundtrip() {
        let rc = RetentionConfig::default();
        let json = serde_json::to_string(&rc).unwrap();
        let parsed: RetentionConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.envelope_days, rc.envelope_days);
    }

    #[test]
    fn services_config_serde_roundtrip() {
        let sc = ServicesConfig::default();
        let json = serde_json::to_string(&sc).unwrap();
        let parsed: ServicesConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.stdb_port, sc.stdb_port);
    }

    #[test]
    fn injection_config_max_payload_bytes() {
        let ic = InjectionConfig::default();
        assert_eq!(ic.max_payload_bytes, m05_constants::MAX_PAYLOAD_BYTES);
    }

    #[test]
    fn injection_config_max_latency() {
        let ic = InjectionConfig::default();
        assert_eq!(ic.max_latency_ms, m05_constants::MAX_INJECTION_LATENCY_MS);
    }
}
