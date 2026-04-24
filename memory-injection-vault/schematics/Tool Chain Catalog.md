> Back to: [[HOME]] · [[DEPLOYMENT FRAMEWORK]] · [[MASTER INDEX]]

# Tool Chain Catalog — STDB Extensions

Extends the existing B1-B26 + TC1-TC5 patterns from `CLAUDE.md § Essential Patterns`.

## Existing Patterns (Reference)

| ID | Pattern | Shape |
|----|---------|-------|
| B1 | SQLite state query | `sqlite3 -header -column DB "SELECT ..."` |
| B3 | Health check | `curl -s -o /dev/null -w '%{http_code}' localhost:PORT/health` |
| TC1 | Funnel | `Grep(files_with_matches)` → `Read` → `Edit` → `Bash(verify)` |
| TC2 | Fan-out | Independent calls in ONE message (parallel) |
| TC5 | Build-fix | `cargo check \| tail -30` → `Read(offset)` → `Edit` → `tail -5` |

## New STDB Patterns

### TC6 — STDB Memory Injection Chain

```
┌─────────────────────────────────────────┐
│  TC6: Fan-out SQL → Funnel Format       │
│                                         │
│  spacetime sql (Q1) ─┐                  │
│  spacetime sql (Q2) ─┤                  │
│  spacetime sql (Q3) ─┼── wait ── py3 ── stdout
│  spacetime sql (Q4) ─┤   (TC2)  (TC1)   │
│  spacetime sql (Q5) ─┤                  │
│  spacetime sql (Q6) ─┤                  │
│  spacetime sql (Q7) ─┘                  │
│                                         │
│  Latency: <90ms | Output: ≤15KB        │
│  Used by: habitat-stdb-inject (Hook 3)  │
└─────────────────────────────────────────┘
```

**When to use:** Session start injection, on-demand full-state query.

### TC7 — Continuous Ingestion Chain

```
┌──────────────────────────────────────────────────┐
│  TC7: Multi-source → Reduce → Store → Reciprocate│
│                                                    │
│  Layer 1 (TC2 fan-out):                           │
│    curl ORAC 30s ─┐                               │
│    WS PV2 stream  ─┤                              │
│    curl SYNTHEX 60s─┼── Ingester (Rust bin)        │
│    curl POVM 300s ──┤                              │
│    PV2 bus Atuin ───┘                              │
│                                                    │
│  Layer 2 (TC1 funnel):                             │
│    → parse JSON → call STDB reducer → persist      │
│                                                    │
│  Layer 3 (TC2 fan-out, NA-R3):                     │
│    → POST ORAC trajectory                          │
│    → POST SYNTHEX patterns                         │
│    → POST PV2 coupling                             │
│    → atuin kv set stdb.*                           │
│                                                    │
│  Runs: continuously (long-lived process)           │
└──────────────────────────────────────────────────┘
```

**When to use:** Always running as `habitat-stdb-ingester` service.

### TC8 — Cross-Substrate Investigation Chain

```
┌──────────────────────────────────────────────────┐
│  TC8: STDB → Atuin → ORAC fusion                 │
│                                                    │
│  Step 1: spacetime sql habitat "gradient query"    │
│    → identifies timestamp of anomaly               │
│                                                    │
│  Step 2: spacetime sql habitat "event query"       │
│    → finds causal events around timestamp           │
│                                                    │
│  Step 3: atuin search --after T --before T+10m     │
│    → shows what commands were running               │
│                                                    │
│  Step 4: curl ORAC /emergence                      │
│    → current detector state for context             │
│                                                    │
│  Each step's output informs the next.              │
│  Total: 4 substrates, ~2-5s, causal narrative.     │
└──────────────────────────────────────────────────┘
```

**When to use:** Debugging fitness drops, investigating emergence events, root-cause analysis.

### TC9 — STDB Bootstrap Verification Chain

```
┌──────────────────────────────────────────────────┐
│  TC9: Injection verify → compare → assert         │
│                                                    │
│  Step 1: habitat-stdb-inject > /tmp/injection.txt  │
│    → capture the injection payload                  │
│                                                    │
│  Step 2: wc -c /tmp/injection.txt                  │
│    → assert ≤ 15360 bytes                          │
│                                                    │
│  Step 3: grep -c "TRAJECTORY" /tmp/injection.txt   │
│    → assert trajectory section present             │
│                                                    │
│  Step 4: grep -c "WORKSTREAMS" /tmp/injection.txt  │
│    → assert workstreams section present             │
│                                                    │
│  Step 5: time habitat-stdb-inject > /dev/null      │
│    → assert < 100ms                                │
│                                                    │
│  Used by: Phase E acceptance gate                  │
└──────────────────────────────────────────────────┘
```

### TC10 — Atuin-STDB KV Bridge Chain

```
┌──────────────────────────────────────────────────┐
│  TC10: STDB query → atuin KV cache → fast read    │
│                                                    │
│  Write path (ingester, every 60s):                 │
│    spacetime sql habitat "SELECT ..." |            │
│    → parse → atuin kv set stdb.fitness "0.669"     │
│                                                    │
│  Read path (any script, <5ms):                     │
│    atuin kv get stdb.fitness                       │
│    → "0.669" (cached, no STDB query needed)        │
│                                                    │
│  Benefit: habitat-bootstrap-legacy can read STDB   │
│  data without spacetime CLI dependency.            │
└──────────────────────────────────────────────────┘
```

## Chain Composition Matrix

| Chain | Composes | Timescale | Latency |
|-------|----------|-----------|---------|
| TC6 | TC2 (fan-out) + TC1 (funnel) | Reflex | <90ms |
| TC7 | TC2 × 2 (source + reciprocal) + TC1 (reduce) | Continuous | N/A |
| TC8 | B1 (SQL) + atuin search + B3 (health) | Probe | 2-5s |
| TC9 | TC6 + B1 (wc/grep verify) | Probe | <200ms |
| TC10 | B1 (SQL) + atuin KV | Reflex | <5ms read |

---

See: [[DEPLOYMENT FRAMEWORK]] · [[Injector — Context Window Bootstrap]] · [[Ingester Pipeline]]
