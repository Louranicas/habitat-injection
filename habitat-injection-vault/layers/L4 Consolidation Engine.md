> Back to: [[HOME]] · [[MASTER INDEX]] · [[Architecture Overview]]

# L4 Consolidation Engine

> **Path:** `src/m4_consolidation/` | **Modules:** 5 | **Dependencies:** [[L1 Foundation]], [[L2 Schema & Persistence]]

Post-session write-back: harvests /save-session checkpoints, captures trajectory, runs Hebbian decay/reinforce cycle, rebuilds injection cache, syncs atuin KV.

---

## Consolidation Flow

```
Session ends
  -> m15_checkpoint_ingest (parse /save-session checkpoint)
  -> m15b_trajectory_capture (ORAC health -> trajectory table)
  -> m16_hebbian_engine (decay unfired, reinforce fired, prune weak, auto-resolve)
  -> m17_cache_builder (rebuild injection_cache from fresh data)
  -> m18_atuin_cache (write last payload to atuin KV for fallback)
```

---

## Modules

### m15_checkpoint_ingest (`m15_checkpoint_ingest.rs`)
Primary write path. Watches `~/projects/shared-context/sessions/` for new `*.md` files. Parses YAML frontmatter + markdown sections. Maps to:
1. `session_checkpoint` — full structured record
2. `session_trajectory` — extracts fitness/r/thermal from service snapshot
3. `workstream` — extracts in-progress/blocked items
4. `causal_chain` — auto-discovers BUG-NNN, trap names, pattern names
5. `reinforced_pattern` — extracts key findings as pattern candidates

### m15b_trajectory_capture (`m15b_trajectory_capture.rs`)
Fallback when /save-session checkpoint is unavailable. Curls `localhost:8133/health`, extracts `ralph_fitness`, `field_r`, `thermal_t`, `ltp_ltd_ratio`, `services_healthy`. Computes delta vs previous session.

### m16_hebbian_engine (`m16_hebbian_engine.rs`)
The learning algorithm:
- **Decay:** `weight *= 0.95` for patterns where `last_fired_session < current`
- **Reinforce:** `weight += 0.1 * (1 - weight)` for fired patterns
- **Prune:** DELETE where `weight < 0.05`
- **Auto-resolve:** resolve chains untriggered for 10 sessions

See [[Hebbian Learning]] for the full algorithm.

### m17_cache_builder (`m17_cache_builder.rs`)
Rebuilds `injection_cache` table: queries all tables with `consent = 'Emit'`, renders each section via m12_prose_renderer, counts tokens, writes with `computed_at` timestamp. Runs post-session and optionally on cron.

### m18_atuin_cache (`m18_atuin_cache.rs`)
Writes last successful injection payload to atuin KV as `habitat.last-injection`. Reads it back for Tier 2 fallback. Uses `atuin kv set/get` via subprocess.

---

## Spec
See `ai_specs/layers/L4_CONSOLIDATION_ENGINE_SPEC.md` for implementation details.
