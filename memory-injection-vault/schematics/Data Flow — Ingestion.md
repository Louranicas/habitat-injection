> Back to: [[HOME]] · [[Ingester Pipeline]] · [[System Topology]]

# Data Flow — Continuous Ingestion

## Ingester Source → Reducer → Table Mapping

```mermaid
flowchart LR
    subgraph Sources
        O[ORAC :8133<br/>poll 30s]
        P[PV2 :8132<br/>WS /bus/ws]
        S[SYNTHEX :8090<br/>poll 60s]
        PO[POVM :8125<br/>poll 300s]
        A[Atuin hooks<br/>via PV2 bus]
    end

    subgraph "Ingester (Rust binary :3001)"
        OB[orac_bridge]
        PB[pv2_bridge]
        SB[synthex_bridge]
        POB[povm_sync]
        AB[atuin_bridge]
        REC[reciprocal_module]
    end

    subgraph "STDB Reducers"
        R1[R1 ingest_event]
        R2[R2 reinforce_edge]
        R3[R3 capture_gradient]
        R4[R4 register_session]
    end

    subgraph "STDB Tables"
        T1[(T1 HabitatEvent)]
        T2[(T2 KnowledgeEdge)]
        T3[(T3 GradientSnapshot)]
        T4[(T4 SessionRecord)]
        T6[(T6 ServiceHealth)]
    end

    O --> OB
    P --> PB
    S --> SB
    PO --> POB
    A --> AB

    OB -->|emergence events| R1
    OB -->|health + thermal| R3
    OB -->|bridge status| R1
    OB -->|session hooks| R4

    PB -->|sphere.* events| R1
    PB -->|field.tick| R3
    PB -->|command.* events| R1

    SB -->|thermal snapshot| R3

    POB -->|pathway weights| R2

    AB -->|command.preexec| R1
    AB -->|command.postexec| R1

    R1 --> T1
    R2 --> T2
    R3 --> T3
    R4 --> T4
    R1 -->|service health events| T6

    subgraph "NA-R3 Reciprocal"
        REC -->|trajectory hints| O
        REC -->|thermal patterns| S
        REC -->|coupling history| P
    end
```

## Event Rate Projections

| Source | Events/min | Events/day | Primary Table |
|--------|-----------|------------|---------------|
| ORAC /health poll | 2 | 2,880 | T3 + T6 |
| ORAC /emergence poll | 2 | 2,880 | T1 |
| PV2 /bus/ws stream | ~5 | 7,200 | T1 |
| SYNTHEX /v3/thermal | 1 | 1,440 | T3 |
| POVM /pathways sync | 0.003 | ~5 | T2 |
| Atuin command events | ~10 | 14,400 | T1 |
| **Total** | **~20** | **~28,800** | |

## Retention Policy (R7)

```mermaid
flowchart TD
    E[Event ingested] --> F{Age?}
    F -->|< 30 days| FULL[Keep full payload]
    F -->|30-90 days| ENV[Strip payload_json<br/>keep envelope only<br/>~50 bytes/row]
    F -->|> 90 days| DEL[Delete entirely]
    
    G[Gradient captured] --> H{Age?}
    H -->|< 7 days| KEEP[Keep all<br/>1/min = 10,080]
    H -->|7-30 days| HOUR[Downsample to 1/hour<br/>552 rows]
    H -->|> 30 days| DAY[Downsample to 1/day<br/>~30 rows]
```

**30-day memory footprint:**
- Events: 28,800/day × 30 × 500 bytes = ~415 MB (pre-compaction)
- After R7 compaction: ~170 MB (envelope-only for days 1-30 + full for last 7)
- Gradients: 10,080 + 552 hourly + 30 daily = ~11K rows × 300 bytes = ~3 MB
- **Total steady-state: ~200 MB** — well within 1 GB limit

---

See: [[Reducers]] · [[T1 — HabitatEvent]] · [[T3 — GradientSnapshot]]
