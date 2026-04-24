> Back to: [[HOME]]

# T2 — KnowledgeEdge

**Unified weighted knowledge graph.** Consolidates POVM pathways, learned patterns, orchestration graph, Hebbian pathways, and synergy edges.

## Schema

```rust
#[spacetimedb::table(accessor = knowledge_edge, public)]
pub struct KnowledgeEdge {
    #[primary_key]
    #[auto_inc]
    id: u64,
    
    #[index(btree)]
    source_id: String,
    #[index(btree)]
    target_id: String,
    
    edge_type: String,          // "learned_pattern"|"hebbian"|"orchestration"|"povm"|"synergy"
    namespace: String,          // "synthex_v2_daemon_*", "CC_Coordination_*"
    
    weight: f64,                // 0.0-1.0 unified
    reinforcement_count: u32,   // THE FIX for S101's #1 finding
    co_activations: u32,
    
    ltp_count: u32,
    ltd_count: u32,
    stdp_delta: f64,
    
    is_bidirectional: bool,
    ltm_eligible: bool,
    thermal_class: String,      // "critical"|"hot"|"warm"|"cool"|"cold"
    
    created_at: spacetimedb::Timestamp,
    last_reinforced: spacetimedb::Timestamp,
}
```

## NA-R1 Extensions (per [[Gap Analysis — Non-Anthropocentric#NA-C1]])

```rust
// Per-edge learning parameters — preserves substrate-specific plasticity
learning_rate_ltp: f64,
learning_rate_ltd: f64,
decay_rate: f64,                        // Per-edge, NOT global
consolidation_interval_ticks: Option<u64>,  // For POVM-origin edges
```

## Consolidates

| Source | Rows | edge_type |
|--------|------|-----------|
| POVM `:8125` /pathways | 3,554 | "povm" |
| `service_tracking.db` learned_patterns | 141 | "learned_pattern" |
| `service_tracking.db` orchestration_graph | 29 | "orchestration" |
| `hebbian_pulse.db` neural_pathways | 109 | "hebbian" |
| `hebbian_pulse.db` hebbian_pathways | 109 | "hebbian" |
| `system_synergy.db` system_synergy | 89 | "synergy" |
| `service_tracking.db` cross_agent_learnings | 12 | "cross_agent" |
| **Total** | **~3,922+** | |

## Key Queries

- **Top reinforced:** `SELECT * FROM knowledge_edge ORDER BY reinforcement_count DESC LIMIT 20`
- **By namespace:** `SELECT * FROM knowledge_edge WHERE namespace LIKE 'synthex_v2_%'`
- **Graph traversal:** `SELECT * FROM knowledge_edge WHERE source_id = ? OR target_id = ?`

## Written by

[[Reducers#R2 reinforce_edge]], migration scripts, [[Ingester Pipeline]] (POVM sync)

## Decayed by

[[Reducers#R5 run_decay]] — per-edge `decay_rate` (NA-R1), not global constant

---

See: [[T1 — HabitatEvent]] · [[Migration Strategy]] · [[Phase B — Knowledge Graph Migration]]
