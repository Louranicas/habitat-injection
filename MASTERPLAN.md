# habitat-injection — Master Plan

## Phase 1: Scaffold (COMPLETE)

6 layers, 24 modules, full documentation tree, quality gate clean.

## Phase 2: Implementation (bottom-up)

Quality gate after EVERY module. No exceptions.

### Required Order

1. L1 Foundation (m01_types-m05_constants) — zero dependencies
2. L2 Schema & Persistence (m06_schema-m10_pattern) — needs L1
3. L3 Injection Engine (m11_parallel_query-m14_consent_filter) — needs L1, L2
4. L4 Consolidation Engine (m15_trajectory_capture-m18_atuin_cache) — needs L1, L2
5. L5 Query & Browser (m19_preset_queries-m21_fzf_browser) — needs L1, L2
6. L6 SpaceTimeDB Migration (m22_stdb_module-m24_migration) — needs L1, L2, L3, L4

## Phase 3: Wiring

- Daemon binary: server startup, health endpoint
- Client binary: connect, query, disconnect

## Dependency Graph

```
L1 (no deps)
L2 -> L1
L3 -> L1 & L2
L4 -> L1 & L2
L5 -> L1 & L2
L6 -> L1 & L2 & L3 & L4
```


## Agent Dispatch Strategy

- Omega -> L1 (foundation, highest priority)
- Validator -> L2 (quality gate per module)
- Validator -> L3 (quality gate per module)
- Validator -> L4 (quality gate per module)
- Validator -> L5 (quality gate per module)
- Validator -> L6 (quality gate per module)
- Scribe -> Documentation (all layers)
