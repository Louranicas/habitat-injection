> Back to: [[HOME]]

# Injector — Context Window Bootstrap

The <100ms CLI tool that delivers complete Habitat state at session start.

## Flow

```
Claude Code starts
     │
     ▼
ORAC SessionStart hook fires
     │
     ▼
Hook calls: habitat-stdb-inject
     │
     ▼
┌────────────────────────────┐
│  Injector CLI              │
│  1. spacetime sql habitat  │
│     "SELECT ..." (one-shot)│
│  2. Format as structured   │
│     text (≤15 KB)          │
│  3. Print to stdout        │
└────────────────────────────┘
     │
     ▼
ORAC injects into Claude Code
system message
```

## Implementation (per [[Gap Analysis — Conventional#I2]])

Uses `spacetime sql` CLI for one-shot queries — NOT the Rust SDK. Simpler, no WebSocket lifecycle management for a one-shot tool.

## NA-R8 Adaptive Payload (per [[Gap Analysis — Non-Anthropocentric#NA-C8]])

Payload adapts to:
1. **Role:** Zen → code quality focus. Watcher → anomaly focus. Default → full view.
2. **Watcher priority:** severity ≥ 7 observation gets top position.
3. **Field state:** high r → coupling details. Low r → service health.

## Target Latency Budget

| Step | Budget |
|------|--------|
| `spacetime sql` query | <40ms |
| JSON parse + format | <10ms |
| stdout write | <5ms |
| ORAC hook overhead | <45ms |
| **Total** | **<100ms** |

---

See: [[Bootstrap Chain — Current vs Target]] · [[Phase E — Bootstrap Revolution]]
