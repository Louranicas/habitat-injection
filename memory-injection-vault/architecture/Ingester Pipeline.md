> Back to: [[HOME]]

# Ingester Pipeline

Multi-source Rust binary that bridges Habitat services → STDB.

## Data Flow

```
ORAC :8133    ──poll 30s──┐
PV2 :8132     ──WS /bus/ws─┤
SYNTHEX :8090 ──poll 60s──►  Ingester  ──►  STDB :3000
POVM :8125    ──poll 300s──►  (Rust)         8 tables
Atuin hooks   ──via PV2 bus─┘                6 reducers
```

## Source → Reducer Mapping

| Source | Endpoint | Interval | Calls Reducer |
|--------|----------|----------|---------------|
| ORAC | `/health`, `/emergence`, `/ralph`, `/coupling` | 30s | R1 `ingest_event`, R3 `capture_gradient` |
| PV2 | `/bus/ws` WebSocket | Real-time | R1 `ingest_event` |
| SYNTHEX | `/v3/thermal` | 60s | R3 `capture_gradient` |
| POVM | `/pathways` | 300s | R2 `reinforce_edge` (weight sync) |
| Atuin | Via PV2 bus `command.*` events | Real-time | R1 `ingest_event` |
| ORAC hooks | SessionStart/Stop | On event | R4 `register_session` |

## NA-R3 Reciprocal Paths (per [[Gap Analysis — Non-Anthropocentric#NA-C3]])

| STDB → Service | What | How |
|---|---|---|
| → ORAC | Trajectory-informed mutation hints | POST `/api/ingest` |
| → SYNTHEX | Cross-session thermal patterns | POST `/api/ingest` |
| → PV2 | Historical coupling effectiveness | POST `/bus/events` |

## Ingester devenv Registration

```toml
[[services]]
id = "habitat-stdb-ingester"
command = "habitat-stdb-ingester"
health_check_url = "http://localhost:3001/health"
dependencies = ["habitat-stdb"]  # Batch 2
```

---

See: [[Sidecar Architecture]] · [[Injector — Context Window Bootstrap]] · [[Reducers]]
