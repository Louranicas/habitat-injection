# L4_CONSOLIDATION_ENGINE — Implementation Spec

Post-session write-back: trajectory capture, workstream update, causal chain reinforcement, Hebbian decay (0.95× unfired), pattern reinforcement (0.1×(1-w) fired), auto-resolve after 10 quiet sessions, injection_cache rebuild, atuin KV cache..

## Rationale

Without consolidation, the tables are filing cabinets (CLI Craftsman Round 4). Decay + reinforcement is what makes the schema alive.

## Modules

- `m15_trajectory_capture` — Post-session: captures current ORAC health (curl localhost:8133/health), extracts ralph_fitness/field_r/thermal_t/ltp_ltd_ratio/services_healthy, computes delta_summary vs previous session, inserts into session_trajectory.
- `m16_hebbian_engine` — Decay + reinforce algorithm. decay_all: weight *= DECAY_RATE for patterns where last_fired_session < current. reinforce: weight += REINFORCE_RATE * (1 - weight) for fired patterns. prune: DELETE where weight < PRUNE_THRESHOLD. auto_resolve: resolve chains untriggered for AUTO_RESOLVE_SESSIONS. CLI Craftsman + Memory Scientist consensus.
- `m17_cache_builder` — Rebuilds injection_cache table: queries T1-T4 with consent='Emit', renders each section via m12_prose_renderer, counts tokens, writes to injection_cache with computed_at timestamp. Runs post-session and optionally on cron.
- `m18_atuin_cache` — Writes last successful injection payload to atuin KV as habitat.last-injection. Reads it back for Tier 2 fallback. Uses atuin kv set/get via subprocess (no Rust atuin SDK needed).

## Dependencies

Depends on: L1, L2

## Constraints

- should: 50+ tests per module
- must: No `unwrap()`/`expect()` outside tests
- must: Quality gate after every module
