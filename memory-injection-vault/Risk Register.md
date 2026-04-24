> Back to: [[HOME]]

# Risk Register

| # | Risk | Impact | Prob | Mitigation |
|---|------|--------|------|------------|
| R1 | STDB standalone crashes under load | Event loss | LOW | WAL persistence + ingester retry logic |
| R2 | STDB memory >1GB | OOM kill | MED | [[Reducers#R7 compact_old_events]], gradient downsampling |
| R3 | Migration data loss | Historical data gone | LOW | Additive migration; SQLite preserved as backup |
| R4 | Ingester-STDB latency spikes | Stale bootstrap | LOW | Local loopback, not remote |
| R5 | STDB SDK version conflict | Build failure | MED | Separate workspace; version pinning strategy |
| R6 | Bootstrap payload >15 KB | Context pressure | LOW | Aggressive top-N limits on queries |
| R7 | POVM→STDB sync drift | Divergent graphs | MED | Periodic full-sync reducer; pathway count monitoring |

---

See: [[Success Criteria]] · [[Session Estimates]]
