> Back to: [[HOME]] · [[MASTER INDEX]] · [[Architecture Overview]]

# Hebbian Learning

## Algorithm

The memory system uses Hebbian-inspired learning to determine which patterns and causal chains matter. Patterns that keep recurring get reinforced; patterns that stop firing decay and eventually get pruned.

## Parameters

| Parameter | Value | Constant | Purpose |
|-----------|-------|----------|---------|
| Decay rate | 0.95 | `DECAY_RATE` | Multiplicative decay per session for unfired patterns |
| Reinforce rate | 0.1 | `REINFORCE_RATE` | Additive reinforcement for fired patterns |
| Prune threshold | 0.05 | `PRUNE_THRESHOLD` | Delete patterns below this weight |
| Auto-resolve sessions | 10 | `AUTO_RESOLVE_SESSIONS` | Resolve chains untriggered for N sessions |

## Equations

**Decay** (unfired patterns):
```
weight_new = weight_old * DECAY_RATE
```

**Reinforce** (fired patterns):
```
weight_new = weight_old + REINFORCE_RATE * (1 - weight_old)
```

The `(1 - weight_old)` term creates diminishing returns: patterns near 1.0 get tiny reinforcements, preventing runaway.

**Prune:**
```
DELETE FROM reinforced_pattern WHERE weight < PRUNE_THRESHOLD
```

## Convergence

- A pattern fired every session converges to `weight = 1.0`
- A pattern never fired again decays: `0.5 -> 0.475 -> 0.451 -> ... -> 0.05 (pruned at ~56 sessions)`
- A pattern fired every 3rd session oscillates around `weight ≈ 0.54`

## Causal Chain Auto-Resolve

Chains (bugs, traps) that go 10 sessions without being triggered are automatically resolved. This prevents stale bug references from consuming injection budget indefinitely.

## Implementation

All learning logic lives in `m16_hebbian_engine` ([[L4 Consolidation Engine]]). It runs as part of the post-session consolidation pipeline.
