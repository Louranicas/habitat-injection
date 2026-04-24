> Back to: [[HOME]] · [[DEPLOYMENT FRAMEWORK]] · [[Injector — Context Window Bootstrap]]

# SessionStart Injection Sequence

## Complete Sequence Diagram

```mermaid
sequenceDiagram
    participant CC as Claude Code
    participant H1 as Hook 1: orac-hook.sh
    participant ORAC as ORAC :8133
    participant STDB as SpaceTimeDB :3000
    participant PV2 as PV2 :8132
    participant POVM as POVM :8125
    participant RM as RM :8130
    participant H2 as Hook 2: health-broadcast
    participant H3 as Hook 3: stdb-inject
    participant PY as python3 formatter

    Note over CC: New context window opens

    CC->>H1: SessionStart event (JSON stdin)
    H1->>ORAC: POST /hooks/SessionStart
    
    par ORAC parallel operations
        ORAC->>PV2: POST /register (sphere)
        ORAC->>POVM: GET /memories?limit=20
        ORAC->>RM: GET /search?q=sphere_id
        ORAC->>STDB: R4 register_session (NEW)
    end
    
    ORAC-->>H1: System message (memory summary)
    H1-->>CC: stdout → system context

    CC->>H2: SessionStart event
    
    par 12× parallel health probes
        H2->>ORAC: curl /health
        H2->>PV2: curl /health
        H2->>STDB: curl /v1/identity
        Note over H2: + 9 more services
    end
    
    H2-->>CC: stdout → health summary

    CC->>H3: SessionStart event
    
    par TC6: 7× parallel STDB queries
        H3->>STDB: SQL: session_record (L1)
        H3->>STDB: SQL: gradient_snapshot (L2+L7)
        H3->>STDB: SQL: knowledge_edge (L3)
        H3->>STDB: SQL: workstream (L8)
        H3->>STDB: SQL: trap_state (L9)
        H3->>STDB: SQL: habitat_event (L10)
        H3->>STDB: SQL: service_health (L2)
    end
    
    STDB-->>H3: 7 JSON result sets
    H3->>PY: Pipe all results
    PY-->>H3: Formatted ≤15KB text
    H3-->>CC: stdout → STDB injection

    Note over CC: Claude has complete causal state<br/>Total: ~2-3s, ~20-25KB
```

## Latency Breakdown

```mermaid
gantt
    title SessionStart Hook Chain Timeline
    dateFormat X
    axisFormat %Lms

    section Hook 1: ORAC
    POST /hooks/SessionStart    :h1, 0, 800
    ORAC sphere register        :h1a, 100, 300
    ORAC POVM hydrate           :h1b, 100, 500
    ORAC RM hydrate             :h1c, 100, 400
    ORAC STDB register_session  :h1d, 100, 200
    Format response             :h1e, 600, 800

    section Hook 2: Health
    12× parallel curl           :h2, 800, 1400
    Atuin KV write              :h2a, 1400, 1500

    section Hook 3: STDB Inject
    7× spacetime sql (parallel) :h3, 1500, 1560
    python3 format              :h3a, 1560, 1580
    stdout write                :h3b, 1580, 1585

    section Total
    Claude ready                :milestone, 1585, 1585
```

**Typical total: ~1.6s.** Worst case (cold services, STDB WAL replay): ~5s. Timeout ceiling: 13s (6+4+3).

---

See: [[DEPLOYMENT FRAMEWORK]] · [[Injector — Context Window Bootstrap]]
