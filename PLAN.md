---
type: strategic-plan
title: SpaceTimeDB Habitat Integration Plan — Ultimate Context & Memory Injection System
version: 1.0
date: 2026-04-24
session: 109
scope: cross-project (synthex-v2 · orac-sidecar · pane-vortex · habitat-zellij · habitat-obsidian · atuin-scripts)
tags: [plan, spacetimedb, memory, context-injection, sidecar, causal-memory, bootstrap]
status: AUTHORED v1 — awaits execution
supersedes: MASTER_PLAN.md Phase 6 (expands from 4-day sketch to full integration plan)
provenance: Master Plan v2.0 Phase 6 + Comms Layer v3 §10.4 + S103 ADR-002/ADR-004 + Memory Injection Roadmap S101 + Deep Audit 2026-04-22
---

> Back to: `~/claude-code-workspace/CLAUDE.md` · `~/claude-code-workspace/CLAUDE.local.md` · [Comms Layer Unification Plan v3](Comms%20Layer%20Unification%20Plan%20%E2%80%94%202026-04-24.md) · [Master Plan v2.0](~/Downloads/MASTER_PLAN.md) · [Memory Injection Roadmap](~/projects/claude_code/Advanced%20Clustered%20Tooling%20%E2%80%94%20S101%20Memory%20Injection%20Roadmap.md) · [Habitat Memory Substrate Audit](~/projects/claude_code/Habitat%20Memory%20Substrate%20%E2%80%94%20Deep%20Audit%202026-04-22.md)

# SpaceTimeDB Habitat Integration Plan
## The Ultimate Context & Memory Injection System for Claude Code

---

## 0. Executive Summary

**Goal:** Deploy SpaceTimeDB as a sidecar service in the Habitat, consolidating 21+ fragmented SQLite databases, 3554 POVM pathways, and 6 memory substrates into a single real-time causal memory substrate. At new context window start, Claude Code gets <100ms injection of *complete* Habitat state — not just current metrics (L0-L6 today), but trajectory, causation chains, session narrative, workstream ledger, synergy graph, and active trap state.

**Architecture decision (S103 ADR-002, confirmed):** Sidecar service, not Zellij plugin. WASM sandbox cannot run STDB SDK, open sockets, or persist data beyond terminal session. Score: Sidecar 91/100, Plugin 22/100 (assessed 2026-04-24).

**Shape:** 5 phases, ~40-50h focused engineering across ~8-10 sessions. Phase A (sidecar deploy) is independently valuable. Each subsequent phase adds a migration dimension. Phase E (the bootstrap revolution) is the payoff — `habitat-bootstrap-stdb` replaces the current 11-layer atuin script with a single STDB subscription that delivers complete causal state in <60ms.

**Key insight from schema audit:** The Habitat currently has 21 tracking databases, of which 11 are dead (zero data). The 10 live ones contain 5 distinct data patterns that map cleanly onto STDB tables. The fragmentation is accidental, not architectural — consolidation is net simplification.

---

## 1. Current State — What We're Consolidating

### 1.1 The Six Memory Substrates (live)

| # | Substrate | Access | Shape | Live Rows | Signal |
|---|-----------|--------|-------|-----------|--------|
| 1 | Auto-Memory | FS read (MEMORY.md + *.md) | Markdown files w/ YAML frontmatter | ~50 files | High — hand-curated, always loaded |
| 2 | Tracking DBs (21) | `sqlite3` | 10 live / 11 dead | ~1,800 rows total | Medium — rich schemas, poor reinforcement |
| 3 | POVM Engine | HTTP `:8125` | Hebbian pathways (pre_id→post_id→weight) | 3,554 pathways | High — densest graph |
| 4 | Reasoning Memory | HTTP `:8130` TSV | Key-value TSV (tick/r/gen/fitness/phase) | ~2,000 entries | Medium — heartbeat only, single-producer |
| 5 | VMS | HTTP `:8120` | Semantic memories, 12D tensor | 1,881 memories | Low — write pond, morphogenic_cycle=0 |
| 6 | Obsidian Vault | FS read | 215 markdown notes, wikilinks | 215 notes | High — canonical docs, human-authored |

### 1.2 The Current Bootstrap Chain (7 layers, 55ms)

| Layer | What | Source | Size |
|-------|------|--------|------|
| L0 | The Ember (identity) | atuin KV | ~500 bytes |
| L1 | Session state | KV + CLAUDE.local.md | ~1 KB |
| L2 | Live metrics | Parallel curl probes (r, gen, fitness, T) | ~800 bytes |
| L3 | Learned patterns | SQLite `service_tracking.db` | ~2 KB |
| L4 | Session context | Recent sessions + POVM crystals | ~1.5 KB |
| L5 | CLI muscle | 82 atuin scripts + binaries list | ~1 KB |
| L6 | Experience | Rolling arc + momentum + desires | ~2 KB |
| **Total** | | | **~9 KB, 55ms** |

### 1.3 What's Missing (from S101 Memory Injection Roadmap)

| Gap | Why it Matters | Current Cost |
|-----|----------------|--------------|
| **Trajectory** | Know fitness is 0.664, not that it climbed from 0.5 | Manual session note read |
| **Active workstreams** | In-flight / blocked / queued / held | Parse CLAUDE.local.md |
| **Causal chains** | "Why did sphere-7 restart?" has no answer surface | Unanswerable |
| **Active trap state** | 18 traps, don't know which are live | Probe each service |
| **Session narrative arc** | Last 3 sessions' feel + carry-forward | Read 4 files at boot |
| **Synergy graph** | Cross-service coupling map | `/cross-tensor` partial |
| **Commit ledger** | Cross-repo 24h activity | `git log` per repo |
| **Pattern reinforcement** | 141 patterns, only 1 ever reinforced >1× | Dead write, no read |

### 1.4 The Five Data Patterns in the Live Tracking DBs

Schema audit reveals 5 canonical shapes across all 10 live tracking databases:

| Pattern | Shape | Examples | STDB Mapping |
|---------|-------|----------|--------------|
| **Event Log** | `(id, timestamp, type, source, data_json)` | `service_events`, `workflow_events`, `pulse_events`, `optimization_events` | Event table + scheduled reducer |
| **Weighted Graph** | `(source, target, weight, type, reinforcement_count)` | `learned_patterns`, `orchestration_graph`, `neural_pathways`, `hebbian_pathways` | Table with STDB indexes + reinforcement reducer |
| **Metric Sample** | `(id, service_id, metric_name, value, timestamp, threshold)` | `performance_metrics`, `gradient_snapshot`, `flow_states` | Table + scheduled snapshot reducer |
| **Entity Registry** | `(id, name, status, config_json, created_at, updated_at)` | `services`, `agents`, `workflows` | Table with lifecycle reducers |
| **Relationship** | `(system_1, system_2, score, latency, success_rate)` | `system_synergy`, `agent_synergy`, `integration_health`, `data_flows` | Table + decay/reinforce reducers |

---

## 2. SpaceTimeDB Module Schema

### 2.1 Design Principles

1. **Every table maps to a live substrate** — no speculative tables
2. **Causal_parent chains** on event tables — the unique capability SQLite doesn't have
3. **STDB subscriptions replace polling** — Claude Code subscribes at session start, gets real-time push
4. **Typed newtypes** at boundaries — `SphereId`, `ServiceId`, `Tick`, `SessionId` (per Coding Excellence Charter)
5. **Reducer-only writes** — all mutations through STDB reducers, never raw INSERT
6. **Public tables for client read** — Claude Code's context injector subscribes
7. **Schedule tables for housekeeping** — decay, compaction, snapshot capture

### 2.2 Core Tables (Phase A)

```rust
// ═══════════════════════════════════════════════
// T1: Causal Event Log — the backbone
// ═══════════════════════════════════════════════
#[spacetimedb::table(accessor = habitat_event, public)]
pub struct HabitatEvent {
    #[primary_key]
    #[auto_inc]
    id: u64,
    
    event_type: String,         // "emergence.detected", "sphere.registered", "command.postexec", etc.
    source_service: String,     // "orac-sidecar", "pane-vortex", "synthex-v2", "atuin-hook"
    sphere_id: Option<String>,  // Subject sphere (consent-gated per NA-R5)
    
    causal_parent: Option<u64>, // ← THE KEY DIFFERENTIATOR: links effect to cause
    
    severity: u8,               // 0-10 (from watcher_observation schema)
    confidence: f64,            // 0.0-1.0 (from emergence detector)
    
    payload_json: String,       // Event-specific data (validated per WS-2b schemas)
    
    session_id: Option<String>, // Which Claude Code session produced/observed this
    tick: u64,                  // ORAC tick at time of event
    
    #[index(btree)]
    timestamp: spacetimedb::Timestamp,
}

// ═══════════════════════════════════════════════
// T2: Weighted Knowledge Graph — consolidates
//     learned_patterns + orchestration_graph +
//     neural_pathways + hebbian_pathways + POVM
// ═══════════════════════════════════════════════
#[spacetimedb::table(accessor = knowledge_edge, public)]
pub struct KnowledgeEdge {
    #[primary_key]
    #[auto_inc]
    id: u64,
    
    #[index(btree)]
    source_id: String,          // pre_id / source_module / pattern_name
    #[index(btree)]
    target_id: String,          // post_id / target_module
    
    edge_type: String,          // "learned_pattern", "hebbian", "orchestration", "povm", "synergy"
    namespace: String,          // "synthex_v2_daemon_*", "CC_Coordination_*", etc.
    
    weight: f64,                // 0.0-1.0 (unified from disparate scales)
    reinforcement_count: u32,   // How many times reinforced (the S101 audit's biggest gap)
    co_activations: u32,        // From POVM pathway data
    
    ltp_count: u32,             // Long-term potentiation events
    ltd_count: u32,             // Long-term depression events
    stdp_delta: f64,            // Net STDP change
    
    is_bidirectional: bool,
    ltm_eligible: bool,         // Long-term memory candidate
    
    thermal_class: String,      // "critical"|"hot"|"warm"|"cool"|"cold" (from v3_pattern_view)
    
    created_at: spacetimedb::Timestamp,
    last_reinforced: spacetimedb::Timestamp,
}

// ═══════════════════════════════════════════════
// T3: Gradient Snapshot — time-series of Habitat
//     vital signs (consolidates gradient_snapshot.db +
//     live probe data + RM heartbeat)
// ═══════════════════════════════════════════════
#[spacetimedb::table(accessor = gradient_snapshot, public)]
pub struct GradientSnapshot {
    #[primary_key]
    #[auto_inc]
    id: u64,
    
    source: String,             // "synthex-v1"|"synthex-v2"|"orac-probe"|"bootstrap"
    
    // Thermal (D0)
    temperature: f64,
    thermal_target: f64,
    thermal_delta: f64,
    
    // PV2 (D1)
    pv2_r: f64,
    pv2_spheres: u32,
    pv2_k_mod: f64,
    
    // RALPH (D2)
    ralph_gen: u64,
    ralph_fitness: f64,
    ralph_phase: String,
    
    // Hebbian (D3-D4)
    ltp_total: u64,
    ltd_total: u64,
    ltp_ltd_ratio: f64,
    
    // POVM (D5)
    povm_pathways: u32,
    povm_memories: u32,
    
    // ME (D6)
    me_health: f64,
    me_fitness: f64,
    
    // Heat sources
    hs_001_hebbian: f64,
    hs_002_cascade: f64,
    hs_003_resonance: f64,
    
    // Flow state (D10)
    flow_state: f64,
    
    // Derived
    is_healthy: bool,
    system_grade: String,       // "S"|"A"|"B"|"C"|"D"|"F"
    
    // Session context
    session_id: Option<String>,
    
    #[index(btree)]
    timestamp: spacetimedb::Timestamp,
}

// ═══════════════════════════════════════════════
// T4: Session Registry — tracks Claude Code
//     sessions across context windows
// ═══════════════════════════════════════════════
#[spacetimedb::table(accessor = session_record, public)]
pub struct SessionRecord {
    #[primary_key]
    session_id: String,         // Claude Code session ID
    
    session_number: u32,        // S108, S109, etc.
    
    #[index(btree)]
    started_at: spacetimedb::Timestamp,
    ended_at: Option<spacetimedb::Timestamp>,
    
    pane_id: Option<String>,    // Zellij pane
    tab_name: Option<String>,   // "Orchestrator", "Fleet-ALPHA", etc.
    persona: Option<String>,    // "Zen", "Cipher", "Watcher", None
    
    model: String,              // "opus-4-7", "sonnet-4-6"
    
    // Metrics at session boundary
    fitness_start: f64,
    fitness_end: Option<f64>,
    fitness_delta: Option<f64>,
    
    events_count: u32,
    commits_count: u32,
    tools_used: u32,
    
    // Carry-forward
    priorities_json: String,    // Parsed from CLAUDE.local.md
    blockers_json: String,
    
    status: String,             // "active"|"completed"|"crashed"|"expired"
}

// ═══════════════════════════════════════════════
// T5: Workstream Ledger — in-flight work across
//     all services (consolidates V3 workflow_state.db
//     + synthex-v2 workflow_tracking.db)
// ═══════════════════════════════════════════════
#[spacetimedb::table(accessor = workstream, public)]
pub struct Workstream {
    #[primary_key]
    id: String,
    
    #[index(btree)]
    service_name: String,
    
    goal: String,
    
    current_tier: u8,           // 1-6 (foundation→deploy)
    current_stage: String,
    status: String,             // "in_progress"|"completed"|"blocked"|"deferred"
    
    confidence: f64,
    
    session_id: Option<String>,
    
    blocker_description: Option<String>,
    
    started_at: spacetimedb::Timestamp,
    updated_at: spacetimedb::Timestamp,
    completed_at: Option<spacetimedb::Timestamp>,
}

// ═══════════════════════════════════════════════
// T6: Service Health Timeline — consolidates
//     bridge_health, service_events, integration_health
// ═══════════════════════════════════════════════
#[spacetimedb::table(accessor = service_health, public)]
pub struct ServiceHealth {
    #[primary_key]
    #[auto_inc]
    id: u64,
    
    #[index(btree)]
    service_id: String,         // "orac-sidecar", "pane-vortex", etc.
    
    port: u16,
    health_status: String,      // "healthy"|"unhealthy"|"unknown"
    http_code: u16,
    
    circuit_state: String,      // "closed"|"open"|"half_open"
    
    successes: u64,
    failures: u64,
    timeouts: u64,
    
    response_time_ms: f64,
    
    #[index(btree)]
    timestamp: spacetimedb::Timestamp,
}

// ═══════════════════════════════════════════════
// T7: Trap State — active trap monitoring
//     (the S101 Roadmap's "habitat-traps-live")
// ═══════════════════════════════════════════════
#[spacetimedb::table(accessor = trap_state, public)]
pub struct TrapState {
    #[primary_key]
    trap_name: String,          // "cp-alias", "pkill-exit-144", "rm-tsv-only", etc.
    
    is_active: bool,
    last_checked: spacetimedb::Timestamp,
    last_triggered: Option<spacetimedb::Timestamp>,
    trigger_count: u32,
    description: String,
}

// ═══════════════════════════════════════════════
// T8: Watcher Observation — from synthex-v2
//     watcher_observation.db (already STDB-shaped)
// ═══════════════════════════════════════════════
#[spacetimedb::table(accessor = watcher_observation, public)]
pub struct WatcherObservation {
    #[primary_key]
    observation_id: String,     // UUIDv7
    
    observer_role: String,      // "observer"|"critic"|"verifier"|"proposer"|"innovator"
    anomaly_class: String,      // "nominal"|"thermal_drift"|"saturation"|etc.
    severity: u8,
    
    metric_json: String,
    classifier_output: Option<String>,
    
    model: String,              // "haiku"|"opus"|"rule-based"
    cost_cents: u32,
    
    // Causal link to HabitatEvent
    caused_by_event: Option<u64>,
    
    timestamp: spacetimedb::Timestamp,
}
```

### 2.3 Context Injection View (Phase E — the payoff)

```rust
// ═══════════════════════════════════════════════
// V1: Bootstrap View — what Claude Code subscribes
//     to at context window start. Replaces habitat-
//     bootstrap's 7 layers + S101's 4 extensions
// ═══════════════════════════════════════════════
#[spacetimedb::view(anonymous)]  
pub fn context_window_bootstrap(ctx: &AnonymousViewContext) -> Vec<BootstrapPayload> {
    // L0-L1: Session state (latest SessionRecord)
    let session = ctx.db.session_record()
        .status().filter("active")  // current session
        .or(ctx.db.session_record().iter().last());  // fallback: most recent
    
    // L2: Live metrics (latest GradientSnapshot)
    let latest_gradient = ctx.db.gradient_snapshot()
        .iter().last();
    
    // L3: Top learned patterns by reinforcement × weight
    let top_patterns = ctx.db.knowledge_edge()
        .edge_type().filter("learned_pattern")
        // top 20 by weight, sorted
        ;
    
    // L7: Trajectory (last 5 gradient snapshots — fitness delta chain)
    let trajectory = ctx.db.gradient_snapshot()
        // last 5 by timestamp
        ;
    
    // L8: Active workstreams
    let workstreams = ctx.db.workstream()
        .status().filter("in_progress")
        ;
    
    // L9: Active traps
    let traps = ctx.db.trap_state()
        // where is_active = true
        ;
    
    // Fuse into single injection payload
    // ...
}
```

### 2.4 Key Reducers

```rust
// ═══════════════════════════════════════════════
// R1: Ingest event — the primary write path.
//     Called by ORAC, PV2, SyntheX, Atuin hooks.
// ═══════════════════════════════════════════════
#[spacetimedb::reducer]
pub fn ingest_event(
    ctx: &ReducerContext,
    event_type: String,
    source_service: String,
    sphere_id: Option<String>,
    causal_parent: Option<u64>,
    severity: u8,
    confidence: f64,
    payload_json: String,
    session_id: Option<String>,
    tick: u64,
) -> Result<(), String> {
    // Consent gate: check sphere consent before persisting
    // Insert into habitat_event
    // If severity >= 7, trigger watcher_observation creation
    ctx.db.habitat_event().insert(HabitatEvent {
        id: 0, // auto_inc
        event_type, source_service, sphere_id,
        causal_parent, severity, confidence,
        payload_json, session_id, tick,
        timestamp: ctx.timestamp,
    });
    Ok(())
}

// ═══════════════════════════════════════════════
// R2: Reinforce pattern — solves the S101 audit's
//     biggest gap ("only 1 pattern reinforced >1×")
// ═══════════════════════════════════════════════
#[spacetimedb::reducer]
pub fn reinforce_edge(
    ctx: &ReducerContext,
    source_id: String,
    target_id: String,
    delta_weight: f64,
    is_ltp: bool,
) -> Result<(), String> {
    // Find existing edge, increment reinforcement_count,
    // adjust weight, update ltp/ltd counters
    // If no edge exists, create with initial weight
    Ok(())
}

// ═══════════════════════════════════════════════
// R3: Capture gradient snapshot — called every
//     60s by scheduled reducer, consolidating
//     probes of all services
// ═══════════════════════════════════════════════
#[spacetimedb::reducer]
pub fn capture_gradient(ctx: &ReducerContext, snapshot: GradientSnapshot) -> Result<(), String> {
    ctx.db.gradient_snapshot().insert(snapshot);
    Ok(())
}

// ═══════════════════════════════════════════════
// R4: Register/close session — called by ORAC
//     SessionStart/Stop hooks
// ═══════════════════════════════════════════════
#[spacetimedb::reducer]
pub fn register_session(
    ctx: &ReducerContext,
    session_id: String,
    session_number: u32,
    pane_id: Option<String>,
    tab_name: Option<String>,
    model: String,
    fitness_start: f64,
) -> Result<(), String> {
    ctx.db.session_record().insert(SessionRecord {
        session_id, session_number,
        started_at: ctx.timestamp,
        ended_at: None,
        pane_id, tab_name,
        persona: None,
        model, fitness_start,
        fitness_end: None, fitness_delta: None,
        events_count: 0, commits_count: 0, tools_used: 0,
        priorities_json: "[]".into(),
        blockers_json: "[]".into(),
        status: "active".into(),
    });
    Ok(())
}

// ═══════════════════════════════════════════════
// R5: Decay — Hebbian decay on stale edges.
//     Scheduled every 6 hours.
// ═══════════════════════════════════════════════
#[spacetimedb::table(accessor = decay_schedule, scheduled(run_decay))]
pub struct DecaySchedule {
    #[primary_key]
    #[auto_inc]
    scheduled_id: u64,
    scheduled_at: spacetimedb::ScheduleAt,
}

#[spacetimedb::reducer]
pub fn run_decay(ctx: &ReducerContext, _arg: DecaySchedule) -> Result<(), String> {
    // For all knowledge_edges not reinforced in 7 days:
    //   weight *= 0.95 (decay factor)
    //   If weight < 0.05, mark for pruning
    // Log decay audit to habitat_event
    Ok(())
}

// ═══════════════════════════════════════════════
// R6: Forget cascade (NA-P-13) — delete all
//     data for a sphere across all tables
// ═══════════════════════════════════════════════
#[spacetimedb::reducer]
pub fn forget_sphere(ctx: &ReducerContext, sphere_id: String, reason: String) -> Result<(), String> {
    // Delete or redact all habitat_events for this sphere
    // Delete knowledge_edges mentioning this sphere
    // Delete gradient_snapshots scoped to this sphere
    // Log the forget event itself (causal trace preserved)
    Ok(())
}
```

---

## 3. Sidecar Architecture

### 3.1 Service Registration

```toml
# ~/.config/devenv/devenv.toml addition
[[services]]
id = "habitat-stdb"
name = "SpaceTimeDB Habitat Memory"
description = "Causal memory substrate — 8 tables, real-time subscriptions, <60ms bootstrap injection"
working_dir = "/home/louranicas/claude-code-workspace/habitat-stdb"
command = "spacetimedb-standalone"
args = ["--root-dir", "./data", "start", "--listen-addr", "127.0.0.1:3000"]
auto_start = true
auto_restart = true
max_restart_attempts = 5
restart_delay_secs = 5
health_check_interval_secs = 30
startup_timeout_secs = 30
shutdown_timeout_secs = 10
dependencies = []  # Batch 1 — no dependencies
health_check_url = "http://localhost:3000/v1/identity"

[services.env]
SPACETIMEDB_LOG = "info"

[services.resource_limits]
max_memory_mb = 1024
max_cpu_percent = 50
```

**Port:** `:3000` (STDB default). Batch 1 — no dependencies (STDB is a persistence substrate, other services depend on it, not the reverse).

### 3.2 Project Structure

```
habitat-stdb/
├── Cargo.toml                    # STDB module workspace
├── CLAUDE.md                     # Project context + traps
├── CLAUDE.local.md               # Session state
├── module/
│   ├── Cargo.toml                # spacetimedb = "2.1", lib.rs
│   └── src/
│       ├── lib.rs                # Module entry, table definitions, init reducer
│       ├── tables.rs             # T1-T8 table definitions
│       ├── reducers.rs           # R1-R6 reducers
│       ├── views.rs              # V1 context_window_bootstrap
│       └── migrations.rs         # Schema migration helpers
├── ingester/
│   ├── Cargo.toml                # Standalone Rust binary
│   └── src/
│       ├── main.rs               # Tokio runtime, multi-source ingester
│       ├── orac_bridge.rs        # Polls ORAC /health, /emergence, /ralph every 30s
│       ├── pv2_bridge.rs         # Subscribes PV2 /bus/ws for real-time events
│       ├── synthex_bridge.rs     # Polls SYNTHEX /v3/thermal every 60s
│       ├── povm_migrator.rs      # One-shot: migrates 3554 POVM pathways → knowledge_edge
│       ├── sqlite_migrator.rs    # One-shot: migrates 10 live tracking DBs → STDB tables
│       └── atuin_bridge.rs       # Subscribes atuin preexec/postexec via WS-4 hook
├── injector/
│   ├── Cargo.toml                # Context injection CLI tool
│   └── src/
│       └── main.rs               # Subscribes to STDB, formats injection payload, prints to stdout
├── scripts/
│   ├── deploy.sh                 # Build + publish module + start ingester
│   ├── migrate-povm.sh           # POVM → STDB one-shot migration
│   ├── migrate-tracking-dbs.sh   # SQLite → STDB one-shot migration
│   └── verify.sh                 # E2E verification script
├── data/                         # STDB data directory (WAL, snapshots)
└── tests/
    └── integration/
        ├── ingest_test.rs
        ├── query_test.rs
        └── bootstrap_test.rs
```

### 3.3 The Ingester — Multi-Source Event Pipeline

```
                    ┌────────────────┐
                    │  ORAC :8133    │──poll 30s──┐
                    └────────────────┘             │
                    ┌────────────────┐             │
                    │  PV2 :8132     │──WS /bus/ws─┤
                    └────────────────┘             │
                    ┌────────────────┐             ▼
                    │  SYNTHEX :8090 │──poll 60s──►┌──────────────┐     ┌─────────────┐
                    └────────────────┘             │   Ingester   │────►│  STDB :3000 │
                    ┌────────────────┐             │  (Rust bin)  │     │  8 tables   │
                    │  POVM :8125   │──poll 300s──►│              │     │  6 reducers │
                    └────────────────┘             └──────────────┘     └─────────────┘
                    ┌────────────────┐             ▲
                    │  Atuin hooks   │──via PV2 bus┘
                    └────────────────┘
```

The ingester is a standalone Rust binary (NOT inside the STDB module — STDB reducers can't do I/O). It:

1. Polls ORAC `/health`, `/emergence`, `/ralph`, `/coupling` every 30s
2. Subscribes to PV2 `/bus/ws` for real-time events (emergence, sphere, field, command)
3. Polls SYNTHEX `/v3/thermal` every 60s
4. Polls POVM `/pathways` every 300s for pathway weight updates
5. Receives Atuin command events via PV2 bus (WS-4)
6. Calls STDB reducers to persist everything

### 3.4 The Injector — Context Window Bootstrap

```
    Claude Code starts
         │
         ▼
    ORAC SessionStart hook fires
         │
         ▼
    Hook calls: habitat-stdb-inject
         │
         ▼
    ┌──────────────────────────┐
    │  Injector CLI            │
    │  1. Connect to STDB :3000│
    │  2. Subscribe to V1 view │
    │  3. Receive bootstrap    │
    │     payload (<60ms)      │
    │  4. Format as structured │
    │     text (≤15 KB)        │
    │  5. Print to stdout      │
    └──────────────────────────┘
         │
         ▼
    ORAC injects into Claude Code
    system message (SessionStart
    hook response body)
```

**Injection payload structure (target ≤15 KB):**

```
═══════════════════════════════════════════════
  HABITAT MEMORY INJECTION — SpaceTimeDB
═══════════════════════════════════════════════

SESSION: S109 | Model: opus-4-7 | Pane: Orchestrator/21
PREVIOUS: S108 (Watcher Persona + WCP v1) — fitness 0.664→0.669 (+0.005)

TRAJECTORY (last 5 snapshots):
  T-4: r=0.000 fit=0.660 gen=25460 phase=Recognize T=0.272
  T-3: r=0.000 fit=0.664 gen=25652 phase=Recognize T=0.244
  T-2: r=0.000 fit=0.664 gen=26068 phase=Recognize T=0.573
  T-1: r=0.000 fit=0.664 gen=26068 phase=Recognize T=0.573
  NOW: r=0.985 fit=0.669 gen=26080 phase=Recognize T=0.500

WORKSTREAMS:
  IN-FLIGHT: Comms Layer v3 (10/16 shipped) | synthex-v2 Phase G (blocked: v1 streaming)
  BLOCKED:   WS-6 habitat-wire (Zellij API verify) | Phase G (external gate)
  DEFERRED:  WS-8 Atuin reciprocation | WS-9 human-focus | WS-0 P4/P5

ACTIVE TRAPS (3/18):
  cp-alias: ACTIVE | povm-hydrate-broken: ACTIVE | rm-tsv-only: ACTIVE

TOP PATTERNS (reinforced):
  session-071-convergence-trap (7×) | clustered-parallel-paradigm (1×) | ...

CAUSAL CHAIN (last significant):
  E12329 emergence.coherence_lock → E12330 thermal_adjustment → E12331 k_modulation

SERVICES: 12/12 healthy | POVM: 3554 pathways | VMS: 1881 memories
═══════════════════════════════════════════════
```

---

## 4. Phase Plan

### Phase A — STDB Deploy + Core Tables (6-8h, 1-2 sessions)

**Deliverables:**
- Self-hosted STDB standalone on `:3000` via devenv
- Module with T1 (HabitatEvent), T3 (GradientSnapshot), T4 (SessionRecord), T6 (ServiceHealth)
- Basic ingester polling ORAC + PV2 + SYNTHEX
- R1 (ingest_event), R3 (capture_gradient), R4 (register/close session)

**Acceptance:**
- `spacetime sql habitat "SELECT COUNT(*) FROM habitat_event"` returns >0 after 5 minutes
- `spacetime logs habitat` shows reducer invocations
- devenv health check passes

**Dependencies:** None. Phase A is independently valuable — STDB captures event history from day 1.

### Phase B — Knowledge Graph Migration (8-10h, 2 sessions)

**Deliverables:**
- T2 (KnowledgeEdge) table + R2 (reinforce_edge) + R5 (decay schedule)
- `povm_migrator`: one-shot migration of 3554 POVM pathways → KnowledgeEdge
- `sqlite_migrator`: one-shot migration of 10 live tracking DBs → appropriate STDB tables
- T5 (Workstream) table populated from V3 workflow_state.db + synthex-v2 workflow_tracking.db
- T7 (TrapState) table populated by a new trap-probe reducer

**Acceptance:**
- `spacetime sql habitat "SELECT COUNT(*) FROM knowledge_edge"` ≥ 3554 (POVM) + 141 (patterns) + 29 (graph) + 109 (hebbian)
- Decay schedule fires every 6h, visible in logs
- `spacetime sql habitat "SELECT * FROM workstream WHERE status = 'in_progress'"` returns active work

**Key decision:** POVM continues as source-of-truth during migration period. The ingester polls POVM and syncs to STDB. After Phase D verification, POVM becomes a read-through cache.

### Phase C — Watcher + Causal Chains (6-8h, 1-2 sessions)

**Deliverables:**
- T8 (WatcherObservation) table — direct port of synthex-v2's watcher_observation.db schema
- Causal chain construction: ingester tags events with causal_parent when source provides attribution
- ORAC emergence events link causal_parent to the coupling/thermal event that triggered detection
- R6 (forget_sphere) — NA-P-13 cascade across all STDB tables
- Watcher proposal tracking (watcher_proposal → proposal_verdict from existing schema)

**Acceptance:**
- `spacetime sql habitat "SELECT * FROM habitat_event WHERE causal_parent IS NOT NULL LIMIT 5"` returns linked events
- Forget cascade: after `forget_sphere("test-sphere")`, zero rows reference that sphere
- Watcher observations from synthex-v2 shadow daemon appear in STDB within 60s

### Phase D — Cross-Service Integration (8-10h, 2 sessions)

**Deliverables:**
- ORAC bridge: SessionStart hook calls R4, Stop hook closes session, PostToolUse increments counters
- PV2 bus integration: ingester subscribes to `/bus/ws` with `client_id = "habitat-stdb-ingester"`
- Atuin integration: command events flow through PV2 bus → ingester → STDB
- Telegram integration: `/query` command routes to STDB SQL via ingester HTTP proxy
- Obsidian integration: Session Timeline view queries STDB via HTTP (not direct SDK — Obsidian is TypeScript)
- Health endpoint: ingester exposes `/health` + `/metrics` on `:3001`

**Acceptance:**
- Full round-trip: shell command → Atuin hook → PV2 bus → ingester → STDB → queryable via `spacetime sql`
- Telegram `/query "why did fitness drop last session?"` → causal chain response
- Obsidian Session Timeline view renders STDB data

### Phase E — Bootstrap Revolution (6-8h, 1-2 sessions)

**Deliverables:**
- `habitat-stdb-inject` CLI binary — the injector (see §3.4)
- V1 bootstrap view in STDB module
- ORAC SessionStart hook integration: calls injector, includes payload in system message
- `atuin scripts run habitat-bootstrap-stdb` — replacement for current `habitat-bootstrap`
- Retirement of 11 dead tracking databases (bus_tracking, code, devenv_tracking, episodic_memory, evolution_tracking, povm_data, povm_engine, security_tracking, synergy_tracking, tensor_memory, workflow_tracking)

**Acceptance:**
- New context window Claude Code session receives ≤15 KB injection with trajectory + workstreams + traps + causal chain + top patterns
- Injection latency: <100ms end-to-end (STDB subscription → format → stdout)
- Old `habitat-bootstrap` still works as fallback (STDB is additive, not replacing yet)
- 11 dead databases deleted from `developer_environment_manager/`

---

## 5. Migration Strategy (SQLite → STDB)

### 5.1 What migrates, what doesn't

| Source | Rows | Migrates To | Phase |
|--------|------|-------------|-------|
| `service_tracking.db` (learned_patterns) | 141 | T2 KnowledgeEdge (edge_type="learned_pattern") | B |
| `service_tracking.db` (orchestration_graph) | 29 | T2 KnowledgeEdge (edge_type="orchestration") | B |
| `hebbian_pulse.db` (neural_pathways) | 109+ | T2 KnowledgeEdge (edge_type="hebbian") | B |
| `hebbian_pulse.db` (hebbian_pathways) | 109 | T2 KnowledgeEdge (edge_type="hebbian") | B |
| `hebbian_pulse.db` (decay_audit_log) | 676 | T1 HabitatEvent (event_type="decay.cycle") | B |
| POVM `:8125` /pathways | 3,554 | T2 KnowledgeEdge (edge_type="povm") | B |
| `service_tracking.db` (service_events) | 27 | T1 HabitatEvent | B |
| `service_tracking.db` (cross_agent_learnings) | 12 | T2 KnowledgeEdge (edge_type="cross_agent") | B |
| `flow_state.db` (flow_states) | 12 | T3 GradientSnapshot (flow_state field) | B |
| `agent_deployment.db` (agents) | 46 | T6 ServiceHealth (agent entries) | B |
| `system_synergy.db` (system_synergy) | 89 | T2 KnowledgeEdge (edge_type="synergy") | B |
| V3 `workflow_state.db` (workflows) | 4 | T5 Workstream | B |
| synthex-v2 `gradient_snapshot.db` | 1 | T3 GradientSnapshot | A |
| synthex-v2 `bridge_health.db` | 9 | T6 ServiceHealth | A |
| synthex-v2 `watcher_observation.db` | 0 (scaffold) | T8 WatcherObservation | C |
| RM `:8130` heartbeat | ~2,000 | T3 GradientSnapshot (time series) | D |
| **11 dead DBs** | 0 | **DELETE** | E |

### 5.2 POVM remains dual-write during transition

```
Phase A-C:  ORAC → POVM (existing)
            Ingester polls POVM → syncs to STDB T2

Phase D:    ORAC → POVM (existing) + ORAC → STDB T1 (new)
            Ingester still syncs POVM → STDB T2

Phase E+:   ORAC → STDB (primary) → periodic snapshot to POVM (backup)
            POVM becomes read-through cache, not source-of-truth
```

POVM is never deleted — it's a reliable 3554-pathway substrate. But STDB becomes the primary write path for new patterns once Phase D is verified.

---

## 6. STDB ↔ Comms Layer v3 Alignment

Per Comms Layer v3 §10.4, every v3 mechanism maps to an STDB primitive:

| v3 Mechanism (shipped) | STDB Migration Path | Phase |
|-------------------------|---------------------|-------|
| `/bus/ingress` consent-gate (WS-3) | Row-level security on `habitat_event` by `sphere_id` | D |
| `/bus/ws` subscription patterns (WS-2a) | STDB typed query subscriptions | E |
| `/bus/forget` cascade (WS-2d) | R6 `forget_sphere` reducer | C |
| Event schemas (WS-2b) | STDB table schemas + typed bindings | A |
| `/bus/self` introspection (WS-2d) | STDB module metadata | D |
| Subscriber identity (WS-2a) | STDB OIDC Identity | Post-E |
| Auth token → OIDC | Single workstream (~5h) when STDB lands | Post-E |

---

## 7. Risk Register

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| STDB standalone crashes under load | Event loss | LOW | WAL persistence + ingester retry logic |
| STDB memory usage exceeds 1GB | OOM kill | MED | Row retention policies (decay reducer), gradient sampling |
| Migration data loss | Historical data gone | LOW | One-shot migration is additive; SQLite sources preserved as backup |
| Ingester-STDB latency spikes | Stale bootstrap data | LOW | Local STDB (loopback), not remote |
| STDB SDK version conflict with synthex-v2 | Build failure | MED | Separate workspace; `habitat-stdb/` is independent of synthex-v2 |
| Bootstrap payload >15 KB | Context pressure | LOW | Aggressive summarization in injector; top-N limits on all queries |
| POVM→STDB sync drift | Divergent graphs | MED | Periodic full-sync reducer; POVM pathway count monitored |

---

## 8. Success Criteria

The system is complete when:

1. **<100ms bootstrap:** New Claude Code context window receives complete causal state injection in under 100ms
2. **Trajectory visible:** Bootstrap shows fitness delta across last 5 snapshots, not just current value
3. **Causal queries answerable:** "Why did fitness drop?" returns a chain of linked events
4. **Single query surface:** `spacetime sql habitat "..."` replaces 6 separate substrate queries
5. **Pattern reinforcement live:** `reinforce_edge` reducer fires on every RALPH generation cycle
6. **Dead weight removed:** 11 empty tracking databases deleted
7. **Round-trip verified:** Command → Atuin → PV2 bus → ingester → STDB → next bootstrap injection

---

## 9. Session Estimates

| Session | Phase | Work | Hours |
|---------|-------|------|-------|
| S110-S111 | A | STDB deploy + core tables + basic ingester | 6-8 |
| S112-S113 | B | Knowledge graph migration + workstream ledger + decay | 8-10 |
| S114 | C | Watcher integration + causal chains + forget cascade | 6-8 |
| S115-S116 | D | Cross-service bridges + Telegram + Obsidian | 8-10 |
| S117-S118 | E | Injector CLI + bootstrap view + dead DB cleanup | 6-8 |
| **Buffer** | | Overrun + verification | 4-6 |
| **Total** | | | **~40-50h across 8-10 sessions** |

---

## 10. What This Replaces vs What It Preserves

### Replaces

- 11 dead tracking databases (zero data, pure schema weight)
- `habitat-bootstrap` atuin script's L3 layer (SQLite learned_patterns query)
- Manual session note reading for trajectory
- Per-service probing for trap state
- `memory-federate` proposed script from S101 (STDB is the unified query surface)

### Preserves

- Auto-Memory (MEMORY.md + *.md files) — human-curated, always loaded by Claude Code
- Obsidian vault — human-authored docs, canonical reference
- POVM Engine — continues as Hebbian pathway substrate, dual-write during transition
- RM Engine — continues as TSV heartbeat surface
- CLAUDE.md / CLAUDE.local.md — project instructions, session state
- `habitat-bootstrap` atuin script — preserved as fallback; STDB injection is additive

### Evolves

- `habitat-bootstrap` gains L7-L10 layers sourced from STDB instead of ad-hoc scripts
- ORAC SessionStart hook gains STDB-powered injection alongside existing memory summary
- Pattern reinforcement becomes a live, queryable, decaying system instead of write-once-read-never

---

*Plan v1 authored S109 · 2026-04-24 · 5 phases · ~40-50h critical path · Sidecar architecture per S103 ADR-002 · Schema aligned with 21-DB audit + 3554-pathway POVM structure + Comms Layer v3 §10.4 STDB mapping · Execution-ready pending Luke's go-ahead*
