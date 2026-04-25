> Back to: [[HOME]] | [[Complete Wiring Schematic]] | [[L6 SpaceTimeDB Migration]] | [[SpaceTimeDB Plan]] | [[README.md]](`~/claude-code-workspace/memory-injection/README.md`)
> POVM namespace: `habitat_injection_stdb_*`

# SpaceTimeDB Phase 2 Wiring — habitat-injection

> 8 STDB table mirrors, 5 ingester sources, 11 reducers, 16-source migration planner.
> Feature-gated behind `stdb` + `ingester`. NOT shipped in Phase 1.
> Created: 2026-04-25 (S111 schematic pass)

---

## Phase 2 Architecture

```mermaid
graph TB
    subgraph "Phase 1 (Current)"
        CLI[CLI Binaries] --> SQLITE[(SQLite<br/>injection.db)]
    end

    subgraph "Phase 2 (Planned)"
        INGESTER[Ingester Daemon<br/>:3001 /health] -->|"poll/subscribe"| ORAC[ORAC :8133]
        INGESTER -->|"poll"| SYNTHEX[SYNTHEX :8090]
        INGESTER -->|"subscribe"| PV2[PV2 :8132]
        INGESTER -->|"sync"| POVM[POVM :8125]
        INGESTER -->|"hook"| ATUIN[Atuin Hooks]

        INGESTER -->|"call reducers"| STDB[(SpaceTimeDB<br/>:3000<br/>8 tables)]

        MIG[Migration Planner<br/>m24] -->|"one-shot transfer"| SQLITE
        MIG -->|"verify checksums"| STDB
    end

    SQLITE -.->|"dual-write transition"| STDB

    style SQLITE fill:#5c3a1a,stroke:#a36d2d,color:#fff
    style STDB fill:#1a3a5c,stroke:#2d6da3,color:#fff
```

---

## 8 STDB Table Mirrors

```mermaid
erDiagram
    HabitatEvent {
        u64 id PK
        string event_type
        string source_service
        u8 severity
        f64 confidence
        string payload_json
        u64 tick
        string timestamp
    }

    KnowledgeEdge {
        u64 id PK
        string source_id
        string target_id
        string edge_type "Learned|Hebbian|Orchestration|Povm|Synergy|CrossAgent"
        string namespace
        f64 weight
        u32 reinforcement_count
        f64 stdp_delta
        bool is_bidirectional
        bool ltm_eligible
    }

    GradientSnapshot {
        u64 id PK
        string source
        f64 temperature
        f64 ralph_fitness
        string ralph_phase
        u64 ltp_total
        u64 ltd_total
        f64 ltp_ltd_ratio
        f64 flow_state
        bool is_healthy
        string system_grade
        string timestamp
    }

    SessionRecord {
        string session_id PK
        u32 session_number
        string started_at
        f64 fitness_start
        u32 events_count
        u32 commits_count
        string status
    }

    StdbWorkstream {
        string id PK
        string service_name
        string goal
        u8 current_tier
        string status
        f64 confidence
    }

    ServiceHealth {
        u64 id PK
        string service_id
        u16 port
        string health_status
        u16 http_code
        string circuit_state
        f64 response_time_ms
    }

    TrapState {
        string trap_name PK
        bool is_active
        u32 trigger_count
        string description
    }

    WatcherObservation {
        string observation_id PK
        string observer_role
        string anomaly_class
        u8 severity
        string metric_json
        string model
        u32 cost_cents
    }

    HabitatEvent ||--o{ KnowledgeEdge : "caused_by_event"
    HabitatEvent ||--o{ WatcherObservation : "caused_by_event"
    SessionRecord ||--o{ HabitatEvent : "session_id"
    SessionRecord ||--o{ GradientSnapshot : "session_id"
```

---

## 11 Reducer Signatures

| Reducer | Signature | Purpose |
|---------|-----------|---------|
| `IngestEvent` | `fn(HabitatEvent) -> Result<(), String>` | Ingest events from any source |
| `ReinforceEdge` | `fn(source_id, target_id, edge_type, namespace) -> Result<(), String>` | Hebbian edge reinforcement |
| `CaptureGradient` | `fn() -> Result<(), String>` | Snapshot all service metrics |
| `RegisterSession` | `fn(session_id, session_number, model) -> Result<(), String>` | Start a new session record |
| `CloseSession` | `fn(session_id) -> Result<(), String>` | End session, compute fitness delta |
| `RunDecay` | `fn() -> Result<(), String>` | Hebbian decay on all edges |
| `ForgetSphere` | `fn(sphere_id) -> Result<(), String>` | Consent-driven sphere removal |
| `CompactOldEvents` | `fn() -> Result<(), String>` | Retention: archive old events |
| `ConsolidateMatureEdges` | `fn() -> Result<(), String>` | LTM promotion for mature edges |
| `WatcherReinforce` | `fn(event_id) -> Result<(), String>` | Watcher-driven edge reinforcement |
| `WatcherAnnotateEvent` | `fn(event_id, anomaly_class, severity, classifier_output) -> Result<(), String>` | Watcher observation annotation |

---

## 5 Ingester Sources

```mermaid
graph LR
    subgraph "Ingester Sources"
        ORAC_SRC["ORAC<br/>:8133<br/>30s poll"]
        PV2_SRC["PaneVortex<br/>:8132<br/>event-driven"]
        SYN_SRC["SYNTHEX<br/>:8090<br/>60s poll"]
        POVM_SRC["POVM<br/>:8125<br/>300s sync"]
        ATUIN_SRC["Atuin Hooks<br/>local<br/>event-driven"]
    end

    subgraph "Event Types"
        ORAC_SRC -->|"GradientSnapshot"| STDB[(SpaceTimeDB)]
        PV2_SRC -->|"HabitatEvent (sphere)"| STDB
        SYN_SRC -->|"GradientSnapshot"| STDB
        POVM_SRC -->|"KnowledgeEdge sync"| STDB
        ATUIN_SRC -->|"HabitatEvent (command)"| STDB
    end
```

### IngesterConfig

| Field | Default | Source |
|-------|---------|--------|
| `stdb_url` | `http://localhost:3000` | `services.stdb_port` |
| `health_port` | 3001 | `services.ingester_health_port` |
| `orac_url` | `http://localhost:8133` | hardcoded |
| `orac_poll_secs` | 30 | `services.orac_poll_secs` |
| `pv2_url` | `http://localhost:8132` | hardcoded |
| `synthex_url` | `http://localhost:8090` | hardcoded |
| `synthex_poll_secs` | 60 | `services.synthex_poll_secs` |
| `povm_url` | `http://localhost:8125` | hardcoded |
| `povm_sync_secs` | 300 | `services.povm_sync_secs` |

### SourceStatus (per-source health tracking)

```rust
struct SourceStatus {
    source: IngesterSource,
    healthy: bool,
    last_poll: Option<String>,
    events_ingested: u64,
    errors: u64,
    last_error: Option<String>,
}
```

---

## Migration Pipeline (m24)

```mermaid
flowchart TD
    START["Migration Planner"] --> PLAN["16-source migration plan"]
    PLAN --> PHASE_A["Phase A: Read SQLite source"]
    PHASE_A --> PHASE_B["Phase B: Transform to STDB types"]
    PHASE_B --> PHASE_C["Phase C: Call STDB reducers"]
    PHASE_C --> VERIFY["Verify: row count + checksum match"]
    VERIFY -->|"match"| DONE["Phase D: Mark SQLite read-only"]
    VERIFY -->|"mismatch"| ROLLBACK["Phase E: Rollback<br/>MigrationError::RowCountMismatch<br/>MigrationError::ChecksumMismatch"]

    style ROLLBACK fill:#5c1a1a,stroke:#a32d2d,color:#fff
```

### Migration Error Types

| Error | Fields | Meaning |
|-------|--------|---------|
| `ConnectionFailed` | endpoint, reason | Can't reach STDB or SQLite |
| `SourceReadFailed` | origin, reason | Can't read source data |
| `RowCountMismatch` | table, source_count, target_count | Data loss during transfer |
| `ChecksumMismatch` | table, source_sum, target_sum | Data corruption |
| `DualWriteTransitionFailed` | reason | Couldn't switch to dual-write mode |
| `ReducerFailed` | reducer, reason | STDB reducer call failed |

---

## Feature Gate Activation Path

```
Phase 1 (default):  sqlite + cli
                     ↓
Phase 2a:           + stdb         (STDB table types available)
                     ↓
Phase 2b:           + ingester     (full daemon with 5 sources)
                     ↓
Phase 2c:           + watcher-digest (Watcher-curated tables)
                     ↓
Phase 3a:           + inhibition    (inhibitory learning edges)
Phase 3b:           + substrate-reciprocal (autonomy scoring HTTP)
                     ↓
Full:               all features enabled
```

**Kill criteria (from plan.toml):** 20 sessions without measurable improvement in injection quality → revert to SQLite-only, remove STDB dependency.

---

## Enum Wiring (m22 enums)

| Enum | Variants | Used By |
|------|----------|---------|
| `EdgeType` | Learned, Hebbian, Orchestration, Povm, Synergy, CrossAgent | `KnowledgeEdge.edge_type` |
| `EventCategory` | Emergence, Sphere, Thermal, Command, Watcher, Session, Service, Hebbian, Other | Event classification |
| `ConsentState` | Emit, Store, Forget | Maps from L1 `ConsentLevel` |

---

## Cross-References

- **Phase 2 Vault:** `memory-injection-vault/HOME.md` — 46 notes, 24 Mermaid diagrams
- **Complete Wiring:** [[Complete Wiring Schematic]]
- **L6 Layer:** [[L6 SpaceTimeDB Migration]]
- **m22 source:** `src/m6_stdb/m22_stdb_module/` (tables.rs, enums.rs, reducers.rs, validation.rs)
- **m23 source:** `src/m6_stdb/m23_ingester.rs`
- **m24 source:** `src/m6_stdb/m24_migration.rs`
- **README:** [`README.md`](~/claude-code-workspace/memory-injection/README.md) — Phase 2 context
- **POVM:** `habitat_injection_stdb_*` namespace
- **Deliberation:** [[DELIBERATION_PLAN]] — Principle 6: "earn your database"
