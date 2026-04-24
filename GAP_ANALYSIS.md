---
type: strategic-analysis
title: SpaceTimeDB Habitat Integration Plan — Gap Analysis
date: 2026-04-24
session: 109
scope: self-critique of SpaceTimeDB-Habitat-Integration-Plan-2026-04-24
tags: [gap-analysis, self-critique, risk, plan-amendment, spacetimedb]
status: AUTHORED — informs plan v2
---

> Back to: [SpaceTimeDB Integration Plan](SpaceTimeDB%20Habitat%20Integration%20Plan%20%E2%80%94%202026-04-24.md)

# SpaceTimeDB Habitat Integration Plan — Gap Analysis

## Summary

17 gaps identified. **5 critical** (would cause plan failure or significant rework). **7 important** (reduce confidence or quality). **5 nice-to-have** (flag but don't block). The strongest single miss is **C1 (STDB reducer I/O prohibition)** — the plan's V1 bootstrap view uses `.iter()` which STDB views explicitly forbid, and the ingester architecture is correct but the injector's subscription model needs rethinking.

---

## C — Critical gaps (address before execution)

### C1. STDB Views cannot use `.iter()` — the bootstrap view is invalid as written

**Evidence:** STDB docs state explicitly: "Views can only access data through indexed lookups (`.find()`, `.filter()`). Full table scans create pessimistic read sets requiring re-evaluation on ANY row change." The plan's V1 `context_window_bootstrap` view (§2.3 lines 370-404) calls `.iter().last()` on `gradient_snapshot` and `session_record`. This will either fail to compile or create catastrophic re-evaluation overhead.

**Impact:** Phase E's core deliverable — the bootstrap view — is architecturally invalid. The injector can't subscribe to a view that doesn't work.

**Recommendation:** Replace the STDB view with one of two alternatives:
- **(a) Procedure-based query (preferred):** Use STDB's beta `#[spacetimedb::procedure]` to build the bootstrap payload. Procedures CAN do I/O and CAN return data to the caller. The injector calls the procedure directly instead of subscribing to a view. Latency trades subscription reactivity for simplicity. ~20 LOC change.
- **(b) Materialised snapshot table:** A scheduled reducer captures the "latest state" into a `bootstrap_snapshot` table every 30s. The injector subscribes to this single-row table. Gets subscription reactivity but with 30s staleness. ~40 LOC.

**Recommended choice:** (a) for Phase E MVP, evolve to (b) if real-time push is needed later.

### C2. Ingester and STDB module are conflated in the Cargo workspace

**Evidence:** §3.2 shows `habitat-stdb/module/` (the WASM module compiled to `wasm32-unknown-unknown`) and `habitat-stdb/ingester/` (a native Rust binary using tokio, reqwest, WebSocket) in the same Cargo workspace. STDB modules compile to WASM; the ingester requires native network I/O. A shared workspace with different compilation targets is a build-system trap — `cargo build` will try to compile both, and the module's dependencies (spacetimedb runtime) conflict with the ingester's (tokio, reqwest).

**Impact:** Build failures at Phase A. Developer confusion about which binary is which.

**Recommendation:** Split into two workspaces:
```
habitat-stdb-module/     # WASM module → spacetime publish
habitat-stdb-ingester/   # Native binary → ~/.local/bin/
habitat-stdb-injector/   # Native binary → ~/.local/bin/
```
Or a single workspace with careful `[target]` cfg and `--workspace --exclude` flags. The split is cleaner. ~0 LOC, structural only.

### C3. No retention policy — `habitat_event` grows unbounded

**Evidence:** T1 `HabitatEvent` receives events from 5 sources (ORAC every 30s, PV2 real-time, SYNTHEX every 60s, POVM every 300s, Atuin on every command). At steady state: ~2 ORAC events/min + ~5 PV2 events/min + 1 SYNTHEX/min + ~10 command events/min = ~18 events/min = ~26,000 events/day. T3 `GradientSnapshot` at 1/min = 1,440/day. After 30 days: ~780K events + 43K snapshots. STDB holds all data in memory — at ~500 bytes/event, that's ~400MB just for events.

**Impact:** Memory pressure against the 1GB resource limit in devenv.toml. After ~60 days, OOM kill risk is real.

**Recommendation:** Add R7 `compact_old_events` scheduled reducer:
- Events older than 30 days: delete payload_json, keep envelope (event_type, source, timestamp, causal_parent) — ~50 bytes/row instead of ~500
- Events older than 90 days: delete entirely
- GradientSnapshots older than 7 days: downsample to 1/hour (keep only the on-the-hour snapshot)
- GradientSnapshots older than 30 days: downsample to 1/day

Add a T1 field `retention_class: String` ("full"|"envelope"|"sampled") to track compaction state. ~80 LOC. Schedule every 24h.

### C4. Causal parent assignment is hand-waved — no concrete algorithm

**Evidence:** §2.2 says `causal_parent: Option<u64>` and §4 Phase C says "ingester tags events with causal_parent when source provides attribution." But no source currently provides attribution in a format the ingester can consume. ORAC's emergence events don't carry a reference to the thermal event that triggered detection. PV2's sphere events don't reference the hook event that registered them. The causal chain is the plan's key differentiator — and it has no concrete wiring.

**Impact:** Phase C's primary deliverable — causal chains — may ship as a schema with zero populated `causal_parent` values. The "why did fitness drop?" query returns nothing useful.

**Recommendation:** Define 5 concrete causal linkage rules for Phase C:

| Event Type | Causal Parent Source | How |
|---|---|---|
| `emergence.detected` | The ORAC tick's thermal/coupling event | ORAC emits `{detector_id, triggered_by_tick}` — ingester looks up the event at that tick |
| `sphere.registered` | The SessionStart hook event | Ingester assigns parent = the `session.start` event for this session |
| `thermal.adjustment` | The gradient_snapshot that crossed threshold | Ingester compares consecutive snapshots, links to the crossing |
| `command.postexec` | The `command.preexec` for same command_hash | Ingester pairs by `command_hash` within same session |
| `watcher.observation` | The gradient_snapshot that triggered the anomaly detector | synthex-v2 watcher already tracks `metric_json` — extract the snapshot reference |

This requires ORAC to add `triggered_by_tick` to emergence event payloads (~5 LOC in ORAC). Without this, causal_parent is aspirational.

### C5. No rollback strategy for migration failures

**Evidence:** §5.1 lists 15 migration sources → STDB tables. §5 says "SQLite sources preserved as backup." But no concrete rollback procedure exists. If the POVM→STDB migration corrupts edge weights (scale mismatch, namespace collision, duplicate handling), the plan says to keep POVM as source-of-truth — but nothing specifies how to detect corruption or trigger rollback.

**Impact:** Silent data corruption in T2 KnowledgeEdge. The unified graph becomes less trustworthy than the fragmented sources it replaced.

**Recommendation:** Add verification gates per migration source:
1. Pre-migration: capture `SELECT COUNT(*), SUM(weight), AVG(weight) FROM source_table` as a checksum
2. Post-migration: query STDB equivalent, compare counts and aggregates
3. Tolerance: ±0.01 on weight aggregates, exact match on counts
4. On failure: log, abort remaining migrations, preserve source DB, file as BUG

Add to Phase B acceptance criteria. ~30 LOC in migration scripts.

---

## I — Important gaps (reduce plan confidence)

### I1. STDB `spacetimedb-standalone` binary not yet built or tested on this machine

**Evidence:** The plan assumes `spacetimedb-standalone` can run from `~/.local/bin/` or be built from the cloned repo at `~/claude-code-workspace/spacetimedb/`. No verification that the repo builds clean, that the standalone binary runs, or that `spacetime publish` works locally. The repo is the upstream STDB source (40+ crates), not a pre-built binary. Building it requires `cargo build --release -p spacetimedb-standalone` which may take 20+ minutes and may have dependency conflicts.

**Recommendation:** Phase A should start with a 30-minute pre-flight:
1. `cargo build --release -p spacetimedb-standalone` from the cloned repo (or `curl -sSf https://install.spacetimedb.com | sh`)
2. `./spacetimedb-standalone start --listen-addr 127.0.0.1:3000` — verify it starts
3. `spacetime publish test-module --module-path <minimal>` — verify publish works
4. `spacetime sql test-module "SELECT 1"` — verify SQL works

If any step fails, the entire plan timeline shifts. Better to know in 30 minutes than discover mid-Phase-A.

### I2. The injector uses STDB Rust SDK — but the SessionStart hook runs as a shell command

**Evidence:** §3.4 shows the injector as a Rust binary connecting to STDB via SDK. But ORAC's SessionStart hook (§3.4) calls it as a shell command and includes its stdout in the hook response. The STDB Rust SDK uses a persistent WebSocket connection (`DbConnection::builder().build()`). A CLI tool that connects, subscribes, receives data, formats, prints, and exits needs to handle the connection lifecycle in <100ms. The SDK's `run_threaded()` / `run_async()` patterns are designed for long-lived connections, not one-shot queries.

**Recommendation:** The injector should use `spacetime sql` CLI for one-shot queries instead of the Rust SDK. Simpler, proven, no connection lifecycle management:
```bash
spacetime sql habitat "SELECT * FROM bootstrap_snapshot LIMIT 1" --format json
```
Or use STDB's HTTP API directly via `curl` — avoids SDK dependency entirely. Reserve the Rust SDK for the long-lived ingester, which does need a persistent connection.

### I3. KnowledgeEdge conflates 5 different edge types into one table — query efficiency concern

**Evidence:** T2 KnowledgeEdge holds POVM pathways (3554), learned_patterns (141), orchestration_graph (29), hebbian_pathways (109), and synergy edges (89). These have different query patterns: POVM edges are queried by namespace prefix, learned_patterns by pattern_name, orchestration by source/target module, synergy by system pair. A single table with `edge_type` filter may work but loses the specialised indexes each source table had.

**Recommendation:** Add compound indexes:
```rust
#[index(btree)]  // on (edge_type, namespace) for POVM queries
#[index(btree)]  // on (edge_type, weight DESC) for "top patterns" queries
#[index(btree)]  // on (source_id, target_id) for graph traversal
```
Verify via `EXPLAIN` on the 5 most common query patterns before declaring Phase B complete. If any query requires full table scan on 3900+ rows, split into per-type tables instead.

### I4. No chaos/failure testing for STDB itself

**Evidence:** Risk register (§7) lists mitigations but no test plan. What happens when:
- STDB crashes mid-reducer? (WAL should recover, but untested)
- Ingester loses connection to STDB for 5 minutes? (Events lost? Buffered? Retried?)
- devenv restarts STDB while ingester is mid-write? (Connection error handling?)
- Disk fills up? (WAL can't flush — what's the failure mode?)

**Recommendation:** Add Phase A acceptance criterion: "After STDB kill -9 and restart, last 10 events are still queryable." Add Phase D criterion: "Ingester reconnects within 30s after STDB restart and resumes ingestion with zero event loss." ~0 additional LOC, just test procedures.

### I5. Ingester port `:3001` for health/metrics is not registered in devenv.toml

**Evidence:** §4 Phase D says "ingester exposes `/health` + `/metrics` on `:3001`" but §3.1 only registers `habitat-stdb` on `:3000`. The ingester is a separate binary — it needs its own devenv registration, or it's invisible to the Habitat's health monitoring.

**Recommendation:** Register ingester as a separate devenv service:
```toml
[[services]]
id = "habitat-stdb-ingester"
command = "habitat-stdb-ingester"
health_check_url = "http://localhost:3001/health"
dependencies = ["habitat-stdb"]
```
Batch 2 (depends on STDB). ~10 lines in devenv.toml.

### I6. VMS (1881 memories) not addressed in migration plan

**Evidence:** §1.1 lists VMS as substrate #5 with 1881 memories. §5.1 migration table doesn't mention VMS. VMS has semantic memories with a 12D morphogenic tensor. These don't map cleanly to any of the 8 STDB tables. The plan silently drops VMS from scope.

**Recommendation:** Explicitly document VMS as out-of-scope with rationale: "VMS memories are high-dimensional semantic vectors unsuited for relational storage. STDB is not a vector database. VMS continues as the semantic memory substrate; STDB covers structured/causal/temporal data." This prevents a future reader from thinking VMS was forgotten.

### I7. STDB `spacetimedb = "2.1"` pinned in synthex-v2 — potential version conflict

**Evidence:** `synthex-v2/plan.toml` pins `spacetimedb = { version = "2.1" }` behind the `sidecar-stdb` feature flag. The new `habitat-stdb-module/` will also use `spacetimedb` as a dependency. If synthex-v2 and habitat-stdb-module use different STDB SDK versions, the module published to the STDB instance may be incompatible with the synthex-v2 client that queries it.

**Recommendation:** Pin both to the same STDB version. Add a workspace-level `[dependencies]` or a shared `stdb-version.toml` that both projects source. Or — since synthex-v2's `sidecar-stdb` feature is not yet activated — defer synthex-v2's STDB usage until habitat-stdb is running, then match versions. Document in both CLAUDE.md files.

---

## N — Nice-to-have (flag but don't block execution)

### N1. No Obsidian vault mirror for the STDB module schema

The plan persists at 2 surfaces (shared-context canonical + Obsidian summary). The STDB module's table schemas should also be documented in an Obsidian note (`[[SpaceTimeDB Module Schema]]`) with wikilinks to each service whose data it consolidates, completing the 4-surface persistence pattern.

### N2. No `spacetime dev` hot-reload integration during development

STDB CLI has `spacetime dev` for hot-reload during development. The plan doesn't mention using it. During Phase A development, `spacetime dev --module-path module/` would auto-republish on file save — faster iteration than manual `spacetime publish`.

### N3. Scheduled reducer interval not specified for gradient capture

R3 `capture_gradient` is "called every 60s by scheduled reducer" but no `DecaySchedule`-like schedule table is defined for it. Needs a `GradientSchedule` table with `ScheduleAt::interval(60_000_000)` (60s in microseconds).

### N4. No token/cost tracking for Watcher observations

T8 `WatcherObservation` carries `cost_cents` from the synthex-v2 schema. The plan doesn't aggregate or report API cost. A `SUM(cost_cents)` query in the bootstrap payload would give session-level LLM cost visibility for free.

### N5. Bootstrap payload doesn't include L0 (The Ember) or L5 (CLI muscle)

The plan positions STDB injection as replacing L0-L6 + adding L7-L10. But L0 (The Ember identity) is sourced from atuin KV, and L5 (CLI muscle — 82 atuin scripts) is sourced from atuin's script registry. Neither is migrated to STDB. The injection payload should either include these from atuin (keeping the existing mechanism) or explicitly state that L0 and L5 remain atuin-sourced. Currently ambiguous.

---

## Bundled plan amendments

If all C + I recommendations are adopted:

| Change | LOC | Time | Impact |
|---|---|---|---|
| C1: Replace view with procedure-based query | ~20 | +1h | Correctness |
| C2: Split into 3 workspaces | 0 | +0.5h | Build sanity |
| C3: Add R7 retention/compaction reducer + schedule | ~80 | +3h | Memory safety |
| C4: Define 5 causal linkage rules + ORAC `triggered_by_tick` | ~50 (ingester) + 5 (ORAC) | +4h | Core differentiator |
| C5: Migration verification checksums | ~30 | +1h | Data integrity |
| I1: Phase A pre-flight build test | 0 | +0.5h | Risk de-risking |
| I2: Injector uses `spacetime sql` CLI not SDK | -50 (simpler) | -1h | Simplification |
| I3: Compound indexes on KnowledgeEdge | ~10 | +0.5h | Query perf |
| I4: Kill-recovery + reconnection tests | 0 | +1h | Resilience |
| I5: Ingester devenv registration | ~10 | +0.25h | Visibility |
| I6: VMS out-of-scope documentation | ~5 | +0.1h | Completeness |
| I7: STDB version pinning strategy | ~5 | +0.25h | Compatibility |
| **Total delta** | **~+100 LOC, -50 LOC** | **~+10h** | |

**Revised total: ~50-60h across 10-12 sessions (was 40-50h / 8-10).**

---

## Revised critical path with amendments

| Session | Work | Notes |
|---|---|---|
| S110 | **Phase A pre-flight** (30min: build STDB, verify publish, verify SQL) + begin Phase A deploy | Gate: STDB runs on this machine |
| S111 | Phase A: core tables (T1,T3,T4,T6) + ingester polling ORAC/PV2/SYNTHEX | Gate: events appearing in STDB |
| S112 | Phase B: T2 KnowledgeEdge + POVM migration + verification checksums | Gate: count/weight aggregates match source |
| S113 | Phase B: remaining SQLite migrations (T5,T7) + decay schedule + compound indexes | Gate: `knowledge_edge` count ≥ 3922 |
| S114 | Phase C: causal linkage rules + ORAC `triggered_by_tick` patch + T8 Watcher + forget cascade | Gate: `causal_parent IS NOT NULL` rows exist |
| S115 | Phase D: ORAC hook integration + PV2 bus subscription + ingester health endpoint + devenv registration | Gate: full round-trip verified |
| S116 | Phase D: Telegram `/query` + Obsidian timeline + retention/compaction reducer | Gate: 30-day projection under 1GB |
| S117 | Phase E: injector CLI (via `spacetime sql` CLI) + ORAC SessionStart integration | Gate: <100ms injection in new session |
| S118 | Phase E: dead DB cleanup + fallback verification + E2E test script | Gate: 11 DBs deleted, old bootstrap still works |
| **Buffer S119** | Any overrun + stress test | |

---

## What this gap analysis is NOT

Not an indictment. The plan is the strongest STDB integration design I've seen for this stack. The schema audit was thorough, the 5-pattern consolidation is real, and the causal_parent concept is genuinely novel in this ecosystem. Most gaps are additive (retention, verification, indexes) rather than corrective. C1 (view limitation) and C4 (causal wiring) are the two that would cause visible failure at demo time — everything else is hardening.

**Recommended minimum set to adopt:** C1, C2, C3, C4, I1, I2. These are the items where skipping them leads to build failure (C2), runtime crash (C3), broken core feature (C1, C4), or late discovery of a showstopper (I1). Total cost: ~+8h. Worth it.

---

*Gap analysis authored S109 · 2026-04-24 · 17 gaps (5C + 7I + 5N) · Recommended minimum: C1-C4 + I1-I2 (+8h) · Full adoption: +10h revised total ~50-60h*
