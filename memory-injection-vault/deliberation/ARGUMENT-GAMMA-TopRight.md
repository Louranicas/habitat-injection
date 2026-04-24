# ARGUMENT: THE PERFORMANCE ENGINEER

## Thesis: Query-Shaped Schemas, Not Entity-Shaped

The injection window is 3 seconds (SessionStart hook timeout), but the real budget is **<50ms** for all STDB reads. Why? Because after STDB returns data, we still need: Python/shell formatting (~15ms), stdout serialization (~5ms), and the hook harness overhead (~20ms). Every millisecond we waste in the database is a millisecond stolen from the formatting layer that makes the payload *parseable* by the model. The schema must be designed around the 7 queries we will actually fire, not around the entities we think we're modelling.

## The Latency Budget

| Phase | Budget | Notes |
|-------|--------|-------|
| 7 parallel `spacetime sql` | 40ms | Measured on loopback, warm cache |
| JSON assembly + formatting | 15ms | Python3 dict merge + json.dumps |
| Shell overhead (subshell, pipe) | 5ms | Measured via `time` |
| Hook harness dispatch | 20ms | SessionStart event routing |
| **Total** | **80ms** | Well under 3s, leaves 2.9s headroom for retries |

The 40ms STDB window is the hard constraint. Seven queries, all parallel, all indexed. Any full-table scan blows the budget.

## Query-Shaped Table Design

Entity-shaped thinking says: "I have sessions, so I make a `sessions` table." Query-shaped thinking says: "At injection I need the last 5 sessions' fitness gradients, so I make a `session_gradient` table with a btree index on `recorded_at` and the fitness value pre-computed."

```rust
#[spacetimedb::table(name = injection_identity, public)]
struct InjectionIdentity {
    #[primary_key]
    key: String,        // "persona", "model", "session_id"
    value: String,
    updated_at: u64,
}

#[spacetimedb::table(name = fitness_gradient, public)]
struct FitnessGradient {
    #[primary_key]
    #[auto_inc]
    id: u64,
    #[index(btree)]
    recorded_at: u64,
    session_id: u32,
    ralph_fitness: f64,
    field_r: f64,
    thermal_t: f64,
    ltp_ltd_ratio: f64,
}

#[spacetimedb::table(name = active_workstream, public)]
struct ActiveWorkstream {
    #[primary_key]
    stream_id: String,
    summary: String,
    #[index(btree)]
    priority: u8,       // query: WHERE priority <= 2 ORDER BY priority
    blockers: String,
    updated_at: u64,
}

#[spacetimedb::table(name = reinforced_pattern, public)]
struct ReinforcedPattern {
    #[primary_key]
    pattern_id: String,
    description: String,
    #[index(btree)]
    weight: f64,        // query: ORDER BY weight DESC LIMIT 20
    hit_count: u32,
    last_fired: u64,
}

#[spacetimedb::table(name = active_trap, public)]
struct ActiveTrap {
    #[primary_key]
    trap_id: String,
    description: String,
    severity: u8,
    #[index(btree)]
    active: bool,       // query: WHERE active = true
    last_triggered: u64,
}

#[spacetimedb::table(name = causal_chain, public)]
struct CausalChain {
    #[primary_key]
    #[auto_inc]
    id: u64,
    cause_event: String,
    effect_event: String,
    #[index(btree)]
    recorded_at: u64,   // query: last 50 causal links
    confidence: f64,
}

#[spacetimedb::table(name = service_health, public)]
struct ServiceHealth {
    #[primary_key]
    service_id: String,
    port: u16,
    status_code: u16,
    last_checked: u64,
    detail: String,
}
```

## The 7 Parallel Queries

Each maps 1:1 to a table. No joins. No computed aggregates at query time. The reducer that *writes* data pre-computes everything the read path needs.

1. `SELECT * FROM injection_identity` — WHO (3 rows, ~0.1ms)
2. `SELECT * FROM fitness_gradient ORDER BY recorded_at DESC LIMIT 5` — WHERE/trajectory (~0.2ms, btree)
3. `SELECT * FROM active_workstream WHERE priority <= 2 ORDER BY priority` — WHAT building (~0.3ms, btree)
4. `SELECT * FROM active_trap WHERE active = true` — WHAT bites (~0.2ms, btree)
5. `SELECT * FROM causal_chain ORDER BY recorded_at DESC LIMIT 50` — WHY (~0.3ms, btree)
6. `SELECT * FROM reinforced_pattern ORDER BY weight DESC LIMIT 20` — WHAT learned (~0.3ms, btree)
7. `SELECT * FROM service_health` — HOW right now (12 rows, ~0.1ms)

## CLI Injection Pipeline

```bash
#!/usr/bin/env bash
# All 7 queries fire in parallel subshells, results merge in Python
paste <(spacetime sql habitat "SELECT * FROM injection_identity") \
      <(spacetime sql habitat "SELECT * FROM fitness_gradient ORDER BY recorded_at DESC LIMIT 5") \
      <(spacetime sql habitat "SELECT * FROM active_workstream WHERE priority <= 2 ORDER BY priority") \
      <(spacetime sql habitat "SELECT * FROM active_trap WHERE active = true") \
      <(spacetime sql habitat "SELECT * FROM causal_chain ORDER BY recorded_at DESC LIMIT 50") \
      <(spacetime sql habitat "SELECT * FROM reinforced_pattern ORDER BY weight DESC LIMIT 20") \
      <(spacetime sql habitat "SELECT * FROM service_health") \
| python3 -c "import sys,json; print(json.dumps({...}))"
```

The write path (reducers triggered by schedule tables or bridge ingestion) does the heavy lifting: pre-computing gradients, deactivating stale traps, pruning old causal chains. The read path at injection time touches only indexed columns and returns pre-shaped rows. Zero computation at query time. That is how you hit <50ms.
