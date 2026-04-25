> Back to: [[HOME]] | [[Architecture Overview]] | [[README.md]](`~/claude-code-workspace/memory-injection/README.md`)
> POVM namespace: `habitat_injection_wiring_*`
> Tracking DB: `~/.local/share/habitat/injection.db` (7 tables, schema v3)

# Complete Wiring Schematic — habitat-injection

> 6 layers | 27 modules | 7 CLI binaries | 3-tier fallback | 4-step Hebbian cycle
> Created: 2026-04-25 (S111 schematic pass)

---

## System Overview

```mermaid
graph TB
    subgraph "Claude Code Session"
        CC[Claude Code Process]
        HOOK[SessionStart Hook Chain]
        SAVE["/save-session Hook"]
    end

    subgraph "habitat-injection Binaries"
        INJECT[habitat-inject<br/>Exit 0 ALWAYS<br/>stdout → Claude]
        CONSOLIDATE[habitat-consolidate<br/>--session N --fired-patterns P1,P2]
        QUERY[habitat-query<br/>trajectory / chains / SQL / fzf]
        INIT[habitat-init<br/>One-time DB setup]
        SEED[habitat-seed<br/>Populate from sources]
        MEMORY[habitat-memory<br/>:8140 health daemon]
    end

    subgraph "Storage Layer"
        DB[(injection.db<br/>7 tables, schema v3<br/>~/.local/share/habitat/)]
        ATUIN[(atuin KV<br/>habitat.last-injection<br/>habitat.last-injection-meta<br/>habitat.last-session)]
    end

    subgraph "Habitat Services (Read-Only Sources)"
        ORAC[ORAC :8133<br/>/health → ralph_fitness, LTP/LTD]
        SYNTHEX[SYNTHEX :8090<br/>/v3/thermal → temperature]
        PV2[Pane-Vortex :8132<br/>/health → field_r, spheres]
        POVM[POVM :8125<br/>/health → pathways]
        PORTS[10 service ports<br/>health probe sweep]
    end

    CC -->|"position 3 in hook chain"| HOOK
    HOOK -->|"3s timeout"| INJECT
    INJECT -->|"<2KB prose, <100ms"| CC

    CC -->|"post-session"| SAVE
    SAVE --> CONSOLIDATE

    CONSOLIDATE -->|"curl"| ORAC
    CONSOLIDATE -->|"curl"| SYNTHEX
    CONSOLIDATE -->|"curl"| PORTS
    CONSOLIDATE -->|"write"| DB
    CONSOLIDATE -->|"write"| ATUIN

    INJECT -->|"Tier 1: cache read"| DB
    INJECT -->|"Tier 2: KV read"| ATUIN

    QUERY -->|"read"| DB
    INIT -->|"create"| DB
    SEED -->|"write"| DB
    MEMORY -->|"health check"| DB

    style INJECT fill:#2d5016,stroke:#4a8c2a,color:#fff
    style CONSOLIDATE fill:#1a3a5c,stroke:#2d6da3,color:#fff
    style DB fill:#5c3a1a,stroke:#a36d2d,color:#fff
    style ATUIN fill:#5c3a1a,stroke:#a36d2d,color:#fff
```

---

## SessionStart Hook Chain (Wiring Position)

```mermaid
sequenceDiagram
    participant CC as Claude Code
    participant H1 as Hook 1: orac-hook.sh
    participant H2 as Hook 2: health-broadcast.sh
    participant H3 as Hook 3: habitat-inject
    participant DB as injection.db
    participant AK as atuin KV
    participant ST as Static Fallback

    CC->>H1: SessionStart (5s timeout)
    H1-->>CC: ORAC sphere registration + POVM hydration

    CC->>H2: SessionStart (4s timeout)
    H2-->>CC: 12x parallel health probes → atuin KV

    CC->>H3: SessionStart (3s timeout)

    H3->>DB: SELECT payload FROM injection_cache<br/>WHERE section='full_payload'<br/>AND age ≤ 60s
    alt Tier 1: Cache Hit
        DB-->>H3: cached payload
    else Tier 2: Cache Miss
        H3->>AK: atuin kv get habitat.last-injection<br/>(500ms subprocess timeout)
        alt KV Hit
            AK-->>H3: last session's payload
        else Tier 3: All Miss
            H3->>ST: static_fallback()
            ST-->>H3: "NO INJECTION STATE"
        end
    end

    H3-->>CC: stdout: <2KB prose payload
    Note over CC: Payload injected into<br/>system context (~1100 tokens)
```

---

## Post-Session Consolidation Pipeline

```mermaid
sequenceDiagram
    participant USER as User / /save-session
    participant HC as habitat-consolidate
    participant ORAC as ORAC :8133
    participant SYN as SYNTHEX :8090
    participant DB as injection.db
    participant HEB as Hebbian Engine
    participant CACHE as Cache Builder
    participant AK as atuin KV

    USER->>HC: --session 110 --fired-patterns cp-alias,stash-pop

    par Health Snapshot
        HC->>ORAC: GET /health
        ORAC-->>HC: ralph_fitness, LTP, LTD
        HC->>SYN: GET /v3/thermal
        SYN-->>HC: temperature
        HC->>HC: Probe 10 service ports
    end

    HC->>DB: capture_trajectory(session=110, snapshot)
    Note over DB: INSERT session_trajectory<br/>compute_delta vs S109

    HC->>HEB: run_consolidation(session=110, [cp-alias, stash-pop])
    Note over HEB: 1. decay_all: weight *= 0.95<br/>2. reinforce: weight += 0.1*(1-w)<br/>3. prune: DELETE weight < 0.05<br/>4. auto_resolve: chains idle ≥10 sessions

    HEB->>DB: UPDATE/DELETE reinforced_pattern
    HEB->>DB: UPDATE causal_chain (resolve stale)

    HC->>CACHE: rebuild_cache(session=110, healthy=11, total=12, T=0.55)
    CACHE->>DB: Query 4 tables (fresh data)
    CACHE->>CACHE: Consent filter (keep only Emit)
    CACHE->>CACHE: Render prose (<2KB, <1100 tokens)
    CACHE->>DB: INSERT OR REPLACE injection_cache

    HC->>AK: write_injection_cache(payload, meta)
    HC->>AK: write_kv("habitat.last-session", "110")
```

---

## Database Schema Wiring (7 Tables)

```mermaid
erDiagram
    causal_chain {
        int id PK "AUTOINCREMENT"
        int origin_session "NOT NULL"
        int resolved_session "NULL = unresolved"
        text chain_type "bug|trap|plan|pattern"
        text label "NOT NULL, indexed"
        text description "NOT NULL"
        int reinforcement_count "DEFAULT 1"
        int last_reinforced_session "NULL"
        text consent "Emit|Store|Forget"
        text created_at "ISO-8601"
        text updated_at "ISO-8601"
    }

    session_trajectory {
        int session_id PK
        real ralph_fitness "NOT NULL"
        real field_r "NOT NULL"
        real thermal_t "NOT NULL"
        real ltp_ltd_ratio "NOT NULL"
        int services_healthy "NOT NULL"
        text delta_summary "NOT NULL"
        text key_achievement "NULL"
        text consent "Emit|Store|Forget"
    }

    workstream {
        text ws_id PK "e.g. comms-v3"
        text title "NOT NULL"
        text status "active|blocked|deferred|complete"
        text blocker "NULL"
        int priority "DEFAULT 5, lower=urgent"
        int last_touched_session "NOT NULL"
        int items_total "NULL"
        int items_done "NULL"
        text resume_context "NOT NULL"
        text consent "Emit|Store|Forget"
    }

    reinforced_pattern {
        text pattern_id PK
        text category "procedural|semantic|trap|feedback"
        text description "NOT NULL"
        text anti_pattern "NULL"
        real weight "0.0-1.0, DEFAULT 0.5"
        int hit_count "DEFAULT 1"
        int last_fired_session "NULL"
        text consent "Emit|Store|Forget"
    }

    injection_cache {
        text section PK "full_payload"
        text payload "NOT NULL"
        int token_count "NOT NULL"
        int computed_at "unix epoch"
        int consent_applied "DEFAULT 1"
    }

    session_checkpoint {
        int id PK "AUTOINCREMENT"
        text label "UNIQUE e.g. s110-close"
        int session_number "NULL"
        text timestamp_utc "ISO-8601"
        text pane_id "NULL"
        text tab "NULL"
        text persona "NULL"
        text git_sha "NULL"
        text git_branch "NULL"
        int services_alive "NOT NULL"
        text accomplished_json "NOT NULL"
        text in_progress_json "NOT NULL"
        text blocked_json "NOT NULL"
        text resume_instructions "NOT NULL"
        text consent "Emit|Store|Forget"
    }

    injection_script {
        text id PK "UUIDv7"
        text name "UNIQUE"
        text description "NOT NULL"
        text tags "comma-separated"
        text shebang "DEFAULT bash"
        text script_body "NOT NULL"
        text template_vars_json "DEFAULT {}"
        int run_count "DEFAULT 0"
        text consent "Emit|Store|Forget"
    }

    injection_cache ||--o{ causal_chain : "pre-renders top 5"
    injection_cache ||--o{ session_trajectory : "pre-renders last 5"
    injection_cache ||--o{ workstream : "pre-renders active+blocked"
    injection_cache ||--o{ reinforced_pattern : "pre-renders top 10"
    session_checkpoint ||--o{ causal_chain : "auto-discovers BUG-NNN + traps"
```

---

## Layer Dependency Wiring

```mermaid
graph BT
    subgraph "L1 Foundation"
        M01[m01_types<br/>10 newtypes]
        M02[m02_errors<br/>6 error enums]
        M03[m03_config<br/>TOML + env]
        M04[m04_traits<br/>4 core traits]
        M05[m05_constants<br/>27 constants]
    end

    subgraph "L2 Schema & Persistence"
        M06[m06_schema<br/>7 tables, v3]
        M07[m07_causal_chain<br/>CRUD + reinforce]
        M08[m08_trajectory<br/>CRUD + OLS trend]
        M09[m09_workstream<br/>CRUD + status FSM]
        M10[m10_pattern<br/>CRUD + decay/prune]
        M10B[m10b_checkpoint<br/>CRUD + JSON arrays]
    end

    subgraph "L3 Injection Engine"
        M11[m11_parallel_query<br/>4-query executor]
        M12[m12_prose_renderer<br/>5-section ≤2KB]
        M13[m13_fallback<br/>3-tier chain]
        M14[m14_consent_filter<br/>Emit/Store/Forget]
    end

    subgraph "L4 Consolidation Engine"
        M15[m15_checkpoint_ingest<br/>BUG-NNN regex]
        M15B[m15b_trajectory_capture<br/>health snapshot]
        M16[m16_hebbian_engine<br/>4-step cycle]
        M17[m17_cache_builder<br/>render + write]
        M18[m18_atuin_cache<br/>3-key KV]
    end

    subgraph "L5 Query & Browser"
        M19[m19_preset_queries<br/>5 presets]
        M20[m20_raw_query<br/>read-only SQL]
        M21[m21_fzf_browser<br/>fzf + fallback]
        M21B[m21b_scripts_engine<br/>CRUD + exec]
    end

    subgraph "L6 SpaceTimeDB (Phase 2)"
        M22[m22_stdb_module<br/>8 tables]
        M23[m23_ingester<br/>5 sources]
        M24[m24_migration<br/>SQLite → STDB]
    end

    M06 --> M01
    M06 --> M02
    M07 --> M06
    M08 --> M06
    M09 --> M06
    M10 --> M06
    M10B --> M06

    M11 --> M07
    M11 --> M08
    M11 --> M09
    M11 --> M10
    M12 --> M01
    M13 --> M11
    M14 --> M01

    M15 --> M07
    M15 --> M10B
    M15B --> M08
    M16 --> M10
    M16 --> M07
    M17 --> M11
    M17 --> M12
    M17 --> M14
    M18 --> M03

    M19 --> M07
    M19 --> M08
    M19 --> M09
    M19 --> M10
    M20 --> M06
    M21 --> M19
    M21B --> M06

    M22 --> M01
    M23 --> M03
    M24 --> M22
    M24 --> M06
```

---

## CLI Binary Wiring Map

```mermaid
graph LR
    subgraph "One-Time Setup"
        INIT[habitat-init<br/>Creates DB + 7 tables]
        SEED[habitat-seed<br/>Populates from sources]
    end

    subgraph "Session Lifecycle"
        INJECT[habitat-inject<br/>SessionStart hook<br/>Exit 0 ALWAYS]
        CONSOLIDATE[habitat-consolidate<br/>Post-session write-back]
    end

    subgraph "Interactive"
        QUERY[habitat-query<br/>trajectory / chains<br/>SQL / fzf / scripts]
        MEMORY[habitat-memory<br/>:8140 /health daemon]
    end

    subgraph "Library Modules Called"
        L1[L1: Config, Types]
        L2[L2: Schema, CRUD]
        L3[L3: Fallback, Renderer]
        L4[L4: Hebbian, Capture, Cache]
        L5[L5: Presets, Raw SQL, fzf]
    end

    INIT --> L1
    INIT --> L2

    SEED --> L1
    SEED --> L2

    INJECT --> L1
    INJECT --> L3

    CONSOLIDATE --> L1
    CONSOLIDATE --> L2
    CONSOLIDATE --> L4

    QUERY --> L1
    QUERY --> L2
    QUERY --> L5

    MEMORY --> L1
    MEMORY --> L2
```

| Binary | Args | Calls | Exit Codes |
|--------|------|-------|------------|
| `habitat-init` | `[db_path]` | `Config::load`, `open_database`, `list_tables`, `schema_version` | 0=ok, 1=error |
| `habitat-inject` | (none) | `Config::load`, `execute_fallback_chain` | 0 ALWAYS |
| `habitat-seed` | `all\|chains\|trajectory\|workstreams\|patterns` | `Config::load`, `open_database`, `insert_*`, `find_by_label`, `reinforce_chain` | 0=ok, 1=error |
| `habitat-consolidate` | `--session N [--fired-patterns P1,P2]` | `Config::load`, `open_database`, `capture_trajectory`, `run_consolidation`, `rebuild_cache`, `write_injection_cache` | 0=ok, 1=error |
| `habitat-query` | `trajectory\|chains\|workstreams\|patterns\|summary\|--interactive\|"SELECT ..."` | `Config::load`, `open_database`, `query_preset`, `execute_raw_formatted`, `browse_table` | 0=ok, 1=error |
| `habitat-memory` | (none, env `HABITAT_MEMORY_PORT`) | `Config::load`, `open_database`, TCP listener, `health_response` | 0=ok, 1=error |

---

## External Service Wiring (Read-Only)

| Service | Port | Endpoint | Data Consumed | Consumer |
|---------|------|----------|---------------|----------|
| ORAC | 8133 | `/health` | `ralph_fitness`, `hebbian_ltp_total`, `hebbian_ltd_total` | `habitat-consolidate` |
| SYNTHEX | 8090 | `/v3/thermal` | `temperature` | `habitat-consolidate` |
| SYNTHEX | 8090 | `/api/health` | health probe | `habitat-consolidate` |
| PV2 | 8132 | `/health` | `field_r`, `spheres` | `habitat-consolidate` (Phase 2) |
| POVM | 8125 | `/health` | `pathways` count | `habitat-consolidate` (Phase 2) |
| 10 ports | various | `/health` | HTTP 200 count | `habitat-consolidate` → `count_healthy_services()` |

**Service ports probed by consolidate:**
`8082, 8083, 8111, 8120, 8125, 8130, 8132, 8133, 8180, 10002` + SYNTHEX `8090/api/health`

---

## Atuin KV Namespace Wiring

| Key | Written By | Read By | Format | Purpose |
|-----|-----------|---------|--------|---------|
| `habitat.last-injection` | `m18_atuin_cache` via consolidate | `m13_fallback` via inject (Tier 2) | Plain text (<2KB) | Fallback injection payload |
| `habitat.last-injection-meta` | `m18_atuin_cache` via consolidate | `m18_atuin_cache::read_injection_cache` | JSON `{payload, token_count, session_number, timestamp_utc}` | Structured cache metadata |
| `habitat.last-session` | `habitat-consolidate` | Scripts, diagnostics | Decimal string (e.g. "110") | Last consolidated session |

---

## Configuration Wiring

```mermaid
graph TB
    subgraph "Config Resolution Order"
        ENV[Environment Variables<br/>HABITAT_DB_PATH<br/>HABITAT_TOKEN_BUDGET<br/>HABITAT_DECAY_RATE<br/>HABITAT_REINFORCE_RATE<br/>HABITAT_STDB_PORT]
        FILE[Config File<br/>config/default.toml<br/>config/production.toml]
        DEFAULT[Compiled Defaults<br/>m05_constants.rs]
    end

    ENV -->|"highest priority"| CONFIG[Config struct]
    FILE -->|"middle priority"| CONFIG
    DEFAULT -->|"lowest priority"| CONFIG

    CONFIG --> DB_PATH["database.path<br/>~/.local/share/habitat/injection.db"]
    CONFIG --> INJ["injection.*<br/>token_budget=1100<br/>max_payload_bytes=15360<br/>max_latency_ms=100"]
    CONFIG --> CONS["consolidation.*<br/>decay_rate=0.95<br/>reinforce_rate=0.1<br/>prune_threshold=0.05<br/>auto_resolve_sessions=10"]
    CONFIG --> RET["retention.*<br/>envelope_days=30<br/>delete_days=90"]
    CONFIG --> SVC["services.*<br/>orac_poll_secs=30<br/>synthex_poll_secs=60<br/>povm_sync_secs=300"]
```

---

## Consent Wiring (Security Gate)

```mermaid
flowchart LR
    subgraph "Write Path (L4)"
        W1[checkpoint_ingest] -->|"consent=Emit"| DB[(injection.db)]
        W2[trajectory_capture] -->|"consent=Emit"| DB
        W3[hebbian_engine] -->|"preserves existing"| DB
    end

    subgraph "Read Path (L3)"
        DB -->|"SELECT *"| RAW[Raw Rows]
        RAW -->|"filter_by_consent()"| FILTER{ConsentBearing<br/>trait check}
        FILTER -->|"Emit ✓"| PASS[Passed Rows]
        FILTER -->|"Store ✗"| DROP1[Dropped + Logged]
        FILTER -->|"Forget ✗"| DROP2[Dropped + Logged]
        PASS --> RENDER[prose_renderer<br/>≤2KB payload]
    end

    subgraph "FilterStats"
        STATS["passed: N<br/>dropped_store: N<br/>dropped_forget: N<br/>total: N"]
    end

    FILTER -.-> STATS
```

Every row in `causal_chain`, `session_trajectory`, `workstream`, `reinforced_pattern`, and `session_checkpoint` carries a `consent` column. Only `"Emit"` rows enter the injection payload. `"Store"` rows persist but stay private. `"Forget"` rows are marked for deletion.

---

## Cross-References

- **POVM pathways:** `habitat_injection_wiring_*` namespace — schematic root + per-diagram anchors
- **Tracking DB:** `~/.local/share/habitat/injection.db` — 7 tables documented above
- **README:** [`README.md`](~/claude-code-workspace/memory-injection/README.md) — architecture summary, module table, deliberation origin
- **Execution Plan:** [[Execution Plan]] — 11-step deployment (S110-S114)
- **Vault layers:** [[L1 Foundation]] | [[L2 Schema & Persistence]] | [[L3 Injection Engine]] | [[L4 Consolidation Engine]] | [[L5 Query & Browser]] | [[L6 SpaceTimeDB Migration]]
- **Architecture:** [[Architecture Overview]] | [[Data Flow]] | [[Dependency Graph]]
- **Operations:** [[Hook Registration]] | [[Binary Map]] | [[Injection Database State]]
