> Back to: [[HOME]] · [[MASTER INDEX]] · [[Architecture Overview]]

# L6 SpaceTimeDB Migration

> **Path:** `src/m6_stdb/` | **Modules:** 3 | **Dependencies:** [[L1 Foundation]], [[L2 Schema & Persistence]], [[L3 Injection Engine]], [[L4 Consolidation Engine]]
> **Feature gates:** `stdb`, `ingester`
> **Phase:** 2 — ships when justified (Adversary consensus: 20-session kill criteria)

Phase 2 migration from SQLite to SpaceTimeDB for real-time subscriptions, causal chain queries, and cascade_forget.

---

## Modules

### m22_stdb_module (`m22_stdb_module.rs`) — feature: `stdb`
SpaceTimeDB WASM module:
- 5 table definitions (Rust `#[spacetimedb::table]`)
- 6 reducers: `ingest_event`, `reinforce_edge`, `capture_gradient`, `register_session`, `run_decay`, `forget_sphere`
- Schedule tables for decay + cache rebuild
- Compiles to `wasm32-unknown-unknown`

### m23_ingester (`m23_ingester.rs`) — feature: `ingester`
Multi-source ingester binary:
- Polls ORAC (30s interval)
- Subscribes PV2 `/bus/ws`
- Polls SYNTHEX (60s interval)
- Syncs POVM (300s interval)
- Calls STDB reducers via SDK
- Exposes health on `:3001`
- Long-running tokio process

### m24_migration (`m24_migration.rs`) — feature: `stdb`
One-shot SQLite -> STDB migration:
- Reads all 6 SQLite tables
- Calls STDB reducers to populate
- Verification checksums (count + weight aggregates)
- Rollback on mismatch

---

## Kill Criteria (Adversary's demand)

After 20 sessions, evaluate:
1. Zero rediscovered traps
2. Reinforcement count active
3. Injection latency under 100ms
4. Injection size under 2KB

**Action on fail:** revert to SQLite, delete STDB module.

---

## Spec
See `ai_specs/layers/L6_SPACETIMEDB_MIGRATION_SPEC.md` for implementation details.

## SpaceTimeDB Plan Vault
Full STDB integration plan at [[SpaceTimeDB Plan]] (95 notes, 24 Mermaid diagrams).
