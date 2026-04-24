> Back to: [[HOME]]

# Phase A — STDB Deploy + Core Tables (6-8h, 1-2 sessions)

## Pre-flight (30 min, per [[Gap Analysis — Conventional#I1]])
1. Build STDB standalone from `~/claude-code-workspace/spacetimedb/` or install via `curl -sSf https://install.spacetimedb.com | sh`
2. `spacetimedb-standalone start --listen-addr 127.0.0.1:3000` — verify starts
3. `spacetime publish test "SELECT 1"` — verify publish + SQL
4. Kill-9 and restart — verify WAL recovery

## Deliverables
- Self-hosted STDB standalone on `:3000` via devenv
- Module with [[T1 — HabitatEvent]], [[T3 — GradientSnapshot]], [[T4 — SessionRecord]], [[T6 — ServiceHealth]]
- Basic [[Ingester Pipeline]] polling ORAC + PV2 + SYNTHEX
- [[Reducers]]: R1 (ingest_event), R3 (capture_gradient), R4 (register_session)
- Migrate synthex-v2 gradient_snapshot.db (1 row) + bridge_health.db (9 rows)

## Acceptance
- `spacetime sql habitat "SELECT COUNT(*) FROM habitat_event"` returns >0 after 5 min
- `spacetime logs habitat` shows reducer invocations
- devenv health check passes on `:3000`
- After kill-9 + restart, last 10 events still queryable

## Dependencies
None. Phase A is independently valuable.

---

See: [[Phase B — Knowledge Graph Migration]] · [[Sidecar Architecture]]
