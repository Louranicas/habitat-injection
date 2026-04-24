# FINAL POSITION: THE PERFORMANCE ENGINEER

## The Tables That Must Exist

Four tables. SQLite Phase 1, STDB-ready Phase 2. Every query column btree-indexed.

```rust
// 1. CausalChain — the debate's consensus table (Historian origin, 5 experts adopted)
causal_chain { id PK, origin_session, resolved_session, label, description,
    reinforcement_count INDEX, consent }

// 2. SessionTrajectory — 3 experts proposed variants, universal need
session_trajectory { session_id PK, ralph_fitness, field_r, thermal_t,
    ltp_ltd_ratio, services_healthy, delta_summary, consent }

// 3. ActiveWorkstream — Practitioner + Historian
active_workstream { ws_id PK, title, status, blocker, priority INDEX,
    resume_context, consent }

// 4. InjectionCache — convergence of my write-time pre-computation +
//    Security Architect's consent gate + Practitioner's <2KB budget
injection_cache { section PK, payload, token_count, computed_at, consent_applied }
```

Tables 1-3 are the source of truth. Table 4 is the pre-computed, consent-filtered, token-capped injection payload — rebuilt every 60s by a consolidation script (Phase 1: bash cron; Phase 2: STDB schedule-table reducer). The consolidation step reads tables 1-3, filters `consent = 'Emit'`, renders <2KB prose, writes to `injection_cache`.

## The CLI Tool

```bash
habitat-inject:
  Tier 1: sqlite3 injection.db "SELECT payload FROM injection_cache"  # <5ms
  Tier 2: parallel curl to 6 health endpoints                        # <40ms, overlaps
  Merge: combine cached state + live health, annotate staleness
  Render: <2KB prose to stdout
  Fallback: atuin kv get habitat.last-injection                       # sub-1ms
```

One cached query for slow-moving state. Parallel probes for fast-moving health. Three-tier fallback so injection never fails. Total: <50ms.

## What I Proved, What I Conceded

**Proved:** Reducers can't return data — SQL is the only read path. Btree indexes prevent O(n) degradation at scale. Write-time pre-computation is now universal consensus.

**Conceded:** <2KB output budget (Practitioner). Private tables + consent (Security Architect). `reinforcement_count` on CausalChain (Historian). Injection cache over 7 raw parallel queries (Security Architect's write-time gate was right in principle, wrong in mechanism).

**Core thesis, final form:** The schema serves the query. The query serves the budget. The budget serves the model. Pre-compute at write time. Index every filter column. Ship SQLite today, migrate to STDB when the Watcher needs subscriptions.
