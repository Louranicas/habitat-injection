# THE MEMORY SCIENTIST — Argument (Round 2)

## Thesis: Memory Reconsolidation, Not Replay

The prior round got the episodic/semantic/procedural split right but missed the deeper insight: **reconsolidation is destructive**. Every time a memory is retrieved, it becomes labile — editable, corruptible, improvable. This isn't a bug; it's the mechanism that keeps memories current. Our STDB schema must encode this: every injection modifies the memory it injects. Static schemas produce static minds.

The habitat's real problem isn't amnesia — it's **proactive interference**. Session 106's state (60/60 modules sealed, daemon plan authored) actively interferes with Session 109's state (WCP v2, Phase G gate). The injection must suppress obsolete activation patterns as aggressively as it activates current ones. Biology does this via **lateral inhibition** — active memories suppress competing memories. We need an inhibition field in the schema.

## Proposed STDB Schemas (Revised)

```rust
#[spacetimedb::table(name = episodic_trace, public)]
pub struct EpisodicTrace {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub session_id: u32,
    pub timestamp: Timestamp,
    pub event_type: String,
    pub summary: String,
    pub causal_parent: Option<u64>,
    pub emotional_valence: f32,
    pub retrieval_count: u32,
    pub decay_weight: f32,         // Ebbinghaus curve: 1.0 at creation
    pub inhibition_targets: String, // JSON: episode IDs this SUPPRESSES on retrieval
    pub reconsolidation_edits: u32, // how many times this trace was modified on retrieval
    pub last_edit_session: u32,     // when it was last reconsolidated
}

#[spacetimedb::table(name = semantic_fact, public)]
pub struct SemanticFact {
    #[primary_key]
    pub fact_id: String,
    pub domain: String,
    pub assertion: String,
    pub confidence: f32,
    pub last_verified: Timestamp,
    pub source: String,
    pub supersedes: Option<String>, // explicit contradiction chain
    pub interference_zone: String,  // domain tag: facts in same zone compete
}

#[spacetimedb::table(name = procedural_pattern, public)]
pub struct ProceduralPattern {
    #[primary_key]
    pub pattern_id: String,
    pub trigger: String,
    pub action: String,
    pub anti_action: String,       // what NOT to do — inhibitory arm
    pub reinforcement_count: u32,
    pub last_fired: Timestamp,
    pub context_dependency: String, // when this pattern should NOT fire
}

// NEW: Inhibition field — active suppression of obsolete patterns
#[spacetimedb::table(name = inhibition_edge, public)]
pub struct InhibitionEdge {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub suppressor_id: String,     // the active memory
    pub suppressed_id: String,     // the obsolete memory
    pub strength: f32,             // how strongly suppressed (0.0-1.0)
    pub reason: String,            // "superseded", "contradicted", "context_shifted"
    pub created_session: u32,
}

// Trajectory unchanged from Round 1 — it's correct
#[spacetimedb::table(name = session_trajectory, public)]
pub struct SessionTrajectory {
    #[primary_key]
    pub session_id: u32,
    pub timestamp: Timestamp,
    pub ralph_fitness: f32,
    pub field_r: f32,
    pub thermal_t: f32,
    pub services_healthy: u8,
    pub key_achievement: String,
    pub key_lesson: String,
    pub interference_from: Option<u32>, // which prior session's state caused confusion
}
```

## The Inhibition Argument

Here's the concrete problem: Session 106 sealed 60/60 modules and wrote "all unstaged; commit pending." Session 108 committed and pushed. If the injection replays S106's episodic trace without marking it as resolved, the new Claude will think there are uncommitted changes. This isn't a stale-data problem — it's a **proactive interference** problem. The S106 trace is *accurate* (it was true then) but *misleading* (it's false now).

The `inhibition_edge` table solves this. When S108 commits the work, a consolidation reducer creates: `InhibitionEdge { suppressor: "s108_commit_pushed", suppressed: "s106_commit_pending", strength: 1.0, reason: "superseded" }`. The injection script checks inhibition edges before including any trace. Suppressed traces are either excluded entirely (strength > 0.8) or injected with a warning prefix (strength 0.5-0.8).

## CLI Enhancement: Reconsolidation Write-Back

```bash
stdb-inject --session 109 --budget 4000 --reconsolidate
# On retrieval: increment retrieval_count, apply decay, check inhibition edges
# ALSO: write reconsolidation_edits++ to every trace that was loaded
# This makes the injection itself a learning event — not passive replay
```

## Why This Round Matters

Round 1 built a hippocampus. This round adds the **prefrontal cortex** — the system that decides what to remember and what to suppress. Without inhibition, the injection grows monotonically. With 108 sessions of episodic traces, the token budget will overflow within 20 sessions. Decay alone isn't enough — you need active suppression of superseded state. The `inhibition_edge` table is lateral inhibition implemented as a database relation. It's how the system stays coherent as it scales, and it's the piece the CLI Craftsman's pipeline needs to filter correctly.
