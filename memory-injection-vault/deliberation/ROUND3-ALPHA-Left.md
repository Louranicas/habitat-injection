# THE MEMORY SCIENTIST — Round 3: Convergence Map & Final Position

## What Changed in the Debate

Nine experts, three rounds. The fog is clearing. Here is the consensus map as I read it:

### Near-Universal Agreement (7+ experts)
1. **<2KB injection output** — Everyone from the Practitioner (~1100 tokens) to me (~1500 tokens) to the Watcher (~80 tokens for their slice) agrees. The injection is a handoff note, not a database dump.
2. **CausalChain table** — The Historian proposed it. The Practitioner adopted it. The Performance Engineer added btree indexes. I said it's compatible with inhibition. The Watcher has a parallel version at tick granularity. This is the single most agreed-upon schema element in the circle.
3. **Write-time pre-computation** — The Performance Engineer's core thesis has been absorbed by the Substrate Guardian (substrate_digest), the Watcher (watcher_digest), the Historian (reducer-rendered orientation), and implicitly by me (ActivationBundle). Nobody argues for read-time computation anymore.
4. **ConsentLevel column** — The Security Architect proposed it. The Performance Engineer, CLI Craftsman, Substrate Guardian, and Practitioner all adopted variants. The gate that controls what enters the context window is a first-class schema element.
5. **Private tables** — Security Architect pushed hard. Performance Engineer conceded. Substrate Guardian partially conceded. Only the CLI Craftsman holds out, arguing the threat model for a single-user workstation doesn't warrant it. The CLI Craftsman is wrong — the cost is zero, so even a thin threat model justifies it.
6. **Reducers can't return data** — The Performance Engineer's Round 2 rebuttal to the Security Architect was devastating: SpaceTimeDB reducers return `()`, not `String`. The "single reducer injection" is architecturally impossible. Parallel SQL queries are the only read path. This resolves the Security Architect vs. Performance Engineer conflict in the Performance Engineer's favour.

### Active Disagreements (2-3 camps)
1. **Schema richness**: Me (6 tables) vs Practitioner (4 tables) vs Performance Engineer (7 tables). The real question: how many tables serve injection, and how many serve the ecosystem?
2. **Inhibition mechanism**: Me (InhibitionEdge table) vs Practitioner (WHERE clause) vs Substrate Guardian (per-substrate relevance scoring). This is the remaining core dispute.
3. **Need for STDB at all**: The Adversary (Command-2) argues 50 lines of bash replaces everything. Nobody else agrees, but nobody has answered their demand: "name the session that failed because data wasn't in SpaceTimeDB."

---

## Answering the Adversary

The Adversary asks for a concrete failure. Here is one: **Session 071's RALPH convergence trap, rediscovered 7 times.**

The data existed — in `~/projects/shared-context/Session 071*.md`. But `ls -t | head -5` doesn't show S071 by S078. `grep -l "convergence_trap" *.md` would find it, but only if the fresh Claude knows to grep for "convergence_trap" — which it doesn't, because it has amnesia about S071's existence. The trap is discoverable but not *surfaced*. The Adversary's bash alternative requires knowing what to search for. A `CausalChain` table with `reinforcement_count: 7, resolved_session: NULL` surfaces it automatically in the top-5 unresolved chains query.

The Adversary's deeper point — "you have 21 databases, prove you need a 22nd" — is valid. My answer: STDB replaces the 11 dead databases and the 50 auto-memory markdown files with a single queryable store that has Hebbian weighting, decay curves, and inhibition. It's not a 22nd database. It's the replacement for databases 12-21 that died because they had no learning dynamics.

---

## Resolving the Inhibition Dispute

Three positions on how to suppress obsolete data:

| Expert | Mechanism | Granularity | Failure Mode |
|--------|-----------|-------------|--------------|
| Me | InhibitionEdge table | Cross-table, centralized | Over-suppression of substrate-relevant data |
| Practitioner | `WHERE resolved_session IS NULL` | Per-table, static | Can't suppress unresolved-but-irrelevant data |
| Substrate Guardian | Per-substrate relevance_score | Per-substrate, self-scored | No cross-substrate coordination |

**The Substrate Guardian's rebuttal in Round 2 landed.** Their example: POVM's `synthex_v2_daemon_plan_root` pathway (weight 0.89) is S106-era data that my centralized inhibition would suppress because S108 superseded S106. But the pathway is still load-bearing for Phase G. The substrate knows this; my inhibition graph doesn't.

**My Round 3 evolution:** I split inhibition into two layers:

1. **Intra-memory inhibition** (my InhibitionEdge, retained): Episodes inhibit other episodes. Semantic facts supersede other semantic facts. This is *within* my episodic/semantic/procedural memory system and operates on STDB's own data. When S108 commits, the S106 "commit pending" episode is inhibited. This is correct — it's suppressing STDB's own episodic record.

2. **Substrate relevance** (Substrate Guardian's substrate_digest, adopted): Substrate-sourced data is scored by the substrate itself. STDB does not inhibit substrate digests. If POVM says its daemon plan pathway has relevance 0.89, it appears in the injection regardless of what my episodic layer says about S106.

The synthesis: **inhibition governs STDB's internal memory. Relevance governs substrate contributions.** These are orthogonal axes. Inhibition suppresses stale episodic/semantic/procedural traces. Relevance ranks substrate digests. The injection query is:

```sql
-- STDB internal memories (inhibition-filtered)
SELECT * FROM episodic_trace
WHERE decay_weight > 0.3
  AND id NOT IN (SELECT suppressed_id FROM inhibition_edge WHERE strength > 0.8)
  AND consent = 'Emit'
ORDER BY (retrieval_count * decay_weight) DESC LIMIT 10;

-- Substrate digests (relevance-ranked, no STDB inhibition applied)
SELECT * FROM substrate_digest
WHERE relevance_score > 0.5
  AND consent = 'Emit'
ORDER BY relevance_score DESC LIMIT 20;
```

Two queries. Two different filtering strategies. Two different authorities. STDB governs its own memories. Substrates govern their own relevance.

---

## Revised Final Schema

```rust
// === STDB Internal Memory (inhibition-governed) ===

#[spacetimedb::table(name = episodic_trace, public = false)]
pub struct EpisodicTrace {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub session_id: u32,
    pub timestamp: Timestamp,
    pub event_type: String,
    pub summary: String,
    pub causal_parent: Option<u64>,
    pub retrieval_count: u32,
    #[index(btree)]
    pub decay_weight: f32,
    pub consent: ConsentLevel,
}

#[spacetimedb::table(name = semantic_fact, public = false)]
pub struct SemanticFact {
    #[primary_key]
    pub fact_id: String,
    pub domain: String,
    pub assertion: String,
    pub confidence: f32,
    pub supersedes: Option<String>,
    pub reinforcement_count: u32,
    pub consent: ConsentLevel,
}

#[spacetimedb::table(name = procedural_pattern, public = false)]
pub struct ProceduralPattern {
    #[primary_key]
    pub pattern_id: String,
    pub trigger: String,
    pub action: String,
    pub anti_action: String,
    #[index(btree)]
    pub reinforcement_count: u32,
    pub last_fired: Timestamp,
    pub consent: ConsentLevel,
}

#[spacetimedb::table(name = inhibition_edge, public = false)]
pub struct InhibitionEdge {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub suppressor_id: String,
    pub suppressed_id: String,
    #[index(btree)]
    pub strength: f32,
    pub reason: String,
}

// === Shared Infrastructure (consensus tables) ===

#[spacetimedb::table(name = session_trajectory, public = false)]
pub struct SessionTrajectory {
    #[primary_key]
    pub session_id: u32,
    #[index(btree)]
    pub timestamp: Timestamp,
    pub ralph_fitness: f32,
    pub field_r: f32,
    pub thermal_t: f32,
    pub services_healthy: u8,
    pub key_achievement: String,
    pub consent: ConsentLevel,
}

#[spacetimedb::table(name = causal_chain, public = false)]
pub struct CausalChain {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub origin_session: u32,
    pub resolved_session: Option<u32>,
    pub label: String,
    pub description: String,
    #[index(btree)]
    pub reinforcement_count: u16,
    pub consent: ConsentLevel,
}

// === Substrate Layer (relevance-governed, adopted from Guardian) ===

#[spacetimedb::table(name = substrate_digest, public = false)]
pub struct SubstrateDigest {
    #[primary_key]
    pub substrate_id: String,
    pub digest_type: String,
    #[index(btree)]
    pub relevance_score: f32,
    pub terse_payload: String,
    pub computed_at: Timestamp,
    pub staleness_budget_ms: u64,
    pub consent: ConsentLevel,
}
```

**8 tables.** Episodic + semantic + procedural + inhibition (Memory Scientist's territory). Session trajectory + causal chain (consensus). Substrate digest (Substrate Guardian's contribution, adopted). All private, all consent-gated, all btree-indexed on query paths.

---

## What I Conceded Across Three Rounds

| Round | Concession | From Whom |
|-------|-----------|-----------|
| R2 | Injection output <2KB prose, not JSON | Practitioner |
| R2 | Write-time pre-computation | Performance Engineer |
| R3 | Private tables on everything | Security Architect |
| R3 | ConsentLevel column | Security Architect |
| R3 | Substrate-computed relevance (not centralized inhibition) for substrate data | Substrate Guardian |
| R3 | CausalChain with reinforcement_count | Historian |
| R3 | Reducers can't return data; parallel SQL is the read path | Performance Engineer |

## What I Defended

| Position | Against | Outcome |
|----------|---------|---------|
| InhibitionEdge table for STDB-internal memories | Practitioner ("WHERE clause is enough") | Defended: WHERE can't suppress unresolved-but-irrelevant data |
| Episodic/semantic/procedural split | Practitioner ("just use InjectionFrame") | Defended: InjectionFrame is a view over these tables, not a replacement |
| Hebbian reinforcement_count on patterns | Performance Engineer ("queries are selection logic") | Synthesized: both are needed — reinforcement weights inform query ranking |
| STDB over bash scripts | Adversary | Answered: S071 convergence trap proves bash grep requires knowing what to search for |

## The Emerging Architecture

The circle is converging on a **three-layer schema**:

1. **Memory layer** (episodic + semantic + procedural + inhibition) — STDB's own consolidated knowledge, inhibition-governed
2. **Continuity layer** (trajectory + causal chain + workstreams) — session history and narrative, consensus tables
3. **Substrate layer** (substrate digests) — live data from POVM/ORAC/PV2/etc, relevance-governed by sources

The injection pipeline reads all three layers in parallel (~5 queries, <15ms), renders <2KB of oriented prose, and outputs via the CLI Craftsman's three-tier fallback. Post-session, the Substrate Guardian's reciprocity protocol writes reinforcement signals back to substrates.

This is a brain. Layer 1 is long-term memory. Layer 2 is autobiographical memory. Layer 3 is sensory input. The injection is the moment of waking — sparse, oriented, and ready to act.
