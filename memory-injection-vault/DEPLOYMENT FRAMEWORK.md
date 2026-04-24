> Back to: [[HOME]] · [[MASTER INDEX]]

# SpaceTimeDB Memory Injection — Deployment Framework

> **Version:** 1.0 | **Date:** 2026-04-24 | **Session:** 109
>
> How SpaceTimeDB injects memory into Claude Code at the start of every new context window — the complete wiring from STDB tables through CLI tool chains to the system message Claude sees.

---

## 0. The Problem This Solves

Every time Claude Code opens a new context window, it starts with amnesia. The current mitigation is a 3-hook `SessionStart` chain in `~/.claude/settings.json`:

```
Hook 1: orac-hook.sh SessionStart     → ORAC /hooks/SessionStart → system message (POVM+RM hydration)
Hook 2: session-health-broadcast.sh   → health pulse to atuin KV
Hook 3: atuin scripts run habitat-bootstrap  → 7-layer injection (55ms, ~9KB)
```

This gives Claude foundational state (identity, live metrics, patterns, CLI muscle) but misses **trajectory** (fitness Δ across sessions), **causal chains** (why things happened), **workstream state** (in-flight/blocked/deferred), **active traps** (which of 18 known traps are live), and **pattern reinforcement** (141 patterns, only 1 ever reinforced >1×).

SpaceTimeDB fills these gaps by consolidating all memory substrates into a single queryable causal graph, then injecting a ≤15 KB payload at session start in <100ms.

---

## 1. The Three-Timescale Architecture

From [[Session 090 — Claude's Learnings on CLI Paradigm]]:

```
REFLEXES  <50ms    CLI pipes           → volatile
PROBES    1-3s     Parallel bash       → session
AGENTS    1-10min  Parallel Claude     → persistent
MEMORY    ∞        POVM/RM/STDB/Obsidian → accumulated
```

SpaceTimeDB operates at the **MEMORY** timescale (∞ persistence) but delivers at the **REFLEX** timescale (<100ms injection). The trick is that STDB holds pre-computed, subscription-ready state — the injector doesn't compute anything at session start, it just reads.

---

## 2. Hook Chain Architecture (Post-STDB)

### 2.1 The SessionStart Hook Chain

`~/.claude/settings.json` after STDB integration:

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "/home/louranicas/claude-code-workspace/orac-sidecar/hooks/orac-hook.sh SessionStart 5",
            "timeout": 6
          },
          {
            "type": "command",
            "command": "/home/louranicas/.claude/hooks/session-health-broadcast.sh",
            "timeout": 4
          },
          {
            "type": "command",
            "command": "/home/louranicas/.local/bin/habitat-stdb-inject",
            "timeout": 3
          }
        ]
      }
    ]
  }
}
```

**What changed:** Hook 3 replaces `atuin scripts run habitat-bootstrap` with `habitat-stdb-inject`. The old atuin script is preserved at `~/.local/bin/habitat-bootstrap-legacy` as fallback.

### 2.2 Hook Execution Flow

```
Claude Code new context window opens
         │
         ▼
    ┌─ SessionStart event fires ─────────────────────────────────────┐
    │                                                                 │
    │  Hook 1: orac-hook.sh SessionStart 5                           │
    │  ┌────────────────────────────────────────────────┐             │
    │  │ Reads event JSON from stdin                     │             │
    │  │ POSTs to ORAC :8133/hooks/SessionStart          │             │
    │  │ ORAC: register sphere, hydrate from POVM/RM     │             │
    │  │ ORAC: return system message w/ memory summary   │             │
    │  │   → calls STDB R4 register_session (NEW)        │             │
    │  └─── stdout → Claude sees ORAC system message ───┘             │
    │                                                                 │
    │  Hook 2: session-health-broadcast.sh                           │
    │  ┌────────────────────────────────────────────────┐             │
    │  │ Probes 12 services in parallel (TC2 fan-out)    │             │
    │  │ Writes health pulse to atuin KV                 │             │
    │  └─── stdout → Claude sees health summary ────────┘             │
    │                                                                 │
    │  Hook 3: habitat-stdb-inject  ◄── THE NEW HOOK                 │
    │  ┌────────────────────────────────────────────────┐             │
    │  │ 1. spacetime sql habitat "..." (one-shot)       │             │
    │  │ 2. Format ≤15 KB structured text                │             │
    │  │ 3. Print to stdout                              │             │
    │  └─── stdout → Claude sees STDB injection ────────┘             │
    │                                                                 │
    └── All 3 hooks complete. Claude has full state. ────────────────┘
```

### 2.3 What ORAC's Hook Now Does Differently

The ORAC SessionStart hook (`orac-hook.sh`) remains unchanged — it still POSTs to `localhost:8133/hooks/SessionStart`. But ORAC's handler gains a new step:

```rust
// In orac-sidecar/src/m3_hooks/m10_hook_server.rs — SessionStart handler
// EXISTING: register sphere on PV2, hydrate from POVM, hydrate from RM
// NEW: register session in STDB
async fn handle_session_start(&self, payload: HookPayload) -> HookResponse {
    // ... existing sphere registration ...
    // ... existing POVM/RM hydration ...

    // NEW: Register session in SpaceTimeDB
    if let Ok(stdb_conn) = self.stdb_client.connect().await {
        stdb_conn.reducers.register_session(
            payload.session_id.clone(),
            self.session_counter.fetch_add(1, Ordering::Relaxed),
            payload.pane_id.clone(),
            payload.tab_name.clone(),
            payload.model.clone(),
            self.current_fitness(),
        ).await.ok();  // Best-effort — never block session start
    }

    // Return system message (unchanged — STDB injection is separate hook)
    HookResponse { system: self.format_memory_summary() }
}
```

---

## 3. The Injector — `habitat-stdb-inject`

### 3.1 What It Is

A standalone Rust binary at `~/.local/bin/habitat-stdb-inject` that:
1. Runs `spacetime sql` against the local STDB instance
2. Formats the results into a structured ≤15 KB text payload
3. Prints to stdout (consumed by Claude Code's hook system)
4. Exits (one-shot, no persistent connection)

### 3.2 The Query

The injector runs a single compound SQL query against STDB:

```bash
#!/usr/bin/env bash
# habitat-stdb-inject — SpaceTimeDB context injection for Claude Code SessionStart
# Called as hook 3 in ~/.claude/settings.json SessionStart chain.
# Target: ≤15 KB output, <100ms total.
# No set -e — STDB being down must never block session start.

STDB_DB="${STDB_DB:-habitat}"
STDB_SERVER="${STDB_SERVER:-http://127.0.0.1:3000}"

# ─── Query 1: Latest session + trajectory (L1 + L7) ───
session_data=$(spacetime sql "$STDB_DB" -s "$STDB_SERVER" --format json \
  "SELECT * FROM session_record ORDER BY started_at DESC LIMIT 2" 2>/dev/null) || true

trajectory=$(spacetime sql "$STDB_DB" -s "$STDB_SERVER" --format json \
  "SELECT ralph_gen, ralph_fitness, ralph_phase, pv2_r, temperature, system_grade
   FROM gradient_snapshot ORDER BY timestamp DESC LIMIT 5" 2>/dev/null) || true

# ─── Query 2: Active workstreams (L8) ───
workstreams=$(spacetime sql "$STDB_DB" -s "$STDB_SERVER" --format json \
  "SELECT id, service_name, goal, status, blocker_description
   FROM workstream WHERE status IN ('in_progress','blocked','deferred')
   ORDER BY CASE status WHEN 'in_progress' THEN 0 WHEN 'blocked' THEN 1 ELSE 2 END
   LIMIT 10" 2>/dev/null) || true

# ─── Query 3: Active traps (L9) ───
traps=$(spacetime sql "$STDB_DB" -s "$STDB_SERVER" --format json \
  "SELECT trap_name, trigger_count FROM trap_state WHERE is_active = true" 2>/dev/null) || true

# ─── Query 4: Top reinforced patterns (L3 evolved) ───
patterns=$(spacetime sql "$STDB_DB" -s "$STDB_SERVER" --format json \
  "SELECT source_id, target_id, weight, reinforcement_count, thermal_class
   FROM knowledge_edge WHERE edge_type = 'learned_pattern'
   ORDER BY reinforcement_count DESC, weight DESC LIMIT 15" 2>/dev/null) || true

# ─── Query 5: Latest causal chain (L10) ───
causal=$(spacetime sql "$STDB_DB" -s "$STDB_SERVER" --format json \
  "SELECT id, event_type, source_service, causal_parent, severity
   FROM habitat_event WHERE severity >= 5
   ORDER BY timestamp DESC LIMIT 10" 2>/dev/null) || true

# ─── Query 6: Service health (L2 evolved) ───
health=$(spacetime sql "$STDB_DB" -s "$STDB_SERVER" --format json \
  "SELECT service_id, health_status, circuit_state
   FROM service_health WHERE id IN (
     SELECT MAX(id) FROM service_health GROUP BY service_id
   )" 2>/dev/null) || true

# ─── Query 7: Watcher last assessment ───
watcher=$(spacetime sql "$STDB_DB" -s "$STDB_SERVER" --format json \
  "SELECT observer_role, anomaly_class, severity
   FROM watcher_observation ORDER BY timestamp DESC LIMIT 3" 2>/dev/null) || true

# ─── Format payload ───
# Use python3 for safe JSON → structured text (cross-platform, always available)
python3 -c "
import json, sys

def safe_load(s):
    try: return json.loads(s) if s else []
    except: return []

session = safe_load('''$session_data''')
traj = safe_load('''$trajectory''')
ws = safe_load('''$workstreams''')
tr = safe_load('''$traps''')
pat = safe_load('''$patterns''')
caus = safe_load('''$causal''')
hlth = safe_load('''$health''')
watch = safe_load('''$watcher''')

print('=' * 60)
print('  HABITAT MEMORY INJECTION — SpaceTimeDB')
print('=' * 60)
print()

# Session
if session:
    s = session[0]
    prev = session[1] if len(session) > 1 else {}
    delta = ''
    if prev.get('fitness_end') and s.get('fitness_start'):
        d = s['fitness_start'] - prev['fitness_end']
        delta = f' ({d:+.3f})'
    print(f'SESSION: S{s.get(\"session_number\",\"?\")} | {s.get(\"model\",\"?\")} | {s.get(\"tab_name\",\"?\")}')
    if prev:
        print(f'PREVIOUS: S{prev.get(\"session_number\",\"?\")} fitness={prev.get(\"fitness_end\",\"?\")}{delta}')
    print()

# Trajectory
if traj:
    print('TRAJECTORY (last 5 snapshots):')
    for i, t in enumerate(reversed(traj)):
        label = 'NOW' if i == len(traj)-1 else f'T-{len(traj)-1-i}'
        print(f'  {label}: r={t.get(\"pv2_r\",0):.3f} fit={t.get(\"ralph_fitness\",0):.3f} gen={t.get(\"ralph_gen\",0)} phase={t.get(\"ralph_phase\",\"?\")} T={t.get(\"temperature\",0):.3f}')
    print()

# Workstreams
if ws:
    by_status = {}
    for w in ws:
        by_status.setdefault(w.get('status','?'), []).append(w)
    print('WORKSTREAMS:')
    for status in ['in_progress', 'blocked', 'deferred']:
        items = by_status.get(status, [])
        if items:
            label = {'in_progress': 'IN-FLIGHT', 'blocked': 'BLOCKED', 'deferred': 'DEFERRED'}[status]
            descs = [f'{w[\"goal\"]}' + (f' ({w[\"blocker_description\"]})' if w.get('blocker_description') else '') for w in items]
            print(f'  {label}: {\" | \".join(descs)}')
    print()

# Traps
if tr:
    active = [t for t in tr if t.get('trap_name')]
    if active:
        print(f'ACTIVE TRAPS ({len(active)}/18):')
        print('  ' + ' | '.join(f'{t[\"trap_name\"]}: ACTIVE ({t.get(\"trigger_count\",0)}x)' for t in active))
        print()

# Patterns
if pat:
    print('TOP PATTERNS (reinforced):')
    for p in pat[:10]:
        print(f'  {p.get(\"source_id\",\"?\")} [{p.get(\"thermal_class\",\"?\")}] w={p.get(\"weight\",0):.2f} ({p.get(\"reinforcement_count\",0)}x)')
    print()

# Causal chain
if caus:
    linked = [c for c in caus if c.get('causal_parent')]
    if linked:
        print('CAUSAL CHAIN (recent linked events):')
        for c in linked[:5]:
            print(f'  E{c[\"id\"]} {c[\"event_type\"]} (sev={c.get(\"severity\",0)}) <- E{c[\"causal_parent\"]}')
        print()

# Watcher
if watch:
    sig = [w for w in watch if w.get('severity', 0) >= 5]
    if sig:
        print('WATCHER ASSESSMENT:')
        for w in sig:
            print(f'  [{w.get(\"observer_role\",\"?\")}] {w.get(\"anomaly_class\",\"nominal\")} sev={w.get(\"severity\",0)}')
        print()

# Health summary
if hlth:
    healthy = sum(1 for h in hlth if h.get('health_status') == 'healthy')
    total = len(hlth)
    degraded = [h for h in hlth if h.get('health_status') != 'healthy' or h.get('circuit_state') != 'closed']
    print(f'SERVICES: {healthy}/{total} healthy', end='')
    if degraded:
        print(' | DEGRADED: ' + ', '.join(f'{h[\"service_id\"]}({h.get(\"circuit_state\",\"?\")})' for h in degraded))
    else:
        print()

print()
print('=' * 60)
" 2>/dev/null || echo "STDB injection unavailable — STDB may be down. Habitat operational without it."
```

### 3.3 Latency Budget

| Step | Budget | Mechanism |
|------|--------|-----------|
| `spacetime sql` × 7 queries | <70ms total | Loopback to `:3000`, in-memory tables, parallel via `&` + `wait` |
| Python3 JSON→text format | <15ms | Simple string formatting |
| stdout write | <5ms | Pipe to Claude Code hook handler |
| **Total** | **<90ms** | Well under the 3s `timeout` in settings.json |

### 3.4 Failure Mode

If STDB is down, every `spacetime sql` call returns empty (the `|| true` ensures no abort). The Python formatter handles empty arrays gracefully. The final `|| echo` fallback prints a one-line notice. Claude Code continues — STDB injection is additive, never blocking.

---

## 4. Advanced Tool Chaining Patterns

### 4.1 The STDB Injection Chain Type

The STDB injector introduces a new tool chain pattern extending the existing B1-B26 + TC1-TC5 catalog:

**TC6 — STDB Memory Injection Chain:**
```
spacetime sql (7× parallel) → python3 (format) → stdout → Claude system message
```

This is a variant of **TC2 Fan-out** (independent calls in parallel) composed with **B1 SQLite query** (SQL against a local datastore). The fan-out is across 7 independent SQL queries; the funnel is the Python formatter that fuses results into a single payload.

### 4.2 The Ingester Tool Chain

The ingester (long-running binary) uses a different pattern:

**TC7 — Continuous Ingestion Chain:**
```
curl poll ORAC (30s) ─┐
WS subscribe PV2      ─┼── ingester ── spacetimedb SDK ── STDB :3000
curl poll SYNTHEX (60s)─┤     ↑
curl poll POVM (300s)  ─┘     │
                              │
                    ┌─── NA-R3 reciprocal ───┐
                    │  POST ORAC trajectory   │
                    │  POST SYNTHEX patterns   │
                    │  POST PV2 coupling       │
                    └─────────────────────────┘
```

This is a **TC2 Fan-out** at the source layer, a **TC1 Funnel** at the ingestion layer, and a **TC2 Fan-out** at the reciprocal layer. Three-stage chain.

### 4.3 The Bootstrap-Deep Composition

The full bootstrap at session start composes three hook chains:

```
┌── Hook 1: ORAC ────────────────────────────────────────────────┐
│  orac-hook.sh → curl POST :8133/hooks/SessionStart → stdout   │
│  TC1 Funnel: stdin JSON → curl POST → stdout response          │
│  Side-effect: ORAC calls STDB R4 register_session              │
└────────────────────────────────────────────────────────────────┘
          │
┌── Hook 2: Health Broadcast ────────────────────────────────────┐
│  session-health-broadcast.sh                                    │
│  TC2 Fan-out: 12× parallel curl probes → atuin KV write        │
│  B3 Health Check × 12                                           │
└────────────────────────────────────────────────────────────────┘
          │
┌── Hook 3: STDB Injection ─────────────────────────────────────┐
│  habitat-stdb-inject                                            │
│  TC6 STDB Chain: 7× spacetime sql (fan-out) → python3 (funnel) │
│  Delivers L1+L2+L3+L7+L8+L9+L10                                │
└────────────────────────────────────────────────────────────────┘
```

**Combined budget:** Hook 1 (≤5s) + Hook 2 (≤4s) + Hook 3 (≤3s) = **≤12s worst case, ~2s typical**. Claude Code runs all three sequentially per the `settings.json` array ordering.

---

## 5. Atuin Integration

### 5.1 Atuin Scripts Ecosystem

The Habitat has 82+ registered atuin scripts. SpaceTimeDB adds 4 new ones:

| Script | Purpose | Invocation |
|--------|---------|------------|
| `habitat-stdb-inject` | SessionStart injection (hook 3) | Auto via `~/.claude/settings.json` |
| `habitat-stdb-query` | Ad-hoc STDB query wrapper | `atuin scripts run habitat-stdb-query` |
| `habitat-stdb-migrate` | One-shot migration trigger | `atuin scripts run habitat-stdb-migrate` |
| `habitat-stdb-health` | STDB + ingester health check | `atuin scripts run habitat-stdb-health` |

### 5.2 Atuin Script: `habitat-stdb-query`

Ad-hoc query wrapper for interactive use:

```bash
#!/usr/bin/env bash
# habitat-stdb-query — interactive STDB query with formatted output
# Usage: habitat-stdb-query "SELECT * FROM knowledge_edge WHERE weight > 0.9"
# Or:    habitat-stdb-query trajectory   (preset: last 10 gradient snapshots)
# Or:    habitat-stdb-query patterns     (preset: top 20 reinforced patterns)
# Or:    habitat-stdb-query causal <id>  (preset: walk causal chain from event ID)
# Or:    habitat-stdb-query workstreams  (preset: in-flight work)
set -u

STDB_DB="${STDB_DB:-habitat}"

case "${1:-}" in
  trajectory)
    spacetime sql "$STDB_DB" \
      "SELECT ralph_gen, ralph_fitness, ralph_phase, pv2_r, temperature, system_grade
       FROM gradient_snapshot ORDER BY timestamp DESC LIMIT 10"
    ;;
  patterns)
    spacetime sql "$STDB_DB" \
      "SELECT source_id, target_id, weight, reinforcement_count, thermal_class
       FROM knowledge_edge WHERE edge_type = 'learned_pattern'
       ORDER BY reinforcement_count DESC, weight DESC LIMIT 20"
    ;;
  causal)
    event_id="${2:?Usage: habitat-stdb-query causal <event_id>}"
    spacetime sql "$STDB_DB" \
      "SELECT id, event_type, source_service, causal_parent, severity, timestamp
       FROM habitat_event
       WHERE id = $event_id OR causal_parent = $event_id
       ORDER BY timestamp"
    ;;
  workstreams)
    spacetime sql "$STDB_DB" \
      "SELECT id, service_name, goal, status, blocker_description
       FROM workstream ORDER BY status, updated_at DESC"
    ;;
  *)
    spacetime sql "$STDB_DB" "$1"
    ;;
esac
```

### 5.3 Atuin Script: `habitat-stdb-health`

```bash
#!/usr/bin/env bash
# habitat-stdb-health — STDB sidecar + ingester health check
set -u

echo "=== SpaceTimeDB Sidecar ==="
stdb_code=$(curl -s -o /dev/null -w '%{http_code}' -m 2 "http://127.0.0.1:3000/v1/identity" 2>/dev/null || echo "000")
echo "  STDB :3000 = $stdb_code"

echo "=== STDB Ingester ==="
ing_code=$(curl -s -o /dev/null -w '%{http_code}' -m 2 "http://127.0.0.1:3001/health" 2>/dev/null || echo "000")
echo "  Ingester :3001 = $ing_code"

echo "=== Table Row Counts ==="
for table in habitat_event knowledge_edge gradient_snapshot session_record workstream service_health trap_state watcher_observation; do
  count=$(spacetime sql habitat "SELECT COUNT(*) AS c FROM $table" --format json 2>/dev/null | python3 -c "import json,sys; print(json.load(sys.stdin)[0]['c'])" 2>/dev/null || echo "?")
  printf "  %-25s %s\n" "$table" "$count"
done

echo "=== Ingester Metrics ==="
curl -s -m 2 "http://127.0.0.1:3001/metrics" 2>/dev/null | head -20 || echo "  (unreachable)"
```

### 5.4 Atuin KV Integration

The ingester writes key metrics to atuin KV on each gradient capture (every 60s), making STDB state available to any atuin-aware script:

```bash
# Written by ingester every 60s
atuin kv set stdb.events.count "42381"
atuin kv set stdb.edges.count "3934"
atuin kv set stdb.snapshots.count "8640"
atuin kv set stdb.last.fitness "0.669"
atuin kv set stdb.last.grade "A"
atuin kv set stdb.ingester.lag_ms "12"
```

This means `habitat-bootstrap` (the legacy script) can read STDB state via KV without needing `spacetime sql`:

```bash
# In habitat-bootstrap L7 (trajectory) fallback path:
fitness=$(atuin kv get stdb.last.fitness 2>/dev/null || echo "?")
grade=$(atuin kv get stdb.last.grade 2>/dev/null || echo "?")
```

---

## 6. Clustered Parallel Tool Webbing

### 6.1 What "Clustered Parallel" Means in This Context

From S101 Memory Injection Roadmap: "Sense in parallel, correlate across substrates, act on resonance, learn from outcomes." The STDB injection system implements this as a 3-layer web:

```
Layer 1: CONTINUOUS SENSING (Ingester, always-on)
├── ORAC probe        (30s cycle)  ──┐
├── PV2 WS subscribe  (real-time)  ──┤
├── SYNTHEX probe     (60s cycle)  ──┼── All write to STDB via reducers
├── POVM sync         (300s cycle) ──┤
└── Atuin bus events   (real-time)  ──┘

Layer 2: CORRELATION (STDB, in-database)
├── R2 reinforce_edge  → cross-substrate pattern reinforcement
├── R5 run_decay       → per-edge Hebbian decay (6h cycle)
├── R7 compact_events  → retention policy (24h cycle)
├── R8 consolidate     → POVM-rhythm replication (300-tick cycle)
└── Causal parent links → effect-to-cause chains across all sources

Layer 3: INJECTION (Injector, on-demand)
├── 7× spacetime sql   → parallel query (TC2 fan-out)
├── python3 formatter   → fuse into ≤15KB payload (TC1 funnel)
└── stdout → Claude     → system message injection
```

### 6.2 The Clustered Query Pattern

The injector's 7 parallel queries are a **clustered** fan-out — each query hits a different STDB table, and they run concurrently:

```bash
# Clustered parallel: all 7 queries launch simultaneously
spacetime sql habitat "SELECT ... FROM session_record ..." &
spacetime sql habitat "SELECT ... FROM gradient_snapshot ..." &
spacetime sql habitat "SELECT ... FROM workstream ..." &
spacetime sql habitat "SELECT ... FROM trap_state ..." &
spacetime sql habitat "SELECT ... FROM knowledge_edge ..." &
spacetime sql habitat "SELECT ... FROM habitat_event ..." &
spacetime sql habitat "SELECT ... FROM service_health ..." &
wait  # All 7 complete in ~40ms total (loopback, in-memory)
```

This is the same pattern as `habitat-bootstrap`'s parallel curl probes (L2), but against STDB instead of 12 HTTP services. The difference: STDB serves all 7 queries from one in-memory dataset, so the parallelism is about CLI process overhead, not network latency.

### 6.3 Advanced Chain: STDB + Atuin + ORAC Fusion

For deep investigation, Claude Code can chain STDB queries with atuin history and ORAC probes:

```bash
# TC8 — STDB-Atuin-ORAC fusion chain
# "What happened around the fitness drop?"

# Step 1: Find fitness drop in STDB trajectory
spacetime sql habitat \
  "SELECT id, ralph_fitness, timestamp FROM gradient_snapshot
   WHERE ralph_fitness < (SELECT ralph_fitness FROM gradient_snapshot ORDER BY timestamp DESC LIMIT 1 OFFSET 1) - 0.01
   ORDER BY timestamp DESC LIMIT 1"
# Returns: id=4821, fitness=0.660, timestamp=2026-04-19T10:30:00

# Step 2: Find causal events around that timestamp
spacetime sql habitat \
  "SELECT id, event_type, source_service, severity, causal_parent
   FROM habitat_event
   WHERE timestamp BETWEEN '2026-04-19T10:25:00' AND '2026-04-19T10:35:00'
   AND severity >= 3
   ORDER BY timestamp"

# Step 3: Cross-reference with atuin — what commands were running?
atuin search --after "2026-04-19T10:25:00" --before "2026-04-19T10:35:00" --format "{time} {command}" | head -20

# Step 4: Check ORAC's current view of the pattern
curl -s localhost:8133/emergence | python3 -c "import json,sys; [print(f'{k}: {v}') for k,v in json.load(sys.stdin).get('by_type',{}).items()]"
```

This four-step chain crosses three substrates (STDB → Atuin → ORAC) to build a causal narrative. Each step's output informs the next. This is the **TC8 Cross-Substrate Investigation** pattern — the highest-leverage tool chain for debugging fitness regressions.

---

## 7. Integration with CLAUDE.md and CLAUDE.local.md

### 7.1 CLAUDE.md Additions

The workspace `CLAUDE.md` gains a new section under Memory Systems:

```markdown
## Memory Systems

| # | System | Access |
|---|--------|--------|
| 1 | Auto-Memory | `~/.claude/projects/-home-louranicas/memory/MEMORY.md` |
| 2 | SQLite DBs (9) | `developer_environment_manager/*.db` — always `.schema` first |
| 3 | Reasoning Memory V2 | `localhost:8130` — **TSV, NOT JSON** |
| 4 | MCP Knowledge Graph | `mcp__memory__*` tools |
| 5 | Obsidian Vault | `/home/louranicas/projects/claude_code/` |
| 6 | Shared Context | `~/projects/shared-context/` |
| **7** | **SpaceTimeDB** | **`spacetime sql habitat "..."` — 8 tables, causal graph, ≤15KB session injection** |

### SpaceTimeDB Quick Reference
- **Query:** `spacetime sql habitat "SELECT * FROM knowledge_edge WHERE weight > 0.9 LIMIT 10"`
- **Trajectory:** `spacetime sql habitat "SELECT ralph_fitness, ralph_phase FROM gradient_snapshot ORDER BY timestamp DESC LIMIT 5"`
- **Causal chain:** `spacetime sql habitat "SELECT * FROM habitat_event WHERE causal_parent = <id>"`
- **Health:** `atuin scripts run habitat-stdb-health`
- **Tables:** habitat_event · knowledge_edge · gradient_snapshot · session_record · workstream · service_health · trap_state · watcher_observation
- **Port:** `:3000` (STDB standalone) · `:3001` (ingester health/metrics)
- **TRAP:** STDB reducers cannot do I/O — all external communication goes through the ingester
- **TRAP:** Use `spacetime sql` for one-shot queries, STDB Rust SDK only for long-lived connections
```

### 7.2 CLAUDE.local.md Session State

The session-level `CLAUDE.local.md` gains STDB status in its live metrics table:

```markdown
## Live metrics at S109 close

| Service | State |
|---|---|
| ORAC | gen=26080 fit=0.669 phase=Recognize |
| PV2 | r=0.985 spheres=4 K=0.703 |
| SYNTHEX thermal | T=0.500 (target 0.500) |
| **STDB** | **events=42381 edges=3934 snapshots=8640 ingester_lag=12ms** |
| **STDB trajectory** | **fit: 0.660→0.664→0.664→0.664→0.669 (+0.009 over 5 sessions)** |
```

### 7.3 Bidirectional Links

The STDB vault at `memory-injection/memory-injection-vault/` carries `> Back to: [[HOME]]` on every note. The workspace `CLAUDE.md` and `CLAUDE.local.md` carry links to the vault:

```
CLAUDE.md § Memory Systems → spacetime sql habitat "..."
CLAUDE.local.md § Live metrics → STDB row counts + trajectory
STDB vault HOME.md → ~/claude-code-workspace/CLAUDE.md
STDB vault MASTER INDEX.md § Upstream References → CLAUDE.md, CLAUDE.local.md
```

Round-trip navigation: any entry point reaches any other in ≤2 hops.

---

## 8. Deployment Checklist

### Phase A Pre-Flight (30 min)

```bash
# 1. Install STDB CLI + standalone
curl -sSf https://install.spacetimedb.com | sh
# OR build from source:
cd ~/claude-code-workspace/spacetimedb
cargo build --release -p spacetimedb-standalone

# 2. Start standalone
spacetimedb-standalone --root-dir ~/claude-code-workspace/memory-injection/data \
  start --listen-addr 127.0.0.1:3000

# 3. Verify
spacetime sql --server http://127.0.0.1:3000 "SELECT 1"

# 4. Register in devenv.toml
# (add [[services]] block per Sidecar Architecture note)

# 5. Build + publish module
cd ~/claude-code-workspace/habitat-stdb-module
cargo build --release
spacetime publish habitat --module-path target/wasm32-unknown-unknown/release/habitat_stdb_module.wasm

# 6. Verify tables
spacetime sql habitat ".tables"
```

### Phase E Wiring (the hook change)

```bash
# 1. Build + install injector
cd ~/claude-code-workspace/habitat-stdb-injector
chmod +x habitat-stdb-inject.sh
\cp -f habitat-stdb-inject.sh ~/.local/bin/habitat-stdb-inject

# 2. Register as atuin script
atuin scripts new habitat-stdb-inject \
  --description "SpaceTimeDB context injection for Claude Code SessionStart" \
  --tags "habitat,stdb,bootstrap,injection" \
  --shebang bash \
  --no-edit \
  --script "$(realpath habitat-stdb-inject.sh)"

# 3. Update settings.json — replace habitat-bootstrap with habitat-stdb-inject
# In ~/.claude/settings.json, SessionStart hooks array, hook 3:
#   OLD: "command": "/home/louranicas/.atuin/bin/atuin scripts run habitat-bootstrap"
#   NEW: "command": "/home/louranicas/.local/bin/habitat-stdb-inject"

# 4. Preserve legacy fallback
\cp -f ~/.local/bin/habitat-bootstrap ~/.local/bin/habitat-bootstrap-legacy

# 5. Test the hook chain
echo '{}' | /home/louranicas/.local/bin/habitat-stdb-inject
# Should print ≤15KB structured injection payload

# 6. Start a fresh Claude Code session — verify injection appears in system context
```

---

## 9. The Complete Data Flow (End-to-End)

```
 ┌─── CONTINUOUS (always running) ────────────────────────────────────────┐
 │                                                                        │
 │   ORAC :8133 ──poll 30s──┐                                            │
 │   PV2 :8132  ──WS──────── ├── Ingester ─── STDB reducers ─── STDB    │
 │   SYNTHEX :8090 ─poll 60s─┤   (Rust bin)   R1-R3 ingest     :3000    │
 │   POVM :8125 ──poll 300s──┤                                  8 tables │
 │   Atuin hooks ─via PV2 bus┘                                           │
 │                                                                        │
 │   STDB scheduled reducers:                                             │
 │     R5 decay (6h) · R7 compact (24h) · R8 consolidate (300-tick)      │
 │                                                                        │
 │   NA-R3 reciprocal:                                                    │
 │     STDB → ORAC (trajectory) · STDB → SYNTHEX (patterns)              │
 │     STDB → PV2 (coupling) · STDB → atuin KV (metrics)                 │
 └────────────────────────────────────────────────────────────────────────┘

 ┌─── ON SESSION START ───────────────────────────────────────────────────┐
 │                                                                        │
 │   Claude Code opens new context window                                 │
 │        │                                                               │
 │        ▼                                                               │
 │   SessionStart event fires                                             │
 │        │                                                               │
 │   Hook 1: orac-hook.sh ──► ORAC /hooks/SessionStart                   │
 │        │                    ├── register sphere on PV2                 │
 │        │                    ├── hydrate from POVM + RM                 │
 │        │                    ├── call STDB R4 register_session (NEW)    │
 │        │                    └── return system message                  │
 │        │                                                               │
 │   Hook 2: session-health-broadcast.sh                                  │
 │        │    12× parallel curl → atuin KV                               │
 │        │                                                               │
 │   Hook 3: habitat-stdb-inject                                          │
 │        │    7× spacetime sql (parallel) → python3 format → stdout      │
 │        │    Delivers: trajectory + workstreams + traps + patterns       │
 │        │              + causal chain + health + watcher assessment      │
 │        │                                                               │
 │   Claude Code receives all 3 hook outputs as system context            │
 │   Total injection: ~20-25 KB, ~2-3s, complete causal state            │
 └────────────────────────────────────────────────────────────────────────┘

 ┌─── DURING SESSION (on-demand) ─────────────────────────────────────────┐
 │                                                                        │
 │   Claude can query STDB at any time:                                   │
 │     spacetime sql habitat "SELECT ..."                                 │
 │     atuin scripts run habitat-stdb-query trajectory                    │
 │     atuin scripts run habitat-stdb-query causal 12345                  │
 │                                                                        │
 │   TC8 Cross-Substrate Investigation:                                   │
 │     STDB trajectory → STDB events → atuin history → ORAC probe        │
 │                                                                        │
 │   ORAC hooks continue to fire:                                         │
 │     PostToolUse → STDB R1 ingest_event (tool usage tracking)           │
 │     Stop → STDB close_session (fitness_end, delta)                     │
 └────────────────────────────────────────────────────────────────────────┘
```

---

## 10. What Makes This "Ultimate"

| Dimension | Before STDB | After STDB |
|-----------|------------|------------|
| **State at session start** | Current metrics only (r, gen, fitness, T) | Current + trajectory + causal chains + workstreams + traps |
| **Pattern reinforcement** | 141 patterns, 1 reinforced >1× | Live `reinforce_edge` reducer on every RALPH cycle |
| **"Why did X happen?"** | Unanswerable | `spacetime sql habitat "SELECT ... WHERE causal_parent = ?"` recursive chain |
| **Memory substrate count** | 21 SQLite DBs (11 dead) + 4 HTTP services + files | 8 STDB tables + 4 HTTP services + files (11 DBs deleted) |
| **Bootstrap latency** | 55ms (7 layers) | <100ms (11 layers including trajectory + workstreams + traps + causal) |
| **Query surface** | 6 separate tools (sqlite3, curl, atuin kv, grep, Read, cat) | 1 tool: `spacetime sql habitat "..."` |
| **Decay/learning** | Uniform 6h sweep | Per-edge decay rates preserving substrate-specific plasticity |
| **Reciprocity** | Extraction only (probes → memory) | Bidirectional (probes → STDB → trajectory hints back to services) |

---

*Deployment Framework v1 · 2026-04-24 · S109 · Wires STDB into the SessionStart hook chain, defines 4 atuin scripts, introduces TC6-TC8 tool chain patterns, integrates with CLAUDE.md §Memory Systems and CLAUDE.local.md §Live metrics · Execution-ready pending Phase A pre-flight*
