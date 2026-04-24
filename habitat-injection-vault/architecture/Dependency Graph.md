> Back to: [[HOME]] · [[MASTER INDEX]] · [[Architecture Overview]]

# Dependency Graph

## Layer Dependencies

```mermaid
graph TD
    L1["L1 Foundation<br/>(m01-m05)"]
    L2["L2 Schema<br/>(m06-m10b)"]
    L3["L3 Injection<br/>(m11-m14)"]
    L4["L4 Consolidation<br/>(m15-m18)"]
    L5["L5 Query<br/>(m19-m21b)"]
    L6["L6 STDB<br/>(m22-m24)"]

    L1 --> L2
    L1 --> L3
    L1 --> L4
    L1 --> L5
    L1 --> L6
    L2 --> L3
    L2 --> L4
    L2 --> L5
    L2 --> L6
    L3 --> L6
    L4 --> L6
```

## Implementation Order

```
L1 (no deps)           ← start here
 └── L2 (needs L1)
      ├── L3 (needs L1, L2)    ← these three are
      ├── L4 (needs L1, L2)    ← independent of
      ├── L5 (needs L1, L2)    ← each other
      └── L6 (needs L1, L2, L3, L4)  ← last
```

## Rules

- No upward imports (L2 cannot import L3)
- No lateral imports between L3/L4/L5 (they share L2 but don't know about each other)
- L6 is the only layer that depends on L3 and L4
- Feature-gated modules (L6) never imported by non-gated code
