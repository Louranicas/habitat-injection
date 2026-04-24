> Back to: [[HOME]] · [[MASTER INDEX]] · [[L3 Injection Engine]]

# Injection Pipeline

## SessionStart Hook Chain

```mermaid
sequenceDiagram
    participant CC as Claude Code
    participant H1 as Hook 1: orac-hook.sh
    participant H2 as Hook 2: session-health-broadcast.sh
    participant H3 as Hook 3: habitat-inject
    participant DB as injection.db
    participant AK as atuin KV

    CC->>H1: SessionStart event
    H1->>CC: ORAC hydration (~20ms)

    CC->>H2: SessionStart event
    H2->>CC: health broadcast (~15ms)

    CC->>H3: SessionStart event
    H3->>DB: parallel query (4 tables)
    DB-->>H3: structured results
    H3->>H3: consent filter (Emit only)
    H3->>H3: prose render (<2KB)
    H3->>AK: cache last payload
    H3->>CC: inject into system message (~60ms)
```

## Latency Budget

```mermaid
gantt
    title Injection Latency Budget (<100ms)
    dateFormat X
    axisFormat %L ms

    section Queries
    causal_chain query    :0, 15
    trajectory query      :0, 10
    workstream query      :0, 10
    pattern query         :0, 15

    section Processing
    consent filter        :15, 25
    prose render          :25, 45
    token count           :45, 50

    section Output
    format payload        :50, 55
    write atuin cache     :55, 70
    stdout                :70, 75
```

## Fallback Tiers

See [[Three-Tier Fallback]] for the degradation chain when SQLite is unavailable.
