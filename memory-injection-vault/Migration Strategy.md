> Back to: [[HOME]]

# Migration Strategy

## What Migrates

| Source | Rows | → STDB Table | Phase |
|--------|------|-------------|-------|
| `service_tracking.db` learned_patterns | 141 | [[T2 — KnowledgeEdge]] ("learned_pattern") | B |
| `service_tracking.db` orchestration_graph | 29 | [[T2 — KnowledgeEdge]] ("orchestration") | B |
| `hebbian_pulse.db` neural_pathways | 109 | [[T2 — KnowledgeEdge]] ("hebbian") | B |
| `hebbian_pulse.db` hebbian_pathways | 109 | [[T2 — KnowledgeEdge]] ("hebbian") | B |
| `hebbian_pulse.db` decay_audit_log | 676 | [[T1 — HabitatEvent]] ("decay.cycle") | B |
| POVM `:8125` /pathways | 3,554 | [[T2 — KnowledgeEdge]] ("povm") | B |
| `service_tracking.db` cross_agent_learnings | 12 | [[T2 — KnowledgeEdge]] ("cross_agent") | B |
| `service_tracking.db` service_events | 27 | [[T1 — HabitatEvent]] | B |
| `flow_state.db` flow_states | 12 | [[T3 — GradientSnapshot]] | B |
| `agent_deployment.db` agents | 46 | [[T6 — ServiceHealth]] | B |
| `system_synergy.db` | 89 | [[T2 — KnowledgeEdge]] ("synergy") | B |
| V3 workflow_state.db workflows | 4 | [[T5 — Workstream]] | B |
| synthex-v2 gradient_snapshot.db | 1 | [[T3 — GradientSnapshot]] | A |
| synthex-v2 bridge_health.db | 9 | [[T6 — ServiceHealth]] | A |
| synthex-v2 watcher_observation.db | 0 | [[T8 — WatcherObservation]] | C |
| RM `:8130` heartbeat | ~2,000 | [[T3 — GradientSnapshot]] | D |

## What Doesn't Migrate

- **VMS (1,881 memories)** — semantic vectors, not relational (per [[Gap Analysis — Conventional#I6]])
- **Auto-Memory (MEMORY.md)** — human-curated, always loaded by Claude Code
- **Obsidian vault (215 notes)** — canonical human-authored docs
- **11 dead tracking DBs** — deleted at [[Phase E — Bootstrap Revolution]]

## POVM Dual-Write Transition

```
Phase A-C:  ORAC → POVM (existing)
            Ingester polls POVM → syncs to STDB T2

Phase D:    ORAC → POVM + ORAC → STDB T1 (new)
            Ingester still syncs POVM → STDB T2

Phase E+:   ORAC → STDB (primary) → periodic snapshot to POVM (backup)
```

POVM is never deleted. STDB becomes primary write path after Phase D verification.

## Verification (per [[Gap Analysis — Conventional#C5]])

Per-source checksums:
1. Pre-migration: `COUNT(*)`, `SUM(weight)`, `AVG(weight)` from source
2. Post-migration: equivalent STDB query
3. Tolerance: ±0.01 on weight aggregates, exact match on counts
4. On failure: abort, preserve source, file BUG

---

See: [[Phase B — Knowledge Graph Migration]] · [[Current State — Memory Substrates]]
