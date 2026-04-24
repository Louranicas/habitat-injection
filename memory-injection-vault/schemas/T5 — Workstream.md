> Back to: [[HOME]]

# T5 — Workstream

**In-flight work ledger across all services.** Consolidates V3 workflow_state.db + synthex-v2 workflow_tracking.db.

## Schema

```rust
#[spacetimedb::table(accessor = workstream, public)]
pub struct Workstream {
    #[primary_key]
    id: String,
    #[index(btree)]
    service_name: String,
    goal: String,
    current_tier: u8,           // 1-6
    current_stage: String,
    status: String,             // "in_progress"|"completed"|"blocked"|"deferred"
    confidence: f64,
    session_id: Option<String>,
    blocker_description: Option<String>,
    started_at: spacetimedb::Timestamp,
    updated_at: spacetimedb::Timestamp,
    completed_at: Option<spacetimedb::Timestamp>,
}
```

## Migrates from

- V3 `workflow_state.db` workflows table (4 rows, 6-tier model with GENERATED columns)
- synthex-v2 `workflow_tracking.db` workflows table (2 rows)

## Key query

```sql
SELECT * FROM workstream WHERE status = 'in_progress' ORDER BY updated_at DESC
```

Powers the WORKSTREAMS section of [[Bootstrap Chain — Current vs Target|bootstrap injection payload]].

---

See: [[T6 — ServiceHealth]] · [[Phase B — Knowledge Graph Migration]]
