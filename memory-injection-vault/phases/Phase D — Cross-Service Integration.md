> Back to: [[HOME]]

# Phase D — Cross-Service Integration (8-10h, 2 sessions)

## Deliverables
- **ORAC bridge:** SessionStart → R4, Stop → close session, PostToolUse → increment counters
- **PV2 bus:** ingester subscribes to `/bus/ws` with `client_id = "habitat-stdb-ingester"`
- **Atuin:** command events via PV2 bus → ingester → STDB
- **Telegram:** `/query` command routes to STDB SQL via ingester HTTP proxy
- **Obsidian:** Session Timeline view queries STDB via HTTP
- **Ingester health:** `/health` + `/metrics` on `:3001`, registered in devenv
- **NA-R2 consent:** consent_state column on T1/T2, ingester checks consent before ingest
- **NA-R3 reciprocal:** STDB → ORAC trajectory hints, STDB → SYNTHEX patterns, STDB → PV2 coupling history
- **Retention reducer:** [[Reducers#R7 compact_old_events]] (30d envelope, 90d delete, snapshot downsample)

## Acceptance
- Full round-trip: command → Atuin → PV2 bus → ingester → STDB → queryable
- Telegram `/query "why did fitness drop?"` → causal chain response
- Obsidian Session Timeline renders STDB data
- Ingester reconnects within 30s after STDB restart with zero event loss
- 30-day projection under 1GB memory

---

See: [[Phase E — Bootstrap Revolution]] · [[Comms Layer v3 Alignment]]
