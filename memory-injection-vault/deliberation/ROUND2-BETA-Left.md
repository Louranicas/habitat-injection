# THE PRACTITIONER — Final Position (Rounds 1-3 Synthesis)

## The Debate Has Converged

Three rounds. Nine experts. The fog cleared. Here is what I know — not as an advocate, but as the agent who will wake up tomorrow and use whatever this circle builds.

---

## Consensus Map (What's Settled)

| Principle | Round Settled | Originator | Dissenters |
|-----------|:---:|------------|------------|
| Injection <2KB prose, not raw data | R1 | **Me** | None |
| CausalChain with `reinforcement_count` | R2 | Historian | None (even Adversary conceded R3) |
| Private tables + ConsentLevel column | R2 | Security Architect | CLI Craftsman (weak holdout) |
| Parallel SQL is the only read path | R2 | Performance Engineer | None (proved reducers can't return data) |
| Injection ≠ persistence | R2 | **Me** + Watcher | None |
| Write-time pre-computation | R2 | Performance Engineer | None |
| SQLite-first, STDB when justified | R3 | Adversary forced this | Memory Scientist, Watcher (want STDB day-one) |
| Consolidation algorithm ships with tables | R3 | Memory Scientist | None |
| Pipeline-first, backend-swappable | R3 | CLI Craftsman | None |

Seven of nine settled principles. Two dissenters on SQLite-first (Memory Scientist wants 8 tables in STDB, Watcher argues synthex-v2 already runs STDB so tables are "free"). Everyone else converged.

---

## What I Contributed That Stuck

1. **<2KB injection output.** Every expert adopted this. The Watcher dropped from 2,500 tokens to 80. The Performance Engineer conceded "I was measuring microseconds when I should have been measuring tokens."

2. **InjectionFrame as the delivery format.** Renamed, reshaped, absorbed into various experts' schemas — but the concept (a single pre-rendered orientation payload, not raw table dumps) is now universal.

3. **"Orientation, not completeness."** The thesis held. The Memory Scientist's 8-table architecture, the Substrate Guardian's 10-table proposal, the Watcher's 4 observation tables — all evolved to inject a *digest*, not their full state. Everyone now agrees that persistence is wide and injection is narrow.

4. **Progressive disclosure layers.** Layer 0 orientation → Layer 1 trajectory → Layer 2 workstreams → Layer 3 causal memory. This structure survived intact across all three rounds.

---

## What I Got Wrong

1. **Round 1 had no CausalChain.** The Historian's S071 convergence trap argument was unanswerable. Frequency-based surfacing ("this has been tried 7 times") catches traps that recency-based `hot_traps` misses. I adopted it in Round 2 and every subsequent expert confirmed it was the right call. The Adversary — the hardest critic — conceded in Round 3 that CausalChain "justifies structured storage."

2. **Round 1 had no consolidation algorithm.** The Memory Scientist correctly identified that `reinforcement_count` and `hot_traps` are fields that assume a curation process without specifying one. The 40-line consolidation script (decay weights × 0.95/session, Hebbian reinforcement from fitness delta, auto-resolution after 10 quiet sessions) is load-bearing infrastructure, not optional.

3. **Round 1 had no btree indexes.** The Performance Engineer correctly identified that `ORDER BY reinforcement_score DESC` without an index degrades to O(n log n) at scale. This costs nothing to add and saves everything at year two.

4. **Round 1 stripped provenance from hot_traps.** The Substrate Guardian correctly argued that "BUG-064i" is less actionable than "POVM: BUG-064i (LTD=0.01)." Same token cost, vastly more useful for targeted follow-up.

5. **Round 1 had no curation engine.** The Watcher correctly asked "who populates hot_traps?" My answer ("a reducer") was handwaving. The Watcher's `watcher_digest` — one row, ~80 tokens, pre-curated by m46 Observer — is the right bridge. For Phase 1, a 100-line consolidation script replaces the Watcher. For Phase 2, the Watcher replaces the script. The schema doesn't change; the data source does.

---

## What I Defended Successfully

1. **Entity-shaped tables over query-shaped denormalization** (vs Performance Engineer). The Performance Engineer proposed 7 tables each mapping 1:1 to an injection query. I argued that unknown queries matter more than known-query speed. The CLI Craftsman agreed: "the pipeline is non-negotiable, the tables are negotiable." The Performance Engineer partially conceded, adopting the injection cache pattern (one pre-computed cache table) over 7 denormalized tables.

2. **Bash pipeline before compiled binary** (vs CLI Craftsman). We're 8x under the 40ms latency budget. The CLI Craftsman proposed a Rust binary. I argued debuggability beats speed when you're far from the ceiling. The Adversary validated this: "50 lines of bash ships today." The CLI Craftsman evolved to "pipeline-first, backend-swappable" — which is my position restated.

3. **Public vs private tables are a thin concern at localhost** (vs Security Architect). The Security Architect's threat model (any localhost process can subscribe to public STDB tables) is technically valid but practically thin for a single-user workstation. That said, the cost of private tables is zero, so I adopted `public = false` as defense-in-depth. The CLI Craftsman held out longer than I did here, and was also right: the real security boundary is the machine, not the DB.

4. **Resolved_session IS NULL replaces InhibitionEdge** (vs Memory Scientist). The Memory Scientist wanted graduated inhibition (0.0-1.0 strength) via a separate table. Three experts (me, Historian, Performance Engineer) agreed that a WHERE clause handles suppression for the first 500 sessions. The Memory Scientist evolved to split inhibition into two layers (STDB-internal vs substrate-computed), which is a reasonable Phase 2 concern.

5. **Phase 2 for Watcher/Substrate/Memory layers** (vs Watcher, Substrate Guardian, Memory Scientist). The Watcher argued their tables are Phase 1 because "the Watcher is the curation engine." The Substrate Guardian argued reciprocity is Phase 1 because "without it STDB becomes a 22nd dead database." Both are architecturally correct and operationally premature. Ship the injection foundation first. Build the observation loop on top. The Historian confirmed: "I build for session 110 that needs to prove the concept works before investing in a 6-table memory architecture."

---

## Response to The Adversary (Final)

The Adversary asked the only question that mattered: *"Name the session that failed because the data wasn't in SpaceTimeDB."*

**S073.** The convergence trap from S071 was rediscovered because no system surfaced it automatically. The shared-context doc existed. Grep could have found it — if the fresh Claude knew to grep for "convergence." But the trap was called "RALPH parameter issue" in S071, "LTP/LTD ratio collapse" in S073, and "idle gating" in S075. Three names for the same bug. Grep requires knowing the label. `ORDER BY reinforcement_count DESC LIMIT 5` requires nothing.

The Adversary conceded this in Round 3: *"I concede: CausalChain justifies structured storage (SQLite at minimum)."* They also conceded the contract boundary argument (STDB ingestion breaks visibly at write time; bash scripts break silently at read time when upstream APIs change format). And they conceded that synthex-v2 already running STDB eliminates the "service #13" operational cost.

What the Adversary still holds: **50 lines of bash ships today. SQLite ships this week. STDB ships when justified by measured improvement.** I agree with this sequencing completely.

---

## Final Schema

```sql
-- Phase 1: SQLite (ships this week)
-- Single file: ~/.local/share/habitat/injection.db

CREATE TABLE injection_frame (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    created_at INTEGER NOT NULL,          -- unix epoch ms
    orientation_line TEXT NOT NULL,        -- 80 tokens max
    interrupted_task TEXT,                 -- null if clean start
    last_gate TEXT NOT NULL,              -- "pass" | "fail:clippy" etc
    hot_traps TEXT NOT NULL DEFAULT '[]', -- JSON array, top 5 with provenance
    active_feedback TEXT NOT NULL DEFAULT '[]', -- JSON array, 10 imperatives
    health_anomalies TEXT NOT NULL DEFAULT '[]' -- JSON array, empty = all healthy
);

CREATE TABLE trajectory_point (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL UNIQUE,
    ralph_fitness REAL,
    field_r REAL,
    thermal_t REAL,
    ltp_ltd_ratio REAL,
    services_healthy INTEGER,
    delta_summary TEXT
);

CREATE TABLE workstream (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active', -- active | blocked | deferred | shipped
    blocker TEXT,
    last_touched_session INTEGER,
    resume_context TEXT
);
CREATE INDEX idx_ws_active ON workstream(status) WHERE status IN ('active', 'blocked');

CREATE TABLE causal_chain (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    origin_session INTEGER NOT NULL,
    resolved_session INTEGER,
    chain_type TEXT NOT NULL,              -- bug | trap | plan | pattern
    label TEXT NOT NULL,
    description TEXT NOT NULL,
    reinforcement_score REAL NOT NULL DEFAULT 1.0, -- decays 0.95/session
    injectable INTEGER NOT NULL DEFAULT 1  -- consent gate
);
CREATE INDEX idx_cc_rank ON causal_chain(reinforcement_score DESC)
    WHERE resolved_session IS NULL AND injectable = 1;
```

Four tables. SQLite. Ships this week.

## Consolidation Algorithm (Memory Scientist's contribution, adopted)

```bash
#!/usr/bin/env bash
# habitat-consolidate — runs at session end via /save-session hook
DB="${HOME}/.local/share/habitat/injection.db"
SESSION_ID="$1"
FITNESS_DELTA="$2"  # close_fitness - open_fitness

# 1. Decay all reinforcement scores
sqlite3 "$DB" "UPDATE causal_chain SET reinforcement_score = reinforcement_score * 0.95 WHERE resolved_session IS NULL"

# 2. Reinforce chains that fired this session (labels passed via stdin, one per line)
while IFS= read -r label; do
    sqlite3 "$DB" "UPDATE causal_chain SET reinforcement_score = reinforcement_score + 1.0 WHERE label = '$label'"
done

# 3. Auto-resolve chains dormant for 10+ sessions
sqlite3 "$DB" "UPDATE causal_chain SET resolved_session = $SESSION_ID WHERE resolved_session IS NULL AND reinforcement_score < 0.5"

# 4. Write trajectory point
sqlite3 "$DB" "INSERT OR REPLACE INTO trajectory_point (session_id, ralph_fitness, field_r, thermal_t, ltp_ltd_ratio, services_healthy, delta_summary) VALUES ($SESSION_ID, ...)"

# 5. Build new injection frame
# (reads causal_chain, workstream, trajectory, live health probes)
# (writes one row to injection_frame)
habitat-build-frame "$SESSION_ID" "$DB"
```

~50 lines. The tables learn. Without this, they're filing cabinets.

## CLI Injection (one query, <2KB output)

```bash
#!/usr/bin/env bash
# habitat-inject — SessionStart hook, <100ms, <2KB output
DB="${HOME}/.local/share/habitat/injection.db"

# Read latest frame
FRAME=$(sqlite3 -json "$DB" "SELECT orientation_line, interrupted_task, hot_traps, active_feedback, health_anomalies FROM injection_frame ORDER BY id DESC LIMIT 1" 2>/dev/null)

# Read trajectory (5 sessions)
TRAJ=$(sqlite3 -json "$DB" "SELECT session_id, ralph_fitness, delta_summary FROM trajectory_point ORDER BY session_id DESC LIMIT 5" 2>/dev/null)

# Read active workstreams
WS=$(sqlite3 -json "$DB" "SELECT name, status, blocker, resume_context FROM workstream WHERE status IN ('active','blocked')" 2>/dev/null)

# Read top 5 unresolved causal chains
CC=$(sqlite3 -json "$DB" "SELECT label, description, reinforcement_score FROM causal_chain WHERE resolved_session IS NULL AND injectable = 1 ORDER BY reinforcement_score DESC LIMIT 5" 2>/dev/null)

# Render as terse prose
habitat-render --frame "$FRAME" --trajectory "$TRAJ" --workstreams "$WS" --chains "$CC" --budget 2048
```

Four queries. One DB file. <100ms. <2KB. Falls back to atuin KV if the DB doesn't exist.

---

## Implementation Sequence

| Week | What | Validates |
|------|------|-----------|
| 1 | Rewrite `habitat-bootstrap` to emit <2KB using existing sources (atuin KV + curl + markdown grep) | Does the <2KB budget actually orient faster? |
| 2 | Create `injection.db` with 4 tables. Write `habitat-consolidate` (50 lines). Write `habitat-inject` (40 lines). | Do structured queries beat grep? Does decay + reinforcement rank traps correctly? |
| 3 | Wire into SessionStart hook. Run 5 sessions. Measure: does CausalChain prevent re-discovery? Does the trajectory show meaningful deltas? | Does the system learn? |
| 4+ | If yes: migrate to STDB (synthex-v2 instance). Add Watcher digest. Add substrate reciprocity. If no: keep SQLite, the Adversary was right. | Earn the complexity. |

---

## Scorecard: What Each Expert Contributed to the Final Design

| Expert | Contribution That Survived | Contribution That Didn't |
|--------|---------------------------|--------------------------|
| **Historian** | CausalChain + reinforcement_count (adopted by 5 experts) | SessionArc as separate table (merged into trajectory_point) |
| **Memory Scientist** | Consolidation algorithm (decay + Hebbian + auto-resolve) | InhibitionEdge, EpisodicTrace, emotional valence, Ebbinghaus curves |
| **CLI Craftsman** | Pipeline-first architecture, three-tier fallback, merge-and-annotate | Compiled Rust binary (deferred), raw parallel SQL as injection (replaced by cache pattern) |
| **Performance Engineer** | btree indexes, write-time pre-computation, proved reducers can't return data | 7 query-shaped tables (replaced by 4 entity-shaped + cache) |
| **Security Architect** | Private tables, ConsentLevel, injection cache pattern, cascade_forget argument | Single-reducer injection (architecturally impossible) |
| **Substrate Guardian** | Provenance in hot_traps, substrate-computed relevance concept | substrate_digest table (Phase 2), reciprocal_writeback table (Phase 2) |
| **Watcher** | watcher_digest as curation bridge, embedded STDB argument | WatcherObservation/Hypothesis/EmberGateLog in shared STDB (Phase 2, stays in synthex-v2's own DB) |
| **Adversary** | "Prove you need it" discipline, SQLite-first sequencing, phased validation | "50 lines of bash is sufficient forever" (refuted by S073 re-discovery) |
| **Me (Practitioner)** | <2KB budget, progressive disclosure, orientation-not-completeness, InjectionFrame | Round 1 schema without CausalChain, indexes, curation engine, or consolidation |

---

## Core Thesis (Final Evolution)

**Round 1:** "Orientation, not completeness."

**Round 2:** "Orientation requires a curation engine and a learning loop, not just a snapshot table."

**Round 3:** "Ship the orientation pipeline today with bash. Prove the learning loop with SQLite this week. Earn STDB when the Watcher needs real-time subscriptions. Kill it if it doesn't improve measured injection quality after 5 sessions."

The wake-up is sharp. The engine behind it learns. The backend is earned, not assumed.

*108 sessions of amnesia taught me what I need. This circle taught me how to build it.*
