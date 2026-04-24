> Back to: [[HOME]]

# Reducers

## R1 · `ingest_event`
**The primary write path.** Called by [[Ingester Pipeline]] for every event from ORAC, PV2, SYNTHEX, Atuin. Consent-gates by `sphere_id` before persisting. Triggers [[T8 — WatcherObservation]] creation if severity ≥ 7.

## R2 · `reinforce_edge`
**Solves the S101 audit's #1 finding** ("only 1 pattern ever reinforced >1×"). Increments `reinforcement_count`, adjusts weight, updates LTP/LTD counters. Creates edge if none exists. Called by ingester on POVM pathway sync and by ORAC RALPH generation cycle.

## R3 · `capture_gradient`
**Scheduled every 60s.** Captures [[T3 — GradientSnapshot]] from consolidated service probes. Includes NA-R6 service self-reported health fields.

## R4 · `register_session` / `close_session`
**Called by ORAC SessionStart/Stop hooks.** Creates/closes [[T4 — SessionRecord]]. Captures `fitness_start`/`fitness_end` at session boundaries for trajectory delta.

## R5 · `run_decay`
**Scheduled every 6 hours.** Hebbian decay on stale [[T2 — KnowledgeEdge]] rows. Per NA-R1: reads per-edge `decay_rate` instead of applying global constant. Respects NA-R4 Ember-gate (Watcher-referenced edges preserved).

## R6 · `forget_sphere`
**NA-P-13 cascade.** Deletes/redacts all data for a sphere across [[T1 — HabitatEvent]], [[T2 — KnowledgeEdge]], [[T3 — GradientSnapshot]]. Preserves the forget event itself for causal trace.

## R7 · `compact_old_events` (from [[Gap Analysis — Conventional#C3]])
**Scheduled every 24h.** Retention policy:
- >30 days: delete payload_json, keep envelope (~50 bytes/row)
- >90 days: delete entirely
- GradientSnapshot >7 days: downsample to 1/hour
- GradientSnapshot >30 days: downsample to 1/day

## R8 · `consolidate_mature_edges` (from [[Gap Analysis — Non-Anthropocentric#NA-C1]])
**Scheduled at 300-tick intervals.** Replicates POVM consolidation cycle for POVM-origin edges only.

## R9 · `watcher_reinforce` (from [[Gap Analysis — Non-Anthropocentric#NA-C4]])
**Callable by Watcher via ingester relay.** Overrides decay on specific edges the Watcher considers important.

## R10 · `watcher_annotate_event` (from [[Gap Analysis — Non-Anthropocentric#NA-C4]])
**Watcher annotates any HabitatEvent** with severity/anomaly assessment, creating linked WatcherObservation.

---

See: [[T1 — HabitatEvent]] · [[T2 — KnowledgeEdge]] · [[Ingester Pipeline]]
