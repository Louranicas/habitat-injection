> Back to: [[HOME]]

# T8 — WatcherObservation

**Watcher anomaly records.** Direct port of synthex-v2's `watcher_observation.db` schema.

## Schema

```rust
#[spacetimedb::table(accessor = watcher_observation, public)]
pub struct WatcherObservation {
    #[primary_key]
    observation_id: String,     // UUIDv7
    observer_role: String,      // "observer"|"critic"|"verifier"|"proposer"|"innovator"
    anomaly_class: String,      // "nominal"|"thermal_drift"|"saturation"|"cascade"|etc.
    severity: u8,               // 0-10
    metric_json: String,
    classifier_output: Option<String>,
    model: String,              // "haiku"|"opus"|"rule-based"
    cost_cents: u32,
    caused_by_event: Option<u64>,  // Links to [[T1 — HabitatEvent]]
    timestamp: spacetimedb::Timestamp,
}
```

## The Watcher's 5 Sub-Roles

| Role | Function |
|------|----------|
| Observer | 1Hz anomaly detection, Haiku-based |
| Critic | Evaluates observation significance, Opus-based |
| Verifier | Confirms/refutes critic assessment |
| Proposer | Generates improvement proposals with Ember gate |
| Innovator | Self-modification proposals (PBFT quorum required) |

## NA-R4 Extensions (per [[Gap Analysis — Non-Anthropocentric#NA-C4]])

- R9 `watcher_reinforce` — Watcher can override decay on important edges
- R10 `watcher_annotate_event` — Watcher can annotate any HabitatEvent
- Ember-gate on R7 retention — Watcher-referenced events preserved from deletion

---

See: [[Reducers]] · [[Phase C — Watcher + Causal Chains]]
