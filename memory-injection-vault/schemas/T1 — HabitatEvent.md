> Back to: [[HOME]]

# T1 — HabitatEvent

**The backbone.** Causal event log with `causal_parent` chains.

## Schema

```rust
#[spacetimedb::table(accessor = habitat_event, public)]
pub struct HabitatEvent {
    #[primary_key]
    #[auto_inc]
    id: u64,
    
    event_type: String,         // "emergence.detected", "sphere.registered", etc.
    source_service: String,     // "orac-sidecar", "pane-vortex", "synthex-v2"
    sphere_id: Option<String>,  // Consent-gated per NA-R5
    
    causal_parent: Option<u64>, // Links effect to cause
    
    severity: u8,               // 0-10
    confidence: f64,            // 0.0-1.0
    payload_json: String,       // Event-specific data
    session_id: Option<String>,
    tick: u64,                  // ORAC tick
    
    #[index(btree)]
    timestamp: spacetimedb::Timestamp,
}
```

## Consolidates

- `service_tracking.db` → service_events (27 rows)
- `hebbian_pulse.db` → pulse_events (36 rows), decay_audit_log (676 rows)
- `service_tracking.db` → optimization_events (51 rows)
- PV2 `/bus/events` (real-time via [[Ingester Pipeline]])
- ORAC `/emergence` (polled every 30s)

## Key Queries

- **Causal chain:** `SELECT * FROM habitat_event WHERE id = ? UNION SELECT * FROM habitat_event WHERE causal_parent = ?` (recursive)
- **Recent by type:** `SELECT * FROM habitat_event WHERE event_type LIKE 'emergence.%' ORDER BY timestamp DESC LIMIT 20`
- **Per-session:** `SELECT COUNT(*), event_type FROM habitat_event WHERE session_id = ? GROUP BY event_type`

## Growth Rate

~26,000 events/day at steady state. Requires [[Gap Analysis — Conventional#C3]] retention policy.

## Causal Linkage Rules (from [[Gap Analysis — Conventional#C4]])

| Event Type | Causal Parent Source |
|---|---|
| `emergence.detected` | ORAC tick's thermal/coupling event |
| `sphere.registered` | SessionStart hook event |
| `thermal.adjustment` | Gradient snapshot that crossed threshold |
| `command.postexec` | `command.preexec` for same command_hash |
| `watcher.observation` | Gradient snapshot that triggered detector |

## Written by

[[Reducers#R1 ingest_event]]

---

See: [[T2 — KnowledgeEdge]] · [[Reducers]] · [[Phase A — STDB Deploy]]
