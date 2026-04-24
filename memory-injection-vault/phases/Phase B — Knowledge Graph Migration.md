> Back to: [[HOME]]

# Phase B — Knowledge Graph Migration (8-10h, 2 sessions)

## Deliverables
- [[T2 — KnowledgeEdge]] table with NA-R1 per-edge learning parameters
- [[Reducers#R2 reinforce_edge]] + [[Reducers#R5 run_decay]] (per-edge decay)
- [[Reducers#R8 consolidate_mature_edges]] (POVM consolidation cycle replication)
- `povm_migrator`: one-shot migration of 3,554 POVM pathways
- `sqlite_migrator`: one-shot migration of 10 live tracking DBs
- [[T5 — Workstream]] populated from V3 + synthex-v2 workflow DBs
- [[T7 — TrapState]] populated by trap-probe reducer
- [[Migration Strategy#Verification|Verification checksums]] per source

## Acceptance
- `knowledge_edge` count ≥ 3,554 (POVM) + 141 + 29 + 109 + 89 + 12 = **3,934**
- Weight aggregates match source (±0.01 tolerance)
- Decay schedule fires every 6h, visible in logs
- `SELECT * FROM workstream WHERE status = 'in_progress'` returns active work
- Consolidation reducer fires at 300-tick intervals for POVM-origin edges

## Key Decision
POVM continues as source-of-truth during this phase. Ingester syncs POVM → STDB.

---

See: [[Phase C — Watcher + Causal Chains]] · [[Migration Strategy]]
