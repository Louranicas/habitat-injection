> Back to: [[HOME]] · [[Current State — Memory Substrates]] · [[What Replaces vs Preserves]]

# Memory Consolidation Map

## Before STDB (6 substrates, 21 DBs, fragmented)

```mermaid
graph TB
    subgraph "Substrate 1: Auto-Memory"
        AM[MEMORY.md<br/>~50 .md files<br/>YAML frontmatter]
    end

    subgraph "Substrate 2: Tracking DBs (21)"
        ST[service_tracking.db<br/>20 tables · 141 patterns]
        HP[hebbian_pulse.db<br/>7 tables · 109 pathways]
        AD[agent_deployment.db<br/>4 tables · 46 agents]
        SS[system_synergy.db<br/>6 tables · 89 pairs]
        PM[performance_metrics.db<br/>3 tables · 95 metrics]
        FS[flow_state.db<br/>3 tables · 12 states]
        SE[security_events.db<br/>3 tables · 9 events]
        CT[compliance_tracking.db<br/>5 tables · 29 records]
        QM[100pct_compliance.db<br/>4 tables · 28 metrics]

        D1[bus_tracking.db DEAD]
        D2[code.db DEAD]
        D3[devenv_tracking.db DEAD]
        D4[episodic_memory.db DEAD]
        D5[evolution_tracking.db DEAD]
        D6[povm_data.db DEAD]
        D7[povm_engine.db DEAD]
        D8[security_tracking.db DEAD]
        D9[synergy_tracking.db DEAD]
        D10[tensor_memory.db DEAD]
        D11[workflow_tracking.db DEAD]
    end

    subgraph "Substrate 3: POVM :8125"
        PO[3554 pathways<br/>Hebbian weights<br/>co-activations]
    end

    subgraph "Substrate 4: RM :8130"
        RM2[~2000 TSV entries<br/>tick/r/gen/fitness/phase]
    end

    subgraph "Substrate 5: VMS :8120"
        VM[1881 memories<br/>12D tensor<br/>morphogenic=0]
    end

    subgraph "Substrate 6: Obsidian"
        OB[215 notes<br/>wikilinks<br/>human-authored]
    end

    style D1 fill:#8b0000,color:#fff
    style D2 fill:#8b0000,color:#fff
    style D3 fill:#8b0000,color:#fff
    style D4 fill:#8b0000,color:#fff
    style D5 fill:#8b0000,color:#fff
    style D6 fill:#8b0000,color:#fff
    style D7 fill:#8b0000,color:#fff
    style D8 fill:#8b0000,color:#fff
    style D9 fill:#8b0000,color:#fff
    style D10 fill:#8b0000,color:#fff
    style D11 fill:#8b0000,color:#fff
```

## After STDB (consolidated + preserved)

```mermaid
graph TB
    subgraph "PRESERVED (unchanged)"
        AM2[Auto-Memory<br/>~50 .md files<br/>always loaded by CC]
        OB2[Obsidian Vault<br/>215 notes<br/>human-authored]
        VM2[VMS :8120<br/>1881 memories<br/>semantic vectors]
        RM3[RM :8130<br/>TSV heartbeat]
        PO2[POVM :8125<br/>dual-write backup<br/>never deleted]
    end

    subgraph "NEW: SpaceTimeDB :3000"
        T1[(T1 HabitatEvent<br/>causal chains)]
        T2[(T2 KnowledgeEdge<br/>3934 edges<br/>unified graph)]
        T3[(T3 GradientSnapshot<br/>time-series<br/>trajectory)]
        T4[(T4 SessionRecord<br/>session lifecycle)]
        T5[(T5 Workstream<br/>in-flight work)]
        T6[(T6 ServiceHealth<br/>health timeline)]
        T7[(T7 TrapState<br/>18 active traps)]
        T8[(T8 WatcherObservation<br/>anomaly records)]
    end

    subgraph "DELETED (11 dead DBs)"
        DEL[11 × empty .db files<br/>zero data<br/>pure schema weight]
    end

    subgraph "ARCHIVED (.db.pre-stdb)"
        ARC[5 × live source DBs<br/>preserved 90 days<br/>backup only]
    end

    PO2 -.->|pathway sync 300s| T2
    RM3 -.->|heartbeat| T3

    style T2 fill:#4a148c,color:#fff
    style DEL fill:#8b0000,color:#fff
    style ARC fill:#555,color:#fff
```

## Net Change

| Metric | Before | After | Delta |
|--------|--------|-------|-------|
| Active databases | 21 SQLite + 4 HTTP | 1 STDB + 4 HTTP | -20 DBs |
| Dead databases | 11 | 0 | -11 |
| Query tools needed | sqlite3, curl, atuin kv, grep, Read | spacetime sql (primary) | -4 tools |
| Bootstrap layers | 7 (L0-L6) | 11 (L0-L10) | +4 layers |
| Bootstrap latency | 55ms | <100ms | +45ms (for 4× more data) |
| Bootstrap data | 9 KB | 15 KB | +6 KB |
| Pattern reinforcement | Write-only (1/141 reinforced) | Live reinforce_edge on RALPH cycle | Fixed |
| Causal queries | Impossible | Recursive chain traversal | New capability |
| Trajectory | Manual session note reading | Last 5 snapshots with delta | New capability |
| Workstream visibility | Parse CLAUDE.local.md | SQL query on T5 | New capability |

---

See: [[Executive Summary]] · [[Migration Strategy]] · [[Bootstrap Chain — Current vs Target]]
