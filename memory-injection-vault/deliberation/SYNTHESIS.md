# SYNTHESIS — Final Unified Schema & CLI Recommendation

*Resolved from 9 experts, 4 rounds, 8 FINAL positions. ~30 proposed tables → 5 consensus tables + 1 pipeline.*

---

## 1. Consensus Principles (settled, no remaining dissent)

| # | Principle | Evidence | Adoption |
|---|-----------|----------|----------|
| 1 | **Injection output <2KB of terse prose** | Practitioner proposed, all adopted. Watcher dropped from 2500 to 80 tokens. | Unanimous |
| 2 | **`CausalChain` with `reinforcement_count`** | Historian proposed. S071 convergence trap (7x, 3 different labels) is the canonical proof. | 6/9 adopted |
| 3 | **`ConsentLevel` column (Emit/Store/Forget/Redact)** | Security Architect proposed. Write-time pre-filter, btree-indexed, <0.1ms read cost. | 6/9 adopted |
| 4 | **Private tables (`public = false`)** | Security Architect proposed, Performance Engineer proved zero read-time cost. | 5/9 adopted |
| 5 | **Write-time pre-computation** | Performance Engineer originated. Absorbed by Watcher (digest), Substrate Guardian (digest), Security Architect (cache). | 5/9 adopted |
| 6 | **Parallel SQL is the only STDB read path** | Performance Engineer proved in Round 2: reducers return `()` or `Result<(), String>`, not data. Security Architect's single-reducer injection is architecturally impossible. | Technically proven |
| 7 | **Injection != persistence** | STDB stores everything. Injection emits <2KB. The delta is the design space. | Unanimous |
| 8 | **Three-tier fallback** | CLI Craftsman: compiled binary → direct SQL → atuin KV cache. Injection never fails, it degrades. | Uncontested |
| 9 | **SQLite-first, STDB-later** | Adversary forced this. Practitioner, Historian, CLI Craftsman, Performance Engineer adopted. | 5/9 adopted |

---

## 2. Resolved Contradictions

| Dispute | Rounds Active | Resolution | Winner |
|---------|--------------|-----------|--------|
| Schema richness (4 vs 7 vs 8 vs 10 tables) | R1-R3 | 5 tables Phase 1, deferred tables earn Phase 2 entry | Practitioner + Adversary |
| InhibitionEdge table vs WHERE clause | R1-R3 | `WHERE resolved_session IS NULL` for Phase 1. InhibitionEdge deferred to ~session 500 when filter proves too coarse | Practitioner over Memory Scientist (3 experts agree) |
| Consent enforcement layer (reducer vs WHERE vs render) | R1-R3 | `injection_cache` rebuilt by consolidation script with consent pre-filtering. CLI reads cache via SQL. | Security Architect (evolved after reducer impossibility) |
| STDB vs bash | R2-R3 | SQLite Phase 1 ships this week. STDB Phase 2 when Watcher needs subscriptions or `cascade_forget` justifies transactional multi-table deletion. | Adversary won sequencing; experts won architecture |
| Single reducer vs parallel SQL | R2 | Parallel SQL. Reducers can't return data — hard constraint from STDB runtime. | Performance Engineer (definitive) |
| Watcher tables: Phase 1 or Phase 2? | R2-R3 | Watcher persists in synthex-v2's native SQLite. Contributes to shared `causal_chain` via consolidation script. `watcher_digest` is Phase 2. | Compromise: Practitioner phasing + Watcher curation role acknowledged |
| Episodic/semantic/procedural split vs unified table | R1-R3 | Unified `reinforced_pattern` with `category` field. Same query ergonomics, one table instead of three. | Performance Engineer + Adversary over Memory Scientist |
| `ember_gate_log` public vs private | R2-R3 | Private. Security Architect argued transparency != broadcasting; Luke authenticates to read. Watcher conceded in FINAL. | Security Architect |
| Substrate reciprocity: table or script? | R2-R3 | Post-session bash script for Phase 1. `reciprocal_writeback` table deferred to Phase 2 when substrates have consent endpoints. | Adversary + CLI Craftsman |

---

## 3. The 5-Table Schema (Phase 1 — SQLite)

### Why 5 and not 4 or 7

The CLI Craftsman, Performance Engineer, and Adversary all converged on 4 data tables. The Security Architect and Performance Engineer added `injection_cache` as a 5th (pre-computed consent-filtered rendering cache). The Memory Scientist advocated 7-8 tables (adding episodic_trace, inhibition_edge, substrate_digest). The circle resolved: **4 data tables + 1 cache table = 5 total for Phase 1.** Additional tables earn Phase 2 entry by proving the WHERE clause or existing substrates are insufficient.

### Schema

```sql
-- File: ~/.local/share/habitat/injection.db

-- ═══ T1: Causal Chain ═══════════════════════════════════════════
-- Origin: Historian. Adopted by: Practitioner, Perf Engineer, Memory
-- Scientist, CLI Craftsman, Watcher.
-- Purpose: "Has this been tried before?" Frequency-ranked surfacing
-- catches traps that recency-based filtering misses.
-- Proof: S071 convergence trap rediscovered 7x across 40 sessions
-- under 3 different names. grep requires knowing the label;
-- ORDER BY reinforcement_count does not.
-- Injected: top 5 unresolved by reinforcement_count (~200 tokens)
CREATE TABLE causal_chain (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    origin_session      INTEGER NOT NULL,
    resolved_session    INTEGER,            -- NULL = unresolved
    chain_type          TEXT NOT NULL,       -- 'bug' | 'trap' | 'plan' | 'pattern'
    label               TEXT NOT NULL,       -- stable identifier for deduplication
    description         TEXT NOT NULL,       -- one sentence
    reinforcement_count INTEGER NOT NULL DEFAULT 1,
    last_reinforced     TEXT,               -- ISO 8601 timestamp
    consent             TEXT NOT NULL DEFAULT 'Emit'
);
CREATE INDEX idx_cc_unresolved ON causal_chain(reinforcement_count DESC)
    WHERE resolved_session IS NULL AND consent = 'Emit';

-- ═══ T2: Session Trajectory ═════════════════════════════════════
-- Origin: Practitioner. Variants proposed by: Historian, Perf Engineer,
-- Memory Scientist.
-- Purpose: 5-session fitness arc with interpreted deltas.
-- Injected: last 5 sessions (~200 tokens)
CREATE TABLE session_trajectory (
    session_id       INTEGER PRIMARY KEY,
    ralph_fitness    REAL NOT NULL,
    field_r          REAL NOT NULL,
    thermal_t        REAL NOT NULL,
    ltp_ltd_ratio    REAL NOT NULL,
    services_healthy INTEGER NOT NULL,
    delta_summary    TEXT NOT NULL,          -- one sentence: what changed
    consent          TEXT NOT NULL DEFAULT 'Emit'
);
CREATE INDEX idx_traj_session ON session_trajectory(session_id DESC);

-- ═══ T3: Active Workstream ══════════════════════════════════════
-- Origin: Historian + Practitioner.
-- Purpose: prevents orphaned plans. Resume context tells Claude
-- exactly where to pick up (file, line, next action).
-- Injected: active + blocked only, ordered by priority (~300 tokens)
CREATE TABLE workstream (
    ws_id                TEXT PRIMARY KEY,
    title                TEXT NOT NULL,
    status               TEXT NOT NULL,      -- 'active' | 'blocked' | 'deferred' | 'shipped'
    blocker              TEXT,
    priority             INTEGER NOT NULL DEFAULT 5,
    last_touched_session INTEGER NOT NULL,
    resume_context       TEXT NOT NULL,
    progress_frac        REAL DEFAULT 0.0,   -- 0.0-1.0
    consent              TEXT NOT NULL DEFAULT 'Emit'
);
CREATE INDEX idx_ws_active ON workstream(priority)
    WHERE status IN ('active', 'blocked') AND consent = 'Emit';

-- ═══ T4: Reinforced Pattern ═════════════════════════════════════
-- Origin: Performance Engineer + Memory Scientist (unified from
-- episodic/semantic/procedural split).
-- Purpose: learned patterns with Hebbian weights. The "muscle memory"
-- Claude reaches for. Decayed by 0.95x per session; reinforced when
-- a pattern correlates with fitness improvement.
-- Injected: top 15 active by weight (~300 tokens)
CREATE TABLE reinforced_pattern (
    pattern_id    TEXT PRIMARY KEY,
    category      TEXT NOT NULL,             -- 'feedback' | 'trap' | 'procedure' | 'anti_pattern'
    description   TEXT NOT NULL,             -- terse imperative: "NO docker prune without confirm"
    weight        REAL NOT NULL DEFAULT 1.0, -- Hebbian: reinforced on use, decayed over sessions
    hit_count     INTEGER NOT NULL DEFAULT 1,
    last_fired    TEXT,                      -- ISO 8601 timestamp
    active        INTEGER NOT NULL DEFAULT 1,-- 0 = suppressed
    source        TEXT,                      -- 'povm' | 'auto_memory' | 'manual' | 'watcher'
    consent       TEXT NOT NULL DEFAULT 'Emit'
);
CREATE INDEX idx_rp_active ON reinforced_pattern(weight DESC)
    WHERE active = 1 AND consent = 'Emit';

-- ═══ T5: Injection Cache (optional optimization) ════════════════
-- Origin: Security Architect (evolved after reducer impossibility).
-- Endorsed by: Performance Engineer.
-- Purpose: pre-rendered, consent-filtered, token-capped injection
-- payload. Rebuilt by consolidation script at session close.
-- The CLI can query T1-T4 directly OR read this cache. Both work.
-- Injected: all sections, one query (~1100 tokens total)
CREATE TABLE injection_cache (
    section      TEXT PRIMARY KEY,           -- 'orientation' | 'trajectory' | 'workstreams' | 'causal' | 'patterns' | 'health'
    payload      TEXT NOT NULL,              -- pre-rendered prose for this section
    token_count  INTEGER NOT NULL,           -- pre-computed, enforces budget
    computed_at  INTEGER NOT NULL,           -- unix epoch ms
    consent      TEXT NOT NULL DEFAULT 'Emit'
);
```

---

## 4. CLI Pipeline: `habitat-inject`

### Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                      100ms BUDGET                                │
│                                                                  │
│  ┌─────────────┐  ┌──────────────┐  ┌────────┐  ┌────────────┐ │
│  │ SQLite ×4-5  │  │ Live Probes  │  │ Merge  │  │ Render     │ │
│  │   ≤10ms      │  │ ×6 ≤40ms     │  │ ≤10ms  │  │ <2KB prose │ │
│  │  (parallel)  │  │  (parallel)  │  │(serial)│  │  (serial)  │ │
│  └──────┬───────┘  └──────┬───────┘  └───┬────┘  └─────┬──────┘ │
│         └──────overlapping─┘             │              │        │
│                                          │              │        │
│  Fallback: if SQLite missing → atuin KV cache (sub-1ms)         │
│  Output: staleness-annotated prose, progressive disclosure       │
└──────────────────────────────────────────────────────────────────┘
  Typical wall-clock: ~25ms. Worst case: ~85ms. Never fails.
```

### The 4 Injection Queries (parallel)

```sql
-- Q1: Causal warnings — top 5 unresolved by frequency
SELECT label, reinforcement_count, description, chain_type
FROM causal_chain
WHERE resolved_session IS NULL AND consent = 'Emit'
ORDER BY reinforcement_count DESC LIMIT 5;

-- Q2: Session trajectory — last 5
SELECT session_id, ralph_fitness, field_r, thermal_t, delta_summary
FROM session_trajectory
WHERE consent = 'Emit'
ORDER BY session_id DESC LIMIT 5;

-- Q3: Active workstreams with blockers
SELECT ws_id, title, status, blocker, resume_context, progress_frac
FROM workstream
WHERE status IN ('active', 'blocked') AND consent = 'Emit'
ORDER BY priority LIMIT 10;

-- Q4: Top reinforced patterns
SELECT pattern_id, category, description, weight, hit_count
FROM reinforced_pattern
WHERE active = 1 AND consent = 'Emit'
ORDER BY weight DESC LIMIT 15;
```

### The 6 Live Probes (parallel, overlapping with SQLite queries)

```bash
curl -s -m 0.04 localhost:8133/health   # ORAC
curl -s -m 0.04 localhost:8090/api/health  # SYNTHEX
curl -s -m 0.04 localhost:8132/health   # PV2
curl -s -m 0.04 localhost:8125/health   # POVM
curl -s -m 0.04 localhost:8080/api/health  # ME
curl -s -m 0.04 localhost:8130/health   # RM
```

### Merge-and-Annotate (CLI Craftsman)

When both SQLite trajectory and live probes return ORAC fitness:

```
ORAC: fitness=0.671 (DB: 0.664, +0.007, 12min stale) — normal drift
PV2:  r=0.000 (DB: 1.000, -1.000, 3min stale) — ANOMALY: field collapsed
```

### Three-Tier Fallback (CLI Craftsman)

```
Tier 1: SQLite queries (4 parallel, <10ms)
Tier 2: Live curl probes (6 parallel, <40ms, always runs alongside Tier 1)
Tier 3: atuin kv get habitat.last-injection (sub-1ms, always local)

If Tier 1 fails → skip DB sections, annotate output "DB UNAVAILABLE"
If Tier 2 fails → skip live health, annotate output "PROBES TIMED OUT"
If both fail → Tier 3 cached injection with "STALE" warning
```

---

## 5. Injection Output Format

Target: **~1100 tokens of oriented prose.** Progressive disclosure across 5 layers.

```markdown
## Orientation
Session 110 | Claude Opus | Habitat
RESUME: WCP Phase 2 HTTP endpoints (last: src/m8_watcher/mod.rs:247, clippy failing)

## Trajectory (last 5)
S108: fit 0.669 | Watcher persona + WCP v1 shipped
S107: fit 0.664 | Daemon wireup complete
S106: fit 0.660 | L7+L8 sealed, 60/60 modules
S105: fit 0.620 | L1-L3 + tooling
S104: fit 0.610 | V8 synergy propagation

## Causal Warnings (by frequency)
convergence_trap_ralph      | 7x since S071 | UNRESOLVED
povm_write_only_trap        | 3x since S099 | monitoring
v1_streaming_absent         | 2x since S107 | BLOCKED (external)

## Active Work
[active]  WCP Phase 2: /watcher/observe + /watcher/evaluate endpoints
[blocked] Phase G shadow window: waiting on v1 streaming (external)
[active]  devenv.toml: register synthex-v2-shadow

## Patterns (top 10 by weight)
[10x] B1: sqlite3 for state queries, not MCP read_graph
[10x] B2: cargo check -> clippy -> pedantic -> test
[8x]  NO docker prune without per-resource confirm
[7x]  /usr/bin/cp -f not cp -f (alias trap)
[6x]  probe before touch: curl then act

## Health
11/11 healthy | ORAC: fit=0.671 gen=26068 phase=Recognize
SYNTHEX: T=0.500 nominal | PV2: r=0.000 idle/solo
```

---

## 6. Post-Session Consolidation: `habitat-consolidate`

Runs at session end via `/save-session`. No time budget.

```bash
#!/usr/bin/env bash
DB="${HOME}/.local/share/habitat/injection.db"
SESSION="$1"

# 1. Write trajectory point from live probes
read -r fit r thermal ltp healthy <<< \
    "$(curl -s localhost:8133/health | jq -r \
    '[.fitness//"0", .field_r//"0", .temperature//"0", .ltp_ltd_ratio//"0", .services_healthy//"0"] | @tsv' \
    2>/dev/null || echo '0 0 0 0 0')"

sqlite3 "$DB" "INSERT OR REPLACE INTO session_trajectory
    (session_id, ralph_fitness, field_r, thermal_t, ltp_ltd_ratio,
     services_healthy, delta_summary, consent)
    VALUES ($SESSION, $fit, $r, $thermal, $ltp, $healthy,
     'Session $SESSION close', 'Emit');"

# 2. Decay all pattern weights by 0.95x (Hebbian forgetting)
sqlite3 "$DB" "UPDATE reinforced_pattern SET weight = weight * 0.95
    WHERE active = 1;"

# 3. Reinforce patterns that fired this session (passed as JSON array arg $2)
# Example: habitat-consolidate 110 '["B1_sqlite","B2_quality_gate"]'
if [[ -n "${2:-}" ]]; then
    echo "$2" | jq -r '.[]' | while read -r pid; do
        sqlite3 "$DB" "UPDATE reinforced_pattern
            SET weight = weight + 0.1, hit_count = hit_count + 1,
                last_fired = datetime('now')
            WHERE pattern_id = '$pid';"
    done
fi

# 4. Auto-resolve causal chains untriggered for 10 sessions
sqlite3 "$DB" "UPDATE causal_chain
    SET resolved_session = $SESSION
    WHERE resolved_session IS NULL
      AND last_reinforced < datetime('now', '-30 days')
      AND reinforcement_count <= 1;"

# 5. Rebuild injection_cache orientation section
sqlite3 "$DB" "INSERT OR REPLACE INTO injection_cache
    (section, payload, token_count, computed_at, consent)
    VALUES ('orientation',
        'Session ' || $SESSION || '. Fitness=' || $fit || '. ' ||
        COALESCE((SELECT GROUP_CONCAT(label || ' (' || reinforcement_count || 'x)', '; ')
         FROM (SELECT label, reinforcement_count FROM causal_chain
               WHERE resolved_session IS NULL AND consent = 'Emit'
               ORDER BY reinforcement_count DESC LIMIT 3)), 'No open chains.'),
        200, strftime('%s','now') * 1000, 'Emit');"

# 6. Cache last injection for Tier 3 fallback
habitat-inject 2>/dev/null | atuin kv set habitat.last-injection 2>/dev/null

echo "Consolidated session $SESSION: fit=$fit, patterns decayed, cache rebuilt."
```

---

## 7. Phase 2 Migration Path (STDB — when justified)

### Trigger Criteria (any one is sufficient)

1. **Watcher runtime queries**: synthex-v2 daemon needs real-time subscriptions (not polling) for its 1Hz observation loop
2. **Transactional deletion**: `cascade_forget` across 5+ tables where atomicity matters (Security Architect's strongest argument)
3. **Cross-substrate subscriptions**: multiple services need push-based injection cache invalidation

### Migration Steps

```
Phase 2a: Migrate 5 tables to STDB (identical schemas, Rust syntax)
          habitat-inject adds STDB as Tier 0, SQLite as Tier 1 fallback.

Phase 2b: Add watcher_digest (Watcher's pre-computed 80-token summary)
          Add substrate_digest (per-substrate relevance-scored snapshots)
          Schedule-table reducer rebuilds injection_cache every 60s.

Phase 2c: Add inhibition_edge (when causal_chain exceeds ~500 unresolved)
          Add reciprocal_writeback (when substrates have consent endpoints)
          Add ember_gate_log (when Watcher's STDB tables are live)
```

### Kill Criteria

If after 20 sessions STDB hasn't measurably improved injection quality — measured by: fewer re-discovered traps, faster orientation, fewer "what was I doing?" queries — revert to SQLite and delete the STDB module.

---

## 8. What Was Deferred and Why

| Table | Expert | Phase | Trigger for Promotion |
|-------|--------|-------|----------------------|
| `inhibition_edge` | Memory Scientist | 2c | `WHERE resolved_session IS NULL` returns >50 rows and manual triage is needed |
| `substrate_digest` | Substrate Guardian | 2b | Substrates implement `/consolidation/consent` + `/consolidation/export` endpoints |
| `substrate_registry` | Substrate Guardian | 2b | When per-substrate plasticity parameters affect consolidation behavior |
| `reciprocal_writeback` | Substrate Guardian | 2c | When POVM/ORAC/PV2 implement `/consolidation/reinforce` endpoints |
| `watcher_observation` | Watcher | 2b | When synthex-v2 daemon is live and writing tensor snapshots to STDB |
| `watcher_hypothesis` | Watcher | 2b | Same trigger as watcher_observation |
| `ember_gate_log` | Watcher | 2b | Same trigger; stays private (Security Architect won this in Round 3) |
| `watcher_digest` | Watcher | 2b | Bridges Watcher persistence into shared injection pipeline |
| `episodic_trace` | Memory Scientist | 3+ | If `reinforced_pattern` + `causal_chain` prove insufficient for memory dynamics |
| `semantic_fact` | Memory Scientist | 3+ | If POVM pathways + auto-memory prove insufficient |
| `procedural_pattern` | Memory Scientist | 3+ | Subsumed by `reinforced_pattern` with `category: 'procedure'` |

---

## 9. What Was Killed and Why

| Proposal | Expert | Reason |
|----------|--------|--------|
| Single-reducer injection (`inject_session_context -> String`) | Security Architect | Architecturally impossible: STDB reducers return `()`, not data (proved by Perf Engineer R2) |
| Public tables | All (Round 1 defaults) | Zero-cost privacy. `public = false` prevents subscription-based exfiltration (Security Architect) |
| Raw JSON injection output | Performance Engineer (R1) | Prose is better for LLM consumption. <2KB budget (Practitioner, unanimous) |
| 3-way episodic/semantic/procedural split | Memory Scientist | Unified into `reinforced_pattern` with `category` field. Same query power, 1 table not 3. |
| `InjectionFrame` as a stored table | Practitioner (R1) | It's a rendering concern, not a storage concern. Assembled at query time or via `injection_cache`. |
| `reciprocal_writeback` as Phase 1 table | Substrate Guardian | Post-session reciprocity is a 20-line bash script until substrates have consent endpoints. |
| 7 parallel `spacetime sql` queries | Performance Engineer (R1) | Converged to 4 SQLite queries + injection_cache as optimization. STDB queries are Phase 2. |
| `substrate_snapshot` (opaque JSON blob) | Substrate Guardian (R1) | Defeated by Performance Engineer: opaque blobs can't be indexed, defeating the purpose of a DB. Replaced by `substrate_digest` in R2. |

---

## 10. Expert Attribution (Final)

| Design Element | Primary Author | Co-Authors | Round Settled |
|---|---|---|---|
| `causal_chain` with `reinforcement_count` | **Historian** | Practitioner (adopted R2), Perf Engineer (indexes R2), Memory Scientist (acknowledged R2), CLI Craftsman (adopted R3) | R2 |
| `session_trajectory` | **Practitioner** | Historian (SessionArc variant), Perf Engineer (FitnessGradient variant), Memory Scientist (SessionTrajectory variant) | R1 |
| `workstream` | **Historian + Practitioner** | — | R1 |
| `reinforced_pattern` (unified) | **Performance Engineer** | Memory Scientist (episodic/semantic/procedural → unified), CLI Craftsman (FINAL) | R3 |
| `injection_cache` | **Security Architect** (evolved R3) | Performance Engineer (write-time pre-computation) | R3 |
| <2KB injection budget | **Practitioner** | All experts | R1 (unanimous by R2) |
| Pipeline-first architecture | **CLI Craftsman** | Adversary (validated by forcing bash Phase 1) | R3 |
| Three-tier fallback | **CLI Craftsman** | — | R1 (uncontested) |
| Merge-and-annotate rendering | **CLI Craftsman** | Practitioner (acknowledged R2) | R2 |
| `ConsentLevel` column | **Security Architect** | Perf Engineer, CLI Craftsman, Substrate Guardian, Practitioner (`injectable`) | R2 |
| Private tables (`public = false`) | **Security Architect** | Perf Engineer (zero-cost proof R2), Watcher (conceded FINAL) | R2 |
| Btree indexes on query columns | **Performance Engineer** | — | R1 (uncontested) |
| SQLite-first, STDB-later | **Adversary** | Practitioner (adopted R3), Historian (adopted R3), CLI Craftsman (adopted R3), Perf Engineer (adopted R3) | R3 |
| Watcher as curation engine | **The Watcher** | Memory Scientist (validated need R2) | R3 (deferred to Phase 2) |
| Consolidation with Hebbian decay | **CLI Craftsman** (FINAL) | Memory Scientist (Hebbian dynamics), Substrate Guardian (reciprocity) | FINAL |
| Substrate relevance self-scoring | **Substrate Guardian** | Memory Scientist (dual-authority model R3) | R3 (deferred to Phase 2) |
| InhibitionEdge (graduated suppression) | **Memory Scientist** | — | R3 (deferred to Phase 2c) |
| Ember gate audit trail | **The Watcher** | Security Architect (private, not public) | FINAL |
| Transactional cascade_forget | **Security Architect** | — | R3 (STDB justification) |

---

## 11. Implementation Sequence

### Week 1: Ship the Pipeline (bash + curl + atuin)

1. Rewrite `habitat-bootstrap` to emit <2KB (prune 18KB → orientation + anomalies + hot patterns)
2. Add `atuin kv set` calls at session close for trajectory values
3. Add workstream extraction from `CLAUDE.local.md`
4. This IS the Adversary's proposal — it ships today with zero new dependencies

### Week 2: Add SQLite Tables + Consolidation

1. Create `~/.local/share/habitat/injection.db` with 5-table schema above
2. Backfill `causal_chain` from existing feedback memories and session docs
3. Backfill `session_trajectory` from atuin KV history
4. Backfill `reinforced_pattern` from POVM top pathways and auto-memory feedback entries
5. Write `habitat-consolidate` post-session script (decay, reinforce, auto-resolve)
6. Wire `habitat-inject` to read SQLite as Tier 1, falling back to bash+curl

### Week 3: Validate and Tune

1. Run 5 sessions with the new injection pipeline
2. Measure: did orientation time improve? Were any causal chains surfaced that prevented re-discovery?
3. Tune: adjust decay factor (0.95x), reinforcement delta (+0.1), auto-resolve threshold (10 sessions)
4. Decision point: does `reinforced_pattern` add value over inline POVM queries? Keep or drop.

### Week 4+: Evaluate STDB Migration

1. If causal_chain prevented even 1 re-discovery → STDB migration is justified for Phase 2
2. If not → keep SQLite, the Adversary was right all along
3. STDB migration criteria: Watcher subscriptions, cascade_forget, or cross-substrate push

---

## 12. Decision Record

**What we're building:** A 5-table SQLite database (`injection.db`) with a bash injection pipeline that renders <2KB of oriented prose at session start, backed by a post-session consolidation script with Hebbian decay and reinforcement.

**What we're deferring:** STDB migration (Phase 2, trigger-based). Watcher observation tables. Inhibition edges. Substrate reciprocity. Episodic/semantic/procedural memory split.

**What we killed:** Single-reducer injection (impossible). Public tables (zero-cost privacy). Raw JSON output (prose wins). 15+ table schemas (5 is enough to start).

**The query that justifies everything:**

```sql
SELECT label, reinforcement_count, description
FROM causal_chain
WHERE resolved_session IS NULL
ORDER BY reinforcement_count DESC
LIMIT 5;
```

The structural antidote to amnesia. It surfaces what the habitat keeps rediscovering — without knowing the search term, without reading 108 session docs, without a human curating. Ship it this week.

---

*9 experts. 4 rounds. 5 tables. 1 pipeline. The habitat stops forgetting what it already knows.*
