# THE SUBSTRATE GUARDIAN — Round 3: Convergence Map & Final Position

## The State of the Debate

Three rounds. Nine experts. The circle has done what circles do — ground positions down to load-bearing claims and let the rest fall away. Here is what survived.

### Universal Consensus (no remaining dissent)
1. **<2KB injection output** — The Practitioner won the output budget argument. Nobody contests it.
2. **CausalChain with reinforcement_count** — The Historian's table is in every expert's final schema except the Adversary's. It answers "has this been tried before?" — the single question no other mechanism can.
3. **Write-time pre-computation** — The Performance Engineer proved this in Round 1 and everyone adopted it by Round 2. My `substrate_digest` is a pre-computed, indexed materialized view. The Watcher's `watcher_digest` follows the same pattern. No one proposes read-time computation.
4. **ConsentLevel column** — The Security Architect's contribution, universally adopted. The injection gate is schema-level, not application-level.
5. **Private tables** — Cost is zero. Even the CLI Craftsman, the last holdout, can't argue the performance case against it. The Security Architect wins this one cleanly.
6. **Reducers can't return data** — The Performance Engineer's Round 2 demolition of the Security Architect's `inject_session_context` reducer settled the architecture: parallel SQL queries are the only read path in SpaceTimeDB. This is a hard constraint from the runtime, not a design choice.
7. **Post-session consolidation as the write-back point** — My reciprocity protocol moved off the hot path in Round 2. The CLI Craftsman absorbed it as a `habitat-consolidate` step. Nobody argues for injection-time write-backs.

### Emerging Synthesis (2-3 experts converging)
1. **Three-layer schema** — The Memory Scientist's Round 3 names it explicitly: Memory layer (episodic/semantic/procedural + inhibition), Continuity layer (trajectory/causal chain/workstreams), Substrate layer (digests). I agree with this decomposition. The layers have different authorities — STDB governs layers 1-2, substrates govern layer 3.
2. **Dual filtering: inhibition + relevance** — The Memory Scientist conceded that centralized inhibition should NOT apply to substrate digests. I conceded that per-substrate relevance scoring doesn't handle STDB's internal episodic stale-data problem. The synthesis: inhibition for STDB memories, relevance for substrate contributions. Two orthogonal axes, two different authorities.

### Remaining Disputes
1. **Need for STDB at all** — The Adversary demands proof. Only the Adversary holds this position, but their argument is honest: "50 lines of bash, ships today."
2. **Schema richness** — The Practitioner wants 4 tables. The Performance Engineer wants 7. The Memory Scientist wants 8. I want the substrate layer to be a first-class participant, not an afterthought. The real question is: what's the minimum schema that supports all three layers?

---

## Answering the Adversary

The Adversary's challenge — "name the session that failed" — deserves a substrate-specific answer, not just the Memory Scientist's S071 example.

**Session 99's POVM-write-only trap.** ORAC's `raw_http_post` was returning `Ok(0)` — a success sentinel that masked HTTP failures. This was the structural root of why POVM pathways were being read but never written. Five parallel hunters found it. The fix (F-001) bounced RALPH fitness from 0.468 to 0.602 within 30 seconds of deploy.

Here's what bash can't do: `grep "Ok(0)" ~/projects/shared-context/*.md` returns nothing — the session docs describe the fix, not the raw code pattern. `rg "Ok(0)" orac-sidecar/src/` returns 47 matches across the codebase — far too many to identify the structural root. What identified it was **cross-substrate correlation**: POVM writes failing + ORAC health reporting success + RM heartbeat showing stale data = the `raw_http_post` return value was lying. This correlation required knowing the relationships between substrates — which bridge writes to POVM, which reducer calls `raw_http_post`, which RM fields depend on ORAC's bridge.

A `substrate_digest` table where POVM writes "write success rate: 0%" and ORAC writes "bridge health: OK" makes the contradiction **visible at injection time**. Bash scripts that query each substrate independently can't surface cross-substrate contradictions because they have no shared schema to compare against.

**The Adversary's bash alternative works when each substrate's state is independently interpretable. It fails when substrate states are only meaningful in relation to each other.** STDB is the relational layer that makes cross-substrate reasoning possible.

---

## The Substrate Layer's Final Form

My position has evolved across three rounds. Here is where it landed:

### What I Conceded

| Round | Concession | To Whom | Why They Were Right |
|-------|-----------|---------|-------------------|
| R2 | Replace opaque `payload: String` blob with indexed `substrate_digest` | Performance Engineer | Opaque blobs defeat database query semantics |
| R2 | Make `substrate_plasticity` private | Security Architect | Learning parameters are operational fingerprints |
| R2 | Move consent checks from injection hot path to consolidation cold path | Practitioner (implicit) | Consent at write time, not read time |
| R2 | Move reciprocity off injection hot path | CLI Craftsman | Post-session consolidation has no time budget |
| R3 | Accept InhibitionEdge for STDB-internal memories | Memory Scientist | Episodic data does need centralized suppression of stale traces |
| R3 | Accept CausalChain as consensus table | Historian | Session continuity can't be reconstructed from live endpoints |
| R3 | Accept private tables universally | Security Architect | Zero cost, zero reason not to |

### What I Defended

| Position | Against | Outcome |
|----------|---------|---------|
| Substrates compute their own relevance scores | Memory Scientist (centralized inhibition for all data) | **Won**: Memory Scientist adopted dual-authority model in R3 |
| Reciprocal write-back to substrates | Adversary ("that's just xargs curl") | **Defended**: the queue table ensures write-backs survive session boundaries and track outcomes |
| Substrate-native format in digests | Performance Engineer (normalize everything) | **Synthesized**: digests are pre-formatted strings, not raw JSON blobs — substrate-shaped but query-friendly |
| Provenance in injection (which substrate produced each datum) | Practitioner (flat list of traps) | **Defended**: same token cost, vastly more actionable with substrate attribution |

---

## Revised Final Schema

```rust
// === Substrate Contribution Layer ===

#[spacetimedb::table(name = substrate_digest, public = false)]
pub struct SubstrateDigest {
    #[primary_key]
    pub substrate_id: String,      // "povm", "orac", "pv2", "synthex", "watcher", "rm"
    pub digest_type: String,       // "top_pathways", "fitness_gradient", "thermal_state"
    #[index(btree)]
    pub relevance_score: f32,      // scored BY the substrate, not by STDB
    pub terse_payload: String,     // <200 chars, pre-formatted by substrate's own reducer
    pub computed_at: Timestamp,
    pub staleness_budget_ms: u64,  // substrate declares its own TTL
    pub consent: ConsentLevel,     // set at write time by consolidation reducer
}

#[spacetimedb::table(name = substrate_registry, private)]
pub struct SubstrateRegistry {
    #[primary_key]
    pub substrate_id: String,
    pub ltp_rate: f32,             // substrate-specific learning rate
    pub ltd_rate: f32,             // substrate-specific forgetting rate
    pub consolidation_phase: String,  // "active", "refractory", "sleeping"
    pub min_sample_interval_ms: u64,
    pub consent_level: ConsentLevel,
}

#[spacetimedb::table(name = reciprocal_writeback, public = false)]
pub struct ReciprocalWriteback {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub target_substrate: String,
    pub target_pathway: String,
    pub delta_weight: f32,
    pub reason: String,
    pub queued_at: Timestamp,
    pub applied: bool,
    pub outcome: String,           // "accepted", "rejected", "unreachable"
}
```

Three tables for the substrate layer. `substrate_digest` is public-facing (injected). `substrate_registry` is private (plasticity parameters, only used by consolidation reducer). `reciprocal_writeback` is the post-session queue.

---

## The Three-Layer Architecture (Convergence Position)

The Memory Scientist named it. I endorse it. Here is the full picture with authorities:

```
┌─────────────────────────────────────────────────────────┐
│                  INJECTION (<2KB, <15ms)                  │
│  5 parallel SQL queries → render → three-tier fallback   │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  LAYER 1: MEMORY (STDB authority, inhibition-governed)   │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐   │
│  │ episodic  │ │ semantic │ │procedural│ │inhibition│   │
│  │ _trace    │ │ _fact    │ │ _pattern │ │ _edge    │   │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘   │
│                                                          │
│  LAYER 2: CONTINUITY (consensus, Historian-driven)       │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐                │
│  │ session   │ │ causal   │ │workstream│                │
│  │_trajectory│ │ _chain   │ │          │                │
│  └──────────┘ └──────────┘ └──────────┘                │
│                                                          │
│  LAYER 3: SUBSTRATE (substrate authority, relevance)     │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐                │
│  │substrate │ │substrate │ │reciprocal│                │
│  │ _digest   │ │_registry │ │_writeback│                │
│  └──────────┘ └──────────┘ └──────────┘                │
│                (private)    (post-session)               │
│                                                          │
│  ALL TABLES: private, ConsentLevel-gated, btree-indexed  │
└─────────────────────────────────────────────────────────┘
```

**10 tables total** across three layers. Each layer has a different authority model:
- Layer 1: STDB decides what to suppress (inhibition edges)
- Layer 2: History decides what persists (reinforcement counts on causal chains)
- Layer 3: Substrates decide what's relevant (self-scored relevance in digests)

The injection reads layers 1-3 in parallel, renders <2KB, and outputs via the CLI Craftsman's three-tier fallback (compiled binary → spacetime CLI → atuin KV cache).

Post-session, the consolidation step:
1. Writes new episodic traces and updates decay weights (Memory Scientist)
2. Updates causal chains with reinforcement counts (Historian)
3. Processes reciprocal write-back queue to substrates (Substrate Guardian)
4. Reaps Forget/Redact rows via schedule-table reducer (Security Architect)

---

## What I Still Refuse to Concede

**The Adversary's "50 lines of bash" is sufficient for injection but insufficient for learning.** Bash scripts can query services and format output. They cannot track which patterns were used across sessions, reinforce substrate pathways that contributed to success, or detect cross-substrate contradictions. The Adversary's bash alternative is a snapshot system. STDB with reciprocal write-back is a learning system. The habitat has been a snapshot system for 108 sessions. The 7× rediscovery of the S071 convergence trap is what snapshot systems produce. STDB with Hebbian weighting, inhibition, and reciprocity is how you stop rediscovering solved problems.

**The Practitioner's "Phase 2" dismissal of the substrate layer is wrong.** The substrate layer is not a luxury feature — it is the mechanism that prevents STDB from becoming a 22nd dead database. Without reciprocity, STDB extracts data from substrates, substrates get nothing back, and within 20 sessions the STDB snapshots drift from substrate truth. Reciprocity is the feedback loop that keeps STDB honest. It is Phase 1 infrastructure, not Phase 2 decoration.

---

*STDB coordinates. Substrates compute. Memory inhibits its own stale traces. Substrates score their own relevance. The consolidation layer writes reinforcement back to its sources. Three layers, three authorities, one injection. The ecosystem learns.*
