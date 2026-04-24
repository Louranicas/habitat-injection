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
//! # Sub-modules
//!
//! | Module | Contents |
//! |--------|----------|
//! | [`enums`] | [`EdgeType`], [`EventCategory`], [`ConsentState`] |
//! | [`tables`] | T1–T8 table structs |
//! | [`reducers`] | R1–R10 type aliases |
//! | [`validation`] | [`validate_event`], [`validate_edge`], [`default_learning_params`] |
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
//! Layer: `m6_stdb`
//! Dependencies: none (plain types only)

pub mod enums;
pub mod reducers;
pub mod tables;
pub mod validation;

// Re-export all public items so existing code using `m22_stdb_module::*` still works.
pub use enums::{ConsentState, EdgeType, EventCategory};
pub use tables::{
    GradientSnapshot, HabitatEvent, KnowledgeEdge, ServiceHealth, SessionRecord, StdbWorkstream,
    TrapState, WatcherObservation,
};
pub use validation::{default_learning_params, validate_edge, validate_event};

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
