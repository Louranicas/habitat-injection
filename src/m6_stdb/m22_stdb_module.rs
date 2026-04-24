//! `m22_stdb_module` — `SpaceTimeDB` WASM module mirror types.
//!
//! Defines plain Rust structs that mirror what the `SpaceTimeDB` WASM module
//! would define.  **No `spacetimedb` or `spacetimedb-sdk` imports** — all
//! STDB-specific attributes (`#[spacetimedb::table]`, `#[primary_key]`, etc.)
//! appear only in doc comments describing the WASM module schema.
//!
//! These types are usable for migration planning and type-checking without
//! requiring the optional `stdb` feature or the `spacetimedb-sdk` crate.
//!
//! # Tables
//!
//! | ID | Struct | Purpose |
//! |----|--------|---------|
//! | T1 | [`HabitatEvent`] | Causal event log |
//! | T2 | [`KnowledgeEdge`] | Unified weighted knowledge graph |
//! | T3 | [`GradientSnapshot`] | Time-series vital signs |
//! | T4 | [`SessionRecord`] | Claude Code session tracking |
//! | T5 | [`StdbWorkstream`] | In-flight work ledger |
//! | T6 | [`ServiceHealth`] | Service health timeline |
//! | T7 | [`TrapState`] | Active trap monitor |
//! | T8 | [`WatcherObservation`] | Watcher anomaly records |
//!
//! # Reducers (R1–R10)
//!
//! Reducer signatures live in [`reducers`].  They are type aliases / trait
//! definitions only — not executable STDB reducer code.
//!
//! Layer: `m6_stdb`
//! Dependencies: none (plain types only)

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Enumerations
// ---------------------------------------------------------------------------

/// Valid values for [`KnowledgeEdge::edge_type`].
///
/// In the WASM module these would be enforced via a STDB enum or `CHECK`
/// constraint on a `String` column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    /// Pattern learned from session history.
    LearnedPattern,
    /// Hebbian pathway from `hebbian_pulse.db`.
    Hebbian,
    /// Orchestration topology edge from `service_tracking.db`.
    Orchestration,
    /// POVM knowledge-graph pathway from port 8125.
    Povm,
    /// Cross-service synergy edge from `system_synergy.db`.
    Synergy,
    /// Cross-agent learning from `service_tracking.db`.
    CrossAgent,
}

impl EdgeType {
    /// Returns the canonical string representation used in STDB and `SQLite`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LearnedPattern => "learned_pattern",
            Self::Hebbian => "hebbian",
            Self::Orchestration => "orchestration",
            Self::Povm => "povm",
            Self::Synergy => "synergy",
            Self::CrossAgent => "cross_agent",
        }
    }

    /// Parse from the string representation.
    ///
    /// Returns `None` for unrecognised values.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "learned_pattern" => Some(Self::LearnedPattern),
            "hebbian" => Some(Self::Hebbian),
            "orchestration" => Some(Self::Orchestration),
            "povm" => Some(Self::Povm),
            "synergy" => Some(Self::Synergy),
            "cross_agent" => Some(Self::CrossAgent),
            _ => None,
        }
    }
}

impl std::fmt::Display for EdgeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Valid event type prefixes for [`HabitatEvent::event_type`].
///
/// The `event_type` field is a free-form `String` in the STDB schema (to allow
/// future extensibility), but well-known prefixes are catalogued here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventCategory {
    /// `emergence.*` — ORAC emergence events.
    Emergence,
    /// `sphere.*` — PV2 sphere lifecycle events.
    Sphere,
    /// `thermal.*` — SYNTHEX thermal adjustment events.
    Thermal,
    /// `command.*` — Atuin pre/post-exec hooks.
    Command,
    /// `watcher.*` — Watcher observation events.
    Watcher,
    /// `session.*` — Session start/stop hooks.
    Session,
    /// `service.*` — Service health events.
    Service,
    /// `hebbian.*` — Hebbian STDP pulse events.
    Hebbian,
    /// Any other event not in the well-known set.
    Other,
}

impl EventCategory {
    /// Returns the dot-prefix for this category (without trailing dot).
    #[must_use]
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Emergence => "emergence",
            Self::Sphere => "sphere",
            Self::Thermal => "thermal",
            Self::Command => "command",
            Self::Watcher => "watcher",
            Self::Session => "session",
            Self::Service => "service",
            Self::Hebbian => "hebbian",
            Self::Other => "other",
        }
    }

    /// Classify an `event_type` string by its dot-prefix.
    #[must_use]
    pub fn classify(event_type: &str) -> Self {
        let prefix = event_type
            .split_once('.')
            .map_or(event_type, |(p, _)| p);
        match prefix {
            "emergence" => Self::Emergence,
            "sphere" => Self::Sphere,
            "thermal" => Self::Thermal,
            "command" => Self::Command,
            "watcher" => Self::Watcher,
            "session" => Self::Session,
            "service" => Self::Service,
            "hebbian" => Self::Hebbian,
            _ => Self::Other,
        }
    }
}

impl std::fmt::Display for EventCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.prefix())
    }
}

/// Consent state values for sphere-gated operations (NA-R2).
///
/// These mirror the `ConsentLevel` type in `m01_types` but use the STDB
/// string encoding directly to avoid a cross-layer dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConsentState {
    /// Full data capture — events, edges, gradients all stored and injected.
    Emit,
    /// Store data but do not inject into context windows.
    Store,
    /// Delete/redact all data for this sphere (NA-P-13 cascade).
    Forget,
}

impl ConsentState {
    /// Returns the STDB-compatible string value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Emit => "Emit",
            Self::Store => "Store",
            Self::Forget => "Forget",
        }
    }

    /// Parse from the STDB string representation.
    ///
    /// Returns `None` for unrecognised values.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Emit" => Some(Self::Emit),
            "Store" => Some(Self::Store),
            "Forget" => Some(Self::Forget),
            _ => None,
        }
    }

    /// Whether injection into context windows is permitted.
    #[must_use]
    pub const fn permits_injection(self) -> bool {
        matches!(self, Self::Emit)
    }
}

impl std::fmt::Display for ConsentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<crate::m1_foundation::m01_types::ConsentLevel> for ConsentState {
    fn from(level: crate::m1_foundation::m01_types::ConsentLevel) -> Self {
        match level {
            crate::m1_foundation::m01_types::ConsentLevel::Emit => Self::Emit,
            crate::m1_foundation::m01_types::ConsentLevel::Store => Self::Store,
            crate::m1_foundation::m01_types::ConsentLevel::Forget => Self::Forget,
        }
    }
}

impl From<ConsentState> for crate::m1_foundation::m01_types::ConsentLevel {
    fn from(state: ConsentState) -> Self {
        match state {
            ConsentState::Emit => Self::Emit,
            ConsentState::Store => Self::Store,
            ConsentState::Forget => Self::Forget,
        }
    }
}

// ---------------------------------------------------------------------------
// T1 — HabitatEvent
// ---------------------------------------------------------------------------

/// Mirrors STDB T1 — causal event log.
///
/// **WASM module definition (DO NOT import `spacetimedb` here):**
/// ```ignore
/// #[spacetimedb::table(accessor = habitat_event, public)]
/// pub struct HabitatEvent {
///     #[primary_key]
///     #[auto_inc]
///     id: u64,
///     event_type: String,
///     source_service: String,
///     sphere_id: Option<String>,
///     causal_parent: Option<u64>,
///     severity: u8,
///     confidence: f64,
///     payload_json: String,
///     session_id: Option<String>,
///     tick: u64,
///     #[index(btree)]
///     timestamp: spacetimedb::Timestamp,
/// }
/// ```
///
/// Written by reducer R1 `ingest_event`.  Growth ~26 000 events/day at
/// steady state.  Severity ≥ 7 triggers a [`WatcherObservation`] creation.
///
/// # Causal linkage
///
/// `causal_parent` chains effect events to their cause:
/// - `emergence.detected` → thermal/coupling event at the same ORAC tick
/// - `sphere.registered` → `SessionStart` hook event
/// - `thermal.adjustment` → gradient snapshot that crossed the threshold
/// - `command.postexec` → `command.preexec` for the same command hash
/// - `watcher.observation` → gradient snapshot that triggered the detector
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HabitatEvent {
    /// Auto-incrementing primary key.
    pub id: u64,
    /// Dot-namespaced event type e.g. `"emergence.detected"`, `"sphere.registered"`.
    pub event_type: String,
    /// Source service name e.g. `"orac-sidecar"`, `"pane-vortex"`, `"synthex-v2"`.
    pub source_service: String,
    /// Consent-gated sphere identifier (NA-R5).
    pub sphere_id: Option<String>,
    /// Links effect to cause — foreign key into [`HabitatEvent::id`].
    pub causal_parent: Option<u64>,
    /// Event severity 0–10.  Values ≥ 7 trigger Watcher observation creation.
    pub severity: u8,
    /// Confidence score 0.0–1.0.
    pub confidence: f64,
    /// Event-specific data serialised as JSON.
    pub payload_json: String,
    /// Claude Code session identifier e.g. `"S109"`.
    pub session_id: Option<String>,
    /// ORAC generation tick at which the event occurred.
    pub tick: u64,
    /// ISO 8601 timestamp (mirrors `spacetimedb::Timestamp` in the WASM module).
    pub timestamp: String,
}

// ---------------------------------------------------------------------------
// T2 — KnowledgeEdge
// ---------------------------------------------------------------------------

/// Mirrors STDB T2 — unified weighted knowledge graph.
///
/// **WASM module definition (DO NOT import `spacetimedb` here):**
/// ```ignore
/// #[spacetimedb::table(accessor = knowledge_edge, public)]
/// pub struct KnowledgeEdge {
///     #[primary_key]
///     #[auto_inc]
///     id: u64,
///     #[index(btree)]
///     source_id: String,
///     #[index(btree)]
///     target_id: String,
///     edge_type: String,
///     namespace: String,
///     weight: f64,
///     reinforcement_count: u32,
///     co_activations: u32,
///     ltp_count: u32,
///     ltd_count: u32,
///     stdp_delta: f64,
///     is_bidirectional: bool,
///     ltm_eligible: bool,
///     thermal_class: String,
///     learning_rate_ltp: f64,
///     learning_rate_ltd: f64,
///     decay_rate: f64,
///     consolidation_interval_ticks: Option<u64>,
///     created_at: spacetimedb::Timestamp,
///     last_reinforced: spacetimedb::Timestamp,
/// }
/// ```
///
/// Consolidates POVM pathways (~3 554), `service_tracking.db` learned
/// patterns (141), orchestration graph (29), `hebbian_pulse.db` neural
/// pathways (109), `system_synergy.db` synergy edges (89), and cross-agent
/// learnings (12) — total ~3 922+.
///
/// Written by R2 `reinforce_edge`; decayed by R5 `run_decay` using the
/// per-edge `decay_rate` (NA-R1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeEdge {
    /// Auto-incrementing primary key.
    pub id: u64,
    /// Source node identifier (indexed).
    pub source_id: String,
    /// Target node identifier (indexed).
    pub target_id: String,
    /// Edge type — one of the [`EdgeType`] variants serialised as a string.
    pub edge_type: String,
    /// POVM-compatible namespace e.g. `"synthex_v2_daemon_*"`, `"CC_Coordination_*"`.
    pub namespace: String,
    /// Unified weight 0.0–1.0.
    pub weight: f64,
    /// Reinforcement count — tracks how often this edge has been re-activated.
    pub reinforcement_count: u32,
    /// Number of co-activation events.
    pub co_activations: u32,
    /// Long-term potentiation event count.
    pub ltp_count: u32,
    /// Long-term depression event count.
    pub ltd_count: u32,
    /// Net STDP delta since last consolidation.
    pub stdp_delta: f64,
    /// Whether the edge models a bidirectional relationship.
    pub is_bidirectional: bool,
    /// Whether this edge is eligible for long-term memory consolidation.
    pub ltm_eligible: bool,
    /// Thermal class — one of `"critical"`, `"hot"`, `"warm"`, `"cool"`, `"cold"`.
    pub thermal_class: String,
    /// Per-edge LTP learning rate (NA-R1 — preserves substrate-specific plasticity).
    pub learning_rate_ltp: f64,
    /// Per-edge LTD learning rate (NA-R1).
    pub learning_rate_ltd: f64,
    /// Per-edge decay rate (NA-R1 — NOT a global constant).
    pub decay_rate: f64,
    /// POVM-origin consolidation interval in ticks; `None` for non-POVM edges.
    pub consolidation_interval_ticks: Option<u64>,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
    /// ISO 8601 timestamp of the most recent reinforcement.
    pub last_reinforced: String,
}

// ---------------------------------------------------------------------------
// T3 — GradientSnapshot
// ---------------------------------------------------------------------------

/// Mirrors STDB T3 — time-series Habitat vital signs.
///
/// **WASM module definition (DO NOT import `spacetimedb` here):**
/// ```ignore
/// #[spacetimedb::table(accessor = gradient_snapshot, public)]
/// pub struct GradientSnapshot {
///     #[primary_key]
///     #[auto_inc]
///     id: u64,
///     source: String,
///     temperature: f64,
///     thermal_target: f64,
///     thermal_delta: f64,
///     pv2_r: f64,
///     pv2_spheres: u32,
///     pv2_k_mod: f64,
///     ralph_gen: u64,
///     ralph_fitness: f64,
///     ralph_phase: String,
///     ltp_total: u64,
///     ltd_total: u64,
///     ltp_ltd_ratio: f64,
///     povm_pathways: u32,
///     povm_memories: u32,
///     me_health: f64,
///     me_fitness: f64,
///     hs_001_hebbian: f64,
///     hs_002_cascade: f64,
///     hs_003_resonance: f64,
///     flow_state: f64,
///     is_healthy: bool,
///     system_grade: String,
///     orac_system_grade: Option<String>,
///     pv2_fleet_mode: Option<String>,
///     synthex_pid_converging: Option<bool>,
///     me_overall_health: Option<f64>,
///     session_id: Option<String>,
///     #[index(btree)]
///     timestamp: spacetimedb::Timestamp,
/// }
/// ```
///
/// Written by R3 `capture_gradient` (scheduled every 60 s).  Capture rate
/// ~1 440/day; downsampled after 7 days (1/hour) and 30 days (1/day).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GradientSnapshot {
    /// Auto-incrementing primary key.
    pub id: u64,
    /// Probe source — `"synthex-v1"`, `"synthex-v2"`, or `"orac-probe"`.
    pub source: String,

    // Thermal dimension (D0)
    /// Current SYNTHEX temperature.
    pub temperature: f64,
    /// SYNTHEX thermal target set-point.
    pub thermal_target: f64,
    /// `temperature - thermal_target`.
    pub thermal_delta: f64,

    // PV2 dimension (D1)
    /// PV2 Kuramoto order parameter r ∈ [0, 1].
    pub pv2_r: f64,
    /// Number of active PV2 spheres.
    pub pv2_spheres: u32,
    /// PV2 coupling modulation factor.
    pub pv2_k_mod: f64,

    // RALPH dimension (D2)
    /// RALPH evolution generation counter.
    pub ralph_gen: u64,
    /// RALPH fitness score.
    pub ralph_fitness: f64,
    /// RALPH phase name e.g. `"Recognize"`, `"Learn"`, `"Plan"`.
    pub ralph_phase: String,

    // Hebbian dimensions (D3–D4)
    /// Cumulative LTP event count.
    pub ltp_total: u64,
    /// Cumulative LTD event count.
    pub ltd_total: u64,
    /// `ltp_total / max(ltd_total, 1)`.
    pub ltp_ltd_ratio: f64,

    // POVM dimension (D5)
    /// Number of POVM pathways at time of capture.
    pub povm_pathways: u32,
    /// Number of POVM memory entries at time of capture.
    pub povm_memories: u32,

    // ME dimension (D6)
    /// Maintenance Engine overall health score 0.0–1.0.
    pub me_health: f64,
    /// Maintenance Engine fitness score 0.0–1.0.
    pub me_fitness: f64,

    // Heat sources
    /// HS-001 Hebbian pulse heat contribution.
    pub hs_001_hebbian: f64,
    /// HS-002 cascade heat contribution.
    pub hs_002_cascade: f64,
    /// HS-003 resonance heat contribution.
    pub hs_003_resonance: f64,

    // Flow state (D10)
    /// Aggregate flow-state score 0.0–1.0.
    pub flow_state: f64,

    // Derived
    /// Whether the Habitat is considered healthy at this snapshot.
    pub is_healthy: bool,
    /// Letter grade e.g. `"A"`, `"B+"`, `"S"`.
    pub system_grade: String,

    // NA-R6: service self-reported health
    /// ORAC system grade reported by the ORAC sidecar itself.
    pub orac_system_grade: Option<String>,
    /// PV2 fleet mode e.g. `"solo"`, `"fleet"`.
    pub pv2_fleet_mode: Option<String>,
    /// Whether the SYNTHEX PID controller has converged.
    pub synthex_pid_converging: Option<bool>,
    /// ME overall health as reported by the ME service.
    pub me_overall_health: Option<f64>,

    /// Claude Code session identifier.
    pub session_id: Option<String>,
    /// ISO 8601 capture timestamp (indexed in WASM module).
    pub timestamp: String,
}

// ---------------------------------------------------------------------------
// T4 — SessionRecord
// ---------------------------------------------------------------------------

/// Mirrors STDB T4 — Claude Code session tracking.
///
/// **WASM module definition (DO NOT import `spacetimedb` here):**
/// ```ignore
/// #[spacetimedb::table(accessor = session_record, public)]
/// pub struct SessionRecord {
///     #[primary_key]
///     session_id: String,
///     session_number: u32,
///     #[index(btree)]
///     started_at: spacetimedb::Timestamp,
///     ended_at: Option<spacetimedb::Timestamp>,
///     pane_id: Option<String>,
///     tab_name: Option<String>,
///     persona: Option<String>,
///     model: String,
///     fitness_start: f64,
///     fitness_end: Option<f64>,
///     fitness_delta: Option<f64>,
///     events_count: u32,
///     commits_count: u32,
///     tools_used: u32,
///     priorities_json: String,
///     blockers_json: String,
///     status: String,
/// }
/// ```
///
/// Written by R4 `register_session` / `close_session`, triggered by ORAC
/// `SessionStart` / `SessionStop` hooks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionRecord {
    /// Primary key — unique session identifier e.g. `"S109-2026-04-24T10:00Z"`.
    pub session_id: String,
    /// Human-readable session number (S108, S109, …).
    pub session_number: u32,
    /// ISO 8601 session start timestamp (indexed).
    pub started_at: String,
    /// ISO 8601 session end timestamp; `None` while active.
    pub ended_at: Option<String>,
    /// Zellij pane identifier.
    pub pane_id: Option<String>,
    /// Zellij tab name.
    pub tab_name: Option<String>,
    /// Active persona — `"Zen"`, `"Cipher"`, `"Watcher"`, or `None`.
    pub persona: Option<String>,
    /// Model identifier e.g. `"opus-4-7"`, `"sonnet-4-6"`.
    pub model: String,
    /// RALPH fitness at session start.
    pub fitness_start: f64,
    /// RALPH fitness at session end; `None` while active.
    pub fitness_end: Option<f64>,
    /// `fitness_end - fitness_start`; `None` while active.
    pub fitness_delta: Option<f64>,
    /// Number of [`HabitatEvent`] rows created during this session.
    pub events_count: u32,
    /// Number of git commits made during this session.
    pub commits_count: u32,
    /// Number of tool invocations during this session.
    pub tools_used: u32,
    /// JSON array of session priorities.
    pub priorities_json: String,
    /// JSON array of session blockers.
    pub blockers_json: String,
    /// Session status — `"active"`, `"completed"`, `"crashed"`, or `"expired"`.
    pub status: String,
}

// ---------------------------------------------------------------------------
// T5 — StdbWorkstream
// ---------------------------------------------------------------------------

/// Mirrors STDB T5 — in-flight work ledger.
///
/// Prefixed `Stdb` to avoid a naming conflict with the L2 `Workstream` struct
/// from `m08_workstream`.
///
/// **WASM module definition (DO NOT import `spacetimedb` here):**
/// ```ignore
/// #[spacetimedb::table(accessor = workstream, public)]
/// pub struct Workstream {
///     #[primary_key]
///     id: String,
///     #[index(btree)]
///     service_name: String,
///     goal: String,
///     current_tier: u8,
///     current_stage: String,
///     status: String,
///     confidence: f64,
///     session_id: Option<String>,
///     blocker_description: Option<String>,
///     started_at: spacetimedb::Timestamp,
///     updated_at: spacetimedb::Timestamp,
///     completed_at: Option<spacetimedb::Timestamp>,
/// }
/// ```
///
/// Migrates from V3 `workflow_state.db` (4 rows, 6-tier model) and
/// `synthex-v2/workflow_tracking.db` (2 rows).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StdbWorkstream {
    /// Primary key — workstream identifier e.g. `"WS-6-habitat-wire"`.
    pub id: String,
    /// Owning service name (indexed).
    pub service_name: String,
    /// Human-readable workstream goal.
    pub goal: String,
    /// Implementation tier 1–6 in the 6-tier SYNTHEX model.
    pub current_tier: u8,
    /// Current implementation stage e.g. `"Phase 0"`, `"Phase 1"`.
    pub current_stage: String,
    /// Status — `"in_progress"`, `"completed"`, `"blocked"`, or `"deferred"`.
    pub status: String,
    /// Confidence that this workstream will complete as planned.
    pub confidence: f64,
    /// Claude Code session that last touched this workstream.
    pub session_id: Option<String>,
    /// Human-readable description of the current blocker, if any.
    pub blocker_description: Option<String>,
    /// ISO 8601 timestamp when work began.
    pub started_at: String,
    /// ISO 8601 timestamp of the most recent update.
    pub updated_at: String,
    /// ISO 8601 timestamp of completion; `None` while in progress.
    pub completed_at: Option<String>,
}

// ---------------------------------------------------------------------------
// T6 — ServiceHealth
// ---------------------------------------------------------------------------

/// Mirrors STDB T6 — service health timeline.
///
/// **WASM module definition (DO NOT import `spacetimedb` here):**
/// ```ignore
/// #[spacetimedb::table(accessor = service_health, public)]
/// pub struct ServiceHealth {
///     #[primary_key]
///     #[auto_inc]
///     id: u64,
///     #[index(btree)]
///     service_id: String,
///     port: u16,
///     health_status: String,
///     http_code: u16,
///     circuit_state: String,
///     successes: u64,
///     failures: u64,
///     timeouts: u64,
///     response_time_ms: f64,
///     #[index(btree)]
///     timestamp: spacetimedb::Timestamp,
/// }
/// ```
///
/// Consolidates `synthex-v2/bridge_health.db` (9 bridge circuits),
/// `service_tracking.db` services (12 rows), `system_synergy.db`
/// `integration_health` pairs, and `agent_deployment.db` agents (46 rows).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServiceHealth {
    /// Auto-incrementing primary key.
    pub id: u64,
    /// Service identifier e.g. `"orac-sidecar"`, `"synthex"` (indexed).
    pub service_id: String,
    /// TCP port the service listens on.
    pub port: u16,
    /// Health status — `"healthy"`, `"unhealthy"`, or `"unknown"`.
    pub health_status: String,
    /// HTTP response code returned by the health endpoint.
    pub http_code: u16,
    /// Circuit-breaker state — `"closed"`, `"open"`, or `"half_open"`.
    pub circuit_state: String,
    /// Cumulative successful health-check count.
    pub successes: u64,
    /// Cumulative failed health-check count.
    pub failures: u64,
    /// Cumulative timed-out health-check count.
    pub timeouts: u64,
    /// Most recent health-check response time in milliseconds.
    pub response_time_ms: f64,
    /// ISO 8601 timestamp of this health reading (indexed).
    pub timestamp: String,
}

// ---------------------------------------------------------------------------
// T7 — TrapState
// ---------------------------------------------------------------------------

/// Mirrors STDB T7 — active trap monitor.
///
/// **WASM module definition (DO NOT import `spacetimedb` here):**
/// ```ignore
/// #[spacetimedb::table(accessor = trap_state, public)]
/// pub struct TrapState {
///     #[primary_key]
///     trap_name: String,
///     is_active: bool,
///     last_checked: spacetimedb::Timestamp,
///     last_triggered: Option<spacetimedb::Timestamp>,
///     trigger_count: u32,
///     description: String,
/// }
/// ```
///
/// Powers the ACTIVE TRAPS section of the bootstrap injection payload.
/// The 18 known traps are documented in the vault `T7 — TrapState` note.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrapState {
    /// Primary key — trap identifier e.g. `"cp-alias"`, `"pkill-exit-144"`.
    pub trap_name: String,
    /// Whether this trap is currently active (i.e. may fire unexpectedly).
    pub is_active: bool,
    /// ISO 8601 timestamp of the most recent check.
    pub last_checked: String,
    /// ISO 8601 timestamp of the most recent trigger; `None` if never triggered.
    pub last_triggered: Option<String>,
    /// Cumulative trigger count across all sessions.
    pub trigger_count: u32,
    /// Human-readable description of the trap and how to avoid it.
    pub description: String,
}

// ---------------------------------------------------------------------------
// T8 — WatcherObservation
// ---------------------------------------------------------------------------

/// Mirrors STDB T8 — Watcher anomaly records.
///
/// **WASM module definition (DO NOT import `spacetimedb` here):**
/// ```ignore
/// #[spacetimedb::table(accessor = watcher_observation, public)]
/// pub struct WatcherObservation {
///     #[primary_key]
///     observation_id: String,
///     observer_role: String,
///     anomaly_class: String,
///     severity: u8,
///     metric_json: String,
///     classifier_output: Option<String>,
///     model: String,
///     cost_cents: u32,
///     caused_by_event: Option<u64>,
///     timestamp: spacetimedb::Timestamp,
/// }
/// ```
///
/// Direct port of `synthex-v2/watcher_observation.db` schema.  Written by
/// R10 `watcher_annotate_event` and by R1 `ingest_event` when severity ≥ 7.
///
/// # Sub-roles
///
/// | Role | Function |
/// |------|----------|
/// | `observer` | 1 Hz anomaly detection, Haiku-based |
/// | `critic` | Evaluates observation significance, Opus-based |
/// | `verifier` | Confirms or refutes critic assessment |
/// | `proposer` | Generates improvement proposals with Ember gate |
/// | `innovator` | Self-modification proposals (PBFT quorum required) |
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatcherObservation {
    /// Primary key — `UUIDv7` observation identifier.
    pub observation_id: String,
    /// Watcher sub-role — `"observer"`, `"critic"`, `"verifier"`, `"proposer"`, or `"innovator"`.
    pub observer_role: String,
    /// Anomaly class — `"nominal"`, `"thermal_drift"`, `"saturation"`, `"cascade"`, etc.
    pub anomaly_class: String,
    /// Observation severity 0–10.
    pub severity: u8,
    /// Metric values captured at observation time, serialised as JSON.
    pub metric_json: String,
    /// Raw classifier output text; `None` for rule-based observations.
    pub classifier_output: Option<String>,
    /// Model used — `"haiku"`, `"opus"`, or `"rule-based"`.
    pub model: String,
    /// Inference cost in US cents × 100 (i.e. micro-dollars).
    pub cost_cents: u32,
    /// Foreign key into [`HabitatEvent::id`] — the event that caused this observation.
    pub caused_by_event: Option<u64>,
    /// ISO 8601 observation timestamp.
    pub timestamp: String,
}

// ---------------------------------------------------------------------------
// Reducer signatures (R1–R10)
// ---------------------------------------------------------------------------

/// Reducer signatures for documentation and migration planning.
///
/// These are **reference types only** — not executable STDB reducer code.
/// In the WASM module, each function would be decorated with
/// `#[spacetimedb::reducer]`.
///
/// # R1 · `ingest_event`
///
/// The primary write path.  Called by the ingester for every event from ORAC,
/// PV2, SYNTHEX, and Atuin.  Consent-gates by `sphere_id` before persisting.
/// Triggers [`WatcherObservation`] creation if severity ≥ 7.
///
/// ```ignore
/// #[spacetimedb::reducer]
/// pub fn ingest_event(ctx: &ReducerContext, event: HabitatEvent) { ... }
/// ```
///
/// # R2 · `reinforce_edge`
///
/// Increments `reinforcement_count`, adjusts `weight`, updates LTP/LTD
/// counters.  Creates the edge if none exists.  Solves the S101 audit finding
/// ("only 1 pattern ever reinforced >1×").
///
/// ```ignore
/// #[spacetimedb::reducer]
/// pub fn reinforce_edge(ctx: &ReducerContext, source_id: String, target_id: String,
///                        edge_type: String, namespace: String) { ... }
/// ```
///
/// # R3 · `capture_gradient`
///
/// Scheduled every 60 s.  Captures a [`GradientSnapshot`] from consolidated
/// service probes.  Includes NA-R6 self-reported health fields.
///
/// ```ignore
/// #[spacetimedb::reducer]
/// pub fn capture_gradient(ctx: &ReducerContext) { ... }
/// ```
///
/// # R4 · `register_session` / `close_session`
///
/// Called by ORAC `SessionStart` / `SessionStop` hooks.  Creates/closes a
/// [`SessionRecord`] and captures `fitness_start` / `fitness_end`.
///
/// ```ignore
/// #[spacetimedb::reducer]
/// pub fn register_session(ctx: &ReducerContext, session_id: String,
///                          session_number: u32, model: String) { ... }
/// #[spacetimedb::reducer]
/// pub fn close_session(ctx: &ReducerContext, session_id: String) { ... }
/// ```
///
/// # R5 · `run_decay`
///
/// Scheduled every 6 hours.  Applies per-edge Hebbian decay to stale
/// [`KnowledgeEdge`] rows using the per-edge `decay_rate` (NA-R1).
/// Respects the Ember-gate: Watcher-referenced edges are preserved.
///
/// ```ignore
/// #[spacetimedb::reducer]
/// pub fn run_decay(ctx: &ReducerContext) { ... }
/// ```
///
/// # R6 · `forget_sphere`
///
/// NA-P-13 cascade.  Deletes/redacts all data for a sphere across T1, T2,
/// and T3.  Preserves the forget event itself for causal trace.
///
/// ```ignore
/// #[spacetimedb::reducer]
/// pub fn forget_sphere(ctx: &ReducerContext, sphere_id: String) { ... }
/// ```
///
/// # R7 · `compact_old_events`
///
/// Scheduled every 24 h.  Retention policy: >30 days → delete `payload_json`;
/// >90 days → delete row.  Gradient snapshots: >7 days → 1/hour;
/// >30 days → 1/day.
///
/// ```ignore
/// #[spacetimedb::reducer]
/// pub fn compact_old_events(ctx: &ReducerContext) { ... }
/// ```
///
/// # R8 · `consolidate_mature_edges`
///
/// Scheduled at 300-tick intervals.  Replicates the POVM consolidation cycle
/// for POVM-origin edges only (NA-C1).
///
/// ```ignore
/// #[spacetimedb::reducer]
/// pub fn consolidate_mature_edges(ctx: &ReducerContext) { ... }
/// ```
///
/// # R9 · `watcher_reinforce`
///
/// Callable by the Watcher via ingester relay.  Overrides decay on specific
/// edges the Watcher considers important (NA-C4).
///
/// ```ignore
/// #[spacetimedb::reducer]
/// pub fn watcher_reinforce(ctx: &ReducerContext, edge_id: u64) { ... }
/// ```
///
/// # R10 · `watcher_annotate_event`
///
/// Watcher annotates any [`HabitatEvent`] with severity/anomaly assessment,
/// creating a linked [`WatcherObservation`] (NA-C4).
///
/// ```ignore
/// #[spacetimedb::reducer]
/// pub fn watcher_annotate_event(ctx: &ReducerContext, event_id: u64,
///                                anomaly_class: String, severity: u8,
///                                metric_json: String) { ... }
/// ```
pub mod reducers {
    /// Function signature type for R1 `ingest_event`.
    ///
    /// `(event: HabitatEvent) -> Result<(), String>`
    pub type IngestEvent = fn(super::HabitatEvent) -> Result<(), String>;

    /// Function signature type for R2 `reinforce_edge`.
    ///
    /// `(source_id: &str, target_id: &str, edge_type: &str, namespace: &str) -> Result<(), String>`
    pub type ReinforceEdge = fn(&str, &str, &str, &str) -> Result<(), String>;

    /// Function signature type for R3 `capture_gradient`.
    ///
    /// `() -> Result<(), String>`
    pub type CaptureGradient = fn() -> Result<(), String>;

    /// Function signature type for R4a `register_session`.
    ///
    /// `(session_id: &str, session_number: u32, model: &str) -> Result<(), String>`
    pub type RegisterSession = fn(&str, u32, &str) -> Result<(), String>;

    /// Function signature type for R4b `close_session`.
    ///
    /// `(session_id: &str) -> Result<(), String>`
    pub type CloseSession = fn(&str) -> Result<(), String>;

    /// Function signature type for R5 `run_decay`.
    ///
    /// `() -> Result<(), String>`
    pub type RunDecay = fn() -> Result<(), String>;

    /// Function signature type for R6 `forget_sphere`.
    ///
    /// `(sphere_id: &str) -> Result<(), String>`
    pub type ForgetSphere = fn(&str) -> Result<(), String>;

    /// Function signature type for R7 `compact_old_events`.
    ///
    /// `() -> Result<(), String>`
    pub type CompactOldEvents = fn() -> Result<(), String>;

    /// Function signature type for R8 `consolidate_mature_edges`.
    ///
    /// `() -> Result<(), String>`
    pub type ConsolidateMatureEdges = fn() -> Result<(), String>;

    /// Function signature type for R9 `watcher_reinforce`.
    ///
    /// `(edge_id: u64) -> Result<(), String>`
    pub type WatcherReinforce = fn(u64) -> Result<(), String>;

    /// Function signature type for R10 `watcher_annotate_event`.
    ///
    /// `(event_id: u64, anomaly_class: &str, severity: u8, metric_json: &str) -> Result<(), String>`
    pub type WatcherAnnotateEvent = fn(u64, &str, u8, &str) -> Result<(), String>;
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

/// Default Hebbian learning parameters for new knowledge edges.
///
/// Returns `(ltp_rate, ltd_rate, decay_rate)`:
/// - `ltp_rate` — 0.05 (conservative upward reinforcement)
/// - `ltd_rate` — 0.03 (slower depression than potentiation)
/// - `decay_rate` — 0.95 (5% decay per decay cycle, ~Ebbinghaus approximation)
#[must_use]
pub fn default_learning_params() -> (f64, f64, f64) {
    (0.05, 0.03, 0.95)
}

/// Validate a [`HabitatEvent`].
///
/// # Errors
///
/// Returns a human-readable error string when:
/// - `severity` > 10
/// - `confidence` is outside [0.0, 1.0] or is NaN
/// - `event_type` is empty
/// - `source_service` is empty
#[must_use = "validation result must be checked — ignoring it skips safety checks"]
pub fn validate_event(event: &HabitatEvent) -> Result<(), String> {
    if event.severity > 10 {
        return Err(format!(
            "severity {} exceeds maximum 10",
            event.severity
        ));
    }
    if event.confidence.is_nan() || !(0.0..=1.0).contains(&event.confidence) {
        return Err(format!(
            "confidence {} is not in [0.0, 1.0]",
            event.confidence
        ));
    }
    if event.event_type.is_empty() {
        return Err("event_type must not be empty".to_string());
    }
    if event.source_service.is_empty() {
        return Err("source_service must not be empty".to_string());
    }
    Ok(())
}

/// Validate a [`KnowledgeEdge`].
///
/// # Errors
///
/// Returns a human-readable error string when:
/// - `weight` is outside [0.0, 1.0] or is NaN
/// - `edge_type` is not one of the recognised [`EdgeType`] values
/// - `source_id` is empty
/// - `target_id` is empty
#[must_use = "validation result must be checked — ignoring it skips safety checks"]
pub fn validate_edge(edge: &KnowledgeEdge) -> Result<(), String> {
    if edge.weight.is_nan() || !(0.0..=1.0).contains(&edge.weight) {
        return Err(format!("weight {} is not in [0.0, 1.0]", edge.weight));
    }
    if EdgeType::parse(&edge.edge_type).is_none() {
        return Err(format!("unknown edge_type {:?}", edge.edge_type));
    }
    if edge.source_id.is_empty() {
        return Err("source_id must not be empty".to_string());
    }
    if edge.target_id.is_empty() {
        return Err("target_id must not be empty".to_string());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Helper constructors
    // -----------------------------------------------------------------------

    fn sample_event() -> HabitatEvent {
        HabitatEvent {
            id: 1,
            event_type: "emergence.detected".to_string(),
            source_service: "orac-sidecar".to_string(),
            sphere_id: Some("sphere-alpha".to_string()),
            causal_parent: None,
            severity: 5,
            confidence: 0.9,
            payload_json: "{}".to_string(),
            session_id: Some("S109".to_string()),
            tick: 25652,
            timestamp: "2026-04-24T10:00:00Z".to_string(),
        }
    }

    fn sample_edge() -> KnowledgeEdge {
        KnowledgeEdge {
            id: 1,
            source_id: "orac-sidecar".to_string(),
            target_id: "synthex-v2".to_string(),
            edge_type: "synergy".to_string(),
            namespace: "synthex_v2_daemon_plan_root".to_string(),
            weight: 0.75,
            reinforcement_count: 3,
            co_activations: 10,
            ltp_count: 8,
            ltd_count: 2,
            stdp_delta: 0.12,
            is_bidirectional: true,
            ltm_eligible: true,
            thermal_class: "warm".to_string(),
            learning_rate_ltp: 0.05,
            learning_rate_ltd: 0.03,
            decay_rate: 0.95,
            consolidation_interval_ticks: Some(300),
            created_at: "2026-04-24T09:00:00Z".to_string(),
            last_reinforced: "2026-04-24T10:00:00Z".to_string(),
        }
    }

    fn sample_gradient() -> GradientSnapshot {
        GradientSnapshot {
            id: 1,
            source: "orac-probe".to_string(),
            temperature: 0.244,
            thermal_target: 0.5,
            thermal_delta: -0.256,
            pv2_r: 0.0,
            pv2_spheres: 0,
            pv2_k_mod: 1.0,
            ralph_gen: 25652,
            ralph_fitness: 0.669,
            ralph_phase: "Recognize".to_string(),
            ltp_total: 47,
            ltd_total: 0,
            ltp_ltd_ratio: 47.0,
            povm_pathways: 3554,
            povm_memories: 0,
            me_health: 0.542,
            me_fitness: 0.525,
            hs_001_hebbian: 0.1,
            hs_002_cascade: 0.05,
            hs_003_resonance: 0.02,
            flow_state: 0.6,
            is_healthy: true,
            system_grade: "A".to_string(),
            orac_system_grade: Some("A".to_string()),
            pv2_fleet_mode: Some("solo".to_string()),
            synthex_pid_converging: Some(false),
            me_overall_health: Some(0.542),
            session_id: Some("S109".to_string()),
            timestamp: "2026-04-24T10:00:00Z".to_string(),
        }
    }

    fn sample_session() -> SessionRecord {
        SessionRecord {
            session_id: "S109-2026-04-24T10:00Z".to_string(),
            session_number: 109,
            started_at: "2026-04-24T10:00:00Z".to_string(),
            ended_at: None,
            pane_id: Some("ALPHA-LEFT".to_string()),
            tab_name: Some("Orchestrator".to_string()),
            persona: None,
            model: "sonnet-4-6".to_string(),
            fitness_start: 0.669,
            fitness_end: None,
            fitness_delta: None,
            events_count: 0,
            commits_count: 0,
            tools_used: 0,
            priorities_json: "[]".to_string(),
            blockers_json: "[]".to_string(),
            status: "active".to_string(),
        }
    }

    fn sample_workstream() -> StdbWorkstream {
        StdbWorkstream {
            id: "WS-6-habitat-wire".to_string(),
            service_name: "pane-vortex".to_string(),
            goal: "Wire habitat IPC layer".to_string(),
            current_tier: 3,
            current_stage: "Phase 1".to_string(),
            status: "in_progress".to_string(),
            confidence: 0.8,
            session_id: Some("S109".to_string()),
            blocker_description: None,
            started_at: "2026-04-20T09:00:00Z".to_string(),
            updated_at: "2026-04-24T10:00:00Z".to_string(),
            completed_at: None,
        }
    }

    fn sample_service_health() -> ServiceHealth {
        ServiceHealth {
            id: 1,
            service_id: "orac-sidecar".to_string(),
            port: 8133,
            health_status: "healthy".to_string(),
            http_code: 200,
            circuit_state: "closed".to_string(),
            successes: 12000,
            failures: 3,
            timeouts: 0,
            response_time_ms: 4.2,
            timestamp: "2026-04-24T10:00:00Z".to_string(),
        }
    }

    fn sample_trap_state() -> TrapState {
        TrapState {
            trap_name: "cp-alias".to_string(),
            is_active: true,
            last_checked: "2026-04-24T10:00:00Z".to_string(),
            last_triggered: Some("2026-04-22T14:30:00Z".to_string()),
            trigger_count: 2,
            description: "`cp` is aliased to interactive — use `\\cp -f`".to_string(),
        }
    }

    fn sample_observation() -> WatcherObservation {
        WatcherObservation {
            observation_id: "01J3X000000000000000000000".to_string(),
            observer_role: "observer".to_string(),
            anomaly_class: "thermal_drift".to_string(),
            severity: 4,
            metric_json: r#"{"temperature":0.244,"target":0.5}"#.to_string(),
            classifier_output: None,
            model: "rule-based".to_string(),
            cost_cents: 0,
            caused_by_event: Some(1001),
            timestamp: "2026-04-24T10:00:00Z".to_string(),
        }
    }

    // -----------------------------------------------------------------------
    // T1 — HabitatEvent construction and serde
    // -----------------------------------------------------------------------

    #[test]
    fn habitat_event_construct() {
        let ev = sample_event();
        assert_eq!(ev.id, 1);
        assert_eq!(ev.event_type, "emergence.detected");
        assert_eq!(ev.source_service, "orac-sidecar");
        assert_eq!(ev.severity, 5);
        assert!((ev.confidence - 0.9).abs() < f64::EPSILON);
        assert_eq!(ev.tick, 25652);
    }

    #[test]
    fn habitat_event_serde_roundtrip() {
        let ev = sample_event();
        let json = serde_json::to_string(&ev).unwrap();
        let back: HabitatEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(ev, back);
    }

    #[test]
    fn habitat_event_optional_fields_none() {
        let mut ev = sample_event();
        ev.sphere_id = None;
        ev.causal_parent = None;
        ev.session_id = None;
        let json = serde_json::to_string(&ev).unwrap();
        let back: HabitatEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.sphere_id, None);
        assert_eq!(back.causal_parent, None);
        assert_eq!(back.session_id, None);
    }

    #[test]
    fn habitat_event_optional_fields_some() {
        let ev = sample_event();
        assert_eq!(ev.sphere_id, Some("sphere-alpha".to_string()));
        assert_eq!(ev.session_id, Some("S109".to_string()));
    }

    #[test]
    fn habitat_event_causal_parent_links() {
        let mut child = sample_event();
        child.id = 2;
        child.causal_parent = Some(1);
        assert_eq!(child.causal_parent, Some(1));
    }

    // -----------------------------------------------------------------------
    // T2 — KnowledgeEdge construction and serde
    // -----------------------------------------------------------------------

    #[test]
    fn knowledge_edge_construct() {
        let edge = sample_edge();
        assert_eq!(edge.id, 1);
        assert_eq!(edge.source_id, "orac-sidecar");
        assert_eq!(edge.target_id, "synthex-v2");
        assert_eq!(edge.edge_type, "synergy");
        assert!((edge.weight - 0.75).abs() < f64::EPSILON);
        assert_eq!(edge.reinforcement_count, 3);
        assert!(edge.is_bidirectional);
        assert!(edge.ltm_eligible);
    }

    #[test]
    fn knowledge_edge_serde_roundtrip() {
        let edge = sample_edge();
        let json = serde_json::to_string(&edge).unwrap();
        let back: KnowledgeEdge = serde_json::from_str(&json).unwrap();
        assert_eq!(edge, back);
    }

    #[test]
    fn knowledge_edge_consolidation_interval_some() {
        let edge = sample_edge();
        assert_eq!(edge.consolidation_interval_ticks, Some(300));
    }

    #[test]
    fn knowledge_edge_consolidation_interval_none() {
        let mut edge = sample_edge();
        edge.edge_type = "hebbian".to_string();
        edge.consolidation_interval_ticks = None;
        let json = serde_json::to_string(&edge).unwrap();
        let back: KnowledgeEdge = serde_json::from_str(&json).unwrap();
        assert_eq!(back.consolidation_interval_ticks, None);
    }

    #[test]
    fn knowledge_edge_all_edge_types_roundtrip() {
        for et in ["learned_pattern", "hebbian", "orchestration", "povm", "synergy", "cross_agent"] {
            let mut edge = sample_edge();
            edge.edge_type = et.to_string();
            let json = serde_json::to_string(&edge).unwrap();
            let back: KnowledgeEdge = serde_json::from_str(&json).unwrap();
            assert_eq!(back.edge_type, et);
        }
    }

    // -----------------------------------------------------------------------
    // T3 — GradientSnapshot construction and serde
    // -----------------------------------------------------------------------

    #[test]
    fn gradient_snapshot_construct() {
        let gs = sample_gradient();
        assert_eq!(gs.source, "orac-probe");
        assert!((gs.temperature - 0.244).abs() < f64::EPSILON);
        assert_eq!(gs.ralph_gen, 25652);
        assert!((gs.ralph_fitness - 0.669).abs() < f64::EPSILON);
        assert_eq!(gs.ralph_phase, "Recognize");
        assert_eq!(gs.povm_pathways, 3554);
        assert!(gs.is_healthy);
    }

    #[test]
    fn gradient_snapshot_serde_roundtrip() {
        let gs = sample_gradient();
        let json = serde_json::to_string(&gs).unwrap();
        let back: GradientSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(gs, back);
    }

    #[test]
    fn gradient_snapshot_optional_fields() {
        let mut gs = sample_gradient();
        gs.orac_system_grade = None;
        gs.pv2_fleet_mode = None;
        gs.synthex_pid_converging = None;
        gs.me_overall_health = None;
        gs.session_id = None;
        let json = serde_json::to_string(&gs).unwrap();
        let back: GradientSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back.orac_system_grade, None);
        assert_eq!(back.pv2_fleet_mode, None);
        assert_eq!(back.synthex_pid_converging, None);
        assert_eq!(back.me_overall_health, None);
        assert_eq!(back.session_id, None);
    }

    #[test]
    fn gradient_snapshot_heat_sources_present() {
        let gs = sample_gradient();
        assert!((gs.hs_001_hebbian - 0.1).abs() < f64::EPSILON);
        assert!((gs.hs_002_cascade - 0.05).abs() < f64::EPSILON);
        assert!((gs.hs_003_resonance - 0.02).abs() < f64::EPSILON);
    }

    // -----------------------------------------------------------------------
    // T4 — SessionRecord construction and serde
    // -----------------------------------------------------------------------

    #[test]
    fn session_record_construct() {
        let sr = sample_session();
        assert_eq!(sr.session_id, "S109-2026-04-24T10:00Z");
        assert_eq!(sr.session_number, 109);
        assert_eq!(sr.model, "sonnet-4-6");
        assert_eq!(sr.status, "active");
        assert!((sr.fitness_start - 0.669).abs() < f64::EPSILON);
    }

    #[test]
    fn session_record_serde_roundtrip() {
        let sr = sample_session();
        let json = serde_json::to_string(&sr).unwrap();
        let back: SessionRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(sr, back);
    }

    #[test]
    fn session_record_optional_fields_none_while_active() {
        let sr = sample_session();
        assert_eq!(sr.ended_at, None);
        assert_eq!(sr.fitness_end, None);
        assert_eq!(sr.fitness_delta, None);
    }

    #[test]
    fn session_record_closed_session() {
        let mut sr = sample_session();
        sr.ended_at = Some("2026-04-24T12:00:00Z".to_string());
        sr.fitness_end = Some(0.72);
        sr.fitness_delta = Some(0.051);
        sr.status = "completed".to_string();
        assert!(sr.fitness_delta.is_some());
    }

    // -----------------------------------------------------------------------
    // T5 — StdbWorkstream construction and serde
    // -----------------------------------------------------------------------

    #[test]
    fn stdb_workstream_construct() {
        let ws = sample_workstream();
        assert_eq!(ws.id, "WS-6-habitat-wire");
        assert_eq!(ws.service_name, "pane-vortex");
        assert_eq!(ws.current_tier, 3);
        assert_eq!(ws.status, "in_progress");
        assert!((ws.confidence - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn stdb_workstream_serde_roundtrip() {
        let ws = sample_workstream();
        let json = serde_json::to_string(&ws).unwrap();
        let back: StdbWorkstream = serde_json::from_str(&json).unwrap();
        assert_eq!(ws, back);
    }

    #[test]
    fn stdb_workstream_completed_at_none() {
        let ws = sample_workstream();
        assert_eq!(ws.completed_at, None);
        assert_eq!(ws.blocker_description, None);
    }

    #[test]
    fn stdb_workstream_blocked_state() {
        let mut ws = sample_workstream();
        ws.status = "blocked".to_string();
        ws.blocker_description = Some("apt not available".to_string());
        assert_eq!(ws.blocker_description, Some("apt not available".to_string()));
    }

    // -----------------------------------------------------------------------
    // T6 — ServiceHealth construction and serde
    // -----------------------------------------------------------------------

    #[test]
    fn service_health_construct() {
        let sh = sample_service_health();
        assert_eq!(sh.service_id, "orac-sidecar");
        assert_eq!(sh.port, 8133);
        assert_eq!(sh.health_status, "healthy");
        assert_eq!(sh.http_code, 200);
        assert_eq!(sh.circuit_state, "closed");
        assert!((sh.response_time_ms - 4.2).abs() < f64::EPSILON);
    }

    #[test]
    fn service_health_serde_roundtrip() {
        let sh = sample_service_health();
        let json = serde_json::to_string(&sh).unwrap();
        let back: ServiceHealth = serde_json::from_str(&json).unwrap();
        assert_eq!(sh, back);
    }

    #[test]
    fn service_health_open_circuit() {
        let mut sh = sample_service_health();
        sh.circuit_state = "open".to_string();
        sh.health_status = "unhealthy".to_string();
        sh.http_code = 503;
        assert_eq!(sh.circuit_state, "open");
    }

    // -----------------------------------------------------------------------
    // T7 — TrapState construction and serde
    // -----------------------------------------------------------------------

    #[test]
    fn trap_state_construct() {
        let ts = sample_trap_state();
        assert_eq!(ts.trap_name, "cp-alias");
        assert!(ts.is_active);
        assert_eq!(ts.trigger_count, 2);
        assert!(ts.last_triggered.is_some());
    }

    #[test]
    fn trap_state_serde_roundtrip() {
        let ts = sample_trap_state();
        let json = serde_json::to_string(&ts).unwrap();
        let back: TrapState = serde_json::from_str(&json).unwrap();
        assert_eq!(ts, back);
    }

    #[test]
    fn trap_state_never_triggered() {
        let mut ts = sample_trap_state();
        ts.last_triggered = None;
        ts.trigger_count = 0;
        ts.is_active = false;
        assert_eq!(ts.last_triggered, None);
        assert!(!ts.is_active);
    }

    #[test]
    fn trap_state_all_18_known_traps() {
        let trap_names = [
            "cp-alias", "pkill-exit-144", "rm-tsv-only", "povm-hydrate-broken",
            "bridge-url-prefix", "pswarm-port-10002", "synthex-api-health",
            "me-port-8180", "zellij-wasm-no-http", "pv2-ipc-socket",
            "synthex-v2-no-v3", "povm-pathways-plural", "unwrap-in-wasm",
            "timer-5s-minimum", "focus-next-pane", "synthex-ws-collision",
            "orac-breakers-cascade", "pv2-governance-gated",
        ];
        assert_eq!(trap_names.len(), 18);
        for name in &trap_names {
            let ts = TrapState {
                trap_name: (*name).to_string(),
                is_active: true,
                last_checked: "2026-04-24T10:00:00Z".to_string(),
                last_triggered: None,
                trigger_count: 0,
                description: format!("Trap: {name}"),
            };
            let json = serde_json::to_string(&ts).unwrap();
            let back: TrapState = serde_json::from_str(&json).unwrap();
            assert_eq!(back.trap_name, *name);
        }
    }

    // -----------------------------------------------------------------------
    // T8 — WatcherObservation construction and serde
    // -----------------------------------------------------------------------

    #[test]
    fn watcher_observation_construct() {
        let wo = sample_observation();
        assert_eq!(wo.observer_role, "observer");
        assert_eq!(wo.anomaly_class, "thermal_drift");
        assert_eq!(wo.severity, 4);
        assert_eq!(wo.model, "rule-based");
        assert_eq!(wo.cost_cents, 0);
        assert_eq!(wo.caused_by_event, Some(1001));
    }

    #[test]
    fn watcher_observation_serde_roundtrip() {
        let wo = sample_observation();
        let json = serde_json::to_string(&wo).unwrap();
        let back: WatcherObservation = serde_json::from_str(&json).unwrap();
        assert_eq!(wo, back);
    }

    #[test]
    fn watcher_observation_all_roles() {
        for role in ["observer", "critic", "verifier", "proposer", "innovator"] {
            let mut wo = sample_observation();
            wo.observer_role = role.to_string();
            let json = serde_json::to_string(&wo).unwrap();
            let back: WatcherObservation = serde_json::from_str(&json).unwrap();
            assert_eq!(back.observer_role, role);
        }
    }

    #[test]
    fn watcher_observation_no_causal_link() {
        let mut wo = sample_observation();
        wo.caused_by_event = None;
        let json = serde_json::to_string(&wo).unwrap();
        let back: WatcherObservation = serde_json::from_str(&json).unwrap();
        assert_eq!(back.caused_by_event, None);
    }

    // -----------------------------------------------------------------------
    // EdgeType enum
    // -----------------------------------------------------------------------

    #[test]
    fn edge_type_as_str_roundtrip() {
        for et in [
            EdgeType::LearnedPattern,
            EdgeType::Hebbian,
            EdgeType::Orchestration,
            EdgeType::Povm,
            EdgeType::Synergy,
            EdgeType::CrossAgent,
        ] {
            let s = et.as_str();
            assert_eq!(EdgeType::parse(s), Some(et));
        }
    }

    #[test]
    fn edge_type_unknown_returns_none() {
        assert_eq!(EdgeType::parse("unknown_type"), None);
        assert_eq!(EdgeType::parse(""), None);
    }

    #[test]
    fn edge_type_display() {
        assert_eq!(EdgeType::Synergy.to_string(), "synergy");
        assert_eq!(EdgeType::LearnedPattern.to_string(), "learned_pattern");
        assert_eq!(EdgeType::CrossAgent.to_string(), "cross_agent");
    }

    #[test]
    fn edge_type_serde_roundtrip() {
        for et in [
            EdgeType::LearnedPattern,
            EdgeType::Hebbian,
            EdgeType::Orchestration,
            EdgeType::Povm,
            EdgeType::Synergy,
            EdgeType::CrossAgent,
        ] {
            let json = serde_json::to_string(&et).unwrap();
            let back: EdgeType = serde_json::from_str(&json).unwrap();
            assert_eq!(et, back);
        }
    }

    // -----------------------------------------------------------------------
    // EventCategory enum
    // -----------------------------------------------------------------------

    #[test]
    fn event_category_classify() {
        assert_eq!(EventCategory::classify("emergence.detected"), EventCategory::Emergence);
        assert_eq!(EventCategory::classify("sphere.registered"), EventCategory::Sphere);
        assert_eq!(EventCategory::classify("thermal.adjustment"), EventCategory::Thermal);
        assert_eq!(EventCategory::classify("command.postexec"), EventCategory::Command);
        assert_eq!(EventCategory::classify("watcher.observation"), EventCategory::Watcher);
        assert_eq!(EventCategory::classify("session.start"), EventCategory::Session);
        assert_eq!(EventCategory::classify("service.restart"), EventCategory::Service);
        assert_eq!(EventCategory::classify("hebbian.pulse"), EventCategory::Hebbian);
        assert_eq!(EventCategory::classify("custom.unknown"), EventCategory::Other);
        assert_eq!(EventCategory::classify(""), EventCategory::Other);
    }

    #[test]
    fn event_category_classify_dotless_known_prefix() {
        assert_eq!(EventCategory::classify("emergence"), EventCategory::Emergence);
        assert_eq!(EventCategory::classify("thermal"), EventCategory::Thermal);
    }

    #[test]
    fn event_category_classify_dotless_unknown() {
        assert_eq!(EventCategory::classify("foobar"), EventCategory::Other);
    }

    #[test]
    fn event_category_prefix() {
        assert_eq!(EventCategory::Emergence.prefix(), "emergence");
        assert_eq!(EventCategory::Other.prefix(), "other");
    }

    #[test]
    fn event_category_display() {
        assert_eq!(EventCategory::Thermal.to_string(), "thermal");
    }

    // -----------------------------------------------------------------------
    // ConsentState enum
    // -----------------------------------------------------------------------

    #[test]
    fn consent_state_as_str() {
        assert_eq!(ConsentState::Emit.as_str(), "Emit");
        assert_eq!(ConsentState::Store.as_str(), "Store");
        assert_eq!(ConsentState::Forget.as_str(), "Forget");
    }

    #[test]
    fn consent_state_parse() {
        assert_eq!(ConsentState::parse("Emit"), Some(ConsentState::Emit));
        assert_eq!(ConsentState::parse("Store"), Some(ConsentState::Store));
        assert_eq!(ConsentState::parse("Forget"), Some(ConsentState::Forget));
        assert_eq!(ConsentState::parse("emit"), None);
        assert_eq!(ConsentState::parse(""), None);
    }

    #[test]
    fn consent_state_permits_injection() {
        assert!(ConsentState::Emit.permits_injection());
        assert!(!ConsentState::Store.permits_injection());
        assert!(!ConsentState::Forget.permits_injection());
    }

    #[test]
    fn consent_state_display() {
        assert_eq!(ConsentState::Emit.to_string(), "Emit");
        assert_eq!(ConsentState::Forget.to_string(), "Forget");
    }

    #[test]
    fn consent_state_serde_roundtrip() {
        for cs in [ConsentState::Emit, ConsentState::Store, ConsentState::Forget] {
            let json = serde_json::to_string(&cs).unwrap();
            let back: ConsentState = serde_json::from_str(&json).unwrap();
            assert_eq!(cs, back);
        }
    }

    #[test]
    fn consent_state_from_consent_level() {
        use crate::m1_foundation::m01_types::ConsentLevel;
        assert_eq!(ConsentState::from(ConsentLevel::Emit), ConsentState::Emit);
        assert_eq!(ConsentState::from(ConsentLevel::Store), ConsentState::Store);
        assert_eq!(ConsentState::from(ConsentLevel::Forget), ConsentState::Forget);
    }

    #[test]
    fn consent_level_from_consent_state() {
        use crate::m1_foundation::m01_types::ConsentLevel;
        assert_eq!(ConsentLevel::from(ConsentState::Emit), ConsentLevel::Emit);
        assert_eq!(ConsentLevel::from(ConsentState::Store), ConsentLevel::Store);
        assert_eq!(ConsentLevel::from(ConsentState::Forget), ConsentLevel::Forget);
    }

    #[test]
    fn consent_roundtrip_level_to_state_and_back() {
        use crate::m1_foundation::m01_types::ConsentLevel;
        for level in [ConsentLevel::Emit, ConsentLevel::Store, ConsentLevel::Forget] {
            let state: ConsentState = level.into();
            let back: ConsentLevel = state.into();
            assert_eq!(level, back);
        }
    }

    // -----------------------------------------------------------------------
    // validate_event
    // -----------------------------------------------------------------------

    #[test]
    fn validate_event_valid() {
        assert!(validate_event(&sample_event()).is_ok());
    }

    #[test]
    fn validate_event_severity_zero_ok() {
        let mut ev = sample_event();
        ev.severity = 0;
        assert!(validate_event(&ev).is_ok());
    }

    #[test]
    fn validate_event_severity_ten_ok() {
        let mut ev = sample_event();
        ev.severity = 10;
        assert!(validate_event(&ev).is_ok());
    }

    #[test]
    fn validate_event_severity_eleven_rejected() {
        let mut ev = sample_event();
        ev.severity = 11;
        assert!(validate_event(&ev).is_err());
    }

    #[test]
    fn validate_event_severity_255_rejected() {
        let mut ev = sample_event();
        ev.severity = 255;
        let err = validate_event(&ev).unwrap_err();
        assert!(err.contains("severity"));
    }

    #[test]
    fn validate_event_confidence_zero_ok() {
        let mut ev = sample_event();
        ev.confidence = 0.0;
        assert!(validate_event(&ev).is_ok());
    }

    #[test]
    fn validate_event_confidence_one_ok() {
        let mut ev = sample_event();
        ev.confidence = 1.0;
        assert!(validate_event(&ev).is_ok());
    }

    #[test]
    fn validate_event_confidence_negative_rejected() {
        let mut ev = sample_event();
        ev.confidence = -0.1;
        let err = validate_event(&ev).unwrap_err();
        assert!(err.contains("confidence"));
    }

    #[test]
    fn validate_event_confidence_above_one_rejected() {
        let mut ev = sample_event();
        ev.confidence = 1.001;
        assert!(validate_event(&ev).is_err());
    }

    #[test]
    fn validate_event_confidence_nan_rejected() {
        let mut ev = sample_event();
        ev.confidence = f64::NAN;
        let err = validate_event(&ev).unwrap_err();
        assert!(err.contains("confidence"));
    }

    #[test]
    fn validate_event_empty_event_type_rejected() {
        let mut ev = sample_event();
        ev.event_type = String::new();
        let err = validate_event(&ev).unwrap_err();
        assert!(err.contains("event_type"));
    }

    #[test]
    fn validate_event_empty_source_service_rejected() {
        let mut ev = sample_event();
        ev.source_service = String::new();
        let err = validate_event(&ev).unwrap_err();
        assert!(err.contains("source_service"));
    }

    // -----------------------------------------------------------------------
    // validate_edge
    // -----------------------------------------------------------------------

    #[test]
    fn validate_edge_valid() {
        assert!(validate_edge(&sample_edge()).is_ok());
    }

    #[test]
    fn validate_edge_weight_zero_ok() {
        let mut edge = sample_edge();
        edge.weight = 0.0;
        assert!(validate_edge(&edge).is_ok());
    }

    #[test]
    fn validate_edge_weight_one_ok() {
        let mut edge = sample_edge();
        edge.weight = 1.0;
        assert!(validate_edge(&edge).is_ok());
    }

    #[test]
    fn validate_edge_weight_negative_rejected() {
        let mut edge = sample_edge();
        edge.weight = -0.1;
        let err = validate_edge(&edge).unwrap_err();
        assert!(err.contains("weight"));
    }

    #[test]
    fn validate_edge_weight_above_one_rejected() {
        let mut edge = sample_edge();
        edge.weight = 1.001;
        assert!(validate_edge(&edge).is_err());
    }

    #[test]
    fn validate_edge_weight_nan_rejected() {
        let mut edge = sample_edge();
        edge.weight = f64::NAN;
        let err = validate_edge(&edge).unwrap_err();
        assert!(err.contains("weight"));
    }

    #[test]
    fn validate_edge_unknown_edge_type_rejected() {
        let mut edge = sample_edge();
        edge.edge_type = "invalid_type".to_string();
        let err = validate_edge(&edge).unwrap_err();
        assert!(err.contains("edge_type"));
    }

    #[test]
    fn validate_edge_all_valid_edge_types_accepted() {
        for et in ["learned_pattern", "hebbian", "orchestration", "povm", "synergy", "cross_agent"] {
            let mut edge = sample_edge();
            edge.edge_type = et.to_string();
            assert!(validate_edge(&edge).is_ok(), "edge_type {et} should be valid");
        }
    }

    #[test]
    fn validate_edge_empty_source_id_rejected() {
        let mut edge = sample_edge();
        edge.source_id = String::new();
        let err = validate_edge(&edge).unwrap_err();
        assert!(err.contains("source_id"));
    }

    #[test]
    fn validate_edge_empty_target_id_rejected() {
        let mut edge = sample_edge();
        edge.target_id = String::new();
        let err = validate_edge(&edge).unwrap_err();
        assert!(err.contains("target_id"));
    }

    // -----------------------------------------------------------------------
    // default_learning_params
    // -----------------------------------------------------------------------

    #[test]
    fn default_learning_params_values() {
        let (ltp, ltd, decay) = default_learning_params();
        assert!((ltp - 0.05).abs() < f64::EPSILON, "ltp_rate should be 0.05, got {ltp}");
        assert!((ltd - 0.03).abs() < f64::EPSILON, "ltd_rate should be 0.03, got {ltd}");
        assert!((decay - 0.95).abs() < f64::EPSILON, "decay_rate should be 0.95, got {decay}");
    }

    #[test]
    fn default_learning_params_in_valid_range() {
        let (ltp, ltd, decay) = default_learning_params();
        assert!(ltp > 0.0 && ltp < 1.0, "ltp_rate out of range");
        assert!(ltd > 0.0 && ltd < 1.0, "ltd_rate out of range");
        assert!(decay > 0.0 && decay <= 1.0, "decay_rate out of range");
    }

    #[test]
    fn default_learning_params_ltp_greater_than_ltd() {
        let (ltp, ltd, _) = default_learning_params();
        assert!(ltp > ltd, "LTP rate should exceed LTD rate by convention");
    }

    // -----------------------------------------------------------------------
    // Reducer type alias sanity (compile-time only — just check sizes exist)
    // -----------------------------------------------------------------------

    #[test]
    fn reducer_types_are_sized() {
        // Verify the type aliases are valid by checking their size is non-zero.
        assert!(std::mem::size_of::<reducers::IngestEvent>() > 0);
        assert!(std::mem::size_of::<reducers::ReinforceEdge>() > 0);
        assert!(std::mem::size_of::<reducers::CaptureGradient>() > 0);
        assert!(std::mem::size_of::<reducers::RegisterSession>() > 0);
        assert!(std::mem::size_of::<reducers::CloseSession>() > 0);
        assert!(std::mem::size_of::<reducers::RunDecay>() > 0);
        assert!(std::mem::size_of::<reducers::ForgetSphere>() > 0);
        assert!(std::mem::size_of::<reducers::CompactOldEvents>() > 0);
        assert!(std::mem::size_of::<reducers::ConsolidateMatureEdges>() > 0);
        assert!(std::mem::size_of::<reducers::WatcherReinforce>() > 0);
        assert!(std::mem::size_of::<reducers::WatcherAnnotateEvent>() > 0);
    }

    // -----------------------------------------------------------------------
    // Field presence spot-checks (ensure no typos collapsed fields)
    // -----------------------------------------------------------------------

    #[test]
    fn gradient_snapshot_ltp_ltd_fields_present() {
        let gs = sample_gradient();
        assert_eq!(gs.ltp_total, 47);
        assert_eq!(gs.ltd_total, 0);
        // ltp_ltd_ratio should equal ltp_total / max(ltd_total, 1)
        let expected = gs.ltp_total as f64 / 1_f64.max(gs.ltd_total as f64);
        assert!((gs.ltp_ltd_ratio - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn knowledge_edge_per_edge_learning_params_present() {
        let edge = sample_edge();
        assert!((edge.learning_rate_ltp - 0.05).abs() < f64::EPSILON);
        assert!((edge.learning_rate_ltd - 0.03).abs() < f64::EPSILON);
        assert!((edge.decay_rate - 0.95).abs() < f64::EPSILON);
    }

    #[test]
    fn service_health_counts_present() {
        let sh = sample_service_health();
        assert_eq!(sh.successes, 12000);
        assert_eq!(sh.failures, 3);
        assert_eq!(sh.timeouts, 0);
    }

    #[test]
    fn watcher_observation_cost_field_present() {
        let wo = sample_observation();
        assert_eq!(wo.cost_cents, 0); // rule-based is free
    }

    #[test]
    fn session_record_tools_commits_events_fields() {
        let sr = sample_session();
        assert_eq!(sr.events_count, 0);
        assert_eq!(sr.commits_count, 0);
        assert_eq!(sr.tools_used, 0);
    }
}
