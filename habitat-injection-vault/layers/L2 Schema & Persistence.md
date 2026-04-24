> Back to: [[HOME]] · [[MASTER INDEX]] · [[Architecture Overview]]

# L2 Schema & Persistence

> **Path:** `src/m2_schema/` | **Modules:** 6 | **Dependencies:** [[L1 Foundation]]

SQLite schema creation, migration runner, 6 table definitions, CRUD operations, query builders, and index management.

---

## Tables

| Table | Key Fields | Purpose |
|-------|-----------|---------|
| `causal_chain` | label, reinforcement_count, resolved_session, consent | Bug/trap tracking with Hebbian reinforcement |
| `session_trajectory` | session_id, fitness, field_r, thermal_t, delta_summary | Per-session vital signs |
| `workstream` | name, status, blocker, last_touched_session | In-flight work ledger |
| `reinforced_pattern` | label, weight, hit_count, last_fired_session | Hebbian-weighted patterns |
| `injection_cache` | section, content, token_count, computed_at | Pre-rendered injection payload |
| `session_checkpoint` | session_id, label, frontmatter JSON, bullets | /save-session structured records |

---

## Modules

### m06_schema (`m06_schema.rs`)
CREATE TABLE + CREATE INDEX statements for all 6 tables. Schema version tracking. Migration runner (up/down). Idempotent — safe to call on existing DB.

### m07_causal_chain (`m07_causal_chain.rs`)
CRUD for `causal_chain`: insert, resolve, reinforce (increment count), find_unresolved (ORDER BY reinforcement_count DESC), find_by_label, auto_resolve_stale.

### m08_trajectory (`m08_trajectory.rs`)
CRUD for `session_trajectory`: insert_point, get_recent(n), compute_delta (fitness diff from previous), get_trend (slope across last N).

### m09_workstream (`m09_workstream.rs`)
CRUD for `workstream`: insert, update_status, set/clear_blocker, get_active, get_blocked, touch (update last_touched_session).

### m10_pattern (`m10_pattern.rs`)
CRUD for `reinforced_pattern`: insert, reinforce (`weight += 0.1*(1-weight)`, `hit_count++`), decay_all (`weight *= 0.95` where unfired), prune_weak (DELETE where `weight < 0.05`), get_top_by_weight.

### m10b_checkpoint (`m10b_checkpoint.rs`)
CRUD for `session_checkpoint`: parse_checkpoint_file (markdown + YAML frontmatter from `shared-context/sessions/*.md`), insert, get_latest, get_by_session, get_by_label, extract_causal_chains (auto-discovers BUG-* and trap references).

---

## Spec
See `ai_specs/layers/L2_SCHEMA_&_PERSISTENCE_SPEC.md` for implementation details.
