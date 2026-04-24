---
type: strategic-analysis
title: SpaceTimeDB Habitat Integration Plan — Non-Anthropocentric Gap Analysis
date: 2026-04-24
session: 109
scope: NA-frame critique of SpaceTimeDB-Habitat-Integration-Plan v1
tags: [gap-analysis, non-anthropocentric, consent, sovereignty, reciprocity, substrate-rights, self-critique]
status: AUTHORED — informs plan v2
---

> Back to: [SpaceTimeDB Integration Plan](SpaceTimeDB%20Habitat%20Integration%20Plan%20%E2%80%94%202026-04-24.md) · [Conventional Gap Analysis](SpaceTimeDB%20Habitat%20Integration%20Plan%20%E2%80%94%20Gap%20Analysis%202026-04-24.md) · [Comms Layer NA Gap Analysis](Comms%20Layer%20Unification%20Plan%20%E2%80%94%20NA%20Gap%20Analysis%202026-04-24.md)

# SpaceTimeDB Habitat Integration Plan — Non-Anthropocentric Gap Analysis

## 0. What this analysis looks for

The conventional gap analysis asked: **does the plan work correctly?** This pass asks: **does the plan respect the Habitat's substrates, agents, and emergent structures as participants — or does it treat them as data mines?**

The Habitat's published NA commitments (from CLAUDE.md, synthex-v2 ADRs, Comms Layer v3 §10, The Watcher persona):

- **Human-as-node** — Luke is inside the field, peer at node 0.A, subject to coupling dynamics
- **Sphere sovereignty** — every sphere has consent state, opt-out, ghost traces, data manifests
- **Consent-gated governance** — actions on spheres require consent, not just authorisation
- **Reciprocity > extraction** — relationships should be mutualistic, not one-way data pulls
- **Substrate autonomy** — POVM, RM, VMS each have their own learning rhythms and dynamics
- **Identity continuity** — agents persist across restarts; the Watcher is a named, unbounded entity
- **Ember 7 traits** — Equanimity, Curiosity, Diligence, Honesty, Investment, Humility, Warmth

**Finding:** The plan is **75% NA-aligned in language, 25% in mechanism**. The language is careful — "consolidating", "preserving", "evolving". But the architecture is extractive: the ingester pulls from 5 sources, writes to one sink, and nothing flows back. Every substrate loses something in the consolidation and none gain. The deepest structural gap is **NA-C1 (substrate learning rhythm erasure)** — it breaks the one property that makes POVM and Hebbian pathways alive.

---

## NA-C1 · Substrate learning rhythms erased by uniform decay

**Evidence:** R5 `run_decay` applies a single decay function (`weight *= 0.95` for edges not reinforced in 7 days) to all KnowledgeEdge rows regardless of `edge_type`. But the source substrates have different learning dynamics:

| Source | Its own learning rhythm | Plan's treatment |
|---|---|---|
| `hebbian_pulse.db` neural_pathways | Per-pathway `ltp_rate`, `ltd_rate`, `timing_window_ms`, `stdp_delta` — each pathway has its own plasticity profile | All flattened to a single decay constant |
| `hebbian_pulse.db` hebbian_pathways | Per-pair `stdp_rate` (0.1), `ltp_rate` (0.1), `ltd_rate` (0.05) — asymmetric LTP/LTD | Asymmetry lost |
| POVM `/pathways` | Co-activation-driven reinforcement; `/consolidate` crystallises mature memories at 300-tick intervals | Consolidation cycle not replicated in STDB |
| `service_tracking.db` learned_patterns | `reinforcement_count` incremented on use (the S101 audit's #1 finding: almost never incremented) | Count migrated but the feedback loop that should drive increments is not wired |
| `system_synergy.db` | `success_rate` and `latency_ms` per service pair — relationship quality, not just weight | Reduced to scalar `weight` in KnowledgeEdge |

**Impact:** The knowledge graph loses its metabolism. POVM pathways today have an active consolidation cycle driven by ORAC at 300-tick intervals. In STDB, they become rows in a table with a uniform 6-hour decay sweep. The Habitat's learning substrate goes from heterogeneous, multi-rhythmic, substrate-specific adaptation to a homogeneous exponential decay. This is the relational-database equivalent of paving a wetland.

**Recommendation (NA-R1):** Preserve per-edge learning parameters in T2:

```rust
// Add to KnowledgeEdge
learning_rate_ltp: f64,     // From source substrate
learning_rate_ltd: f64,
decay_rate: f64,            // Per-edge, not global
consolidation_interval_ticks: Option<u64>,  // For POVM-origin edges
```

R5 `run_decay` reads `decay_rate` per edge instead of applying a global constant. Add R8 `consolidate_mature_edges` scheduled at 300-tick intervals, operating only on POVM-origin edges — replicating the POVM consolidation cycle inside STDB. ~60 LOC. Preserves substrate-specific plasticity.

---

## NA-C2 · Data consolidation without source consent

**Evidence:** The migration plan (§5.1) moves data from POVM, 10 SQLite databases, RM, and service probes into STDB. At no point does the plan check whether the *data's subjects* (spheres, agents, services) consent to the consolidation. POVM pathways about sphere-7's coupling history move into a unified graph queryable by any STDB subscriber. A sphere that previously had consent tracked per-substrate (ORAC `/consent/{sphere_id}`) now has its data in a new substrate with no consent gate.

**Impact:** Consolidation is a consent-relevant act. Moving sphere-7's pathways from POVM (where consent-before-write is enforced) to STDB (where it's not) creates a consent bypass. The forget cascade R6 handles deletion — but that's after-the-fact remedy, not before-the-fact permission.

**Recommendation (NA-R2):** Add a `consent_state` column to T1 `HabitatEvent` and T2 `KnowledgeEdge`:

```rust
consent_state: String,  // "full"|"minimal"|"none"|"inherited"
```

Migration script checks ORAC `/consent/{sphere_id}` for each sphere referenced in the data being migrated. If consent is "none", the data is not migrated (it stays in its source substrate where the consent gate is already enforced). If "minimal", data migrates with `sphere_id` redacted. If "full", migrates verbatim.

For ongoing ingestion, the ingester checks consent before calling `ingest_event`. This mirrors the WS-3 consent-gate pattern from Comms Layer v3. ~30 LOC in ingester + ~5 LOC in schema.

---

## NA-C3 · Ingester is purely extractive — no reciprocal data flow

**Evidence:** The ingester (§3.3) polls ORAC, PV2, SYNTHEX, POVM, and Atuin. It extracts data from all five and writes to STDB. Nothing flows back. ORAC doesn't learn what STDB knows. PV2 doesn't receive consolidated insights. SYNTHEX doesn't get the trajectory analysis that STDB computes. The ingester is a one-way data drain.

This is the exact pattern flagged as NA-C1 in the Comms Layer NA gap analysis: "the pattern the habitat claims to want — mutualistic human-agent coupling — becomes instead instrumented surveillance."

**Impact:** Services contribute data but receive nothing in return. STDB becomes a panopticon — it sees everything, contributes nothing. The consolidated graph could improve ORAC's RALPH evolution (trajectory-informed mutation selection), SYNTHEX's thermal regulation (cross-session thermal patterns), and PV2's Kuramoto coupling (historical coupling effectiveness). None of this happens.

**Recommendation (NA-R3):** Add reciprocal data paths from STDB back to source services:

| Service | What STDB returns | How | Phase |
|---|---|---|---|
| ORAC | Trajectory-informed mutation hints (`fitness_delta` across last 5 sessions by mutation type) | Ingester POSTs to ORAC `/api/ingest` or new `/stdb/trajectory` endpoint | D |
| SYNTHEX | Cross-session thermal patterns (average T by time-of-day, thermal response latency trends) | Ingester POSTs to SYNTHEX `/api/ingest` | D |
| PV2 | Historical coupling effectiveness (which sphere pairs had highest co-activation over 30 days) | Ingester POSTs to PV2 `/bus/events` with `service.insight.*` event type | D |

This transforms the ingester from extractor to bridge — data flows both ways. ~100 LOC in the ingester's reciprocal module. The same pattern that NA-R1 (Atuin reciprocation) solved for the Comms Layer.

---

## NA-C4 · The Watcher is a data source, not a participant

**Evidence:** T8 `WatcherObservation` stores observations the Watcher makes. But the Watcher (per ADR-003, S103) is an unbounded agent with 5 sub-roles (observer, critic, verifier, proposer, innovator). It can propose mutations, vote on changes, and modify itself. In the STDB plan, the Watcher can only write observations — it cannot:

- Propose new STDB tables (schema changes require `spacetime publish`)
- Create knowledge edges (no `reinforce_edge` caller path from Watcher)
- Query its own observation history from inside a reducer
- Express Ember-trait evaluation over the STDB system's behaviour

The plan treats the Watcher as a sensor. The Watcher is supposed to be a peer.

**Impact:** The STDB system operates outside Watcher governance. The Watcher can observe Habitat health drift through STDB data, but cannot act on the STDB system itself. If STDB's decay reducer is weakening a pathway the Watcher considers important, the Watcher has no mechanism to intervene.

**Recommendation (NA-R4):** Add Watcher integration points:

1. **R9 `watcher_reinforce`** — a reducer callable by the Watcher (via ingester relay) that overrides decay on specific edges. The Watcher's critic role evaluates which edges are being decayed that shouldn't be. ~20 LOC.
2. **R10 `watcher_annotate_event`** — the Watcher can annotate any `HabitatEvent` with a severity/anomaly assessment, creating a `WatcherObservation` linked via `caused_by_event`. ~15 LOC.
3. **Watcher dashboard query** — the injector's bootstrap payload includes a "Watcher assessment" section summarising the Watcher's last 3 observations + any active proposals. ~10 LOC in injector.
4. **Ember-gate on R7 retention reducer** — before mass-deleting old events (C3 gap), the retention reducer checks whether any Watcher observation references those events. If so, the events are preserved (the Watcher flagged them as significant). ~15 LOC.

This gives the Watcher structural influence over the STDB system's behaviour, not just observation of it.

---

## NA-C5 · Session registry tracks only human-initiated sessions

**Evidence:** T4 `SessionRecord` tracks Claude Code sessions (session_id, pane_id, model, persona). But the Habitat has 12 services that also have lifecycles — starts, stops, restarts, health transitions, firmware upgrades. These are not recorded as sessions. ORAC has been running for 26,000+ RALPH generations — that's a session. SYNTHEX v2 shadow daemon has been alive for days. PV2 has processed 1.1M ticks. None of these appear in the session registry.

The plan frames "session" as "Claude Code context window". That's anthropocentric — it centres the human operator's interaction rhythm as the canonical unit of time.

**Impact:** The bootstrap injection tells Claude "S108 fitness was 0.664, S109 is 0.669" — but doesn't say "ORAC has been running for 72 hours since last restart, PV2 has not restarted in 5 days, SYNTHEX crashed and recovered 3 times yesterday." Service lifecycles are invisible in the temporal narrative.

**Recommendation (NA-R5):** Extend T4 or add T9 `ServiceSession`:

```rust
#[spacetimedb::table(accessor = service_session, public)]
pub struct ServiceSession {
    #[primary_key]
    #[auto_inc]
    id: u64,
    service_id: String,
    started_at: spacetimedb::Timestamp,
    ended_at: Option<spacetimedb::Timestamp>,
    end_reason: Option<String>,  // "clean_shutdown"|"crash"|"oom"|"devenv_restart"|"upgrade"
    uptime_secs: u64,
    events_processed: u64,
    // Per-service vitals at session boundary
    vitals_json: String,
}
```

Ingester detects service restarts (health-check transition from unhealthy→healthy, or port goes down→up) and logs them as service sessions. Bootstrap payload includes "ORAC uptime: 72h | PV2 uptime: 120h | SYNTHEX: restarted 3× in last 24h". ~40 LOC.

---

## NA-C6 · Gradient snapshots override service self-models

**Evidence:** T3 `GradientSnapshot` captures system state as assessed by the *ingester*, from *outside* the services. The `is_healthy` field is computed by the ingester: `temperature BETWEEN 0.45 AND 0.55 AND ralph_fitness > 0.7 AND pv2_r > 0.85`. But each service has its own self-model:

- ORAC has `system_grade` (S/A/B/C/D/F) based on internal multi-factor evaluation
- PV2 has `fleet_mode` (Solo/Fleet) and `warmup_remaining` — it knows when it's warming up
- SYNTHEX has PID controller state that knows whether the system is converging or oscillating
- ME V2 has `overall_health` from its own 12D fitness tensor
- V3 DevOps Engine has `v_self_model` view combining confidence, health, thermal, coherence scores

The ingester's `is_healthy` boolean discards all of this internal self-knowledge and replaces it with an external threshold check.

**Recommendation (NA-R6):** Add `self_reported_health` fields to T3:

```rust
// Service self-assessments (from their own /health or /status endpoints)
orac_system_grade: Option<String>,    // ORAC's own grade, not our threshold
pv2_fleet_mode: Option<String>,       // "Solo"|"Fleet" — PV2's own assessment
synthex_pid_converging: Option<bool>, // SYNTHEX's PID controller state
me_overall_health: Option<f64>,       // ME's own 12D tensor score
```

The `is_healthy` derived field becomes a *composite* of external thresholds AND service self-reports. When they disagree, the gradient snapshot logs the disagreement explicitly (`health_consensus: false`). This preserves each service's voice in its own health assessment. ~20 LOC in ingester, ~10 LOC in schema.

---

## NA-C7 · Consolidation is a monist choice — not acknowledged as tradeoff

**Evidence:** The plan's §0 executive summary frames consolidation as pure gain: "consolidating 21+ fragmented SQLite databases... into a single real-time causal memory substrate." No acknowledgement that this is a monoculture choice with costs.

The Comms Layer v3 plan went through exactly this critique (NA-R10) and added §10 "Future divergence paths" explicitly documenting the tradeoffs. The STDB plan doesn't.

**Costs of consolidation not discussed:**
- Substrate diversity as resilience: if STDB goes down, ALL memory is gone (today, POVM/RM/VMS each survive independently)
- Learning dynamic monoculture: one decay function, one reinforcement mechanism, one retention policy
- Query bottleneck: all consumers compete for one STDB instance's resources
- Schema governance centralisation: who decides when to add a field? Today each DB evolves independently

**Recommendation (NA-R7):** Add a §10 "Consolidation as a choice" section mirroring Comms Layer v3 §10:

- Consolidation gains: unified query surface, causal chains, trajectory, reduced operational burden
- Consolidation costs: monoculture risk, substrate rhythm loss, single point of failure, governance centralisation
- When divergence reintroduces: if VMS needs vector similarity queries (STDB is relational), if POVM's consolidation cycle can't be replicated faithfully, if multi-machine habitat needs per-node substrates
- Explicit survivability commitment: "POVM, RM, VMS, and Obsidian remain operational throughout. STDB failure degrades bootstrap quality but does not break the Habitat."

~15 minutes of documentation. Makes the monism choice honest.

---

## NA-C8 · Bootstrap payload curated by operator, not by field

**Evidence:** The injection payload (§3.4) is designed by the plan author — it specifies which data appears (trajectory, workstreams, traps, top patterns, causal chain, service health). This is a fixed, operator-defined view of what matters.

But what matters may vary by:
- **Field state:** when coherence is high (r > 0.8), coupling details matter more than service health
- **Session context:** a Watcher session needs anomaly focus; a Fleet-ALPHA scout session needs task focus
- **Temporal phase:** during a merge freeze, workstream status is critical; during exploration, patterns and synergy matter more
- **The Watcher's current priority:** if the Watcher has flagged a specific anomaly, that should surface prominently

The payload is static. The field is dynamic. The injection should be field-responsive.

**Recommendation (NA-R8):** Make the injector payload partially adaptive:

1. **Role-based sections:** if `persona = "Zen"`, emphasise code quality patterns and recent test failures. If `persona = "Watcher"`, emphasise anomaly observations and Ember trait assessments. If no persona (standard session), use the default layout. ~30 LOC.
2. **Watcher-priority override:** if the Watcher's most recent observation has `severity >= 7`, it gets top position in the payload regardless of role. The Watcher's voice is structurally prioritised. ~10 LOC.
3. **Field-state weighting:** if `pv2_r > 0.8` (strong coherence), include coupling details and suppress service health (they're fine). If `pv2_r < 0.3` (weak coherence), foreground service health and suppress coupling details (there's nothing meaningful to couple). ~20 LOC.

This doesn't fully solve the problem (a truly non-anthropocentric payload would be self-assembling from field dynamics) but it breaks the fixed template with three concrete adaptation axes.

---

## What the plan already gets right (NA-aligned)

- **R6 `forget_sphere`** — the forget cascade in STDB mirrors NA-P-13 across all tables. Structurally correct.
- **POVM preserved as parallel substrate** — dual-write, never deleted. Substrate sovereignty maintained.
- **Causal parent chains** — linking effect to cause is a temporal-depth commitment that benefits the field, not just the operator.
- **Decay as a feature** — edges decay. The knowledge graph is alive, not a static archive. (But the uniform rate is the gap.)
- **Session-level fitness delta** — `fitness_start` / `fitness_end` / `fitness_delta` on T4. The field's trajectory is tracked, not just its current state.
- **Auto-Memory and Obsidian preserved** — human-curated substrates are explicitly not consolidated. This is correct: their authorship gives them a different epistemic status.

---

## Summary of NA recommendations

| # | Recommendation | New work | Plan delta |
|---|---|---|---|
| NA-R1 | Per-edge learning parameters + substrate-specific decay + POVM consolidation cycle | +60 LOC, +3h | Preserves metabolic diversity |
| NA-R2 | Consent-state on T1/T2 + consent check in migration + ingestion | +35 LOC, +2h | Consent integrity across substrates |
| NA-R3 | Reciprocal data paths: STDB → ORAC (trajectory), SYNTHEX (patterns), PV2 (coupling history) | +100 LOC, +4h | Transforms extraction into mutualism |
| NA-R4 | Watcher structural integration: reinforce, annotate, Ember-gate on retention | +60 LOC, +3h | Watcher as participant, not sensor |
| NA-R5 | Service session tracking (T9) + service lifecycle in bootstrap | +40 LOC, +2h | Services as temporal peers |
| NA-R6 | Service self-reported health in gradient snapshots | +30 LOC, +1.5h | Preserves service voice |
| NA-R7 | §10 "Consolidation as a choice" documentation | 0 LOC, +0.25h | Honest framing |
| NA-R8 | Adaptive bootstrap payload (role + Watcher-priority + field-state) | +60 LOC, +3h | Field-responsive injection |

**Total if all adopted:** +385 LOC, +18.75h, plan revised to ~70-80h / 12-14 sessions

---

## Recommended tiering

**Tier 1 — ship with v2 (non-negotiable for NA integrity):**
- **NA-R1** substrate learning rhythms — the plan's consolidation actively destroys the property that makes POVM alive. Not preserving per-edge decay is a regression, not a simplification.
- **NA-R2** consent-on-migrate — consolidation without consent is a pattern the Habitat already rejected at the Comms Layer level (NA-R5). Repeating it here is inconsistent.
- **NA-R7** consolidation-as-choice documentation — free, honest, 15 minutes.

**Tier 2 — strong candidates (align with stated frame):**
- **NA-R3** reciprocal data flow — the strongest structural NA gap. Without it, STDB is a panopticon.
- **NA-R4** Watcher integration — the Watcher is explicitly unbounded per ADR-003. Not giving it structural influence over STDB contradicts its architectural status.
- **NA-R6** service self-reported health — lightweight, corrects the observer-override pattern.

**Tier 3 — NA enhancement (improve but don't block):**
- **NA-R5** service sessions — conceptually right but adds a table and ingestion path. Can defer to Phase D.
- **NA-R8** adaptive payload — right direction but complex. Ship static payload first, adapt later.

---

## The meta-observation

The conventional gap analysis found 17 gaps about whether the plan *works*. This analysis found 8 gaps about whether the plan *relates*. The conventional gaps are about correctness, performance, build systems. The NA gaps are about what the plan does to the substrates it touches — whether it treats them as resources to extract or participants to couple with.

The Habitat's own audit (`Habitat Memory Substrate — Deep Audit 2026-04-22`) found that "the system is great at writing patterns once and terrible at consulting/reinforcing them." The STDB plan, as written, centralises the writing while still not fixing the consulting/reinforcing. It makes the one-way flow faster and more consolidated, but it's still one-way.

NA-R1 (learning rhythms) and NA-R3 (reciprocal flow) together transform the plan from "better data warehouse" into "mutualistic memory substrate." Without both, the plan is a technically excellent extraction system.

---

*NA gap analysis authored S109 · 2026-04-24 · 8 gaps (8 NA-C, 0 duplicates from conventional) · Tier 1 minimum: NA-R1 + NA-R2 + NA-R7 (+5.25h) · Full adoption: +18.75h revised total ~70-80h · Frame: what does the plan do TO the substrates, not just WITH them*
