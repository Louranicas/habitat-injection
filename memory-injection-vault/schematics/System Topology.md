> Back to: [[HOME]] · [[MASTER INDEX]] · [[DEPLOYMENT FRAMEWORK]]

# System Topology — Complete Service Wiring

## Full Habitat + STDB Topology

```mermaid
graph TB
    subgraph "HUMAN LAYER"
        CLAUDE[Claude Code<br/>SessionStart hook chain<br/>3 hooks → system message]
        OBS[Obsidian Plugin<br/>Session Timeline view]
        TEL[Telegram Bot<br/>/query → STDB SQL]
        ATUIN[Atuin<br/>82+ scripts · KV store<br/>shell history oracle]
    end

    subgraph "STDB LAYER (NEW)"
        STDB[(SpaceTimeDB :3000<br/>8 tables · 10 reducers<br/>WAL persistence · in-memory)]
        ING[Ingester :3001<br/>5-source bridge<br/>reciprocal paths]
        INJ[Injector CLI<br/>7× SQL → format<br/>≤15KB · <100ms]
    end

    subgraph "ENGINE LAYER"
        ORC[ORAC :8133<br/>26 endpoints · RALPH<br/>8 emergence detectors]
        PV2[PV2 :8132<br/>38 routes · Kuramoto<br/>IPC bus · /bus/ws]
        HNC[Nerve :8083<br/>11-service aggregator]
    end

    subgraph "ORCHESTRATION LAYER"
        SX1[SyntheX v1 :8090<br/>REST + WS :8091<br/>thermal PID]
        SX2[SyntheX v2 :8091<br/>shadow · 60 modules<br/>Watcher · HMX]
        ME[ME V2 :8180<br/>12D tensor · PBFT]
    end

    subgraph "PERSISTENCE LAYER"
        POVM[POVM :8125<br/>3554 pathways<br/>dual-write w/ STDB]
        RM[RM :8130<br/>TSV heartbeat]
        VMS[VMS :8120<br/>1881 memories<br/>12D tensor]
    end

    subgraph "AUXILIARY"
        DEV[DevOps V3 :8082]
        CS[CodeSynthor V8 :8111]
        PSW[Pswarm V2 :10002]
        TL[Tool Library :8105]
    end

    %% Ingester sources
    ING -->|poll 30s| ORC
    ING -->|WS /bus/ws| PV2
    ING -->|poll 60s| SX1
    ING -->|poll 300s| POVM
    ING -->|via PV2 bus| ATUIN

    %% Ingester → STDB
    ING -->|reducers R1-R4| STDB

    %% NA-R3 Reciprocal paths
    ING -.->|trajectory hints| ORC
    ING -.->|thermal patterns| SX1
    ING -.->|coupling history| PV2
    ING -.->|stdb.* KV keys| ATUIN

    %% Injector
    INJ -->|7× spacetime sql| STDB
    INJ -->|stdout ≤15KB| CLAUDE

    %% ORAC hooks
    ORC -->|R4 register_session| STDB
    CLAUDE -->|SessionStart hook| ORC

    %% Consumer surfaces
    OBS -.->|HTTP query| ING
    TEL -.->|/query → SQL| ING

    %% Existing engine wiring
    ORC -->|HTTP 5s + IPC| PV2
    ORC -->|HTTP 6-tick| SX1
    ORC -->|HTTP 60-tick| POVM
    ORC -->|TSV 60-tick| RM
    ORC -->|HTTP 30-tick| VMS
    ORC -->|HTTP 12-tick| ME
    PV2 -->|HTTP 6-tick| SX1
    HNC -->|probe 30s| ORC & PV2 & SX1 & ME
```

## Port Map (Post-STDB)

| Port | Service | Batch | Role |
|------|---------|-------|------|
| `:3000` | **SpaceTimeDB** | **1** | **Causal memory substrate** |
| `:3001` | **STDB Ingester** | **2** | **Multi-source bridge** |
| `:8082` | DevOps V3 | 1 | Neural orchestration |
| `:8083` | Nerve Center | 4 | 11-service aggregator |
| `:8090` | SyntheX v1 | 2 | REST + thermal PID |
| `:8091` | SyntheX v2 shadow | 2 | 60 modules, Watcher |
| `:8105` | Tool Library | 3 | 65 tools |
| `:8110` | CodeSynthor V8 | 1 | Pattern library |
| `:8120` | VMS | 4 | Semantic memory |
| `:8125` | POVM | 1 | Hebbian pathways |
| `:8130` | RM | 3 | TSV persistence |
| `:8132` | Pane-Vortex | 4 | Kuramoto field |
| `:8133` | ORAC | 4 | RALPH + emergence |
| `:8180` | ME V2 | 2 | 12D fitness tensor |
| `:10002` | Pswarm V2 | 2 | 40 agents |

---

See: [[Sidecar Architecture]] · [[Ingester Pipeline]] · [[Batch Ordering]]
