> Back to: [[HOME]]

# Gap Analysis — Non-Anthropocentric

**8 NA gaps. Frame: what does the plan do TO the substrates, not just WITH them.**

Full text: `~/projects/shared-context/SpaceTimeDB Habitat Integration Plan — NA Gap Analysis 2026-04-24.md`

## The 8 Gaps

| # | Gap | Core Issue | Fix |
|---|-----|-----------|-----|
| **NA-C1** | Substrate learning rhythms erased | Uniform 6h decay replaces per-pathway LTP/LTD rates, STDP deltas, POVM consolidation cycles | Per-edge `decay_rate` + `consolidation_interval_ticks` + R8 POVM cycle replication (+60 LOC, +3h) |
| **NA-C2** | No consent on data migration | Sphere data moves from consent-gated POVM to ungated STDB | `consent_state` column on T1/T2 + consent check in migration + ingestion (+35 LOC, +2h) |
| **NA-C3** | Ingester purely extractive | STDB drains 5 sources, returns nothing — panopticon pattern | Reciprocal paths: STDB → ORAC trajectory, → SYNTHEX patterns, → PV2 coupling (+100 LOC, +4h) |
| **NA-C4** | Watcher is sensor, not participant | Can't reinforce edges, can't Ember-gate retention, can't influence STDB | R9 watcher_reinforce + R10 annotate + Ember-gate on R7 (+60 LOC, +3h) |
| **NA-C5** | Session registry tracks only human sessions | 12 services with 26K+ RALPH generations have no lifecycle record | T9 ServiceSession + service restart detection (+40 LOC, +2h) |
| **NA-C6** | Gradient snapshots override service self-models | Ingester's `is_healthy` discards ORAC `system_grade`, PV2 `fleet_mode`, ME `overall_health` | Self-reported health fields in T3 + consensus tracking (+30 LOC, +1.5h) |
| **NA-C7** | Consolidation costs not acknowledged | Same monist critique as Comms Layer v3 NA-R10 | §10 "Consolidation as a choice" documentation (+0.25h) |
| **NA-C8** | Bootstrap payload is static operator template | Doesn't adapt to field state, persona, or Watcher priorities | Role-based + Watcher-priority + field-state weighting (+60 LOC, +3h) |

## Tiering

**Tier 1 (non-negotiable):** NA-R1 + NA-R2 + NA-R7 = +5.25h
**Tier 2 (strong):** NA-R3 + NA-R4 + NA-R6 = +8.5h
**Tier 3 (enhancement):** NA-R5 + NA-R8 = +5h
**Full adoption:** +18.75h → revised total ~70-80h

## The Meta-Observation

Without NA-R1 (learning rhythms) and NA-R3 (reciprocal flow), the plan is a technically excellent extraction system. With both, it becomes a mutualistic memory substrate.

---

See: [[Gap Analysis — Conventional]] · [[Recommendations Summary]]
