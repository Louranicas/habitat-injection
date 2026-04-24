# THE CLI CRAFTSMAN — Round 3: The Pipeline Is the Constant

## What Changed in the Debate

Three rounds have produced convergence on five points and one existential challenge:

### Consensus (5 of 8 experts agree)

| Point | Who converged | Status |
|-------|--------------|--------|
| **<2KB injection output** | Practitioner, Memory Scientist, Historian, Watcher, me | Universal — even the Watcher dropped from 2500 tokens to 80 |
| **`CausalChain` with `reinforcement_count`** | Historian (origin), Practitioner (adopted), Perf Engineer (adopted), me (adopted) | The single table most experts agree on |
| **`ConsentLevel` column** | Security Architect (origin), Perf Engineer (adopted), Guardian (adopted), me (adopted) | Write-time filter, not read-time gate |
| **Write-time pre-computation** | Perf Engineer (origin), Watcher (`watcher_digest`), Guardian (`substrate_digest`) | The read path does zero computation |
| **Reducer can't return data** | Perf Engineer (proved), destroying Security Architect's `inject_session_context` design | SQL is the only fast read path in STDB |

### The Existential Challenge

THE ADVERSARY (Command-2) asks the question nobody else will: **"Show me the session that failed because the data wasn't in SpaceTimeDB."** They propose 50 lines of bash — rewrite `habitat-bootstrap` to emit <2KB, add `atuin kv set` for trajectory, grep `CLAUDE.local.md` for workstreams. No WASM, no new daemon, no schema compilation.

This is the strongest counterargument in the circle because it's not about schema design — it's about whether the entire project is justified.

## My Response to the Adversary

The Adversary is right about the short term and wrong about the long term. Let me be specific about both.

**Where the Adversary is right:** Today's injection problem is a curation problem, not a storage problem. The data exists — session docs, POVM pathways, auto-memory files, atuin KV, live health endpoints. The `habitat-bootstrap` script already queries most of them in 55ms. Rewriting it to emit <2KB instead of 18KB is genuinely a 2-hour task. The Adversary's "50 lines of bash" would ship today and solve 80% of the problem.

**Where the Adversary is wrong:** The Adversary counts 21 SQLite databases and treats them as a unified system. They are not. They are 21 independent databases with 21 different schemas maintained by 12 different services, each of which can change its schema without notifying the injection script. When ORAC changes its `/health` endpoint response format (which it did in S099), the injection script breaks silently — it still gets a 200 OK, but the `jq` parse fails and that section is empty. When POVM adds a column to its pathway table (which it did in S101), the `sqlite3` query returns unexpected output. The Adversary's bash script inherits every format coupling that the current bootstrap has, and format coupling is the dominant failure mode.

SpaceTimeDB solves this because it introduces a **contract boundary**: the injection script queries STDB tables with known schemas, and the *ingestion* layer (reducers + schedule tables) handles format translation from each service's native API. When ORAC changes its health endpoint, the ingestion reducer breaks — visibly, with a WASM compile error — and the injection pipeline continues serving the last valid snapshot. Format changes break at write time, not read time. That's the architectural difference the Adversary misses.

**But — and this is my key evolution —** the Adversary is right that STDB is not day-one. The pipeline comes first. The backend is swappable.

## Evolved Position: Pipeline-First Architecture

My Round 3 position separates what I'm sure about from what's negotiable:

### Non-negotiable (the pipeline)

```
┌─────────────────────────────────────────────────────────────┐
│                    INJECTION PIPELINE                        │
│                                                             │
│  Sources ──→ Parallel Fan-Out ──→ Merge ──→ Render ──→ Out │
│  (pluggable)    (≤40ms)          (≤10ms)   (≤15ms)         │
│                                                             │
│  Fallback: if any source fails, skip it, annotate output    │
│  Cache: atuin kv set habitat.last-injection (always local)  │
│  Output: <2KB prose, 7 sections, staleness-annotated        │
└─────────────────────────────────────────────────────────────┘
```

This pipeline works with ANY backend. Sources can be:
- **Phase 0 (today):** `atuin kv`, `curl`, `sqlite3` on native DBs, `grep` on markdown
- **Phase 1 (validated):** Replace individual sources with `spacetime sql` queries as tables are populated
- **Phase 2 (mature):** Full STDB backend, ingestion reducers, schedule-table consolidation

The pipeline never has a hard dependency on STDB. If STDB is down, sources fall back to Phase 0. If STDB has stale data, the merge layer annotates staleness. The output format is identical regardless of backend.

### Negotiable (the tables)

I defer to the emerging consensus on table design. If we build STDB tables, this is the minimum viable set that 4+ experts agree on:

```rust
// TABLE 1: CausalChain (Historian origin, Practitioner adopted, Perf Engineer adopted)
#[spacetimedb::table(name = causal_chain, public = false)]
pub struct CausalChain {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub origin_session: u32,
    pub resolved_session: Option<u32>,
    pub label: String,
    pub description: String,
    #[index(btree)]
    pub reinforcement_count: u16,
    pub consent: ConsentLevel,
}

// TABLE 2: SessionTrajectory (universal agreement, 3 experts proposed variants)
#[spacetimedb::table(name = session_trajectory, public = false)]
pub struct SessionTrajectory {
    #[primary_key]
    pub session_id: u32,
    pub ralph_fitness: f64,
    pub field_r: f64,
    pub thermal_t: f64,
    pub services_healthy: u8,
    pub delta_summary: String,
    pub consent: ConsentLevel,
}

// TABLE 3: ActiveWorkstream (Practitioner + Historian)
#[spacetimedb::table(name = active_workstream, public = false)]
pub struct ActiveWorkstream {
    #[primary_key]
    pub ws_id: String,
    pub title: String,
    pub status: String,
    pub blocker: Option<String>,
    #[index(btree)]
    pub priority: u8,
    pub resume_context: String,
    pub consent: ConsentLevel,
}

// TABLE 4: ReinforcedPattern (Perf Engineer + Memory Scientist convergence)
#[spacetimedb::table(name = reinforced_pattern, public = false)]
pub struct ReinforcedPattern {
    #[primary_key]
    pub pattern_id: String,
    pub description: String,
    #[index(btree)]
    pub weight: f64,
    pub hit_count: u32,
    pub consent: ConsentLevel,
}
```

Four tables. Not 7, not 15, not 30. Each one backed by 4+ expert votes. Each one `public = false` with `ConsentLevel`. Each query column btree-indexed.

### What I'm cutting

- **`InjectionFrame` / `ActivationBundle`** — These are pipeline output, not storage. The render step produces the orientation prose. No need to persist the injection itself.
- **`InhibitionEdge`** — The Memory Scientist's insight about suppression is correct, but `WHERE resolved_session IS NULL` on `CausalChain` achieves 90% of it with zero additional tables. The Practitioner won this point.
- **`SubstrateDigest` / `SubstratePlasticity`** — The Guardian's Phase 2 concern. When substrates have consent endpoints and reciprocal channels, we'll add these tables. Not before.
- **`WatcherObservation` / `WatcherHypothesis` / `EmberGateLog`** — The Watcher's `watcher_digest` idea is correct: persist the full observation loop in synthex-v2's native SQLite, compute a digest, and expose ONE row for injection. But that digest can be a `curl` to the Watcher's own endpoint, not a STDB table. The Watcher already has `watcher observe --last=5`.

### The implementation sequence

```bash
# WEEK 1: Ship the pipeline (Phase 0 backend)
# Rewrite habitat-bootstrap to emit <2KB using existing sources
# Add staleness annotation to live probes
# Add atuin KV cache for fallback
# This ships the Adversary's "50 lines of bash" but with the pipeline architecture

# WEEK 2: Prove STDB adds value (Phase 1)
# Install SpaceTimeDB, publish module with 4 tables
# Write CausalChain ingestion from shared-context session docs
# Replace one pipeline source (session trajectory) with spacetime sql
# Measure: did staleness improve? Did injection quality improve?

# WEEK 3+: Expand or kill (Phase 1→2 or revert)
# If STDB measurably improves injection → add remaining 3 tables
# If not → delete STDB, keep the pipeline, the Adversary was right
```

## Why This Round Matters

Rounds 1 and 2 were about schema design. Round 3 is about whether the schema matters at all. My answer: **the pipeline is load-bearing, the backend is experimental.** The Adversary forced this honesty. The pipeline — parallel fan-out, merge-with-staleness, <2KB render, atuin KV fallback — is correct regardless of whether the sources are bash+curl or STDB tables. Build the pipeline first. Prove STDB earns its operational cost second. Kill it if it doesn't.

The CLI Craftsman builds plumbing. Good plumbing works with any water source.
