> Back to: [[HOME]] · [[MASTER INDEX]] · [[Architecture Overview]]

# Data Flow

## Overview

Memory flows through three stages: **capture** (services -> consolidation), **storage** (SQLite tables), and **injection** (tables -> context window).

```mermaid
flowchart LR
    subgraph Capture
        SS["/save-session checkpoint"]
        ORAC["ORAC /health"]
        POVM["POVM pathways"]
    end

    subgraph Consolidation
        CI["m15 checkpoint_ingest"]
        TC["m15b trajectory_capture"]
        HE["m16 hebbian_engine"]
        CB["m17 cache_builder"]
        AC["m18 atuin_cache"]
    end

    subgraph Storage
        CC["causal_chain"]
        ST["session_trajectory"]
        WS["workstream"]
        RP["reinforced_pattern"]
        IC["injection_cache"]
        SC["session_checkpoint"]
    end

    subgraph Injection
        PQ["m11 parallel_query"]
        CF["m14 consent_filter"]
        PR["m12 prose_renderer"]
        FB["m13 fallback"]
    end

    SS --> CI --> CC & ST & WS & RP & SC
    ORAC --> TC --> ST
    CI --> HE --> CC & RP
    HE --> CB --> IC
    CB --> AC

    IC --> PQ --> CF --> PR --> CTX["Context Window"]
    AC --> FB --> CTX
```

## Write Path (Consolidation)

1. /save-session writes checkpoint to `~/projects/shared-context/sessions/*.md`
2. `m15_checkpoint_ingest` parses YAML frontmatter + markdown sections
3. Harvests into 5 tables (checkpoint, trajectory, workstream, causal_chain, pattern)
4. `m16_hebbian_engine` decays unfired patterns, reinforces fired ones, prunes weak
5. `m17_cache_builder` rebuilds `injection_cache` from fresh data
6. `m18_atuin_cache` writes to atuin KV for fallback

## Read Path (Injection)

1. SessionStart hook fires `habitat-inject`
2. `m11_parallel_query` runs 4 SQLite queries concurrently
3. `m14_consent_filter` drops non-Emit rows
4. `m12_prose_renderer` renders <2KB prose payload
5. If SQLite fails: `m13_fallback` tries atuin KV, then static
6. Payload injected into context window system message
