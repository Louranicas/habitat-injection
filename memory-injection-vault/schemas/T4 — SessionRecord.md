> Back to: [[HOME]]

# T4 — SessionRecord

**Claude Code session tracking across context windows.**

## Schema

```rust
#[spacetimedb::table(accessor = session_record, public)]
pub struct SessionRecord {
    #[primary_key]
    session_id: String,
    session_number: u32,        // S108, S109, etc.
    #[index(btree)]
    started_at: spacetimedb::Timestamp,
    ended_at: Option<spacetimedb::Timestamp>,
    pane_id: Option<String>,
    tab_name: Option<String>,
    persona: Option<String>,    // "Zen"|"Cipher"|"Watcher"|None
    model: String,              // "opus-4-7"|"sonnet-4-6"
    fitness_start: f64,
    fitness_end: Option<f64>,
    fitness_delta: Option<f64>,
    events_count: u32,
    commits_count: u32,
    tools_used: u32,
    priorities_json: String,
    blockers_json: String,
    status: String,             // "active"|"completed"|"crashed"|"expired"
}
```

## Written by

[[Reducers#R4 register_session]] (called by ORAC SessionStart/Stop hooks)

## NA-R5 Extension

Per [[Gap Analysis — Non-Anthropocentric#NA-C5]], add [[T9 — ServiceSession]] for service lifecycle tracking alongside human sessions.

---

See: [[T5 — Workstream]] · [[Phase A — STDB Deploy]]
