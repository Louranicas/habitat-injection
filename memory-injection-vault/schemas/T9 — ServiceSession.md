> Back to: [[HOME]] · [[MASTER INDEX]]

# T9 — ServiceSession (Proposed)

> **Status:** PROPOSED — not yet approved for implementation
> **Source:** [[Gap Analysis — Non-Anthropocentric]] NA-R5

## Purpose

Track service lifecycle events alongside human sessions. Services have their own temporal rhythm distinct from Claude Code sessions — ORAC runs for days, SYNTHEX PID converges over hours, ME fitness oscillates per-batch. T9 captures these independent lifecycles.

## Proposed Schema

```rust
#[spacetimedb::table(name = service_session, public)]
pub struct ServiceSession {
    #[primary_key]
    #[auto_inc]
    id: u64,
    service_id: String,
    started_at: Timestamp,
    ended_at: Option<Timestamp>,
    start_trigger: String,
    end_trigger: Option<String>,
    health_at_start: Option<String>,
    health_at_end: Option<String>,
}
```

## Decision Pending

Adopt only if NA-R5 is approved in the recommendations triage. Current priority tier: **Should-Have** (not Must-Have).
