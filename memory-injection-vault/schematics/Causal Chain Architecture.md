> Back to: [[HOME]] · [[T1 — HabitatEvent]] · [[DEPLOYMENT FRAMEWORK]]

# Causal Chain Architecture

The key differentiator. Every [[T1 — HabitatEvent]] carries `causal_parent: Option<u64>` linking effect to cause.

## Causal Chain Example

```mermaid
graph TD
    E1[E12325<br/>gradient_snapshot<br/>T=0.62, fitness=0.660<br/>sev=0]
    E2[E12326<br/>thermal.overshoot<br/>T crossed 0.55 threshold<br/>sev=4]
    E3[E12327<br/>thermal.adjustment<br/>PID output=-0.18<br/>sev=2]
    E4[E12328<br/>k_modulation<br/>K adjusted 1.0→0.92<br/>sev=2]
    E5[E12329<br/>emergence.coherence_lock<br/>r locked at 0.97<br/>sev=6]
    E6[E12330<br/>ralph.phase_transition<br/>Recognize→Analyze<br/>sev=3]
    E7[E12331<br/>watcher.observation<br/>thermal_drift anomaly<br/>sev=7]

    E1 -->|causal_parent| E2
    E2 -->|causal_parent| E3
    E3 -->|causal_parent| E4
    E4 -->|causal_parent| E5
    E5 -->|causal_parent| E6
    E2 -->|causal_parent| E7

    style E1 fill:#2d5016,color:#fff
    style E7 fill:#8b0000,color:#fff
    style E5 fill:#8b6914,color:#fff
```

## The 5 Causal Linkage Rules

```mermaid
flowchart LR
    subgraph "Rule 1: Emergence → Trigger"
        EM[emergence.detected] -->|triggered_by_tick| TICK[gradient/thermal<br/>event at that tick]
    end

    subgraph "Rule 2: Sphere → Session"
        SP[sphere.registered] -->|session_id match| SS[session.start<br/>hook event]
    end

    subgraph "Rule 3: Thermal → Gradient"
        TH[thermal.adjustment] -->|threshold crossing| GR[gradient_snapshot<br/>that crossed]
    end

    subgraph "Rule 4: Command Pair"
        POST[command.postexec] -->|command_hash match| PRE[command.preexec<br/>same hash]
    end

    subgraph "Rule 5: Watcher → Gradient"
        WO[watcher.observation] -->|metric_json ref| GS[gradient_snapshot<br/>that triggered]
    end
```

## Query Patterns for Causal Investigation

### Walk chain forward (effects of event X)
```sql
-- All events caused by event 12325
SELECT id, event_type, severity, timestamp
FROM habitat_event
WHERE causal_parent = 12325
ORDER BY timestamp;
```

### Walk chain backward (root cause of event X)
```sql
-- Recursive walk to root cause
WITH RECURSIVE chain AS (
  SELECT * FROM habitat_event WHERE id = 12331
  UNION ALL
  SELECT e.* FROM habitat_event e
  JOIN chain c ON e.id = c.causal_parent
)
SELECT id, event_type, severity, timestamp FROM chain
ORDER BY timestamp;
```

### Find unlinked high-severity events (orphan anomalies)
```sql
SELECT id, event_type, severity, source_service, timestamp
FROM habitat_event
WHERE severity >= 5
AND causal_parent IS NULL
ORDER BY timestamp DESC LIMIT 20;
```

### TC8 Cross-Substrate Investigation
```bash
# Step 1: Find the fitness drop in STDB
spacetime sql habitat \
  "SELECT id, ralph_fitness, timestamp FROM gradient_snapshot
   WHERE ralph_fitness < 0.66 ORDER BY timestamp DESC LIMIT 1"

# Step 2: Find events around that timestamp
spacetime sql habitat \
  "SELECT id, event_type, severity, causal_parent
   FROM habitat_event
   WHERE timestamp BETWEEN '2026-04-19T10:25:00' AND '2026-04-19T10:35:00'
   AND severity >= 3 ORDER BY timestamp"

# Step 3: What commands were running? (Atuin)
atuin search --after "2026-04-19T10:25:00" --before "2026-04-19T10:35:00"

# Step 4: ORAC's current emergence view
curl -s localhost:8133/emergence | python3 -c \
  "import json,sys; d=json.load(sys.stdin); [print(f'{k}: {v}') for k,v in d.get('by_type',{}).items()]"
```

## ORAC Patch Required (C4)

ORAC's emergence detector must emit `triggered_by_tick` in event payloads:

```rust
// In orac-sidecar/src/m4_emergence/m37_emergence_detector.rs
// EXISTING: EmergenceEvent { detector_id, event_type, confidence, ... }
// ADD: triggered_by_tick: u64  (~5 LOC)
```

Without this, Rule 1 linkage has no source data. This is the **C4 critical gap** — the chain's most important link.

---

See: [[Gap Analysis — Conventional]] · [[T1 — HabitatEvent]] · [[Phase C — Watcher + Causal Chains]]
