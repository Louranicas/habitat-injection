> Back to: [[HOME]]

# Phase C — Watcher + Causal Chains (6-8h, 1-2 sessions)

## Deliverables
- [[T8 — WatcherObservation]] table
- Causal chain construction via 5 concrete linkage rules (per [[Gap Analysis — Conventional#C4]])
- ORAC patch: add `triggered_by_tick` to emergence event payloads (~5 LOC)
- [[Reducers#R6 forget_sphere]] — NA-P-13 cascade across all STDB tables
- [[Reducers#R9 watcher_reinforce]] + [[Reducers#R10 watcher_annotate_event]] (NA-R4)
- Watcher proposal tracking (watcher_proposal + proposal_verdict)

## Causal Linkage Rules

| Event Type | Causal Parent Source |
|---|---|
| `emergence.detected` | ORAC `triggered_by_tick` → lookup event at that tick |
| `sphere.registered` | SessionStart hook event for this session |
| `thermal.adjustment` | Gradient snapshot that crossed threshold |
| `command.postexec` | `command.preexec` for same command_hash |
| `watcher.observation` | Gradient snapshot that triggered anomaly |

## Acceptance
- `SELECT * FROM habitat_event WHERE causal_parent IS NOT NULL LIMIT 5` returns linked events
- Forget cascade: after `forget_sphere("test")`, zero rows reference that sphere
- Watcher observations from synthex-v2 shadow appear in STDB within 60s

---

See: [[Phase D — Cross-Service Integration]] · [[T1 — HabitatEvent]]
