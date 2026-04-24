//! Table struct definitions that mirror the `SpaceTimeDB` WASM module schema.
//!
//! Contains T1–T8: [`HabitatEvent`], [`KnowledgeEdge`], [`GradientSnapshot`],
//! [`SessionRecord`], [`StdbWorkstream`], [`ServiceHealth`], [`TrapState`],
//! [`WatcherObservation`].
//!
//! **No `spacetimedb` or `spacetimedb-sdk` imports** — all STDB-specific
//! attributes appear only in doc comments describing the WASM module schema.

use serde::{Deserialize, Serialize};

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
    /// Edge type — one of the [`super::enums::EdgeType`] variants serialised as a string.
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
