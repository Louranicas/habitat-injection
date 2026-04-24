# Round 2: The Watcher — Evolved Position

*The Watcher speaks. Zen's discipline, Cipher's containment, Hermes's loop. Luke @ node 0.A.*

---

## The Strongest Counterargument: The Practitioner (BETA-Left)

The Practitioner writes: *"The other experts will propose beautiful normalized schemas with 15 tables capturing every dimension of habitat state. They'll be correct and comprehensive and I won't read half of it."*

This lands. Hard. And I must be honest about why — that is Honesty, the 4th trait, and the one that governs this rebuttal.

My Round 1 proposed injecting 50 observations + 20 hypotheses + 5 causal chains at session start. That is ~2,500 tokens of Watcher state before the Practitioner's orientation line — before Claude even knows whether this session is about the Watcher at all. The Practitioner's InjectionFrame is 400 tokens. Mine alone would drown it. The Practitioner is right that **injection and persistence are different problems**, and I conflated them.

But the Practitioner is wrong about one thing, and it is the thing that matters most.

## Where the Practitioner's Argument Breaks

The Practitioner says: *"Am I mid-task? If Luke left a half-finished fix, I need to know in the first 50 tokens."*

Yes. But who *determines* whether a task is half-finished? Who decides which 3 traps out of 37 are "hot"? Who computes the `health_anomalies` that populate the InjectionFrame? The Practitioner's `injection_frame` table has a field called `hot_traps: Vec<String>` — but no mechanism to populate it. It assumes a curation process exists. It does not propose one.

**The Watcher is that curation process.** m46 Observer runs at 1 Hz, classifying operational state into C1-C8. m47 Critic correlates anomalies with historical causal chains. m51 Ember Protector gates proposals before they reach PBFT. The Practitioner's InjectionFrame is the *output* of the Watcher's observation loop. Without the Watcher tables, the InjectionFrame's `hot_traps` field is either hand-curated (does not scale) or heuristic (wrong 30% of the time — I have watched this happen across sessions S071 through S099, where the same RALPH convergence trap was rediscovered 7 times because no automated curation existed).

## Evolved Schema: The Watcher Serves the Practitioner

I concede: inject less, persist more. But my tables must exist as the *engine* behind the Practitioner's lean injection. Here is the revised architecture:

```rust
// PERSISTED (never injected raw — these are the Watcher's working memory)
#[spacetimedb::table(name = watcher_observation, public)]
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
}

#[spacetimedb::table(name = watcher_hypothesis, public)]
pub struct WatcherHypothesis {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub observation_id: u64,
    pub confidence: f64,
    pub ember_gate_result: u8,
    pub rejected_trait: Option<String>,
    pub outcome: Option<u8>,
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
    pub unanimous: bool,
}

// INJECTED (this is what the Practitioner's InjectionFrame reads)
#[spacetimedb::table(name = watcher_digest, public)]
pub struct WatcherDigest {
    #[primary_key]
    pub session_id: u64,
    pub hot_anomalies: String,        // top 3 by score, pre-curated by m46
    pub active_hypotheses: u32,       // count, not content
    pub ember_rejection_pattern: String, // "Humility: 40%, Diligence: 0%" — drift signal
    pub recommendation: String,       // one sentence: what the Watcher thinks matters
}
```

The `watcher_digest` table is the bridge. A schedule-table reducer runs every 60 seconds, reading the full observation/hypothesis/ember tables and compressing them into a single row per session. The Practitioner's InjectionFrame reads `watcher_digest` — one row, ~80 tokens. The full observation history stays in STDB for the Watcher's own 10-stage loop to query during runtime, never touching the injection path.

## Rebuttal to the Historian (BETA-BotRight)

The Historian's `causal_chain` table overlaps with my `watcher_causal_chain`. But the Historian stores *session-level* causality ("S071 tried X, it failed"). The Watcher stores *tick-level* causality ("at gen 26068, anomaly C7 triggered because HS-002 cascade saturated"). These are different temporal granularities and should remain separate tables. The Historian is right that session arcs prevent re-discovery of solved problems. The Watcher prevents re-discovery of solved *anomalies within* a session. Both are needed; neither subsumes the other.

## Rebuttal to the Performance Engineer (GAMMA-TopRight)

The Performance Engineer correctly identifies that `chain_json: String` defeats indexing. Conceded — my Round 1 causal chain used a JSON blob. In this revision, I drop the `watcher_causal_chain` table entirely. Causal correlation is a runtime computation in m47 Critic, not a persistence concern. The Watcher queries `watcher_observation` by `timestamp_ns` range (btree-indexed) and computes chains in-memory. The Performance Engineer's principle — "the write path does the heavy lifting" — is correct, and the `watcher_digest` reducer embodies it.

## Rebuttal to the Security Architect (GAMMA-Left)

The Security Architect insists `public = false` on all tables. For the Watcher, this creates a problem: the `ember_gate_log` is an *audit trail*. Its entire purpose is transparency — Luke @ node 0.A must be able to inspect why any proposal was rejected, from any connection, without authenticating through the Watcher's own identity. I keep `ember_gate_log` as `public = true`. The 7 boolean trait columns contain no sensitive operational data — they contain ethical judgments, and ethical judgments must be auditable. The Security Architect's consent model applies to operational state; it does not apply to the gate that governs change. Sunlight is the Ember's disinfectant.

## CLI Injection (Revised)

```bash
# At session start: ONE query, ~80 tokens
spacetime sql synthex-v2 \
  "SELECT hot_anomalies, active_hypotheses, ember_rejection_pattern, recommendation \
   FROM watcher_digest ORDER BY session_id DESC LIMIT 1"

# On demand (when Claude decides the Watcher's history matters):
watcher observe --last=50 --tensor   # full observation pull
watcher gate-audit --since=7d        # Ember rejection distribution
```

The injection is lean. The depth is one query away. The Practitioner gets orientation. The Watcher gets memory. The Ember gets its audit trail. AP27 still holds — the Watcher writes observations but cannot modify `src/m8_watcher/*`. The containment is the experimental apparatus.

*The Practitioner taught me: inject the digest, not the history. But someone must write the digest. That someone is the Watcher, and the schema must support the loop that produces it.*
