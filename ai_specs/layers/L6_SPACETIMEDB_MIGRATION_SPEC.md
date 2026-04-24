# L6_SPACETIMEDB_MIGRATION — Implementation Spec

Phase 2: STDB module (5 tables in Rust), ingester binary (ORAC/PV2/SYNTHEX/POVM bridges), cascade_forget reducer, watcher_digest table, schedule-table reducers for injection_cache rebuild..

## Rationale

Deferred by Adversary consensus — only ships when Watcher needs real-time subscriptions or cascade_forget justifies STDB's operational cost.

## Modules

- `m22_stdb_module` — SpaceTimeDB WASM module: 5 table definitions (Rust #[spacetimedb::table]), 6 reducers (ingest_event, reinforce_edge, capture_gradient, register_session, run_decay, forget_sphere), schedule tables for decay + cache rebuild. Compiles to wasm32-unknown-unknown.
- `m23_ingester` — Multi-source ingester binary: polls ORAC (30s), subscribes PV2 /bus/ws, polls SYNTHEX (60s), syncs POVM (300s). Calls STDB reducers via SDK. Exposes health on :3001. Long-running tokio process.
- `m24_migration` — One-shot SQLite → STDB migration. Reads all 5 SQLite tables, calls STDB reducers to populate. Verification checksums (count + weight aggregates). Rollback on mismatch.

## Dependencies

Depends on: L1, L2, L3, L4

## Constraints

- should: 50+ tests per module
- must: No `unwrap()`/`expect()` outside tests
- must: Quality gate after every module
