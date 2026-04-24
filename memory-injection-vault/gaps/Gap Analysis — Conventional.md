> Back to: [[HOME]]

# Gap Analysis — Conventional

**17 gaps: 5 critical, 7 important, 5 nice-to-have**

Full text: `~/projects/shared-context/SpaceTimeDB Habitat Integration Plan — Gap Analysis 2026-04-24.md`

## Critical (5)

| # | Gap | Fix | Cost |
|---|-----|-----|------|
| **C1** | STDB views cannot use `.iter()` — V1 bootstrap view invalid | Use procedure-based query or `spacetime sql` CLI | +1h |
| **C2** | WASM module + native ingester in same workspace — build collision | Split into 3 workspaces | +0.5h |
| **C3** | No retention policy — `habitat_event` grows ~26K rows/day → OOM in 60 days | Add [[Reducers#R7 compact_old_events]] | +3h |
| **C4** | Causal parent assignment hand-waved — no concrete wiring | Define 5 linkage rules + ORAC `triggered_by_tick` patch | +4h |
| **C5** | No rollback strategy for migration failures | Count/weight aggregate checksums per source | +1h |

## Important (7)

| # | Gap | Fix |
|---|-----|-----|
| **I1** | STDB standalone never tested on this machine | 30min pre-flight at Phase A start |
| **I2** | Injector uses Rust SDK for one-shot query — wrong tool | Use `spacetime sql` CLI (-50 LOC) |
| **I3** | KnowledgeEdge missing compound indexes | Add 3 btree indexes |
| **I4** | No chaos/failure testing | Add kill-recovery + reconnection tests |
| **I5** | Ingester port not in devenv | Register as separate service |
| **I6** | VMS silently dropped from migration | Document as out-of-scope |
| **I7** | STDB version conflict risk with synthex-v2 | Pin both to same version |

## Nice-to-have (5)

N1 Obsidian schema note · N2 `spacetime dev` hot-reload · N3 gradient schedule table · N4 cost tracking · N5 L0/L5 sourcing ambiguity

## Recommended Minimum

C1 + C2 + C3 + C4 + I1 + I2 = **+8h**. Revised total: **~50-60h**.

---

See: [[Gap Analysis — Non-Anthropocentric]] · [[Recommendations Summary]]
