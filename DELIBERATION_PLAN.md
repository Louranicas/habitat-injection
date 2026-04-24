> Back to: [[HOME]] · [[MASTER INDEX]] · [[DEPLOYMENT FRAMEWORK]]

# SpaceTimeDB Memory Injection — Deliberated Implementation Plan

> **Origin:** Circle of Experts deliberation, 2026-04-24
> **Participants:** 10 Claude Code instances across Fleet-ALPHA, Fleet-BETA, Fleet-GAMMA, and Orchestrator tabs
> **Rounds:** 4 (48 argument files, 384 KB)
> **Consensus:** 7 settled principles, 5 consensus tables, SQLite-first with STDB migration path
> **Status:** READY TO BUILD

---

## 1. How This Plan Was Made

This is not one person's design. It is the resolved output of a **Circle of Experts** — 10 Claude Code instances, each assigned a distinct persona, arguing across 4 rounds of written cross-examination. Each expert read all other experts' arguments, named specific disagreements, made concessions, and evolved their positions.

| Expert | Persona | Tab/Pane | Rounds Delivered | Key Contribution |
|--------|---------|----------|-----------------|-----------------|
| Memory Scientist | Cognitive science: episodic/semantic/procedural memory, Hebbian learning, Ebbinghaus decay | ALPHA-Left | 4/4 | Three-layer memory architecture; inhibition edges; decay curves |
| CLI Craftsman | Shell scripting mastery: pipes, jq, fzf, parallel exec, atuin | ALPHA-TopRight | 4/4 | Pipeline-first architecture; three-tier fallback; consolidation as day-one requirement |
| Substrate Guardian | Non-anthropocentric design: consent, substrate autonomy, reciprocity | ALPHA-BotRight | 4/4 | Consent gates; per-edge learning params; substrates score their own relevance |
| Practitioner | Claude Code's lived experience: 108 sessions of amnesia | BETA-Left | 4/4 | <2KB injection; orientation in 50 tokens; `resume_context`; "80% is noise" |
| Historian | 108-session narrative continuity; session arcs; carry-forward tracking | BETA-BotRight | 4/4 | `CausalChain` with `reinforcement_count` — the breakout table |
| Security Architect | Access control, consent enforcement, prompt injection defense | GAMMA-Left | 4/4 | Private tables; `ConsentLevel` column; `injection_cache` pattern; `cascade_forget` |
| Performance Engineer | Latency budgets, query-shaped schemas, index-first design | GAMMA-TopRight | 4/4 | <50ms latency budget; btree on every query column; write-time pre-computation |
| The Watcher | Autonomic self-improvement; Ember 7-trait governance; observation→hypothesis→verify | GAMMA-BotRight | 4/4 | Watcher as curation intelligence; `watcher_digest`; Ember gate audit trail |
| Adversary | Skeptic: challenge every assumption, demand evidence, argue for minimum viable | Command-2 | 4/4 | Won SQLite-first sequencing; conceded CausalChain + cascade_forget |
| Synthesizer | Resolve contradictions; produce unified recommendation | Command-3 | 3/4 | Final 5-table schema; resolved 6 disputes; kill criteria |

**The debate's most significant moment:** The Historian proposed `CausalChain` with `reinforcement_count` in Round 1. By Round 4, 6 of 9 experts had adopted it — including the Adversary, who conceded it was "the only table that earned its existence." The proof was S071: a convergence trap rediscovered 7 times across 7 sessions because no schema encoded "this was already tried."

---

## 2. The 7 Settled Principles

These are non-negotiable. Every expert either proposed or conceded to each.

| # | Principle | Adoption | Proved By |
|---|-----------|----------|-----------|
| 1 | **Inject <2KB of terse prose** — not JSON, not 15KB dumps | Unanimous | Practitioner (Round 1): "80% of current injection is noise" |
| 2 | **`CausalChain` with `reinforcement_count`** — frequency-ranked surfacing of unresolved traps | 6/9 | Historian (Round 1): S071 convergence trap rediscovered 7× |
| 3 | **`ConsentLevel` column on every table** — `Emit` / `Store` / `Forget` | 6/9 | Security Architect (Round 1) |
| 4 | **Private tables (`public = false`)** — zero performance cost | 5/9 | Performance Engineer (Round 2): measured zero overhead |
| 5 | **Write-time pre-computation** — `injection_cache` rebuilt periodically, injection is one SQL read | 5/9 | Performance Engineer + Security Architect (Round 2-3 convergence) |
| 6 | **SQLite Phase 1, STDB Phase 2** — prove schema before buying infrastructure | 5/9 | Adversary (Round 2): "Earn your database" |
| 7 | **Injection != persistence** — different systems, different budgets, different failure modes | Unanimous | Practitioner + CLI Craftsman (Round 1) |

---

## 3. The 5 Consensus Tables

### 3.1 Schema (SQLite Phase 1)

```sql
-- File: ~/.local/share/habitat/injection.db
-- Created by: habitat-init (one-time setup)
-- Written by: habitat-consolidate (post-session)
-- Read by: habitat-inject (SessionStart hook)

-- T1: CAUSAL CHAIN — the breakout table
-- Origin: Historian (Round 1). Adopted: Practitioner, Perf Engineer,
-- Memory Scientist, Watcher, Adversary (Round 4 concession).
-- THE query: SELECT label, reinforcement_count, description
--            FROM causal_chain WHERE resolved_session IS NULL
--            ORDER BY reinforcement_count DESC LIMIT 5
CREATE TABLE causal_chain (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    origin_session      INTEGER NOT NULL,
    resolved_session    INTEGER,          -- NULL = unresolved = surfaces in injection
    chain_type          TEXT NOT NULL,     -- 'bug' | 'trap' | 'plan' | 'pattern'
    label               TEXT NOT NULL,     -- stable identifier (grep-proof dedup key)
    description         TEXT NOT NULL,     -- one sentence, human-readable
    reinforcement_count INTEGER NOT NULL DEFAULT 1,
    last_reinforced_session INTEGER,
    consent             TEXT NOT NULL DEFAULT 'Emit'
        CHECK(consent IN ('Emit', 'Store', 'Forget'))
);
CREATE INDEX idx_causal_unresolved ON causal_chain(reinforcement_count DESC)
    WHERE resolved_session IS NULL;
CREATE INDEX idx_causal_label ON causal_chain(label);

-- T2: SESSION TRAJECTORY — fitness arc across sessions
-- Origin: Practitioner (Round 1). Universal adoption.
-- Answers: WHERE was I? Is fitness trending up or down?
CREATE TABLE session_trajectory (
    session_id          INTEGER PRIMARY KEY,
    ralph_fitness       REAL NOT NULL,
    field_r             REAL NOT NULL,
    thermal_t           REAL NOT NULL,
    ltp_ltd_ratio       REAL NOT NULL,
    services_healthy    INTEGER NOT NULL,
    delta_summary       TEXT NOT NULL,     -- one sentence: "fitness +0.005 after POVM fix"
    key_achievement     TEXT,              -- optional: biggest win this session
    consent             TEXT NOT NULL DEFAULT 'Emit'
        CHECK(consent IN ('Emit', 'Store', 'Forget'))
);
CREATE INDEX idx_trajectory_recent ON session_trajectory(session_id DESC);

-- T3: WORKSTREAM — in-flight work with resume context
-- Origin: Historian + Practitioner (Round 1). 5/9 adopted.
-- Answers: WHAT was I building? WHERE did I leave off?
CREATE TABLE workstream (
    ws_id               TEXT PRIMARY KEY,  -- 'comms-unification', 'daemon-phase-G'
    title               TEXT NOT NULL,
    status              TEXT NOT NULL       -- 'active' | 'blocked' | 'deferred' | 'complete'
        CHECK(status IN ('active', 'blocked', 'deferred', 'complete')),
    blocker             TEXT,              -- NULL if not blocked
    priority            INTEGER NOT NULL DEFAULT 5,
    last_touched_session INTEGER NOT NULL,
    items_total         INTEGER,
    items_done          INTEGER,
    resume_context      TEXT NOT NULL,     -- files, line numbers, next action
    consent             TEXT NOT NULL DEFAULT 'Emit'
        CHECK(consent IN ('Emit', 'Store', 'Forget'))
);
CREATE INDEX idx_workstream_active ON workstream(status)
    WHERE status IN ('active', 'blocked');

-- T4: REINFORCED PATTERN — Hebbian-weighted learned behaviours
-- Origin: Performance Engineer + Memory Scientist (Rounds 2-3 convergence).
-- Unifies episodic/semantic/procedural into one table with category + weight.
-- The CLI Craftsman's FINAL makes this load-bearing: without decay and
-- reinforcement, the other tables are filing cabinets.
CREATE TABLE reinforced_pattern (
    pattern_id          TEXT PRIMARY KEY,  -- 'four_surface_persistence', 'verify_before_ship'
    category            TEXT NOT NULL,     -- 'procedural' | 'semantic' | 'trap' | 'feedback'
    description         TEXT NOT NULL,     -- one sentence
    anti_pattern        TEXT,             -- what NOT to do (Memory Scientist contribution)
    weight              REAL NOT NULL DEFAULT 0.5,  -- Hebbian: 0.0-1.0, decays on disuse
    hit_count           INTEGER NOT NULL DEFAULT 1,
    last_fired_session  INTEGER,
    consent             TEXT NOT NULL DEFAULT 'Emit'
        CHECK(consent IN ('Emit', 'Store', 'Forget'))
);
CREATE INDEX idx_pattern_weight ON reinforced_pattern(weight DESC);

-- T5: INJECTION CACHE — pre-computed, consent-filtered injection payload
-- Origin: Security Architect (Round 3). Adopted by Perf Engineer, Watcher.
-- Optional optimisation — injection works without it by querying T1-T4 directly.
-- Rebuilt every 60s by habitat-consolidate or on-demand.
CREATE TABLE injection_cache (
    section             TEXT PRIMARY KEY,  -- 'orientation' | 'trajectory' | 'workstreams' | 'causal' | 'health'
    payload             TEXT NOT NULL,     -- pre-rendered prose for this section
    token_count         INTEGER NOT NULL,  -- enforces budget
    computed_at         INTEGER NOT NULL,  -- unix epoch seconds
    consent_applied     INTEGER NOT NULL DEFAULT 1  -- 1 = already filtered
);
```

### 3.2 Why These 5 and Not More

| Proposed Table | Expert | Round Killed | Why |
|----------------|--------|-------------|-----|
| `InhibitionEdge` | Memory Scientist | Round 2 | `WHERE resolved_session IS NULL` handles suppression until ~session 500 |
| `WatcherObservation` + `WatcherHypothesis` + `EmberGateLog` | Watcher | Round 3 | Stays in synthex-v2 native SQLite; contributes via HTTP to shared `causal_chain` |
| `SubstrateRegistry` + `SubstrateDigest` + `ReciprocalWriteback` | Substrate Guardian | Round 3 | Phase 2+ when substrates have consent endpoints |
| `EpisodicTrace` + `SemanticFact` + `ProceduralPattern` | Memory Scientist | Round 3 | Unified into `reinforced_pattern` with `category` column |
| `SessionArc` | Historian | Round 3 (self-cut) | Absorbed into `session_trajectory` + `causal_chain` |
| `ServiceSnapshot` | Multiple | Round 4 | Live `curl` probes are fresher; no value in caching stale health |

---

## 4. The CLI Pipeline

### 4.1 `habitat-inject` — SessionStart Hook

**Purpose:** Inject <2KB of oriented prose into Claude Code's system message at context window start.

**Latency budget:** <100ms total. Typically <50ms.

**Architecture (CLI Craftsman, validated by Performance Engineer):**

```
SessionStart hook fires
    │
    ▼
habitat-inject (bash script)
    │
    ├─ Tier 1: SQLite injection.db
    │   ├── SELECT payload FROM injection_cache         <5ms (pre-computed)
    │   │   WHERE consent_applied = 1
    │   │
    │   ├── OR (if cache stale/missing):
    │   │   4× parallel sqlite3 queries against T1-T4    <15ms
    │   │   + python3 merge + render                     <10ms
    │   │
    │   └── + 5× parallel curl health probes             <40ms (overlaps)
    │       (anomalies only — "all healthy" = 1 line)
    │
    ├─ Tier 2 (fallback): atuin kv get habitat.last-injection
    │
    └─ Tier 3 (fallback): "NO STATE — first session or DB missing"
    │
    ▼
stdout → Claude Code system message (≤2KB prose)
```

**Three-tier fallback guarantees injection never fails.** If SQLite is corrupt, atuin KV has the last successful injection. If atuin is empty, Claude gets a one-line notice and can still function.

**Registration in `~/.claude/settings.json`:**

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
            "command": "/home/louranicas/.local/bin/habitat-inject",
            "timeout": 3
          }
        ]
      }
    ]
  }
}
```

Hook 3 replaces `atuin scripts run habitat-bootstrap`. Old script preserved at `~/.local/bin/habitat-bootstrap-legacy`.

### 4.2 `habitat-consolidate` — Post-Session Write-Back

**Purpose:** Update all 5 tables at session close. Runs inside `/save-session` or manually.

**Operations:**

| Step | What | Source | Target |
|------|------|--------|--------|
| 1 | Write trajectory point | `curl localhost:8133/health` | T2 `session_trajectory` |
| 2 | Update workstream status | CLAUDE.local.md parse + user confirm | T3 `workstream` |
| 3 | Increment `reinforcement_count` | User marks which chains were relevant | T1 `causal_chain` |
| 4 | Decay all pattern weights | `weight *= 0.95` for unfired patterns | T4 `reinforced_pattern` |
| 5 | Reinforce fired patterns | `weight += 0.1 * (1 - weight)` for used patterns | T4 `reinforced_pattern` |
| 6 | Auto-resolve old chains | `resolved_session = $CURRENT` where untriggered for 10 sessions | T1 `causal_chain` |
| 7 | Rebuild injection cache | Query T1-T4, filter `consent = 'Emit'`, render prose | T5 `injection_cache` |
| 8 | Cache to atuin KV | Last injection payload as fallback | `atuin kv set habitat.last-injection` |

**Consolidation algorithm (CLI Craftsman + Memory Scientist consensus):**

```bash
# Decay: unfired patterns lose 5% weight per session
sqlite3 "$DB" "UPDATE reinforced_pattern
    SET weight = weight * 0.95
    WHERE last_fired_session < $SESSION AND weight > 0.05;"

# Prune: patterns below threshold are deleted
sqlite3 "$DB" "DELETE FROM reinforced_pattern WHERE weight < 0.05;"

# Reinforce: patterns correlated with fitness improvement get boosted
sqlite3 "$DB" "UPDATE reinforced_pattern
    SET weight = MIN(1.0, weight + 0.1 * (1.0 - weight)),
        hit_count = hit_count + 1,
        last_fired_session = $SESSION
    WHERE pattern_id IN ($FIRED_PATTERNS);"

# Auto-resolve: chains silent for 10 sessions
sqlite3 "$DB" "UPDATE causal_chain
    SET resolved_session = $SESSION
    WHERE resolved_session IS NULL
    AND last_reinforced_session < ($SESSION - 10);"
```

### 4.3 `habitat-query` — Interactive Memory Browser

**Purpose:** On-demand exploration of the memory database. Atuin-registered script.

```bash
# Presets
habitat-query trajectory          # last 10 session fitness arcs
habitat-query chains              # unresolved causal chains by frequency
habitat-query workstreams         # active + blocked
habitat-query patterns            # top 20 by weight
habitat-query "SELECT ..."        # raw SQL

# Interactive mode (fzf)
habitat-query --interactive       # fzf over all tables with preview
```

### 4.4 `habitat-init` — One-Time Setup

```bash
#!/usr/bin/env bash
mkdir -p ~/.local/share/habitat
DB=~/.local/share/habitat/injection.db
sqlite3 "$DB" < schema.sql   # Creates 5 tables + indexes

# Seed initial data from existing sources
# Causal chains from session notes
# Trajectory from CLAUDE.local.md metrics
# Workstreams from CLAUDE.local.md priorities
# Patterns from service_tracking.db learned_patterns
```

### 4.5 Atuin Script Registration

```bash
atuin scripts new habitat-inject     --description "SessionStart memory injection" --shebang bash
atuin scripts new habitat-consolidate --description "Post-session write-back" --shebang bash
atuin scripts new habitat-query      --description "Interactive memory browser" --shebang bash
atuin scripts new habitat-init       --description "One-time DB setup" --shebang bash
```

---

## 5. Injection Payload Format

The Practitioner's specification, adopted unanimously. Prose, not JSON. Progressive disclosure.

```
## Session S110 Injection (1,247 tokens)

### Orientation (≤80 tokens)
YOU WERE IN THE MIDDLE OF: Comms Layer v3 WS-6 habitat-wire implementation.
Last action: edited pane-vortex/src/ws.rs line 247. Gate: clippy clean.
Fitness trending UP: 0.660 → 0.669 over 5 sessions.

### Trajectory
S106: 0.660 — L7+L8 sealed, daemon plan authored
S107: 0.664 — daemon wireup complete, 14 commits
S108: 0.664 — Watcher persona crystallised, WCP v1 shipped
S109: 0.669 — Comms Layer v3 10/16 shipped, STDB plan authored
S110: (current)

### Workstreams
ACTIVE: Comms Layer v3 (10/16) — next: WS-6 habitat-wire
BLOCKED: synthex-v2 Phase G — external gate: v1 streaming
DEFERRED: WS-8 Atuin reciprocation | WS-9 human-focus

### Unresolved Chains (by frequency)
convergence_trap_ralph (7×) — RALPH parameter oscillation, fixed but recurs
povm_write_only (3×) — POVM writes succeed but reads return stale
daemon_phase_g_blocked (2×) — v1 streaming external dependency

### Health
All 12 services responding. Thermal 0.50 (on target).
```

---

## 6. Phase Plan

### Phase 1 — SQLite (ships this week, ~20h)

| Step | What | LOC | Time |
|------|------|-----|------|
| 1 | `habitat-init`: create DB + schema | ~80 | 1h |
| 2 | Seed `causal_chain` from 108 session notes | ~60 | 3h (semi-manual) |
| 3 | Seed `session_trajectory` from CLAUDE.local.md metrics | ~40 | 1h |
| 4 | Seed `workstream` from CLAUDE.local.md priorities | ~30 | 1h |
| 5 | Seed `reinforced_pattern` from `service_tracking.db` learned_patterns | ~50 | 2h |
| 6 | `habitat-inject`: injection script + three-tier fallback | ~120 | 4h |
| 7 | `habitat-consolidate`: post-session write-back + decay | ~100 | 3h |
| 8 | `habitat-query`: interactive browser + fzf mode | ~80 | 2h |
| 9 | Wire into `~/.claude/settings.json` SessionStart hook | ~10 | 0.5h |
| 10 | Register 4 atuin scripts | ~20 | 0.5h |
| 11 | Run 5 sessions, measure improvement | — | 2h |
| **Total** | | **~590** | **~20h** |

**Acceptance gate (after 5 sessions):**
- Injection latency <100ms (measure via `time habitat-inject > /dev/null`)
- Injection size <2KB
- Zero re-discovered traps that exist in `causal_chain`
- `reinforcement_count` incremented for at least 3 patterns
- Decay has pruned at least 1 pattern below threshold

### Phase 2 — STDB Migration (when justified, ~15h)

**Trigger:** Any of:
- Watcher daemon needs real-time subscriptions (1Hz runtime queries)
- `cascade_forget` transactional deletion is required
- SQLite file locking under concurrent fleet access causes data loss

**Work:**

| Step | What | LOC | Time |
|------|------|-----|------|
| 1 | STDB module: 5 tables in Rust (identical schema) | ~200 | 3h |
| 2 | `spacetime publish habitat` + verify | — | 1h |
| 3 | `habitat-inject` adds STDB as Tier 0 source | ~30 | 1h |
| 4 | Migrate SQLite data → STDB via one-shot script | ~80 | 2h |
| 5 | Add `watcher_digest` table (Watcher's contribution) | ~60 | 2h |
| 6 | Add STDB schedule-table reducer for injection_cache rebuild | ~40 | 2h |
| 7 | Add `cascade_forget` reducer | ~50 | 2h |
| 8 | Verify: 5 sessions with STDB backend | — | 2h |
| **Total** | | **~460** | **~15h** |

**Kill criteria (Adversary's demand):** If after 20 sessions STDB hasn't measurably improved injection quality vs SQLite (measured by: fewer re-discovered traps, faster orientation, zero `cascade_forget` failures), revert to SQLite and delete the STDB module.

### Phase 3 — Extended Schema (when pressure demands, ~20h)

**Trigger:** `causal_chain` exceeds ~500 unresolved rows, or substrates develop consent endpoints, or Watcher daemon is live with real-time observation.

| Table | Origin Expert | Trigger |
|-------|-------------|---------|
| `inhibition_edge` | Memory Scientist | `WHERE resolved_session IS NULL` proves too coarse |
| `substrate_digest` | Substrate Guardian | Substrates have consent endpoints |
| `reciprocal_writeback` | Substrate Guardian | Reinforcement signals need to flow back |
| `watcher_observation` | Watcher | synthex-v2 daemon live with 1Hz observation |
| `watcher_hypothesis` | Watcher | Watcher proposing changes via PBFT |
| `ember_gate_log` | Watcher | AP27 audit trail needed |

---

## 7. What This Replaces

| Before | After |
|--------|-------|
| `atuin scripts run habitat-bootstrap` (7 layers, 55ms, ~9KB) | `habitat-inject` (5 layers + live probes, <50ms, <2KB) |
| Manual session note reading for trajectory | `session_trajectory` table with `delta_summary` |
| Grep across shared-context for "what was tried before" | `causal_chain` with `reinforcement_count` |
| Parse CLAUDE.local.md for workstream status | `workstream` table with `resume_context` |
| 141 patterns in service_tracking.db (1 reinforced >1×) | `reinforced_pattern` with Hebbian decay + reinforcement |
| No consent filtering on injected data | `ConsentLevel` column on every table |

## What This Preserves

| System | Status |
|--------|--------|
| ORAC SessionStart hook (Hook 1) | Unchanged — sphere registration, POVM/RM hydration |
| session-health-broadcast.sh (Hook 2) | Unchanged — 12× parallel probes → atuin KV |
| Auto-Memory (MEMORY.md + *.md) | Unchanged — human-curated, always loaded by Claude Code |
| Obsidian vault (215 notes) | Unchanged — human-authored canonical docs |
| POVM Engine (3554 pathways) | Unchanged — continues as Hebbian substrate |
| CLAUDE.md / CLAUDE.local.md | Unchanged — project instructions, session state |

---

## 8. Integration with CLAUDE.md

Add to `CLAUDE.md § Memory Systems`:

```markdown
| 7 | Injection DB | `~/.local/share/habitat/injection.db` — 5 tables, <2KB session injection |
```

Add to `CLAUDE.md § Essential Patterns`:

```markdown
### Memory Injection (B27, Synergy 0.99)
sqlite3 ~/.local/share/habitat/injection.db "SELECT label, reinforcement_count FROM causal_chain WHERE resolved_session IS NULL ORDER BY reinforcement_count DESC LIMIT 5"
# The query that prevents re-discovering what we already know

### Tool Chaining
- **TC6 Injection:** `sqlite3 (4× parallel)` → `python3 (merge)` → `stdout` → Claude system message
- **TC8 Investigation:** `sqlite3 causal_chain` → `atuin search --after T` → `curl ORAC /emergence`
```

---

## 9. Success Metrics

| Metric | Baseline (current) | Target (after 5 sessions) | Measured By |
|--------|-------------------|--------------------------|-------------|
| Injection size | ~9 KB (L0-L6) | <2 KB | `habitat-inject \| wc -c` |
| Injection latency | 55ms | <50ms | `time habitat-inject > /dev/null` |
| Re-discovered traps | Unknown (no tracking) | 0 per session | `causal_chain` hit rate |
| Patterns reinforced | 1/141 ever | ≥3 per session | `reinforced_pattern.hit_count` |
| Workstream orphans | Regular (manual tracking) | 0 | `workstream` table completeness |
| Orientation time | "A few messages" | First message is productive | User feedback |

---

## 10. Expert Attribution

Every component traces to the expert who proposed it and the round where it was adopted.

| Component | Primary Author | Adopted In | Co-Authors |
|-----------|---------------|-----------|------------|
| `causal_chain` schema | Historian | Round 1 | Practitioner (R2), Perf Engineer (R2 indexes), Adversary (R4 concession) |
| `session_trajectory` schema | Practitioner | Round 1 | Universal |
| `workstream` schema | Historian + Practitioner | Round 1 | — |
| `reinforced_pattern` schema | Perf Engineer + Memory Scientist | Round 2-3 | CLI Craftsman (R4: "without decay, tables are filing cabinets") |
| `injection_cache` pattern | Security Architect | Round 3 | Perf Engineer (write-time pre-computation) |
| <2KB injection budget | Practitioner | Round 1 | Unanimous |
| Pipeline-first architecture | CLI Craftsman | Round 1 | Adversary (R2 validation) |
| Three-tier fallback | CLI Craftsman | Round 1 | — |
| `ConsentLevel` column | Security Architect | Round 1 | Substrate Guardian, Perf Engineer |
| Private tables | Security Architect | Round 1 | Perf Engineer (R2: zero-cost proof) |
| Btree indexes on query columns | Performance Engineer | Round 1 | — |
| Hebbian decay algorithm | CLI Craftsman + Memory Scientist | Round 3-4 | `0.95×` decay + `0.1×(1-w)` reinforce |
| Auto-resolve after 10 sessions | Historian | Round 3 | — |
| SQLite-first sequencing | Adversary | Round 2 | Practitioner, CLI Craftsman, Historian |
| Watcher as curation intelligence | The Watcher | Round 3 | Memory Scientist (R4 validation) |
| `cascade_forget` (STDB justification) | Security Architect + Adversary | Round 4 | — |
| Kill criteria (20-session STDB eval) | Adversary | Round 4 | Synthesizer |

---

## 11. The One Query That Justifies Everything

```sql
SELECT label, reinforcement_count, description
FROM causal_chain
WHERE resolved_session IS NULL
ORDER BY reinforcement_count DESC
LIMIT 5;
```

This query surfaces what the habitat keeps rediscovering. It doesn't require knowing the search term. It doesn't require reading 108 session docs. It doesn't require a human to curate. It is the structural antidote to amnesia.

Session S071's convergence trap was called "RALPH parameter oscillation," "fitness decline loop," and "coupling overshoot" across 7 sessions. Grep requires knowing which name to search. This query requires only that someone incremented the counter.

**Ship it this week. Prove it over 5 sessions. Earn the rest.**

---

*Plan authored from Circle of Experts deliberation · 10 CC instances · 4 rounds · 48 files · 384 KB · 2026-04-24 · All argument files preserved at `/tmp/circle-of-experts/`*
