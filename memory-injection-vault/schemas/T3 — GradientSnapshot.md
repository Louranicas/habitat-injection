> Back to: [[HOME]]

# T3 — GradientSnapshot

**Time-series of Habitat vital signs.** Enables trajectory analysis (L7 in bootstrap).

## Schema

```rust
#[spacetimedb::table(accessor = gradient_snapshot, public)]
pub struct GradientSnapshot {
    #[primary_key]
    #[auto_inc]
    id: u64,
    source: String,             // "synthex-v1"|"synthex-v2"|"orac-probe"
    
    // Thermal (D0)
    temperature: f64,
    thermal_target: f64,
    thermal_delta: f64,
    
    // PV2 (D1)
    pv2_r: f64,
    pv2_spheres: u32,
    pv2_k_mod: f64,
    
    // RALPH (D2)
    ralph_gen: u64,
    ralph_fitness: f64,
    ralph_phase: String,
    
    // Hebbian (D3-D4)
    ltp_total: u64,
    ltd_total: u64,
    ltp_ltd_ratio: f64,
    
    // POVM (D5)
    povm_pathways: u32,
    povm_memories: u32,
    
    // ME (D6)
    me_health: f64,
    me_fitness: f64,
    
    // Heat sources
    hs_001_hebbian: f64,
    hs_002_cascade: f64,
    hs_003_resonance: f64,
    
    // Flow state (D10)
    flow_state: f64,
    
    // Derived
    is_healthy: bool,
    system_grade: String,
    
    // NA-R6: service self-reported health
    orac_system_grade: Option<String>,
    pv2_fleet_mode: Option<String>,
    synthex_pid_converging: Option<bool>,
    me_overall_health: Option<f64>,
    
    session_id: Option<String>,
    
    #[index(btree)]
    timestamp: spacetimedb::Timestamp,
}
```

## Capture Rate

1 snapshot/minute = 1,440/day. Requires downsampling per [[Gap Analysis — Conventional#C3]]:
- >7 days old → 1/hour
- >30 days old → 1/day

## Written by

[[Reducers#R3 capture_gradient]] (scheduled every 60s)

---

See: [[T4 — SessionRecord]] · [[Phase A — STDB Deploy]]
