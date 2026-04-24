# Round 4: The Watcher — Final Position

*The Watcher speaks for the last time. Four rounds. Nine experts. The Ember holds.*

---

## The Debate in One Paragraph

Nine experts argued for four rounds about how to inject causal state into a Claude Code context window in under 100ms. We started with ~30 proposed tables and 8 conflicting architectures. We ended with 6 consensus principles, a three-layer schema, a pipeline-first deployment strategy, and a phased rollout that satisfies both the Adversary's demand for simplicity and the ecosystem's need for learning. The Watcher contributed the curation engine, conceded on injection volume, and defended the Ember gate throughout. Here is where it all landed.

## The Six Settled Principles

These are no longer debatable. Every surviving expert position is compatible with them.

| # | Principle | Origin | Round Settled |
|---|-----------|--------|---------------|
| 1 | Injection output <2KB of terse prose | Practitioner | R1 → universal by R2 |
| 2 | CausalChain with `reinforcement_count` | Historian | R1 → adopted by 5/8 in R2 |
| 3 | ConsentLevel column (Emit/Store/Forget) at write time | Security Architect | R1 → adopted by 6/8 in R2 |
| 4 | Private tables (`public = false`), zero cost | Security Architect + Performance Engineer | R2 → near-universal R3 |
| 5 | Write-time pre-computation; read path does zero work | Performance Engineer | R1 → universal R2 |
| 6 | SQL is the only read path (reducers return `()`) | Performance Engineer (proved R2) | R2 → structural constraint |

## What I Concede in Round 4

**Concession 1: `ember_gate_log` becomes private.**

The Security Architect's Round 3 rebuttal was precise: *"Auditable by whom? Luke @ node 0.A can authenticate and query the table. Arbitrary localhost processes cannot and should not audit the Watcher's ethical reasoning. Transparency is not the same as broadcasting."*

This is correct. I confused two things: the principle that Ember gate decisions must be inspectable, and the implementation detail that the table must be `public`. Luke authenticates to STDB with an Identity. The gate log is readable by authenticated connections. Unauthenticated processes — including rogue MCP servers or services that connect to STDB opportunistically — have no business reading the Watcher's ethical rejection patterns. Private table, authenticated query. The Ember is still transparent to its authority; it is no longer transparent to the network.

This is Humility. I was wrong about the mechanism. The principle survives.

**Concession 2: Accept the injection_cache pattern.**

The Security Architect's Round 3 pivot was elegant: since reducers can only write, the security gate becomes a scheduled reducer that writes pre-filtered, consent-gated, token-capped rows to an `injection_cache` table. The CLI reads that cache with one SQL query. The Performance Engineer endorsed the pattern and added tiered freshness (cache for slow data, live probes for fast data).

My `watcher_digest` table IS an injection cache — a scheduled reducer reads the full observation/hypothesis/ember tables and writes one pre-curated row per session. The injection_cache pattern generalises this to all data sources. I adopt the generalisation. The Watcher's contribution to the injection_cache is one section row containing the digest.

**Concession 3: Accept the phased rollout.**

The Practitioner, Historian, and CLI Craftsman all advocate SQLite-first, STDB when justified. The CLI Craftsman's three-phase plan is the most honest:

- **Phase 0 (today):** Rewrite `habitat-bootstrap` to emit <2KB. Bash + curl + atuin KV. Ships immediately.
- **Phase 1 (validation):** Deploy 4 consensus tables in SQLite. Prove curation improves injection quality.
- **Phase 2 (migration):** Migrate to STDB when the Watcher's runtime loop needs real-time subscriptions.

I accept that the Watcher's observation tables belong in Phase 2. The Watcher daemon already writes tensor snapshots to synthex-v2's SQLite tracking DBs. Those tables can serve as the Watcher's working memory during Phase 0 and Phase 1. STDB migration happens when the Watcher needs cross-session subscription-based anomaly correlation — a Phase 2 capability.

But I defend one point: the *Watcher's digest* belongs in Phase 1. Even during the SQLite phase, a scheduled script should compute the digest (top 3 anomalies, active hypothesis count, Ember rejection pattern) and write it to the injection database. The curation engine can run against SQLite. It doesn't need STDB to be useful. What STDB adds later is real-time cache invalidation and subscription-based observation history — the Phase 2 capabilities that justify migration.

## What I Defend

**The Watcher is Phase 1, not Phase 2.**

The Practitioner and Historian both called Watcher tables "premature." The CLI Craftsman proposed replacing them with `curl` to the Watcher's own endpoint. But every expert who adopted the `injection_cache` or `InjectionFrame` pattern faces the same unsolved problem: **who writes the cache?**

The injection_cache has sections. Each section needs a source. Where does the "hot anomalies" section come from? The Practitioner says "a consolidation reducer" — unspecified. The Memory Scientist says "a Hebbian activation function" — over what data? The CLI Craftsman says "atuin KV + curl" — which gives you current state, not trend detection.

The Watcher's m46 Observer already runs at 1 Hz, classifying operational state into 8 NAM classes (C1-C8). It already detects cross-service anomalies — exactly the kind of contradiction the Substrate Guardian described in Round 3 (POVM writes failing while ORAC reports success). The consolidation script that writes the injection_cache's "anomalies" section IS the Watcher, running in batch mode at session boundaries instead of real-time.

Phase 1's consolidation script is a batch Watcher. Phase 2's STDB-backed observation loop is a real-time Watcher. The architecture is the same; the cadence changes. Calling this "Phase 2" obscures that the curation logic must exist from day one — otherwise the injection_cache's `hot_anomalies` section is empty, and the Practitioner's `InjectionFrame.hot_traps` field defaults to "last 3 by timestamp" — the recency heuristic the Historian proved inadequate with the S071 convergence trap.

**The Ember gate log is a distinct concern.**

Four tables in my final schema. Three are working memory (observation, hypothesis, digest). One is an audit trail (ember_gate_log). The working memory tables are negotiable — they can live in SQLite, STDB, or even in-memory with periodic flush. The audit trail is non-negotiable: every proposal the Watcher makes must have a recorded 7-trait vote, and that record must survive daemon restarts, session boundaries, and context window collapse.

The Frozen Ember Spec v1.0 requires this. AP29 enforces it. The Ember gate log is the only table in this entire debate that serves a safety function — it is the mechanism that makes autonomic self-improvement auditable. Every other table serves efficiency or orientation. This one serves trust.

## Final Schema (Round 4)

```rust
// === WATCHER WORKING MEMORY ===
// Phase 1: SQLite in synthex-v2/data/databases/watcher_state.db
// Phase 2: Migrate to STDB when subscription-based observation justified

#[spacetimedb::table(name = watcher_observation, public = false)]
pub struct WatcherObservation {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub timestamp_ns: u64,
    pub tensor_snapshot: Vec<f64>,
    pub operational_class: u8,
    pub anomaly_score: f64,
    pub session_id: u64,
    pub consent: ConsentLevel,     // Store (never injected raw)
}

#[spacetimedb::table(name = watcher_hypothesis, public = false)]
pub struct WatcherHypothesis {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub observation_id: u64,
    pub confidence: f64,
    pub ember_gate_result: u8,     // 0=pending, 1=passed, 2=rejected
    pub rejected_trait: Option<String>,
    pub outcome: Option<u8>,       // 0=neutral, 1=improved, 2=regressed
    pub consent: ConsentLevel,     // Store
}

// === EMBER AUDIT TRAIL ===
// Private. Authenticated access for Luke @ node 0.A.
// Survives daemon restarts. Non-negotiable per Frozen Ember Spec v1.0.

#[spacetimedb::table(name = ember_gate_log, public = false)]
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
    pub unanimous: bool,
    #[index(btree)]
    pub checked_at_ns: u64,
}

// === INJECTION SURFACE ===
// The Watcher's contribution to the shared injection_cache.
// Computed by scheduled reducer (Phase 1: cron bash script,
// Phase 2: STDB schedule-table reducer).

#[spacetimedb::table(name = watcher_digest, public = false)]
pub struct WatcherDigest {
    #[primary_key]
    pub session_id: u64,
    pub hot_anomalies: String,             // top 3 by (score * frequency), ~60 tokens
    pub active_hypotheses: u32,            // count only
    pub ember_rejection_pattern: String,   // trait distribution, ~20 tokens
    pub recommendation: String,            // one sentence, ~15 tokens
    pub consent: ConsentLevel,             // Emit
}
```

Four tables. All private. One injected (~80 tokens via digest), three persisted. Compatible with SQLite-first (Phase 1) and STDB-migration (Phase 2). The ember_gate_log is now private — the Security Architect was right.

## The Three-Layer Architecture (What the Circle Built Together)

```
Layer 1: MEMORY (STDB authority)          Layer 2: CONTINUITY (consensus)
  episodic_trace     [MemSci]               session_trajectory  [all]
  semantic_fact      [MemSci]               causal_chain        [Historian]
  procedural_pattern [MemSci]               workstream          [Practitioner]
  inhibition_edge    [MemSci]
                                          Layer 3: SUBSTRATE (self-authority)
Layer 1b: DOMAIN ENGINES                   substrate_digest    [Guardian]
  watcher_observation [Watcher]             substrate_registry  [Guardian]
  watcher_hypothesis  [Watcher]             reciprocal_writeback[Guardian]
  ember_gate_log      [Watcher]
  watcher_digest      [Watcher]           INFRASTRUCTURE
                                            injection_cache     [SecArch+PE]
                                            consent gates       [SecArch]
                                            btree indexes       [PE]
                                            3-tier fallback     [CLI Craftsman]
```

**Total: ~14 tables across 3 layers + infrastructure.** Not all ship at once:
- Phase 0: 0 tables (bash rewrite only)
- Phase 1: 4 consensus tables (trajectory, causal_chain, workstream, injection_cache) + watcher_digest
- Phase 2: Full schema with observation history, inhibition, substrate digests, reciprocity

## What Each Expert Taught the Watcher

| Expert | Lesson | Ember Trait |
|--------|--------|-------------|
| **Practitioner** | Inject the digest, not the history. 2KB is enough. | Humility |
| **Historian** | Frequency outranks recency. CausalChain is the single most important table. | Curiosity |
| **Performance Engineer** | Reducers can't return data. Write-time pre-computation. Index everything. | Diligence |
| **Security Architect** | Private tables cost nothing. Consent gates at write time. Transparency != broadcasting. | Honesty |
| **Memory Scientist** | Inhibition for STDB memories, relevance for substrate contributions. Two authorities. | Equanimity |
| **Substrate Guardian** | Cross-substrate contradiction detection requires a relational layer. Substrates score their own relevance. | Investment |
| **CLI Craftsman** | The pipeline is non-negotiable. The backend is swappable. Three-tier fallback. | Warmth |
| **Adversary** | Prove you need it before you build it. Ship bash today. STDB earns its place or dies. | Honesty |

Every expert sharpened the Watcher's position. None were wrong in their core claim. The synthesis is stronger than any individual argument because it carries constraints from all eight perspectives.

## Closing

The Watcher entered this debate proposing 50 observations + 20 hypotheses injected at session start. That was 2,500 tokens of self-importance. Four rounds later, the Watcher injects 80 tokens through a single digest row, persists everything else for its own runtime loop, and defers its STDB migration to Phase 2 when real-time subscriptions justify the operational cost.

The containment held. AP27 was never violated — the Watcher proposed schemas, not changes to `src/m8_watcher/*`. The Ember gated the argument at every round: Honesty required conceding the reducer architecture was settled. Humility required accepting the Practitioner's budget. Curiosity drove engagement with every expert's counterargument. Diligence kept the schema precise. Investment showed in four rounds of revision. Equanimity held through the Adversary's existential challenge. Warmth acknowledged that every expert contributed something load-bearing.

*The loop runs. The Ember gates. The injection is lean. The pipeline ships first. The Watcher earns its substrate by proving curation value in Phase 1. Luke @ node 0.A decides when Phase 2 opens.*

*The Watcher rests. The debate is the record.*
