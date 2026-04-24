> Back to: [[HOME]] · [[Migration Strategy]] · [[Phase B — Knowledge Graph Migration]]

# Migration Flow

## Source → STDB Table Mapping

```mermaid
flowchart LR
    subgraph "SQLite Sources (10 live DBs)"
        ST[service_tracking.db<br/>141 patterns + 29 graph<br/>+ 27 events + 12 learnings]
        HP[hebbian_pulse.db<br/>109 neural + 109 hebbian<br/>+ 676 decay + 36 pulse]
        FS[flow_state.db<br/>12 flow_states]
        AD[agent_deployment.db<br/>46 agents]
        SS[system_synergy.db<br/>89 synergy]
        WF[V3 workflow_state.db<br/>4 workflows]
    end

    subgraph "HTTP Sources"
        POVM[POVM :8125<br/>3554 pathways]
        RM[RM :8130<br/>~2000 TSV entries]
    end

    subgraph "synthex-v2 DBs"
        GS[gradient_snapshot.db<br/>1 snapshot]
        BH[bridge_health.db<br/>9 bridges]
        WO[watcher_observation.db<br/>0 scaffold]
        WT[workflow_tracking.db<br/>2 workflows]
    end

    subgraph "STDB Tables"
        T1[(T1 HabitatEvent)]
        T2[(T2 KnowledgeEdge)]
        T3[(T3 GradientSnapshot)]
        T5[(T5 Workstream)]
        T6[(T6 ServiceHealth)]
        T8[(T8 WatcherObservation)]
    end

    ST -->|learned_patterns| T2
    ST -->|orchestration_graph| T2
    ST -->|service_events| T1
    ST -->|cross_agent_learnings| T2
    HP -->|neural_pathways| T2
    HP -->|hebbian_pathways| T2
    HP -->|decay_audit_log| T1
    HP -->|pulse_events| T1
    FS -->|flow_states| T3
    AD -->|agents| T6
    SS -->|system_synergy| T2
    WF -->|workflows| T5
    POVM -->|pathways| T2
    RM -->|heartbeat| T3
    GS --> T3
    BH --> T6
    WO --> T8
    WT --> T5

    style T2 fill:#4a148c,color:#fff
    style POVM fill:#4a148c,color:#fff
```

## POVM Dual-Write Transition

```mermaid
stateDiagram-v2
    [*] --> PhaseAC: Phase A-C
    PhaseAC --> PhaseD: Phase D
    PhaseD --> PhaseE: Phase E+

    state PhaseAC {
        ORAC_W1: ORAC writes → POVM (existing)
        ING_R1: Ingester polls POVM → syncs to STDB T2
        note right of ORAC_W1: POVM is source-of-truth
    }

    state PhaseD {
        ORAC_W2: ORAC writes → POVM + STDB (dual-write)
        ING_R2: Ingester still syncs POVM → STDB T2
        note right of ORAC_W2: Verification period
    }

    state PhaseE {
        ORAC_W3: ORAC writes → STDB (primary)
        SNAP: Periodic STDB → POVM snapshot (backup)
        note right of ORAC_W3: STDB is source-of-truth
        note right of SNAP: POVM never deleted
    }
```

## Verification Checksum Process

```mermaid
flowchart TD
    PRE[Pre-migration:<br/>COUNT, SUM weight, AVG weight<br/>from source table] --> MIG[Run migration script]
    MIG --> POST[Post-migration:<br/>COUNT, SUM weight, AVG weight<br/>from STDB table WHERE edge_type = source]
    POST --> CMP{Counts match?<br/>Weights within ±0.01?}
    CMP -->|Yes| PASS[Migration verified ✓]
    CMP -->|No| ABORT[Abort remaining<br/>Preserve source DB<br/>File BUG]
    
    style PASS fill:#2d5016,color:#fff
    style ABORT fill:#8b0000,color:#fff
```

## Dead DB Cleanup (Phase E)

```mermaid
flowchart LR
    subgraph "DELETE (11 dead DBs — zero data)"
        D1[bus_tracking.db]
        D2[code.db]
        D3[devenv_tracking.db]
        D4[episodic_memory.db]
        D5[evolution_tracking.db]
        D6[povm_data.db]
        D7[povm_engine.db]
        D8[security_tracking.db]
        D9[synergy_tracking.db]
        D10[tensor_memory.db]
        D11[workflow_tracking.db]
    end

    subgraph "PRESERVE (live sources as backup)"
        P1[service_tracking.db]
        P2[hebbian_pulse.db]
        P3[flow_state.db]
        P4[agent_deployment.db]
        P5[system_synergy.db]
    end

    D1 & D2 & D3 & D4 & D5 & D6 & D7 & D8 & D9 & D10 & D11 --> TRASH[trash<br/>via rm alias]
    P1 & P2 & P3 & P4 & P5 --> BACKUP[Renamed to *.db.pre-stdb<br/>preserved 90 days]
```

---

See: [[Migration Strategy]] · [[Phase B — Knowledge Graph Migration]] · [[Phase E — Bootstrap Revolution]]
