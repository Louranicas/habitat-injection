# L2_SCHEMA_&_PERSISTENCE — Implementation Spec

SQLite schema creation, migration runner, 6 table definitions (causal_chain, session_trajectory, workstream, reinforced_pattern, injection_cache, session_checkpoint), CRUD operations, query builders, index management.

## Rationale

Schema before logic — tables must exist before anything reads or writes them. Phase 2: STDB module definitions mirror these exactly.

## Modules

- `m06_schema` — CREATE TABLE statements for all 6 tables. CREATE INDEX statements. Schema version tracking (currently v2). Migration runner (up/down with `column_exists` idempotency guards). Idempotent — safe to call on existing DB. PRAGMA configuration (WAL, busy_timeout, foreign_keys).
- `m07_causal_chain` — CRUD for causal_chain table. insert_chain, resolve_chain, reinforce_chain (increment count + auto-seed), find_unresolved (ORDER BY reinforcement_count DESC), find_by_label, auto_resolve_stale, count_unresolved. The key table from Historian.
- `m08_trajectory` — CRUD for session_trajectory table. insert_point, get_recent(n), compute_delta (fitness diff from previous), get_trend (OLS slope across last N), get_by_session, count. Practitioner's universal table.
- `m09_workstream` — CRUD for workstream table. insert_workstream, update_status, set_blocker, clear_blocker, get_active, get_blocked, touch, get_by_id, update_progress, count_by_status. Historian + Practitioner.
- `m10b_checkpoint` — CRUD for session_checkpoint table. insert_checkpoint (via CheckpointInsert builder), get_by_label, get_by_session, get_recent, get_latest, count. JSON array columns (accomplished, in_progress, blocked, key_findings) with serde round-trip. Consent enum with FromStr/Display.
- `m10_pattern` — CRUD for reinforced_pattern table. insert_pattern, reinforce (weight += 0.1*(1-weight), hit_count++), decay_all (weight *= rate where fired — i.e. last_fired_session IS NOT NULL), prune_weak (DELETE where weight < threshold), get_top_by_weight, get_by_id, get_by_category, count. Memory Scientist + Performance Engineer.

## Dependencies

Depends on: L1

## Constraints

- should: 50+ tests per module
- must: No `unwrap()`/`expect()` outside tests
- must: Quality gate after every module
