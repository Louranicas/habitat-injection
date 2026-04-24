//! `m23_ingester` — Ingester pipeline configuration, source definitions, and bridge types.
//!
//! Defines the configuration and type model for the multi-source ingester binary
//! that polls ORAC (30 s), subscribes to PV2 `/bus/ws`, polls SYNTHEX (60 s), and
//! syncs POVM (300 s), then forwards events to `SpaceTimeDB` via its SDK.
//!
//! This module is intentionally **pure types + sync logic only**. No `tokio`,
//! `reqwest`, or `axum` imports appear here. The async polling loop is Phase 2
//! runtime code compiled under `#[cfg(feature = "ingester")]`.
//!
//! Layer: `m6_stdb`
//! Dependencies: `m01_types`, `m05_constants`

use serde::{Deserialize, Serialize};

use crate::m1_foundation::m05_constants::{
    INGESTER_HEALTH_PORT, ORAC_POLL_INTERVAL_SECS, POVM_SYNC_INTERVAL_SECS,
    STDB_PORT, SYNTHEX_POLL_INTERVAL_SECS,
};

// ---------------------------------------------------------------------------
// IngesterConfig
// ---------------------------------------------------------------------------

/// Configuration for the ingester binary.
///
/// Loaded from a TOML file or environment variables at startup.
/// Call [`IngesterConfig::validate`] before using any URL or port.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngesterConfig {
    /// `SpaceTimeDB` base URL, e.g. `"http://localhost:3000"`.
    pub stdb_url: String,
    /// TCP port on which the ingester exposes its `/health` endpoint.
    pub health_port: u16,
    /// ORAC sidecar base URL, e.g. `"http://localhost:8133"`.
    pub orac_url: String,
    /// How often (seconds) to poll ORAC for new events.
    pub orac_poll_secs: u64,
    /// `PaneVortex` base URL, e.g. `"http://localhost:8132"`.
    pub pv2_url: String,
    /// SYNTHEX base URL, e.g. `"http://localhost:8090"`.
    pub synthex_url: String,
    /// How often (seconds) to poll SYNTHEX for new events.
    pub synthex_poll_secs: u64,
    /// POVM engine base URL, e.g. `"http://localhost:8125"`.
    pub povm_url: String,
    /// How often (seconds) to sync POVM pathway snapshots.
    pub povm_sync_secs: u64,
}

impl Default for IngesterConfig {
    /// Constructs a config populated with the canonical habitat defaults
    /// taken from [`crate::m1_foundation::m05_constants`].
    fn default() -> Self {
        Self {
            stdb_url: format!("http://localhost:{STDB_PORT}"),
            health_port: INGESTER_HEALTH_PORT,
            orac_url: String::from("http://localhost:8133"),
            orac_poll_secs: ORAC_POLL_INTERVAL_SECS,
            pv2_url: String::from("http://localhost:8132"),
            synthex_url: String::from("http://localhost:8090"),
            synthex_poll_secs: SYNTHEX_POLL_INTERVAL_SECS,
            povm_url: String::from("http://localhost:8125"),
            povm_sync_secs: POVM_SYNC_INTERVAL_SECS,
        }
    }
}

impl IngesterConfig {
    /// Validates the config for common misconfigurations.
    ///
    /// Returns `Ok(())` when all fields satisfy:
    /// - URL fields are non-empty strings.
    /// - `health_port` is non-zero.
    /// - All polling intervals are non-zero.
    ///
    /// # Errors
    ///
    /// Returns a human-readable `String` describing the first validation
    /// failure found.
    #[must_use = "validation result must be checked — ignoring it skips safety checks"]
    pub fn validate(&self) -> Result<(), String> {
        if self.stdb_url.is_empty() {
            return Err(String::from("stdb_url must not be empty"));
        }
        if self.orac_url.is_empty() {
            return Err(String::from("orac_url must not be empty"));
        }
        if self.pv2_url.is_empty() {
            return Err(String::from("pv2_url must not be empty"));
        }
        if self.synthex_url.is_empty() {
            return Err(String::from("synthex_url must not be empty"));
        }
        if self.povm_url.is_empty() {
            return Err(String::from("povm_url must not be empty"));
        }
        if self.health_port == 0 {
            return Err(String::from("health_port must be > 0"));
        }
        if self.orac_poll_secs == 0 {
            return Err(String::from("orac_poll_secs must be > 0"));
        }
        if self.synthex_poll_secs == 0 {
            return Err(String::from("synthex_poll_secs must be > 0"));
        }
        if self.povm_sync_secs == 0 {
            return Err(String::from("povm_sync_secs must be > 0"));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// IngesterSource
// ---------------------------------------------------------------------------

/// Represents a data source that the ingester polls or subscribes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IngesterSource {
    /// ORAC sidecar — polled every 30 s for events and RALPH evolution signals.
    Orac,
    /// `PaneVortex` — WebSocket `/bus/ws` subscription for real-time bus messages.
    PaneVortex,
    /// SYNTHEX — polled every 60 s for thermal + homeostasis snapshots.
    Synthex,
    /// POVM engine — synced every 300 s for pathway graph snapshots.
    Povm,
    /// Atuin hooks — injected via the PV2 bus; no separate poll cadence.
    AtuinHooks,
}

impl IngesterSource {
    /// Returns the human-readable display name for this source.
    #[must_use]
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Orac => "ORAC",
            Self::PaneVortex => "PV2",
            Self::Synthex => "SYNTHEX",
            Self::Povm => "POVM",
            Self::AtuinHooks => "Atuin",
        }
    }

    /// Returns the default polling interval in seconds for this source.
    ///
    /// [`IngesterSource::AtuinHooks`] and [`IngesterSource::PaneVortex`] return
    /// `0` because they are event-driven (WebSocket / bus delivery) rather than
    /// polled on a fixed cadence.
    #[must_use]
    pub fn default_interval_secs(&self) -> u64 {
        match self {
            Self::Orac => ORAC_POLL_INTERVAL_SECS,
            Self::Synthex => SYNTHEX_POLL_INTERVAL_SECS,
            Self::Povm => POVM_SYNC_INTERVAL_SECS,
            // Event-driven sources: WebSocket subscription or PV2 bus delivery.
            Self::PaneVortex | Self::AtuinHooks => 0,
        }
    }

    /// Builds the full health-check URL for this source using the supplied
    /// [`IngesterConfig`].
    ///
    /// For [`IngesterSource::AtuinHooks`] the PV2 base URL is returned because
    /// Atuin hooks reach the ingester via the `PaneVortex` bus.
    #[must_use]
    pub fn endpoint(&self, config: &IngesterConfig) -> String {
        match self {
            Self::Orac => format!("{}/health", config.orac_url),
            // AtuinHooks are delivered via the PV2 bus — share the PV2 endpoint.
            Self::PaneVortex | Self::AtuinHooks => format!("{}/health", config.pv2_url),
            Self::Synthex => format!("{}/api/health", config.synthex_url),
            Self::Povm => format!("{}/health", config.povm_url),
        }
    }
}

// ---------------------------------------------------------------------------
// SourceStatus
// ---------------------------------------------------------------------------

/// Status of a single ingester source at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceStatus {
    /// Which source this status report belongs to.
    pub source: IngesterSource,
    /// Whether the most recent poll succeeded.
    pub healthy: bool,
    /// ISO 8601 timestamp of the last successful poll, if any.
    pub last_poll: Option<String>,
    /// Total number of events ingested from this source since startup.
    pub events_ingested: u64,
    /// Total number of poll errors since startup.
    pub errors: u64,
    /// Human-readable description of the last error, if any.
    pub last_error: Option<String>,
}

impl SourceStatus {
    /// Creates an initial (not-yet-polled) [`SourceStatus`] for `source`.
    ///
    /// All counters are zero and `healthy` is `false` until the first
    /// successful poll is recorded.
    #[must_use]
    pub fn new(source: IngesterSource) -> Self {
        Self {
            source,
            healthy: false,
            last_poll: None,
            events_ingested: 0,
            errors: 0,
            last_error: None,
        }
    }

    /// Records a successful poll that ingested `count` events at `timestamp`.
    pub fn record_success(&mut self, timestamp: String, count: u64) {
        self.healthy = true;
        self.last_poll = Some(timestamp);
        self.events_ingested = self.events_ingested.saturating_add(count);
        self.last_error = None;
    }

    /// Records a failed poll with the given error message.
    pub fn record_error(&mut self, error: String) {
        self.healthy = false;
        self.errors = self.errors.saturating_add(1);
        self.last_error = Some(error);
    }
}

// ---------------------------------------------------------------------------
// IngesterHealth
// ---------------------------------------------------------------------------

/// Aggregate health report for the ingester process.
///
/// Returned by the `/health` endpoint on `health_port`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngesterHealth {
    /// `true` only when **all** sources are healthy and STDB is reachable.
    pub healthy: bool,
    /// Seconds elapsed since the ingester process started.
    pub uptime_secs: u64,
    /// Per-source status breakdown.
    pub sources: Vec<SourceStatus>,
    /// Total events ingested across all sources since startup.
    pub total_events: u64,
    /// Whether the `SpaceTimeDB` connection is live.
    pub stdb_connected: bool,
}

impl IngesterHealth {
    /// Builds an [`IngesterHealth`] from a slice of per-source statuses.
    ///
    /// `healthy` is `true` only when every source is healthy **and**
    /// `stdb_connected` is `true`.
    #[must_use]
    pub fn from_sources(
        sources: &[SourceStatus],
        uptime_secs: u64,
        stdb_connected: bool,
    ) -> Self {
        let all_healthy = stdb_connected && sources.iter().all(|s| s.healthy);
        let total_events = sources
            .iter()
            .fold(0u64, |acc, s| acc.saturating_add(s.events_ingested));

        Self {
            healthy: all_healthy,
            uptime_secs,
            sources: sources.to_vec(),
            total_events,
            stdb_connected,
        }
    }
}

// ---------------------------------------------------------------------------
// IngestableEvent
// ---------------------------------------------------------------------------

/// An event ready for ingestion into `SpaceTimeDB` via the `ingest_event` reducer.
///
/// All fields are plain Rust / serde types so this struct can be constructed
/// and validated without an async runtime or network connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestableEvent {
    /// Category label for the event, e.g. `"orac.emergence"` or `"synthex.thermal"`.
    pub event_type: String,
    /// Canonical service name that produced the event, e.g. `"orac-sidecar"`.
    pub source_service: String,
    /// Severity on a 0–10 scale (inclusive).
    pub severity: u8,
    /// Confidence in the event classification in `[0.0, 1.0]`.
    pub confidence: f64,
    /// JSON-encoded event payload; must be valid JSON.
    pub payload_json: String,
    /// Optional STDB row ID of the causal parent event.
    pub causal_parent: Option<u64>,
    /// Optional habitat session ID string, e.g. `"S109"`.
    pub session_id: Option<String>,
}

impl IngestableEvent {
    /// Validates the event fields for correctness.
    ///
    /// # Errors
    ///
    /// Returns a human-readable `String` when:
    /// - `event_type` is empty.
    /// - `source_service` is empty.
    /// - `severity` exceeds 10.
    /// - `confidence` is outside `[0.0, 1.0]`.
    #[must_use = "validation result must be checked — ignoring it skips safety checks"]
    pub fn validate(&self) -> Result<(), String> {
        if self.event_type.is_empty() {
            return Err(String::from("event_type must not be empty"));
        }
        if self.source_service.is_empty() {
            return Err(String::from("source_service must not be empty"));
        }
        if self.severity > 10 {
            return Err(format!(
                "severity {} exceeds maximum of 10",
                self.severity
            ));
        }
        if !(0.0..=1.0).contains(&self.confidence) {
            return Err(format!(
                "confidence {} is outside [0.0, 1.0]",
                self.confidence
            ));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // IngesterConfig — default
    // ------------------------------------------------------------------

    #[test]
    fn config_default_stdb_url_nonempty() {
        let cfg = IngesterConfig::default();
        assert!(!cfg.stdb_url.is_empty());
    }

    #[test]
    fn config_default_stdb_url_contains_port() {
        let cfg = IngesterConfig::default();
        assert!(cfg.stdb_url.contains("3000"));
    }

    #[test]
    fn config_default_health_port_is_3001() {
        let cfg = IngesterConfig::default();
        assert_eq!(cfg.health_port, 3001);
    }

    #[test]
    fn config_default_orac_url_nonempty() {
        let cfg = IngesterConfig::default();
        assert!(!cfg.orac_url.is_empty());
    }

    #[test]
    fn config_default_orac_poll_secs_is_30() {
        let cfg = IngesterConfig::default();
        assert_eq!(cfg.orac_poll_secs, 30);
    }

    #[test]
    fn config_default_pv2_url_nonempty() {
        let cfg = IngesterConfig::default();
        assert!(!cfg.pv2_url.is_empty());
    }

    #[test]
    fn config_default_synthex_url_nonempty() {
        let cfg = IngesterConfig::default();
        assert!(!cfg.synthex_url.is_empty());
    }

    #[test]
    fn config_default_synthex_poll_secs_is_60() {
        let cfg = IngesterConfig::default();
        assert_eq!(cfg.synthex_poll_secs, 60);
    }

    #[test]
    fn config_default_povm_url_nonempty() {
        let cfg = IngesterConfig::default();
        assert!(!cfg.povm_url.is_empty());
    }

    #[test]
    fn config_default_povm_sync_secs_is_300() {
        let cfg = IngesterConfig::default();
        assert_eq!(cfg.povm_sync_secs, 300);
    }

    // ------------------------------------------------------------------
    // IngesterConfig — validate
    // ------------------------------------------------------------------

    #[test]
    fn config_validate_default_is_ok() {
        assert!(IngesterConfig::default().validate().is_ok());
    }

    #[test]
    fn config_validate_empty_stdb_url_errors() {
        let mut cfg = IngesterConfig::default();
        cfg.stdb_url = String::new();
        let result = cfg.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("stdb_url"));
    }

    #[test]
    fn config_validate_empty_orac_url_errors() {
        let mut cfg = IngesterConfig::default();
        cfg.orac_url = String::new();
        let result = cfg.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("orac_url"));
    }

    #[test]
    fn config_validate_empty_pv2_url_errors() {
        let mut cfg = IngesterConfig::default();
        cfg.pv2_url = String::new();
        let result = cfg.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("pv2_url"));
    }

    #[test]
    fn config_validate_empty_synthex_url_errors() {
        let mut cfg = IngesterConfig::default();
        cfg.synthex_url = String::new();
        let result = cfg.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("synthex_url"));
    }

    #[test]
    fn config_validate_empty_povm_url_errors() {
        let mut cfg = IngesterConfig::default();
        cfg.povm_url = String::new();
        let result = cfg.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("povm_url"));
    }

    #[test]
    fn config_validate_zero_health_port_errors() {
        let mut cfg = IngesterConfig::default();
        cfg.health_port = 0;
        let result = cfg.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("health_port"));
    }

    #[test]
    fn config_validate_zero_orac_poll_errors() {
        let mut cfg = IngesterConfig::default();
        cfg.orac_poll_secs = 0;
        let result = cfg.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("orac_poll_secs"));
    }

    #[test]
    fn config_validate_zero_synthex_poll_errors() {
        let mut cfg = IngesterConfig::default();
        cfg.synthex_poll_secs = 0;
        let result = cfg.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("synthex_poll_secs"));
    }

    #[test]
    fn config_validate_zero_povm_sync_errors() {
        let mut cfg = IngesterConfig::default();
        cfg.povm_sync_secs = 0;
        let result = cfg.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("povm_sync_secs"));
    }

    // ------------------------------------------------------------------
    // IngesterConfig — serde roundtrip
    // ------------------------------------------------------------------

    #[test]
    fn config_serde_roundtrip() {
        let cfg = IngesterConfig::default();
        let json = serde_json::to_string(&cfg).expect("serialize");
        let back: IngesterConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(cfg.stdb_url, back.stdb_url);
        assert_eq!(cfg.health_port, back.health_port);
        assert_eq!(cfg.orac_poll_secs, back.orac_poll_secs);
        assert_eq!(cfg.synthex_poll_secs, back.synthex_poll_secs);
        assert_eq!(cfg.povm_sync_secs, back.povm_sync_secs);
    }

    #[test]
    fn config_clone_is_independent() {
        let cfg = IngesterConfig::default();
        let mut clone = cfg.clone();
        clone.health_port = 9999;
        assert_ne!(cfg.health_port, clone.health_port);
    }

    // ------------------------------------------------------------------
    // IngesterSource — display_name
    // ------------------------------------------------------------------

    #[test]
    fn source_orac_display_name() {
        assert_eq!(IngesterSource::Orac.display_name(), "ORAC");
    }

    #[test]
    fn source_pv2_display_name() {
        assert_eq!(IngesterSource::PaneVortex.display_name(), "PV2");
    }

    #[test]
    fn source_synthex_display_name() {
        assert_eq!(IngesterSource::Synthex.display_name(), "SYNTHEX");
    }

    #[test]
    fn source_povm_display_name() {
        assert_eq!(IngesterSource::Povm.display_name(), "POVM");
    }

    #[test]
    fn source_atuin_display_name() {
        assert_eq!(IngesterSource::AtuinHooks.display_name(), "Atuin");
    }

    // ------------------------------------------------------------------
    // IngesterSource — default_interval_secs
    // ------------------------------------------------------------------

    #[test]
    fn source_orac_interval_is_30() {
        assert_eq!(IngesterSource::Orac.default_interval_secs(), 30);
    }

    #[test]
    fn source_pv2_interval_is_zero() {
        assert_eq!(IngesterSource::PaneVortex.default_interval_secs(), 0);
    }

    #[test]
    fn source_synthex_interval_is_60() {
        assert_eq!(IngesterSource::Synthex.default_interval_secs(), 60);
    }

    #[test]
    fn source_povm_interval_is_300() {
        assert_eq!(IngesterSource::Povm.default_interval_secs(), 300);
    }

    #[test]
    fn source_atuin_interval_is_zero() {
        assert_eq!(IngesterSource::AtuinHooks.default_interval_secs(), 0);
    }

    // ------------------------------------------------------------------
    // IngesterSource — endpoint
    // ------------------------------------------------------------------

    #[test]
    fn source_orac_endpoint_uses_orac_url() {
        let cfg = IngesterConfig::default();
        let ep = IngesterSource::Orac.endpoint(&cfg);
        assert!(ep.starts_with(&cfg.orac_url));
        assert!(ep.ends_with("/health"));
    }

    #[test]
    fn source_pv2_endpoint_uses_pv2_url() {
        let cfg = IngesterConfig::default();
        let ep = IngesterSource::PaneVortex.endpoint(&cfg);
        assert!(ep.starts_with(&cfg.pv2_url));
    }

    #[test]
    fn source_synthex_endpoint_uses_api_health() {
        let cfg = IngesterConfig::default();
        let ep = IngesterSource::Synthex.endpoint(&cfg);
        assert!(ep.ends_with("/api/health"));
    }

    #[test]
    fn source_povm_endpoint_uses_povm_url() {
        let cfg = IngesterConfig::default();
        let ep = IngesterSource::Povm.endpoint(&cfg);
        assert!(ep.starts_with(&cfg.povm_url));
        assert!(ep.ends_with("/health"));
    }

    #[test]
    fn source_atuin_endpoint_uses_pv2_url() {
        let cfg = IngesterConfig::default();
        let ep = IngesterSource::AtuinHooks.endpoint(&cfg);
        assert!(ep.starts_with(&cfg.pv2_url));
    }

    #[test]
    fn source_endpoint_includes_port() {
        let cfg = IngesterConfig::default();
        let ep = IngesterSource::Orac.endpoint(&cfg);
        assert!(ep.contains("8133"));
    }

    // ------------------------------------------------------------------
    // IngesterSource — serde + equality
    // ------------------------------------------------------------------

    #[test]
    fn source_serde_roundtrip_orac() {
        let s = IngesterSource::Orac;
        let json = serde_json::to_string(&s).expect("serialize");
        let back: IngesterSource = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(s, back);
    }

    #[test]
    fn source_serde_roundtrip_all_variants() {
        let variants = [
            IngesterSource::Orac,
            IngesterSource::PaneVortex,
            IngesterSource::Synthex,
            IngesterSource::Povm,
            IngesterSource::AtuinHooks,
        ];
        for v in &variants {
            let json = serde_json::to_string(v).expect("serialize");
            let back: IngesterSource = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(v, &back);
        }
    }

    #[test]
    fn source_equality_reflexive() {
        assert_eq!(IngesterSource::Synthex, IngesterSource::Synthex);
    }

    #[test]
    fn source_inequality_between_variants() {
        assert_ne!(IngesterSource::Orac, IngesterSource::Povm);
    }

    // ------------------------------------------------------------------
    // SourceStatus — construction
    // ------------------------------------------------------------------

    #[test]
    fn source_status_new_healthy_false() {
        let s = SourceStatus::new(IngesterSource::Orac);
        assert!(!s.healthy);
    }

    #[test]
    fn source_status_new_last_poll_none() {
        let s = SourceStatus::new(IngesterSource::Synthex);
        assert!(s.last_poll.is_none());
    }

    #[test]
    fn source_status_new_events_zero() {
        let s = SourceStatus::new(IngesterSource::Povm);
        assert_eq!(s.events_ingested, 0);
    }

    #[test]
    fn source_status_new_errors_zero() {
        let s = SourceStatus::new(IngesterSource::PaneVortex);
        assert_eq!(s.errors, 0);
    }

    #[test]
    fn source_status_new_last_error_none() {
        let s = SourceStatus::new(IngesterSource::AtuinHooks);
        assert!(s.last_error.is_none());
    }

    #[test]
    fn source_status_new_source_field_preserved() {
        let s = SourceStatus::new(IngesterSource::Orac);
        assert_eq!(s.source, IngesterSource::Orac);
    }

    #[test]
    fn source_status_record_success_sets_healthy() {
        let mut s = SourceStatus::new(IngesterSource::Orac);
        s.record_success(String::from("2026-04-24T12:00:00Z"), 5);
        assert!(s.healthy);
    }

    #[test]
    fn source_status_record_success_sets_last_poll() {
        let mut s = SourceStatus::new(IngesterSource::Orac);
        s.record_success(String::from("2026-04-24T12:00:00Z"), 5);
        assert_eq!(s.last_poll.as_deref(), Some("2026-04-24T12:00:00Z"));
    }

    #[test]
    fn source_status_record_success_adds_events() {
        let mut s = SourceStatus::new(IngesterSource::Orac);
        s.record_success(String::from("2026-04-24T12:00:00Z"), 5);
        s.record_success(String::from("2026-04-24T12:01:00Z"), 3);
        assert_eq!(s.events_ingested, 8);
    }

    #[test]
    fn source_status_record_error_sets_unhealthy() {
        let mut s = SourceStatus::new(IngesterSource::Orac);
        s.record_success(String::from("2026-04-24T12:00:00Z"), 1);
        s.record_error(String::from("connection refused"));
        assert!(!s.healthy);
    }

    #[test]
    fn source_status_record_error_increments_errors() {
        let mut s = SourceStatus::new(IngesterSource::Orac);
        s.record_error(String::from("timeout"));
        s.record_error(String::from("timeout"));
        assert_eq!(s.errors, 2);
    }

    #[test]
    fn source_status_serde_roundtrip() {
        let s = SourceStatus::new(IngesterSource::Povm);
        let json = serde_json::to_string(&s).expect("serialize");
        let back: SourceStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.source, s.source);
        assert_eq!(back.healthy, s.healthy);
        assert_eq!(back.events_ingested, s.events_ingested);
    }

    // ------------------------------------------------------------------
    // IngesterHealth — aggregation
    // ------------------------------------------------------------------

    fn make_healthy_status(source: IngesterSource, events: u64) -> SourceStatus {
        let mut s = SourceStatus::new(source);
        s.record_success(String::from("2026-04-24T12:00:00Z"), events);
        s
    }

    #[test]
    fn health_all_healthy_sources_and_stdb_is_healthy() {
        let sources = vec![
            make_healthy_status(IngesterSource::Orac, 10),
            make_healthy_status(IngesterSource::Synthex, 5),
        ];
        let h = IngesterHealth::from_sources(&sources, 60, true);
        assert!(h.healthy);
    }

    #[test]
    fn health_stdb_disconnected_is_unhealthy() {
        let sources = vec![
            make_healthy_status(IngesterSource::Orac, 10),
            make_healthy_status(IngesterSource::Synthex, 5),
        ];
        let h = IngesterHealth::from_sources(&sources, 60, false);
        assert!(!h.healthy);
    }

    #[test]
    fn health_one_unhealthy_source_is_unhealthy() {
        let unhealthy = SourceStatus::new(IngesterSource::Povm); // never polled
        let sources = vec![
            make_healthy_status(IngesterSource::Orac, 3),
            unhealthy,
        ];
        let h = IngesterHealth::from_sources(&sources, 120, true);
        assert!(!h.healthy);
    }

    #[test]
    fn health_all_unhealthy_is_unhealthy() {
        let sources = vec![
            SourceStatus::new(IngesterSource::Orac),
            SourceStatus::new(IngesterSource::Synthex),
        ];
        let h = IngesterHealth::from_sources(&sources, 10, false);
        assert!(!h.healthy);
    }

    #[test]
    fn health_total_events_sums_all_sources() {
        let sources = vec![
            make_healthy_status(IngesterSource::Orac, 10),
            make_healthy_status(IngesterSource::Synthex, 5),
            make_healthy_status(IngesterSource::Povm, 3),
        ];
        let h = IngesterHealth::from_sources(&sources, 60, true);
        assert_eq!(h.total_events, 18);
    }

    #[test]
    fn health_uptime_preserved() {
        let sources = vec![make_healthy_status(IngesterSource::Orac, 1)];
        let h = IngesterHealth::from_sources(&sources, 999, true);
        assert_eq!(h.uptime_secs, 999);
    }

    #[test]
    fn health_stdb_connected_field_preserved() {
        let sources: Vec<SourceStatus> = vec![];
        let h = IngesterHealth::from_sources(&sources, 0, true);
        assert!(h.stdb_connected);
    }

    #[test]
    fn health_empty_sources_stdb_connected_is_healthy() {
        // no sources means nothing is unhealthy
        let sources: Vec<SourceStatus> = vec![];
        let h = IngesterHealth::from_sources(&sources, 0, true);
        assert!(h.healthy);
    }

    #[test]
    fn health_empty_sources_total_events_zero() {
        let sources: Vec<SourceStatus> = vec![];
        let h = IngesterHealth::from_sources(&sources, 0, true);
        assert_eq!(h.total_events, 0);
    }

    #[test]
    fn health_sources_count_preserved() {
        let sources = vec![
            make_healthy_status(IngesterSource::Orac, 1),
            make_healthy_status(IngesterSource::Synthex, 2),
            make_healthy_status(IngesterSource::Povm, 3),
        ];
        let h = IngesterHealth::from_sources(&sources, 30, true);
        assert_eq!(h.sources.len(), 3);
    }

    #[test]
    fn health_serde_roundtrip() {
        let sources = vec![make_healthy_status(IngesterSource::Orac, 7)];
        let h = IngesterHealth::from_sources(&sources, 42, true);
        let json = serde_json::to_string(&h).expect("serialize");
        let back: IngesterHealth = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.total_events, h.total_events);
        assert_eq!(back.healthy, h.healthy);
        assert_eq!(back.uptime_secs, h.uptime_secs);
    }

    // ------------------------------------------------------------------
    // IngestableEvent — validate
    // ------------------------------------------------------------------

    fn valid_event() -> IngestableEvent {
        IngestableEvent {
            event_type: String::from("orac.emergence"),
            source_service: String::from("orac-sidecar"),
            severity: 5,
            confidence: 0.9,
            payload_json: String::from(r#"{"gen":100}"#),
            causal_parent: None,
            session_id: Some(String::from("S109")),
        }
    }

    #[test]
    fn event_validate_valid_ok() {
        assert!(valid_event().validate().is_ok());
    }

    #[test]
    fn event_validate_empty_event_type_errors() {
        let mut e = valid_event();
        e.event_type = String::new();
        assert!(e.validate().is_err());
    }

    #[test]
    fn event_validate_empty_source_service_errors() {
        let mut e = valid_event();
        e.source_service = String::new();
        let result = e.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("source_service"));
    }

    #[test]
    fn event_validate_severity_10_ok() {
        let mut e = valid_event();
        e.severity = 10;
        assert!(e.validate().is_ok());
    }

    #[test]
    fn event_validate_severity_11_errors() {
        let mut e = valid_event();
        e.severity = 11;
        let result = e.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("severity"));
    }

    #[test]
    fn event_validate_severity_0_ok() {
        let mut e = valid_event();
        e.severity = 0;
        assert!(e.validate().is_ok());
    }

    #[test]
    fn event_validate_severity_255_errors() {
        let mut e = valid_event();
        e.severity = 255;
        assert!(e.validate().is_err());
    }

    #[test]
    fn event_validate_confidence_zero_ok() {
        let mut e = valid_event();
        e.confidence = 0.0;
        assert!(e.validate().is_ok());
    }

    #[test]
    fn event_validate_confidence_one_ok() {
        let mut e = valid_event();
        e.confidence = 1.0;
        assert!(e.validate().is_ok());
    }

    #[test]
    fn event_validate_confidence_negative_errors() {
        let mut e = valid_event();
        e.confidence = -0.1;
        let result = e.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("confidence"));
    }

    #[test]
    fn event_validate_confidence_over_one_errors() {
        let mut e = valid_event();
        e.confidence = 1.01;
        assert!(e.validate().is_err());
    }

    #[test]
    fn event_validate_no_causal_parent_ok() {
        let mut e = valid_event();
        e.causal_parent = None;
        assert!(e.validate().is_ok());
    }

    #[test]
    fn event_validate_with_causal_parent_ok() {
        let mut e = valid_event();
        e.causal_parent = Some(42);
        assert!(e.validate().is_ok());
    }

    #[test]
    fn event_validate_no_session_id_ok() {
        let mut e = valid_event();
        e.session_id = None;
        assert!(e.validate().is_ok());
    }

    // ------------------------------------------------------------------
    // IngestableEvent — serde + clone
    // ------------------------------------------------------------------

    #[test]
    fn event_serde_roundtrip() {
        let e = valid_event();
        let json = serde_json::to_string(&e).expect("serialize");
        let back: IngestableEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.event_type, e.event_type);
        assert_eq!(back.severity, e.severity);
        assert!((back.confidence - e.confidence).abs() < f64::EPSILON);
    }

    #[test]
    fn event_clone_is_independent() {
        let e = valid_event();
        let mut clone = e.clone();
        clone.severity = 9;
        assert_eq!(e.severity, 5);
    }

    #[test]
    fn event_debug_contains_event_type() {
        let e = valid_event();
        let s = format!("{e:?}");
        assert!(s.contains("orac.emergence"));
    }

    // ------------------------------------------------------------------
    // Misc — derive smoke tests
    // ------------------------------------------------------------------

    #[test]
    fn ingester_health_debug_works() {
        let h = IngesterHealth::from_sources(&[], 0, false);
        let _ = format!("{h:?}");
    }

    #[test]
    fn source_status_debug_works() {
        let s = SourceStatus::new(IngesterSource::PaneVortex);
        let _ = format!("{s:?}");
    }
}
