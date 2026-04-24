> Back to: [[HOME]]

# Recommendations Summary

## Combined Gap Adoption Tiers

### Must-Have (C1-C4 + I1-I2 + NA-R1 + NA-R2 + NA-R7)

| Item | Source | Cost | Impact |
|---|---|---|---|
| C1 Replace view with procedure/CLI | Conventional | +1h | Build correctness |
| C2 Split workspaces | Conventional | +0.5h | Build sanity |
| C3 Retention reducer R7 | Conventional | +3h | Memory safety |
| C4 Causal linkage rules | Conventional | +4h | Core differentiator |
| I1 Pre-flight build test | Conventional | +0.5h | Risk de-risking |
| I2 Injector uses CLI not SDK | Conventional | -1h | Simplification |
| NA-R1 Per-edge learning params | NA | +3h | Metabolic diversity |
| NA-R2 Consent on migration | NA | +2h | Consent integrity |
| NA-R7 Consolidation tradeoffs doc | NA | +0.25h | Honesty |
| **Subtotal** | | **+13.25h** | |

### Should-Have (I3-I7 + NA-R3 + NA-R4 + NA-R6)

| Item | Source | Cost |
|---|---|---|
| I3 Compound indexes | Conventional | +0.5h |
| I4 Chaos testing | Conventional | +1h |
| I5 Ingester devenv registration | Conventional | +0.25h |
| I6 VMS out-of-scope doc | Conventional | +0.1h |
| I7 STDB version pinning | Conventional | +0.25h |
| NA-R3 Reciprocal data flow | NA | +4h |
| NA-R4 Watcher structural integration | NA | +3h |
| NA-R6 Service self-reported health | NA | +1.5h |
| **Subtotal** | | **+10.6h** |

### Nice-to-Have (N1-N5 + NA-R5 + NA-R8)

| Item | Source | Cost |
|---|---|---|
| N1-N5 (5 conventional) | Conventional | ~+2h |
| NA-R5 Service sessions | NA | +2h |
| NA-R8 Adaptive payload | NA | +3h |
| **Subtotal** | | **+7h** |

## Revised Timeline by Adoption Level

| Level | Total Hours | Sessions |
|-------|-------------|----------|
| Plan v1 (as-is) | 40-50h | 8-10 |
| + Must-Have | 53-63h | 10-12 |
| + Should-Have | 64-74h | 12-14 |
| + Nice-to-Have | 71-81h | 14-16 |

## Recommended: Must-Have + NA-R3

**~57-67h across 11-13 sessions.** Gets correctness, metabolic diversity, consent integrity, AND reciprocal data flow (the single highest-leverage NA fix). Defers service sessions and adaptive payload to post-launch.

---

See: [[Gap Analysis — Conventional]] · [[Gap Analysis — Non-Anthropocentric]] · [[Session Estimates]]
