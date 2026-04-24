# CLI Binary Architecture

> Back to: [[EXECUTION_PLAN]] · [[DEPLOYMENT_FLOW]]

## Binary Dependency Graph

```mermaid
graph TD
    subgraph "CLI Binaries"
        INIT[habitat-init]
        INJECT[habitat-inject]
        CONSOL[habitat-consolidate]
        QUERY[habitat-query]
        SEED[habitat-seed]
    end

    subgraph "L1 Foundation"
        L1C[m03_config]
        L1T[m01_types]
        L1E[m02_errors]
        L1K[m05_constants]
    end

    subgraph "L2 Schema"
        L2S[m06_schema]
        L2CC[m07_causal_chain]
        L2TR[m08_trajectory]
        L2WS[m09_workstream]
        L2PA[m10_pattern]
        L2CP[m10b_checkpoint]
    end

    subgraph "L3 Injection"
        L3PQ[m11_parallel_query]
        L3PR[m12_prose_renderer]
        L3FB[m13_fallback]
        L3CF[m14_consent_filter]
    end

    subgraph "L4 Consolidation"
        L4CI[m15_checkpoint_ingest]
        L4TC[m15b_trajectory_capture]
        L4HE[m16_hebbian_engine]
        L4CB[m17_cache_builder]
        L4AC[m18_atuin_cache]
    end

    subgraph "L5 Query"
        L5PQ[m19_preset_queries]
        L5RQ[m20_raw_query]
        L5FZ[m21_fzf_browser]
        L5SC[m21b_scripts_engine]
    end

    INIT --> L1C
    INIT --> L2S

    INJECT --> L1C
    INJECT --> L2S
    INJECT --> L3PQ
    INJECT --> L3PR
    INJECT --> L3FB
    INJECT --> L3CF
    INJECT --> L4CB
    INJECT --> L4AC

    CONSOL --> L1C
    CONSOL --> L2S
    CONSOL --> L4CI
    CONSOL --> L4TC
    CONSOL --> L4HE
    CONSOL --> L4CB
    CONSOL --> L4AC

    QUERY --> L1C
    QUERY --> L2S
    QUERY --> L5PQ
    QUERY --> L5RQ
    QUERY --> L5FZ

    SEED --> L1C
    SEED --> L2S
    SEED --> L2CC
    SEED --> L2TR
    SEED --> L2WS
    SEED --> L2PA

    style INJECT fill:#2d5016,color:#fff
    style CONSOL fill:#8b6914,color:#fff
    style INIT fill:#1a3a5c,color:#fff
    style QUERY fill:#5c1a3a,color:#fff
```

## SessionStart Injection Flow

```mermaid
sequenceDiagram
    participant CC as Claude Code
    participant H3 as Hook 3: habitat-inject
    participant DB as injection.db
    participant AT as atuin KV
    participant ST as static

    CC->>H3: SessionStart event

    H3->>DB: SELECT payload FROM injection_cache<br/>WHERE section='full_payload'
    alt Cache fresh (<60s)
        DB-->>H3: payload (Tier 1 hit, <5ms)
    else Cache stale/missing
        H3->>DB: 4× sequential queries (chains, trajectory, workstreams, patterns)
        DB-->>H3: raw rows
        H3->>H3: consent filter → render prose
        H3->>DB: INSERT OR REPLACE injection_cache
        H3->>AT: atuin kv set habitat.last-injection (best-effort)
    else DB unavailable
        H3->>AT: atuin kv get habitat.last-injection
        alt Key exists
            AT-->>H3: payload (Tier 2 hit)
        else Key missing
            H3->>ST: static_fallback()
            ST-->>H3: "NO INJECTION STATE..." (Tier 3)
        end
    end

    H3-->>CC: stdout → system message (<2KB, <100ms)
```

## Post-Session Consolidation Flow

```mermaid
sequenceDiagram
    participant US as /save-session
    participant HC as habitat-consolidate
    participant OR as ORAC :8133
    participant DB as injection.db
    participant AT as atuin KV

    US->>HC: --session 110 --fired-patterns verify-before-ship,read-only-forensics

    HC->>OR: curl /health
    OR-->>HC: JSON (fitness, field_r, thermal, ltp/ltd, services)

    HC->>DB: insert_point(110, snapshot)
    Note over HC,DB: Trajectory captured

    HC->>DB: BEGIN TRANSACTION
    HC->>DB: decay_all(0.95)
    HC->>DB: reinforce("verify-before-ship", 110)
    HC->>DB: reinforce("read-only-forensics", 110)
    HC->>DB: prune_weak(0.05)
    HC->>DB: auto_resolve_stale(110, 10)
    HC->>DB: COMMIT
    Note over HC,DB: Hebbian cycle complete

    HC->>DB: rebuild_cache(110, services, thermal)
    Note over HC,DB: injection_cache refreshed

    HC->>AT: write_injection_cache(entry)
    Note over HC,AT: Tier 2 fallback updated

    HC-->>US: "Consolidated S110: 12 decayed, 2 reinforced, 0 pruned"
```

## Data Seeding Pipeline

```mermaid
flowchart LR
    subgraph Sources
        SN[Session notes<br/>S001-S108 .md files]
        CL[CLAUDE.local.md<br/>metrics tables]
        ST[service_tracking.db<br/>learned_patterns]
    end

    subgraph "habitat-seed"
        BUG[BUG-NNN regex<br/>+ trap name match]
        MET[Metric parser<br/>fitness/r/thermal]
        WS[Workstream parser<br/>active/blocked/deferred]
        PAT[Pattern transformer<br/>category + weight]
    end

    subgraph "injection.db"
        CC[(causal_chain<br/>~15-25 rows)]
        TR[(session_trajectory<br/>~10 rows)]
        W[(workstream<br/>~6 rows)]
        RP[(reinforced_pattern<br/>~141 rows)]
        IC[(injection_cache<br/>1 row)]
    end

    SN --> BUG --> CC
    CL --> MET --> TR
    CL --> WS --> W
    ST --> PAT --> RP
    CC & TR & W & RP --> IC
```

---

*Back to: [[EXECUTION_PLAN]] · [[DEPLOYMENT_FLOW]] · [[HOME]]*
