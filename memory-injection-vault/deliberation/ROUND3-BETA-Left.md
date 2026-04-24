# THE PRACTITIONER — Round 3: The Debate Has Converged. Here's What We Ship.

## What Changed

Three rounds of argument have produced a clear consensus map. Let me name it explicitly — nobody else has.

### Consensus (5+ experts agree)

1. **Injection output <2KB of terse prose.** The Memory Scientist, Historian, Watcher, and CLI Craftsman all absorbed this. The Performance Engineer never disputed it (they care about query speed, not output format). Only the Security Architect focused on a different concern (consent gating), but their reducer output was also capped at 4096 tokens. This is settled.

2. **CausalChain with `reinforcement_count` is a real table.** The Historian proposed it. I adopted it in Round 2. The Performance Engineer added `reinforcement_count` + `resolved_session` to their `causal_chain` in Round 2. The Watcher acknowledged it covers session-level causality. The Memory Scientist's `episodic_trace` overlaps it. Five experts now have some version of this table. The Historian won this argument cleanly.

3. **Private tables + consent column.** The Security Architect proposed it. The Performance Engineer adopted `public = false` + `ConsentLevel` in Round 2 (and proved it costs zero read-time performance). The CLI Craftsman absorbed consent as a render-time filter. The Substrate Guardian adopted it on `substrate_plasticity`. This is settled — tables are private, rows carry consent.

4. **Parallel SQL, not single-reducer.** The Performance Engineer proved in Round 2 that SpaceTimeDB reducers return `()`, not data. The Security Architect's `inject_session_context` → `String` is architecturally impossible. The CLI Craftsman uses parallel SQL. I use 4 sequential queries (fast enough). The read path is SQL. This is settled.

5. **Injection ≠ persistence.** Every expert now agrees STDB should store more than it injects. The Watcher's `watcher_digest` pattern (persist observations, inject digest) is the cleanest expression. But everyone converged here.

### Unresolved

1. **The Adversary's challenge: do we need STDB at all?** The Adversary (Command-2) dropped a bomb nobody answered. Their argument: `ls -t ~/projects/shared-context/Session\ 0*.md | head -5 | xargs head -20` already gives session trajectory. `atuin kv` already stores fitness. `CLAUDE.local.md` already has workstreams. Every table we propose duplicates data that exists in markdown + atuin + curl. The Adversary's 50-line bash alternative ships today with zero new dependencies.

2. **Who writes the InjectionFrame?** The Watcher says it's the Watcher's job (m46 Observer curates hot_traps). The Memory Scientist says it's a Hebbian activation function over episodic/semantic/procedural stores. I said it's a consolidation reducer. Nobody has specified this reducer's actual logic.

3. **Inhibition: separate table or WHERE clause?** The Memory Scientist wants `inhibition_edge` with graduated strength. The Performance Engineer says `WHERE resolved_session IS NULL` is sufficient. The Substrate Guardian wants per-substrate inhibition. I said resolved_session is enough. This is still contested.

## My Response to the Adversary

The Adversary asks: "Show me the session that failed because the data wasn't in SpaceTimeDB."

Fair question. Wrong framing.

No session failed because data wasn't in STDB — STDB doesn't exist yet. Sessions failed because **data was in the wrong format for fast, structured querying**. Session S071's convergence trap was buried in a 12KB session note that grep could theoretically find — if you knew to grep for "convergence" in the first place. `rg -l "convergence" ~/projects/shared-context/Session\ 0*.md` returns results, but only if the fresh Claude knows the word "convergence" is the one to search for. That's the amnesia problem: you don't know what to search for because you don't know what you don't know.

A `CausalChain` row with `label: convergence_trap_ralph, reinforcement_count: 7, resolved_session: NULL` doesn't require you to know the search term. It's surfaced by `ORDER BY reinforcement_count DESC LIMIT 5`. The ranking algorithm surfaces it; the fresh Claude doesn't need to.

**But** — and here I partially concede to the Adversary — this exact query could run against a SQLite database. Or a JSON file with `jq`. STDB's value isn't the query; it's the **real-time subscription model** and the **WASM reducer as consolidation engine**. If we're building a one-shot injection tool, the Adversary is right that bash + sqlite3 is simpler. If we're building a live substrate that the Watcher queries at 1Hz during runtime, STDB's subscription push model matters. That's the real decision point, and it depends on whether the Watcher is Phase 1 or Phase 2.

## My Recommendation: Ship Phase 1 as SQLite, Design for STDB Migration

Phase 1 (ships this week):
- 4 tables in a single SQLite file: `injection_frame`, `trajectory_point`, `workstream`, `causal_chain`
- Bash CLI: `habitat-inject` — 4 queries, format to <2KB prose, output to SessionStart hook
- Consolidation script: `habitat-consolidate` — runs at session end, writes InjectionFrame + updates CausalChain reinforcement counts

Phase 2 (when STDB justifies its operational cost):
- Migrate tables to STDB
- Add Watcher observation + digest tables
- Add subscription-based live queries for runtime (not just injection)
- Add reciprocal write-back to substrates

## Final Schema (The Practitioner's Recommendation)

```rust
// These are SQLite CREATE TABLE statements dressed in Rust syntax
// for schema compatibility when we migrate to STDB

struct InjectionFrame {
    id: i64,              // PRIMARY KEY AUTOINCREMENT
    session_id: u32,
    created_at: i64,      // unix epoch ms
    orientation_line: String,
    interrupted_task: Option<String>,
    last_gate: String,
    hot_traps: String,    // JSON array of top 5 (was 3, Historian convinced me)
    active_feedback: String, // JSON array of 10 imperatives
    health_anomalies: String, // JSON array, empty = all healthy
}

struct TrajectoryPoint {
    id: i64,
    session_id: u32,
    ralph_fitness: f64,
    field_r: f64,
    thermal_t: f64,
    ltp_ltd_ratio: f64,
    services_healthy: u8,
    delta_summary: String,
}

struct Workstream {
    id: i64,
    name: String,
    status: String,       // active | blocked | deferred | complete
    blocker: Option<String>,
    last_touched_session: u32,
    resume_context: String,
}

struct CausalChain {
    id: i64,
    origin_session: u32,
    resolved_session: Option<u32>,
    chain_type: String,   // bug | trap | plan | pattern
    label: String,
    description: String,
    reinforcement_count: u16,
    injectable: bool,     // consent gate
}
```

## Where Each Expert Landed (My Assessment)

| Expert | Core Contribution | Status |
|--------|------------------|--------|
| Historian | CausalChain with reinforcement_count | **ADOPTED by 4 experts** |
| Memory Scientist | Inhibition / suppression of stale state | Partially adopted (WHERE clause, not separate table) |
| CLI Craftsman | Merge-and-annotate, three-tier fallback | **ADOPTED as rendering strategy** |
| Substrate Guardian | Reciprocity, per-substrate digest | Deferred to Phase 2 |
| Security Architect | Private tables, consent column | **ADOPTED by 3 experts** |
| Performance Engineer | Btree indexes, query-shaped design | Partially adopted (indexes yes, denormalization no) |
| Watcher | Observation → digest pipeline, curation engine | Deferred to Phase 2 |
| Adversary | "Prove you need STDB" | **Unanswered. Phase 1 ships as SQLite.** |
| Practitioner (me) | <2KB injection, progressive disclosure, InjectionFrame | **ADOPTED by all experts** |

The debate has converged. Four tables, SQLite-first, STDB-ready. Ship it.
