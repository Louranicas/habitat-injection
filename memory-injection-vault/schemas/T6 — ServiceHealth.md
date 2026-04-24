> Back to: [[HOME]]

# T6 — ServiceHealth

**Service health timeline.** Consolidates bridge_health, service_events, integration_health.

## Schema

```rust
#[spacetimedb::table(accessor = service_health, public)]
pub struct ServiceHealth {
    #[primary_key]
    #[auto_inc]
    id: u64,
    #[index(btree)]
    service_id: String,
    port: u16,
    health_status: String,      // "healthy"|"unhealthy"|"unknown"
    http_code: u16,
    circuit_state: String,      // "closed"|"open"|"half_open"
    successes: u64,
    failures: u64,
    timeouts: u64,
    response_time_ms: f64,
    #[index(btree)]
    timestamp: spacetimedb::Timestamp,
}
```

## Migrates from

- synthex-v2 `bridge_health.db` bridge_status (9 rows — 9 bridge circuits)
- `service_tracking.db` services table (12 rows)
- `system_synergy.db` integration_health (unique pairs)
- `agent_deployment.db` agents (46 rows)

---

See: [[T7 — TrapState]] · [[Phase A — STDB Deploy]]
