> Back to: [[HOME]] · [[Reducers]]

# Reducer Lifecycle — When Each Fires

## Trigger Sources

```mermaid
flowchart TD
    subgraph "Event-Driven (real-time)"
        ING[Ingester] -->|every event| R1[R1 ingest_event]
        ING -->|POVM sync 300s| R2[R2 reinforce_edge]
        ING -->|every 60s| R3[R3 capture_gradient]
        ORAC[ORAC hook] -->|SessionStart| R4A[R4 register_session]
        ORAC -->|Stop| R4B[R4 close_session]
        WATCH[Watcher daemon] -->|anomaly| R9[R9 watcher_reinforce]
        WATCH -->|observation| R10[R10 watcher_annotate]
    end

    subgraph "Scheduled (autonomous)"
        S5[Every 6 hours] --> R5[R5 run_decay]
        S7[Every 24 hours] --> R7[R7 compact_old_events]
        S8[Every 300 ticks] --> R8[R8 consolidate_mature]
    end

    subgraph "On-Demand (human-triggered)"
        CLI[spacetime sql / CLI] --> R6[R6 forget_sphere]
    end

    R1 --> T1[(T1 HabitatEvent)]
    R2 --> T2[(T2 KnowledgeEdge)]
    R3 --> T3[(T3 GradientSnapshot)]
    R4A & R4B --> T4[(T4 SessionRecord)]
    R5 --> T2
    R6 --> T1 & T2 & T3
    R7 --> T1 & T3
    R8 --> T2
    R9 --> T2
    R10 --> T8[(T8 WatcherObservation)]
```

## Scheduled Reducer Cadence

```mermaid
gantt
    title Reducer Firing Pattern (24-hour window)
    dateFormat HH:mm
    axisFormat %H:%M

    section R3 capture_gradient
    Capture  :r3a, 00:00, 1m
    Capture  :r3b, 00:01, 1m
    ...1440 per day  :r3c, 00:02, 1m

    section R5 run_decay
    Decay cycle 1  :r5a, 00:00, 5m
    Decay cycle 2  :r5b, 06:00, 5m
    Decay cycle 3  :r5c, 12:00, 5m
    Decay cycle 4  :r5d, 18:00, 5m

    section R7 compact_old_events
    Compaction run  :r7a, 03:00, 15m

    section R8 consolidate
    Consolidate (300-tick intervals, ~4× per hour)  :r8a, 00:00, 2m
    ...96 per day  :r8b, 00:15, 2m
```

## Reducer Conflict Matrix

| Reducer | Reads | Writes | Can Conflict With |
|---------|-------|--------|-------------------|
| R1 ingest | — | T1 | R7 (concurrent delete+insert) |
| R2 reinforce | T2 | T2 | R5 (decay vs reinforce race) |
| R3 capture | — | T3 | R7 (snapshot downsample) |
| R5 decay | T2 | T2 | R2, R9 (concurrent weight change) |
| R6 forget | T1,T2,T3 | T1,T2,T3 | All (mass delete) |
| R7 compact | T1,T3 | T1,T3 | R1, R3 (concurrent insert+delete) |

**STDB guarantees:** All reducers run in transactions. Conflicts are resolved by STDB's MVCC — concurrent reducers see consistent snapshots. No application-level locking needed.

---

See: [[Reducers]] · [[Data Flow — Ingestion]]
