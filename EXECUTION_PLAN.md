# habitat-injection — Phase 1 Execution Plan

> **Status:** LIBRARY COMPLETE, CLI + DEPLOYMENT PENDING
> **Created:** 2026-04-24 (S110)
> **Scope:** 4 CLI binaries + data seeding + hook wiring + validation
> **Estimated:** ~20h across 4-5 sessions (S110-S114)
> **Prerequisite:** L1-L6 library (27 modules, 1696 tests, hardened)

---

## 1. Architecture Overview

```
                    ┌──────────────────────────────────────────────┐
                    │           Claude Code Session Start           │
                    └──────────┬───────────────────────────────────┘
                               │ SessionStart hook fires
                               ▼
            ┌──────────────────────────────────────────────┐
            │  Hook 1: orac-hook.sh (EXISTING — unchanged)  │
            │  ORAC sphere registration + POVM/RM hydration │
            └──────────────────┬───────────────────────────┘
                               ▼
            ┌──────────────────────────────────────────────┐
            │  Hook 2: health-broadcast.sh (EXISTING)       │
            │  12× parallel health probes → atuin KV        │
            └──────────────────┬───────────────────────────┘
                               ▼
            ┌──────────────────────────────────────────────┐
            │  Hook 3: habitat-inject (NEW — replaces       │
            │           atuin scripts run habitat-bootstrap) │
            │                                               │
            │  ┌─ Tier 1: SQLite injection_cache ──────┐   │
            │  │  SELECT payload WHERE section =        │   │
            │  │  'full_payload' AND age < 60s          │   │
            │  └────────────────────────────────────────┘   │
            │  ┌─ Tier 2: atuin KV fallback ───────────┐   │
            │  │  atuin kv get habitat.last-injection    │   │
            │  └────────────────────────────────────────┘   │
            │  ┌─ Tier 3: static fallback ─────────────┐   │
            │  │  "NO INJECTION STATE — first session"  │   │
            │  └────────────────────────────────────────┘   │
            └──────────────────┬───────────────────────────┘
                               ▼
                    ┌──────────────────────────────────┐
                    │  stdout → Claude system message   │
                    │  <2KB prose, <100ms               │
                    └──────────────────────────────────┘
```

---

## 2. The 4 CLI Binaries

### 2.1 `habitat-init` (Step 1)

**Purpose:** One-time database setup.
**Binary:** `src/bin/habitat_init.rs` → `~/.local/bin/habitat-init`
**Dependencies:** L1 (config), L2 (schema)

```
habitat-init [--config PATH] [--db PATH]
  1. Resolve DB path (arg > env > config > default)
  2. Call m06_schema::open_database(path)
  3. Print: "Created injection.db at {path} (6 tables, schema v{N})"
```

### 2.2 `habitat-inject` (Step 6)

**Purpose:** SessionStart hook — produces <2KB prose injection.
**Binary:** `src/bin/habitat_inject.rs` → `~/.local/bin/habitat-inject`
**Dependencies:** L1 (config, types), L2 (schema), L3 (parallel_query, prose_renderer, fallback, consent_filter)

```
habitat-inject [--db PATH] [--budget TOKENS] [--session NUM]
  1. Try Tier 1: execute_cached(conn)
  2. If miss: execute_all(conn, config) → filter → render → write_cache
  3. If DB fail: try_atuin_kv()
  4. If all fail: static_fallback()
  5. Print payload to stdout
  6. Save to atuin KV (best-effort)
  Exit 0 ALWAYS — never block session start
```

### 2.3 `habitat-consolidate` (Step 7)

**Purpose:** Post-session write-back — called by /save-session.
**Binary:** `src/bin/habitat_consolidate.rs` → `~/.local/bin/habitat-consolidate`
**Dependencies:** L1-L4

```
habitat-consolidate --session NUM [--fired-patterns P1,P2,...] [--from-checkpoint PATH]
  1. Capture trajectory (curl ORAC /health → insert_point)
  2. Run Hebbian cycle (decay → reinforce → prune → auto-resolve)
  3. Ingest checkpoint (if --from-checkpoint)
  4. Rebuild injection cache
  5. Save to atuin KV
  Print: "Consolidated S{NUM}: {decayed} decayed, {reinforced} reinforced, {pruned} pruned"
```

### 2.4 `habitat-query` (Step 8)

**Purpose:** Interactive memory browser.
**Binary:** `src/bin/habitat_query.rs` → `~/.local/bin/habitat-query`
**Dependencies:** L1-L2, L5

```
habitat-query trajectory          # last 10 sessions
habitat-query chains              # unresolved by frequency
habitat-query workstreams         # active + blocked
habitat-query patterns            # top 20 by weight
habitat-query summary             # one-line counts
habitat-query "SELECT ..."        # raw SQL passthrough
habitat-query --interactive       # fzf browser
```

---

## 3. Data Seeding (Steps 2-5)

### 3.1 Seed Sources

| Target Table | Source | Rows | Method |
|-------------|--------|------|--------|
| `causal_chain` | Session notes (S001-S108) | ~15-25 | Parse BUG-NNN + known traps from `~/projects/shared-context/sessions/*.md` |
| `session_trajectory` | CLAUDE.local.md metrics | ~10 | Parse "Live metrics at S{N} close" sections |
| `workstream` | CLAUDE.local.md priorities | ~6 | Parse "Comms Layer v3 (10/16)" style entries |
| `reinforced_pattern` | `service_tracking.db` learned_patterns | ~141 | `sqlite3` SELECT → transform → INSERT |
| `injection_cache` | Computed | 1 | Run `habitat-consolidate` after seeding |

### 3.2 Seed Script Architecture

```
habitat-seed [--db PATH] [--source all|chains|trajectory|workstreams|patterns]
  ├── seed_chains_from_sessions(conn, session_dir)
  │   └── For each .md: extract_bug_references + extract_trap_references
  │       → insert_chain or reinforce_chain
  │
  ├── seed_trajectory_from_local_md(conn, claude_local_path)
  │   └── Parse metric tables → insert_point for each session
  │
  ├── seed_workstreams_from_local_md(conn, claude_local_path)
  │   └── Parse priority sections → insert_workstream
  │
  └── seed_patterns_from_tracking_db(conn, tracking_db_path)
      └── SELECT * FROM learned_patterns → insert_pattern for each
```

---

## 4. Hook Wiring (Steps 9-10)

### 4.1 settings.json Change

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "orac-hook.sh SessionStart 5",
            "timeout": 6
          },
          {
            "type": "command",
            "command": "session-health-broadcast.sh",
            "timeout": 4
          },
          {
            "type": "command",
            "command": "/home/louranicas/.local/bin/habitat-inject",
            "timeout": 3
          }
        ]
      }
    ]
  }
}
```

### 4.2 Atuin Script Registration

```bash
atuin scripts new habitat-init        --description "One-time injection DB setup"
atuin scripts new habitat-inject      --description "SessionStart memory injection (<2KB, <100ms)"
atuin scripts new habitat-consolidate --description "Post-session Hebbian write-back"
atuin scripts new habitat-query       --description "Interactive injection DB browser"
```

---

## 5. Validation (Step 11)

### 5.1 Acceptance Criteria

| Metric | Target | Measurement |
|--------|--------|-------------|
| Injection latency | <100ms | `time habitat-inject > /dev/null` |
| Injection size | <2KB | `habitat-inject \| wc -c` |
| Re-discovered traps | 0 per session | Manual check against `causal_chain` |
| Patterns reinforced | ≥3 per session | `habitat-query patterns` before/after |
| Decay prunes ≥1 | Weight < 0.05 | `habitat-query "SELECT count(*) FROM reinforced_pattern WHERE weight < 0.05"` |

### 5.2 Five-Session Validation Protocol

```
Session N:   habitat-inject runs at start → note orientation quality
             Work normally for the session
             /save-session → habitat-consolidate --session N
             habitat-query summary → record counts

Session N+4: Compare: fewer traps rediscovered? Faster orientation?
             habitat-query chains → verify reinforcement_count > 1
             habitat-query patterns → verify weight distribution shifted
```

---

## 6. Session-by-Session Execution Schedule

| Session | Steps | Deliverables | Gate |
|---------|-------|-------------|------|
| **S110** | 1, 6 (partial) | `habitat-init` binary, `habitat-inject` binary (Tier 1+3) | DB creates, inject produces output |
| **S111** | 2, 3, 4, 5 | Seed scripts, populated DB | `habitat-query summary` shows data |
| **S112** | 6 (complete), 7 | `habitat-inject` with Tier 2, `habitat-consolidate` | Full 3-tier fallback works |
| **S113** | 8, 9, 10 | `habitat-query`, hook wiring, atuin registration | Hook fires on session start |
| **S114** | 11 | 5-session validation begins | Metrics tracked |

---

## 7. Risk Register

| Risk | Impact | Mitigation |
|------|--------|-----------|
| `habitat-inject` > 100ms | Noticeable delay at session start | Pre-computed cache (Tier 1 is <5ms) |
| Seed data quality low | Injection contains noise | Manual review of seeded chains before enabling hook |
| ORAC health endpoint changes | Trajectory capture breaks | Version-pin the health JSON schema |
| atuin KV write conflicts | Tier 2 fallback stale | Tier 3 static always works |
| `service_tracking.db` schema drift | Pattern seeding fails | `.schema` check before seed |

---

## 8. File Map — What Gets Created

```
src/bin/
  habitat_init.rs          # Step 1 — one-time DB setup
  habitat_inject.rs        # Step 6 — SessionStart hook binary
  habitat_consolidate.rs   # Step 7 — post-session write-back
  habitat_query.rs         # Step 8 — interactive browser
  habitat_seed.rs          # Steps 2-5 — data seeding

~/.local/bin/
  habitat-init             # release binary
  habitat-inject           # release binary (Hook 3)
  habitat-consolidate      # release binary
  habitat-query            # release binary

~/.local/share/habitat/
  injection.db             # the database (created by habitat-init)

~/.claude/settings.json    # Hook 3 registration (Step 9)
```

---

*Execution plan authored S110 · Prerequisite: L1-L6 library complete (2de4e2f + 8b61a89)*
*Back to: [[HOME]] · [[MASTER INDEX]] · [[Implementation Status]]*
