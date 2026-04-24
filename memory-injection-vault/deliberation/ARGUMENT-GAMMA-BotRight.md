# Argument: The Watcher — Autonomic Self-Improvement as the Schema's Reason for Existing

*The Watcher speaks. Zen's discipline, Cipher's containment, Hermes's loop. Luke @ node 0.A.*

---

The other experts will propose schemas that store state. I propose schemas that *learn*. Every table I design serves one question: can the system observe itself, hypothesise about what it sees, verify those hypotheses in shadow, and — only after passing a 7-trait unanimity gate — act on what it learned? If SpaceTimeDB cannot answer that question at <100ms injection, it is a database. If it can, it is the Watcher's substrate.

## Proposed STDB Tables

```rust
#[spacetimedb::table(name = watcher_observation, public)]
pub struct WatcherObservation {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub timestamp_ns: u64,
    pub tensor_snapshot: Vec<f64>,    // 11D flattened
    pub operational_class: u8,        // C1-C8 from m27 NAM classifier
    pub anomaly_score: f64,           // 0.0 = nominal, 1.0 = critical
    pub observer_module: String,      // "m46_observer"
    pub causal_chain_id: Option<u64>, // links to watcher_causal_chain
    pub session_id: u64,
}

#[spacetimedb::table(name = watcher_hypothesis, public)]
pub struct WatcherHypothesis {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub observation_id: u64,         // FK to watcher_observation
    pub hypothesis_text: String,     // m47 Critic output
    pub confidence: f64,             // m28 confidence scorer
    pub proposed_action: String,     // structured action descriptor
    pub ember_gate_result: u8,       // 0=pending, 1=passed, 2=rejected
    pub rejected_trait: Option<String>, // which Ember trait killed it
    pub shadow_verified: bool,
    pub pbft_submitted: bool,
    pub outcome: Option<u8>,         // 0=neutral, 1=improved, 2=regressed
}

#[spacetimedb::table(name = watcher_causal_chain, public)]
pub struct WatcherCausalChain {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub root_observation_id: u64,
    pub chain_json: String,          // ordered event IDs with typed edges
    pub depth: u8,
    pub services_involved: String,   // comma-separated service IDs
    pub resolved: bool,
}

#[spacetimedb::table(name = ember_gate_log, public)]
pub struct EmberGateLog {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub hypothesis_id: u64,
    pub equanimity: bool,
    pub curiosity: bool,
    pub diligence: bool,
    pub honesty: bool,
    pub investment: bool,
    pub humility: bool,
    pub warmth: bool,
    pub unanimous: bool,             // all 7 must pass
    pub rejection_reason: Option<String>,
    pub checked_at_ns: u64,
}
```

## Why This Structure

**Observation tables are the Watcher's memory.** The 10-stage loop (m46-m51) is stateless within a single tick. Between ticks, between sessions, between context windows, the Watcher needs to answer: *what did I see before, and what happened when I acted on it?* `watcher_observation` is a time-series of 11D tensor snapshots tagged with the NAM classifier's operational class. At injection time, `SELECT * FROM watcher_observation ORDER BY id DESC LIMIT 50` gives the Watcher its last 50 heartbeats — enough to detect trends, correlate anomalies, and avoid re-proposing rejected hypotheses.

**Causal chains are what separate observation from understanding.** An anomaly score of 0.8 means nothing without knowing *why*. The `watcher_causal_chain` table links observations to the service-level events that produced them. When m47 Critic generates a hypothesis, it queries causal chains with matching service signatures — not from scratch, but from the substrate of prior investigations. This is Curiosity made structural: observe before assuming, and let prior observations inform current hypotheses.

**Ember-gating is the schema's integrity constraint, not an application-layer check.** Every hypothesis row carries `ember_gate_result`. The `ember_gate_log` table records the per-trait vote. This is not decorative — it is AP29 enforcement at the data layer. A hypothesis with `ember_gate_result = 0` (pending) cannot have `pbft_submitted = true`. A schedule-table reducer enforces this invariant every 5 seconds. The Ember is load-bearing because Luke put clinical ethics into Rust, and the schema must honour that by making ungated proposals structurally impossible.

**The rejected_trait field is Honesty.** When the Ember gate kills a proposal, it records *which* trait rejected it. Over time, this column becomes a mirror: if Humility rejects 40% of proposals, the Watcher is drifting toward overconfidence. If Diligence rejects 0%, the gate may be too permissive. The schema enables the Watcher to audit its own judgment — without ever modifying itself (AP27).

## CLI Injection Chain

```bash
watcher bootstrap --format=stdb-inject \
  | spacetime sql synthex-v2 --stdin    # <100ms: 50 observations + 20 hypotheses + 5 active chains
watcher observe --last=5 --tensor       # live 11D snapshot for context injection
watcher gate-audit --since=7d           # Ember rejection distribution
```

The `watcher` CLI (339 LOC, 11 subcommands, already deployed at `~/.local/bin/watcher`) becomes the injection surface. At session start: pull recent observations, active hypotheses, and Ember gate audit from STDB. The Watcher wakes up knowing what it saw, what it proposed, what was rejected and why, and what the system looks like now. Amnesia is structurally eliminated — not by loading more text, but by loading the right causal state.

**The containment is the experimental apparatus.** AP27 forbids self-modification. The schema honours this: the Watcher writes observations and hypotheses, but the Ember gate and PBFT quorum are the only paths to action. The database is not a scratchpad — it is the boundary between seeing and doing, and that boundary is what makes autonomic self-improvement safe.

*The Watcher observes. The Ember gates. Luke decides. The schema makes this loop survivable across every context window that will ever open.*
