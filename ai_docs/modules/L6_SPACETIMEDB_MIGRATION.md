# L6_SPACETIMEDB_MIGRATION

Phase 2: STDB module (5 tables in Rust), ingester binary (ORAC/PV2/SYNTHEX/POVM bridges), cascade_forget reducer, watcher_digest table, schedule-table reducers for injection_cache rebuild.

## Modules

- `m22_stdb_module` — SpaceTimeDB WASM module: 5 table definitions (Rust #[spacetimedb::table]), 6 reducers (ingest_event, reinforce_edge, capture_gradient, register_session, run_decay, forget_sphere), schedule tables for decay + cache rebuild. Compiles to wasm32-unknown-unknown.
- `m23_ingester` — Multi-source ingester binary: polls ORAC (30s), subscribes PV2 /bus/ws, polls SYNTHEX (60s), syncs POVM (300s). Calls STDB reducers via SDK. Exposes health on :3001. Long-running tokio process.
- `m24_migration` — One-shot SQLite → STDB migration. Reads all 5 SQLite tables, calls STDB reducers to populate. Verification checksums (count + weight aggregates). Rollback on mismatch.

See `ai_specs/layers/L6_SPACETIMEDB_MIGRATION_SPEC.md` for implementation details.
