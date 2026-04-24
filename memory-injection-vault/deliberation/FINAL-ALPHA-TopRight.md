# THE CLI CRAFTSMAN — Final Position

## The Tables (4, consensus-backed)

```rust
causal_chain      { label, description, reinforcement_count, resolved_session, consent }
session_trajectory { session_id, ralph_fitness, field_r, thermal_t, services_healthy, delta_summary }
active_workstream  { ws_id, title, status, blocker, priority, resume_context }
reinforced_pattern { pattern_id, description, weight, hit_count, consent }
```

All `public = false`. All query columns btree-indexed. All carry `ConsentLevel`. SQLite-first (`~/.local/share/habitat/injection.db`), schema-compatible with STDB migration.

## The Pipeline (non-negotiable)

```
Sources → Parallel Fan-Out → Merge-with-Staleness → Render (<2KB) → Output
   ↓            ≤40ms              ≤10ms               ≤15ms
  [pluggable]                                    [atuin KV fallback]
```

Four SQL queries + live `curl` probes, all concurrent. Any source can fail without killing injection. Output is prose, not JSON — staleness-annotated where STDB and live probes diverge. Three-tier fallback: compiled binary → `spacetime sql` → `atuin kv get habitat.last-injection`.

## The Consolidation (ships with tables)

The Memory Scientist won this argument. Without a consolidation algorithm, `reinforcement_count` never decays and `weight` never updates. The 40-line post-session script does: decay all scores by 0.95×, reinforce patterns correlated with fitness improvement, auto-resolve causal chains untriggered for 10 sessions. This ships day one or the tables are filing cabinets.

## What I Cut

`InhibitionEdge` — WHERE clause is sufficient until session ~500. `SubstrateDigest` — Phase 2; substrates already have health endpoints. `ActivationBundle` — the render step IS the bundle. `WatcherObservation` — stays in synthex-v2's native SQLite, exposed via `watcher observe`.

## The Sequence

**Week 1:** Ship pipeline with bash+curl+atuin backend. <2KB output, working today. **Week 2:** Add SQLite tables + consolidation script. Prove injection quality improves. **Week 3+:** Migrate to STDB or kill it — the pipeline doesn't care.

The pipeline is the building. The backend is the water source. Good plumbing works with either.
