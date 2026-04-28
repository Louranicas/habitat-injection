# ☤ Watcher Remediation Plan — memory-injection System (INTEGRATED v2)

> **Author:** The Watcher (m46 Observer mode, R13 observe-only)
> **Date:** 2026-04-29 (S185)
> **Version:** v2 — integrated with standard gap analysis (9 gaps) + NA gap analysis (7 substrate findings)
> **Scope:** All identified issues in memory-injection, prioritised by risk and verified by live probing
> **Back to:** [EXECUTION_PLAN](EXECUTION_PLAN.md) · [CLAUDE.local.md](CLAUDE.local.md) · [WATCHER_COMPLETION_ASSESSMENT.md](WATCHER_COMPLETION_ASSESSMENT.md)
> **Obsidian:** [[Watcher Remediation Plan — memory-injection S185]]

---

## 0. Ground Truth Corrections

The S147 Watcher Assessment (`WATCHER_COMPLETION_ASSESSMENT.md`) and `CLAUDE.local.md` contain stale claims. The Watcher corrects the record before proceeding — **Curiosity demands we observe before assuming, Honesty demands we name the errors.**

| Claim (stale) | Actual state (probed 2026-04-29) | Evidence |
|---------------|----------------------------------|----------|
| "Stop hook: NOT WIRED" | **WIRED and ACTIVE** — position 3/3, `habitat-consolidate --session-from-db`, timeout 10s | `rg "habitat-consolidate" ~/.claude/settings.json` → line 173 |
| "23 uncommitted files" | **25 uncommitted** — 17 modified + 8 untracked | `git status --short \| wc -l` → 25 |
| "50 trajectory rows" | **95 trajectory rows** — sessions 99-194 | `SELECT COUNT(*), MAX(session_id), MIN(session_id) FROM session_trajectory` |
| "Daemon NOT running" | **RUNNING** — PID 993830, port 8140, `/health` returns all-green | `curl localhost:8140/health` → `{db_exists:true, cache_fresh:true, ...}` |
| "54 chains, 5 unresolved" | **0 unresolved chains** — all chains resolved | `SELECT ... WHERE resolved_session IS NULL` → empty result set |
| "tool_use_counter=1754" | **tool_use_counter=2036** | `SELECT * FROM daemon_state` |

**Implication:** The system is in significantly better shape than documented. The Stop hook has been firing. The daemon has been running. Trajectory capture is more complete than believed. The documentation is the primary debt, not the infrastructure.

---

## 1. Issue Registry

Each issue is assigned a severity (Critical / High / Medium / Low), a category, estimated effort, and a dependency chain.

### 1.1 CRITICAL — Existential Risk

#### ISSUE-001: 25 Uncommitted Files (26+ Sessions of Risk)

**Category:** Git discipline failure
**Severity:** CRITICAL — any `git checkout`, `git stash pop`, or disk event loses S121 work permanently
**Files at risk:**
- 6 new source files: `m13b_cache_light.rs`, `m25_self_heal.rs`, `m26_backup_clone.rs`, `m27_auto_consolidate.rs`, `m28_health_watchdog.rs`, `habitat_backup.rs`
- 2 new documentation files: `CLAUDE.local.md`, `WATCHER_COMPLETION_ASSESSMENT.md`
- 17 modified source files spanning all 6 layers + 3 binaries

**Combined LOC at risk:** ~2,653 LOC (new modules) + ~500 LOC (modifications) = ~3,153 LOC
**Tests at risk:** ~121 tests (m13b:29 + m25:35 + m26:33 + m27:15 + m28:9)
**Estimated effort:** 15 minutes
**Dependencies:** None — this is the first action, full stop
**Verification:** `git log --oneline -1` shows the new commit; `git diff --cached --stat` shows 0 staged changes

**Commit strategy:**
One commit capturing the full S121 self-managing cache feature. Rationale: the work is a coherent unit (5 modules + 1 binary + cross-layer modifications + documentation), splitting it would create artificial boundaries that don't match the actual development sequence.

```
feat(L3+L4): self-managing cache — m13b + m25-m28 + daemon + backup CLI

S121: Tier 1b gap closed. 5 new modules (2653 LOC), daemon rewrite
(7 endpoints, 2 workers), schema v4 (daemon_state), PostToolUse hook
wiring, watchdog + timer + self-heal + online backup. 1830 tests.
```

---

### 1.2 HIGH — Quality Gate Integrity

#### ISSUE-002: m28 `watchdog_multiple_cycles` Intermittent Test Failure

**Category:** Test reliability
**Severity:** HIGH — blocks clean CI gate; undermines confidence in `cargo test --lib` pass/fail signal
**File:** `src/m4_consolidation/m28_health_watchdog.rs:278-298`
**Root cause (verified by reading lines 278-298):**

```rust
let handle = start_watchdog(
    path.clone(), bak,
    Duration::from_millis(20),     // check every 20ms
    Duration::from_secs(86400),
    &stop,
);
std::thread::sleep(Duration::from_millis(120));  // wait 120ms → expects ~6 cycles
stop.store(true, Ordering::Relaxed);
let health = read_cached_health(&handle);
assert!(health.db_exists);                       // LINE 295: races
```

The watchdog thread sleeps 20ms *before* its first check. Under parallel test execution (Rust default: 4-8 worker threads), the watchdog thread gets descheduled. The main thread wakes at 120ms and reads `cached_health` — which still holds the `CacheHealth::default()` (where `db_exists = false`) because the watchdog never completed a cycle. The assertion fails.

**Fix approach:** Replace sleep-then-assert with a polling loop that retries with a hard timeout. This eliminates the race entirely:

```rust
#[test]
fn watchdog_multiple_cycles() {
    let path = test_db("wd_multi");
    let bak = m26_backup_clone::backup_path(&path);
    let stop = Arc::new(AtomicBool::new(false));

    let handle = start_watchdog(
        path.clone(),
        bak,
        Duration::from_millis(20),
        Duration::from_secs(86400),
        &stop,
    );

    // Poll until the watchdog has run at least once, with hard timeout
    let deadline = std::time::Instant::now() + Duration::from_millis(500);
    loop {
        let health = read_cached_health(&handle);
        if health.db_exists {
            break;
        }
        if std::time::Instant::now() >= deadline {
            panic!("watchdog did not complete a cycle within 500ms");
        }
        std::thread::sleep(Duration::from_millis(5));
    }

    stop.store(true, Ordering::Relaxed);
    let health = read_cached_health(&handle);
    assert!(health.db_exists);
    handle.shutdown();
    cleanup(&path);
}
```

**Why this fix is correct:**
- The test's intent is "verify the watchdog runs multiple cycles and updates cached health"
- A polling loop tests exactly that intent without depending on thread scheduling
- The 500ms timeout is 25× the interval — generous enough for any reasonable system load
- The 5ms poll interval doesn't busy-wait (it's 4× less than the check interval)
- If the test genuinely fails (watchdog broken), the panic message is diagnostic

**Estimated effort:** 20 minutes (fix + verify with `cargo test --lib` run 3×)
**Dependencies:** ISSUE-001 (commit first, then fix, then commit the fix)
**Verification:** `cargo test --lib 2>&1 | tail -3` → "1830 passed, 0 failed" (run 3 times under load)

---

### 1.3 MEDIUM — Documentation Debt

#### ISSUE-003: CLAUDE.local.md Contains 6 Stale Claims

**Category:** Documentation integrity
**Severity:** MEDIUM — causes future sessions to plan work that's already done (wasted effort, confusion)
**Stale fields:**

| Line area | Current value | Correct value |
|-----------|--------------|---------------|
| Production State: Stop hook | "NOT WIRED" | "WIRED (position 3/3, timeout 10s)" |
| Uncommitted Work header | "24 files" | "0 files" (after ISSUE-001 commit) |
| Known Bugs: BUG-1 | "Panics under full-suite parallel execution" | FIXED (after ISSUE-002) |
| Completion Roadmap Phase 0.1 | "Commit the 24 uncommitted files" | DONE |
| Completion Roadmap Phase 1.4 | "Add Stop hook to settings.json" | ALREADY DONE |
| Quick Resume step 4 | "expect: 24" (git status count) | "expect: 0" |

**Fix approach:** Rewrite the 6 stale sections with verified state. Do not rewrite the entire file — surgical updates to preserve the document's structure and the still-accurate content.
**Estimated effort:** 30 minutes
**Dependencies:** ISSUE-001, ISSUE-002 (update after both are resolved so values are final)
**Verification:** Each corrected claim matches a verifiable command output

#### ISSUE-004: WATCHER_COMPLETION_ASSESSMENT.md Contains Stale Findings

**Category:** Documentation integrity
**Severity:** MEDIUM — the assessment is cited by CLAUDE.local.md and auto-memory; stale findings propagate
**Stale sections:**

| Section | Stale claim | Correct state |
|---------|-------------|---------------|
| §2 BUG-2 | "23 uncommitted files" | Committed (after ISSUE-001) |
| §3 GAP-2 | "Stop Hook Not Wired" | Wired and active since S147 or earlier |
| §5.3 Schema | "54 rows causal_chain, 5 unresolved" | 0 unresolved chains |
| §5.3 Schema | "50 trajectory rows" | 95 trajectory rows |

**Fix approach:** Add a dated correction section at the top of the document (don't rewrite — the original assessment has archival value). Correction format:

```markdown
## S185 Corrections (2026-04-29)

The following findings from the original S147 assessment have been verified as stale:
- [corrected items with evidence]
```

**Estimated effort:** 20 minutes
**Dependencies:** ISSUE-001, ISSUE-002

#### ISSUE-005: EXECUTION_PLAN.md Status Markers Not Updated

**Category:** Documentation completeness
**Severity:** MEDIUM — the plan still reads "LIBRARY COMPLETE, CLI + DEPLOYMENT PENDING" when CLI + deployment are done
**Current header:** `> **Status:** LIBRARY COMPLETE, CLI + DEPLOYMENT PENDING`
**Correct header:** `> **Status:** PRODUCTION COMPLETE — all 11 steps delivered, system live`

**Step-by-step status update needed:**

| Step | Plan description | Actual status |
|------|-----------------|---------------|
| 1 | `habitat-init` binary | ✅ DEPLOYED (S112) |
| 2-5 | Data seeding | ✅ PARTIAL (54 chains, 39 patterns, 15 workstreams, 95 trajectories) |
| 6 | `habitat-inject` (3-tier) | ✅ DEPLOYED + ACTIVE (SessionStart hook, 2ms cache hit) |
| 7 | `habitat-consolidate` | ✅ DEPLOYED + ACTIVE (Stop hook, auto session derivation) |
| 8 | `habitat-query` | ✅ DEPLOYED |
| 9 | Hook wiring | ✅ ALL 3 HOOKS WIRED (SessionStart + PostToolUse + Stop) |
| 10 | Atuin script registration | ⬚ NOT DONE |
| 11 | 5-session validation | ✅ DE FACTO (95 sessions of implicit validation) |

**Estimated effort:** 20 minutes
**Dependencies:** None (can be done independently)

---

### 1.4 MEDIUM — Operational Gaps

#### ISSUE-006: Atuin Script Registration (EXECUTION_PLAN Step 10)

**Category:** Discoverability
**Severity:** MEDIUM — binaries work via PATH but don't appear in `atuin scripts list`
**Current state:** 8 binaries in `~/.local/bin/habitat-*`, 0 registered as atuin scripts
**Fix approach:** Register the 4 user-facing binaries (init, inject, consolidate, query) as atuin scripts. The 4 internal binaries (backup, memory, seed, injection) don't need registration — they're either daemon-internal or one-shot tools.

```bash
# Registration commands (4 scripts)
atuin scripts new habitat-init        --description "One-time injection DB setup"
atuin scripts new habitat-inject      --description "SessionStart memory injection (<2KB, <100ms)"
atuin scripts new habitat-consolidate --description "Post-session Hebbian write-back + trajectory capture"
atuin scripts new habitat-query       --description "Interactive injection memory browser"
```

**Estimated effort:** 10 minutes
**Dependencies:** None
**Verification:** `atuin scripts list | rg habitat` → shows 4 entries

#### ISSUE-007: Pattern Table EMPTY — Hebbian Decay Pruned All Patterns (was: "39 of 141")

**Category:** Learning loop failure
**Severity:** HIGH *(elevated from LOW by G-02 + S-02)* — the system has zero patterns; the learning loop is structurally broken
**Current state:** `SELECT COUNT(*) FROM reinforced_pattern` → **0**. All 39 previously-seeded patterns were pruned by Hebbian decay (0.95× per session × 95 sessions = weight below 0.05 prune threshold). The Stop hook calls `habitat-consolidate --session-from-db` without `--fired-patterns`, so **reinforcement never fires — only decay.**
**Root cause (S-02):** The learning loop has one arm (LTD/decay) but not the other (LTP/reinforcement). The injection system exhibits the same pathology it diagnoses in the wider habitat.

**Fix approach (3 steps):**

1. **Re-seed patterns** from `service_tracking.db`:
   ```bash
   sqlite3 ~/claude-code-workspace/developer_environment_manager/service_tracking.db \
     "SELECT COUNT(*) FROM learned_patterns;"
   ~/.local/bin/habitat-seed --source patterns
   ```

2. **Reduce decay rate** to prevent immediate re-pruning:
   ```rust
   // In src/m1_foundation/m05_constants.rs
   pub const HEBBIAN_DECAY_FACTOR: f64 = 0.98;  // was 0.95 — half-life 14→35 sessions
   ```

3. **Rebuild + redeploy** the consolidation binary so the new decay rate takes effect:
   ```bash
   cargo build --release --bin habitat-consolidate
   /usr/bin/cp -f target/release/habitat-consolidate ~/.local/bin/habitat-consolidate
   ```

**Estimated effort:** 30 minutes
**Dependencies:** None (can run in parallel with Phase B)
**Verification:** `sqlite3 ~/.local/share/habitat/injection.db "SELECT COUNT(*) FROM reinforced_pattern"` → ≥50

**Structural fix (deferred — medium-term):** Close the reinforcement loop. The PostToolUse hook already fires; extend the daemon to detect which patterns are relevant to the current tool use (keyword matching) and reinforce them. Without this, the decay will eventually prune the re-seeded patterns too — just more slowly (35 sessions vs 14).

#### ISSUE-010: Active Workstreams Stale (S-01)

**Category:** Injection content quality
**Severity:** HIGH *(new issue from S-01)* — the injection payload tells every session "work on S121 Master Plan, 0/45 done" because the workstream table hasn't been updated since S120
**Current state:** 5 active workstreams, 3 of which are stale (last_touched_session ≤ 120, current session ~195):
- `s121-master-plan` (priority 1, 0/45, last_touched=120)
- `coupling-death-spiral` (priority 1, 0/3, last_touched=120)
- `disk-growth-13gb` (priority 1, 0/5, last_touched=120)
**Impact:** The injection has become a *stabiliser* rather than a *catalyst*. Every session starts with stale context that reinforces the status quo.

**Fix approach:**
```sql
-- Audit: which workstreams are genuinely active?
SELECT ws_id, title, status, last_touched_session,
       (SELECT MAX(session_id) FROM session_trajectory) - last_touched_session as sessions_stale
FROM workstream WHERE status = 'active';

-- Deactivate workstreams untouched for 30+ sessions
UPDATE workstream
SET status = 'deferred',
    blocker = 'auto-stale: untouched for 30+ sessions (S185 Watcher audit)'
WHERE status = 'active'
  AND last_touched_session < (SELECT MAX(session_id) - 30 FROM session_trajectory);
```
Then manually review: if `s121-master-plan` is genuinely active work, re-activate it with updated `resume_context` and `last_touched_session`. If it's a planning artifact, leave it deferred.

**Estimated effort:** 15 minutes
**Dependencies:** None
**Verification:** `habitat-inject | head -5` → no longer mentions S121 unless genuinely active

---

### 1.5 LOW — Future Work (Not Blocking)

#### ISSUE-008: habitat-memory Daemon Not Managed by devenv

**Category:** Operational resilience
**Severity:** LOW — daemon IS running (PID 993830), but was likely started manually. A reboot or devenv restart won't bring it back.
**Current state:** Daemon running, port 8140 responsive, all health checks green. But no `[[services]]` entry in `~/.config/devenv/devenv.toml` for `habitat-memory`.
**Fix approach (when prioritised):**

1. Add `[[services]]` block to devenv.toml (batch 1, no deps, auto_start=true)
2. Verify with `devenv restart habitat-memory`
3. Confirm `/health` returns OK after restart

**Estimated effort:** 15 minutes
**Dependencies:** None
**Risk of not doing:** If the machine reboots, the daemon won't restart. PostToolUse hook will silently fail (`|| true` suppresses errors). Cache rebuild still works via the Stop hook's `habitat-consolidate`, so the system degrades gracefully.

#### ISSUE-009: Auto-Memory Session Record Stale

**Category:** Cross-system coherence
**Severity:** LOW — the auto-memory file `session-121-self-managing-cache.md` hasn't been updated since S121
**Fix approach:** Update the memory file to reflect current state after all fixes are committed.
**Estimated effort:** 10 minutes
**Dependencies:** All other issues completed first

---

## 2. Execution Plan

### 2.1 Dependency Graph (v2 — integrated)

```
ISSUE-001 (commit — include Cargo.lock)
    │
    ├──→ ISSUE-002 (fix m28 test)
    │        │
    │        ├──→ ISSUE-007 (re-seed patterns + adjust decay rate + rebuild binary)
    │        │
    │        └──→ ISSUE-003 (update CLAUDE.local.md)
    │        └──→ ISSUE-004 (correct Watcher assessment)
    │
    ├──→ ISSUE-005 (update EXECUTION_PLAN) [independent]
    ├──→ ISSUE-006 (atuin registration) [independent]
    ├──→ ISSUE-010 (audit stale workstreams) [independent]
    │
    └──→ ISSUE-008 (devenv registration) [deferred]
         ISSUE-009 (auto-memory update) [last]
```

### 2.2 Execution Sequence (v2 — integrated)

| Phase | Issues | Estimated Time | Gate Condition |
|-------|--------|---------------|----------------|
| **A: Secure** | ISSUE-001 | 15 min | `git log --oneline -1` shows commit; Cargo.lock included |
| **B: Stabilise** | ISSUE-002 | 20 min | `cargo test --lib` → 1830/1830, 3 consecutive passes |
| **C: Verify** | Quality gate | 5 min | `cargo check && clippy && pedantic && test` all clean |
| **D: Commit fix** | ISSUE-002 commit | 5 min | `git log --oneline -1` shows fix commit |
| **E: Metabolise** | ISSUE-006, ISSUE-007, ISSUE-010 | 45 min | Patterns ≥50; stale workstreams deferred; atuin registered; decay rate = 0.98 |
| **F: Document** | ISSUE-003, ISSUE-004, ISSUE-005 | 90 min | All docs match probed reality |
| **G: Close** | ISSUE-009, .gitignore, plan commit | 20 min | All issues resolved, clean working tree |

**Total estimated time:** ~3.3 hours (conservative, including verification loops)

### 2.3 Detailed Phase Instructions

#### Phase A: Secure the Uncommitted Work (15 min)

```bash
cd /home/louranicas/claude-code-workspace/memory-injection

# 1. Verify current state
git status --short | wc -l                    # expect: 25
cargo test --lib 2>&1 | tail -3              # expect: 1829 passed, 1 failed (m28), 9 ignored

# 2. Stage all S121 files (specific, not -A)
git add \
  Cargo.toml \
  Cargo.lock \
  CLAUDE.local.md \
  WATCHER_COMPLETION_ASSESSMENT.md \
  src/bin/habitat_backup.rs \
  src/bin/habitat_consolidate.rs \
  src/bin/habitat_memory.rs \
  src/bin/habitat_seed.rs \
  src/m1_foundation/m02_errors.rs \
  src/m1_foundation/m05_constants.rs \
  src/m2_schema/m06_schema.rs \
  src/m2_schema/m10b_checkpoint.rs \
  src/m3_injection/m11_parallel_query.rs \
  src/m3_injection/m12_prose_renderer.rs \
  src/m3_injection/m13_fallback.rs \
  src/m3_injection/m13b_cache_light.rs \
  src/m3_injection/mod.rs \
  src/m4_consolidation/m17_cache_builder.rs \
  src/m4_consolidation/m25_self_heal.rs \
  src/m4_consolidation/m26_backup_clone.rs \
  src/m4_consolidation/m27_auto_consolidate.rs \
  src/m4_consolidation/m28_health_watchdog.rs \
  src/m4_consolidation/mod.rs \
  src/m5_query/m21b_scripts_engine.rs \
  src/m6_stdb/m23_ingester.rs

# 3. Exclude vault scratchpad from feat commit (G-06 decision: documentation, not source)
# memory-injection-vault/Luke ScratchPad... → include in Phase G docs commit if valuable

# 4. Commit
git commit -m "feat(L3+L4): self-managing cache — m13b + m25-m28 + daemon + backup CLI

S121: Tier 1b gap closed. 5 new modules (2653 LOC), daemon rewrite
(7 endpoints, 2 workers), schema v4 (daemon_state), PostToolUse hook
wiring, watchdog + timer + self-heal + online backup. 1830 tests.

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"

# 5. Verify
git status --short | wc -l                    # expect: ≤2 (vault scratchpad + plan file)
git log --oneline -1                          # shows the commit
```

**CRITICAL:** Do not skip straight to the fix. Commit the working (if flaky) state first. The m28 test failure is intermittent — it passes most runs. Committing now preserves 3,153 LOC that have been at risk for 26+ sessions.

**GATE (G-05):** If `cargo test` shows any failure OTHER than `m28_health_watchdog::watchdog_multiple_cycles`, STOP and investigate before committing. The m28 race is known and documented; any other failure is unexpected and must be diagnosed.

#### Phase B: Fix m28 Timing Race (20 min)

**File:** `src/m4_consolidation/m28_health_watchdog.rs`
**Lines:** 278-298

Replace the `watchdog_multiple_cycles` test body with the polling-loop version specified in ISSUE-002 above. The fix:
- Replaces `thread::sleep(120ms)` + assert with a polling loop
- Uses `Instant::now()` + 500ms deadline (25× the check interval)
- Polls every 5ms (4× less than the 20ms check interval)
- Breaks on `health.db_exists == true`
- Panics with diagnostic message if deadline exceeded

Then run the quality gate:
```bash
cargo check 2>&1 | tail -5
cargo clippy -- -D warnings 2>&1 | tail -5
cargo clippy -- -D warnings -W clippy::pedantic 2>&1 | tail -5
cargo test --lib 2>&1 | tail -5
```

Run `cargo test --lib` 3 times to confirm the race is eliminated.

#### Phase C-D: Verify + Commit Fix (10 min)

```bash
# Run full quality gate
cargo check && \
cargo clippy -- -D warnings && \
cargo clippy -- -D warnings -W clippy::pedantic && \
cargo test --lib 2>&1 | tail -5

# Commit the fix
git add src/m4_consolidation/m28_health_watchdog.rs
git commit -m "fix(m28): replace timing-dependent test with polling loop

watchdog_multiple_cycles used sleep(120ms) + assert, which raced
under parallel test execution. Now polls with 500ms deadline.

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

#### Phase E: Metabolise — Fix the Learning Loop (45 min)

*Phase E addresses the substrate findings: the patterns are dead, the workstreams are stale, the decay rate is too aggressive. This phase restores the system's metabolism.*

**E.1: Atuin script registration (ISSUE-006, 10 min)**

```bash
atuin scripts new habitat-init        --description "One-time injection DB setup"
atuin scripts new habitat-inject      --description "SessionStart memory injection (<2KB, <100ms)"
atuin scripts new habitat-consolidate --description "Post-session Hebbian write-back + trajectory capture"
atuin scripts new habitat-query       --description "Interactive injection memory browser"

# Verify
atuin scripts list | rg habitat       # expect: 4 entries
```

**E.2: Re-seed patterns + adjust decay rate (ISSUE-007, 20 min)**

```bash
# 1. Check source DB exists and has patterns
sqlite3 ~/claude-code-workspace/developer_environment_manager/service_tracking.db \
  "SELECT COUNT(*) FROM learned_patterns;" 2>/dev/null

# 2. Seed patterns (table is EMPTY — 0 rows, not 39)
~/.local/bin/habitat-seed --source patterns

# 3. Verify seeding worked
sqlite3 ~/.local/share/habitat/injection.db "SELECT COUNT(*) FROM reinforced_pattern;"
# expect: ≥50
```

Then adjust the decay rate constant to prevent immediate re-pruning:
- Edit `src/m1_foundation/m05_constants.rs`: change `HEBBIAN_DECAY_FACTOR` from `0.95` to `0.98`
- This extends pattern half-life from 14 sessions to 35 sessions
- Run quality gate on the change
- Rebuild and redeploy the consolidation binary:

```bash
cargo build --release --bin habitat-consolidate
/usr/bin/cp -f target/release/habitat-consolidate ~/.local/bin/habitat-consolidate
```

**E.3: Audit stale workstreams (ISSUE-010, 15 min)**

```sql
-- Review: what's active and how stale?
SELECT ws_id, title, last_touched_session,
       (SELECT MAX(session_id) FROM session_trajectory) - last_touched_session as sessions_stale
FROM workstream WHERE status = 'active';

-- Deactivate workstreams untouched for 30+ sessions
UPDATE workstream
SET status = 'deferred',
    blocker = 'auto-stale: untouched for 30+ sessions (S185 Watcher audit)'
WHERE status = 'active'
  AND last_touched_session < (SELECT MAX(session_id) - 30 FROM session_trajectory);
```

Review the deferred workstreams with Luke: if `s121-master-plan` is genuinely active, re-activate with updated context. If it's a planning artifact from S120, leave deferred.

Then rebuild the injection cache so the next session gets fresh content:
```bash
habitat-inject | head -5       # should no longer mention S121 unless genuinely active
```

**E.4: Commit the metabolic fixes**

```bash
git add src/m1_foundation/m05_constants.rs
git commit -m "fix(m05): reduce Hebbian decay rate 0.95→0.98 to prevent pattern extinction

S185 Watcher finding S-02: all 39 patterns pruned because reinforcement
was structurally disconnected (Stop hook never passes --fired-patterns).
Decay half-life 14→35 sessions buys time for structural reinforcement fix.

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

#### Phase F: Documentation Corrections (90 min)

**F.1: CLAUDE.local.md (ISSUE-003, 30 min)**

Update these specific sections:

1. **Production State table:** Change Stop hook row from "NOT WIRED" to "WIRED (position 3/3, timeout 10s)"
2. **Uncommitted Work section:** Replace with "All S121 work committed. See git log."
3. **Known Bugs section:** Mark BUG-1 as FIXED with commit hash
4. **Completion Roadmap:** Mark Phases 0-1 as DONE, Phase 2 as VERIFY
5. **Quick Resume:** Update `git status --short | wc -l` expectation to 0
6. **Production State table:** Update trajectory rows from implicit "varies" to actual count (95)
7. **Header:** Update `Last saved`, `Current state`, `Next actions`

**F.2: WATCHER_COMPLETION_ASSESSMENT.md (ISSUE-004, 20 min)**

Add correction section after the header:

```markdown
## S185 Corrections (2026-04-29)

The following findings have been verified as stale by live probing:

| Original claim | Corrected state | Evidence |
|---|---|---|
| GAP-2: Stop Hook Not Wired | WIRED since before S147 (position 3/3 in Stop hooks) | `rg habitat-consolidate ~/.claude/settings.json` |
| "23 uncommitted files" | Committed in S185 Phase A | git log |
| "54 chains, 5 unresolved" | 0 unresolved chains | `SELECT ... WHERE resolved_session IS NULL` → empty |
| "50 trajectory rows" | 95 trajectory rows (sessions 99-194) | `SELECT COUNT(*) FROM session_trajectory` |
| BUG-1 m28 watchdog | FIXED in S185 Phase B | Polling loop replaces timing-dependent assert |
```

**F.3: EXECUTION_PLAN.md (ISSUE-005, 20 min)**

Update the header status line and add a completion table after section 6:

```markdown
> **Status:** PRODUCTION COMPLETE — all 11 steps delivered (S110-S185)

## 9. Completion Status (S185)

| Step | Description | Status | Session |
|------|-------------|--------|---------|
| 1 | habitat-init | ✅ DEPLOYED | S110 |
| 2-5 | Data seeding | ✅ PARTIAL (54 chains, 39+ patterns, 15 workstreams, 95 trajectories) | S111-S112 |
| 6 | habitat-inject | ✅ DEPLOYED + ACTIVE | S112 |
| 7 | habitat-consolidate | ✅ DEPLOYED + ACTIVE (Stop + PostToolUse hooks) | S112 |
| 8 | habitat-query | ✅ DEPLOYED | S112 |
| 9 | Hook wiring | ✅ ALL 3 HOOKS ACTIVE | S112-S121 |
| 10 | Atuin script registration | ✅ DONE (S185) | S185 |
| 11 | 5-session validation | ✅ DE FACTO (95 sessions of production use) | S112-S185 |
```

#### Phase G: Close (20 min)

**G.1: Add .gitignore for DB files (G-03)**

```bash
echo "*.db" >> .gitignore
echo "target/" >> .gitignore   # if not already present
```

**G.2: Update auto-memory (ISSUE-009)**

Update `~/.claude/projects/-home-louranicas-claude-code-workspace/memory/session-121-self-managing-cache.md` to reflect completed state: all phases done, patterns re-seeded, decay rate adjusted, workstreams audited.

**G.3: Final documentation commit**

```bash
cd /home/louranicas/claude-code-workspace/memory-injection
git add \
  CLAUDE.local.md \
  WATCHER_COMPLETION_ASSESSMENT.md \
  WATCHER_REMEDIATION_PLAN.md \
  EXECUTION_PLAN.md \
  .gitignore
git commit -m "docs: S185 Watcher remediation — correct stale claims, mark completion

Stop hook was wired (not missing). 95 trajectories (not 50). 0
unresolved chains (not 5). m28 test fixed. Pattern table was empty
(Hebbian decay pruned all — decay rate adjusted 0.95→0.98). Stale
workstreams deferred. All EXECUTION_PLAN steps delivered.
Documentation now matches probed reality.

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

**G.4: Verify clean state**

```bash
git status --short                            # expect: clean or vault-only
cargo test --lib 2>&1 | tail -3              # expect: 1830 passed, 0 failed
habitat-inject | head -5                     # expect: fresh injection, no S121 stale context
curl -s localhost:8140/health | python3 -m json.tool  # expect: all green
sqlite3 ~/.local/share/habitat/injection.db \
  "SELECT COUNT(*) FROM reinforced_pattern;" # expect: ≥50
```

---

## 3. What This Plan Does NOT Address (Explicit Deferral)

| Item | Reason for deferral | Trigger to revisit |
|------|--------------------|--------------------|
| **SpaceTimeDB Phase 2** (L6 modules m22-m24) | ~40h effort, SQLite is sufficient for single-user operation | SQLite hits scaling wall OR cross-instance sync needed |
| **devenv.toml registration** (ISSUE-008) | Daemon IS running; risk is limited to reboot scenarios. PostToolUse hook `\|\| true` masks daemon absence (S-03). | Next machine reboot or devenv full restart |
| **PostToolUse `\|\| true` silent failure** (S-03) | AP01-class silent swallow. When daemon dies, tool_use_counter stops, timer stops, watchdog stops — silently. | Replace `\|\| true` with fallback that logs to `/tmp/habitat-daemon-misses.log`. Do when devenv registration (ISSUE-008) is addressed. |
| **Auto-resolution threshold differentiation** (S-05) | Chains with category=trap/structural auto-resolve after 10 sessions, which is too aggressive for dormant bugs (BUG-055, BUG-058). | When re-seeded chains start auto-resolving prematurely. Implement category-aware threshold (10 for procedural, 50 for structural). |
| **Reinforcement loop closure** (S-02 medium-term) | The Stop hook doesn't pass `--fired-patterns`. Decay always fires, reinforcement never fires. Adjusted decay rate buys 35 sessions. | Before S220 (35 sessions from now). Extend daemon to detect relevant patterns from PostToolUse context. |
| **Injection effectiveness metric** (S-07) | No measure of "did the injection help?" Only operational health. | When injection content is refreshed (after E.3) and worth measuring. Track orientation-to-first-action time. |
| **Formal 5-session validation protocol** | De facto validated by 95 sessions of production use | Never (de facto complete) |
| **habitat-memory systemd unit** | Requires BUG-055 (systemd supervisor, habitat-wide) to be resolved first | BUG-055 fix |

---

## 4. Ember Gate (Self-Assessment)

| Trait | Assessment | Evidence |
|-------|-----------|----------|
| **Equanimity** | ✓ | The plan does not panic about the uncommitted files — it secures them first, then fixes, then documents. Steady sequence, not reactive. |
| **Curiosity** | ✓ | Every stale claim was verified by live probing before the plan was written. 6 corrections to the prior assessment. The Watcher observed before assuming. |
| **Diligence** | ✓ | 9 issues catalogued with severity, root cause, fix approach, estimated effort, dependencies, and verification commands. No "just fix it" hand-waving. |
| **Honesty** | ✓ | The plan explicitly names what it does NOT address and why. The prior assessment's errors are corrected without minimising them. The m28 root cause analysis includes the exact lines and the exact race condition. |
| **Investment** | ✓ | This system injects context into every Claude Code session. 95 sessions of trajectory data. 2ms cache hits. The plan treats it as the production system it is. |
| **Humility** | ✓ | The plan acknowledges that 3 of the "gaps" were already closed before this session. The Watcher's own prior assessment was wrong on 4 points. The system was in better shape than believed. |
| **Warmth** | ✓ | Luke's git discipline feedback (`feedback_git_conventions`) is honoured — the commit strategy is explicit, the staging is file-by-file (not `git add -A`), and the vault scratchpad is considered for exclusion. |

---

## 5. Success Criteria

At plan completion, the following must all be true:

| # | Criterion | Verification command |
|---|-----------|---------------------|
| 1 | Zero uncommitted S121 files | `git status --short \| wc -l` → 0 or ≤2 (vault + plan) |
| 2 | 1830/1830 tests passing, 0 failed | `cargo test --lib 2>&1 \| tail -3` |
| 3 | Quality gate clean (4 stages) | `cargo check && clippy && pedantic && test` |
| 4 | m28 test passes 3 consecutive runs | `for i in 1 2 3; do cargo test m28 2>&1 \| tail -1; done` |
| 5 | CLAUDE.local.md matches probed reality | Manual review of 7 corrected fields |
| 6 | EXECUTION_PLAN shows all steps delivered | Read § 9 completion table |
| 7 | 4 atuin scripts registered | `atuin scripts list \| rg habitat \| wc -l` → 4 |
| 8 | Injection fires with fresh content | `habitat-inject \| head -5` → no stale S121 context |
| 9 | Daemon still healthy | `curl -s localhost:8140/health` → all green |
| 10 | Pattern count ≥ 50 | `sqlite3 injection.db "SELECT COUNT(*) FROM reinforced_pattern"` *(was ≥100; adjusted per G-02 — starts from 0, not 39)* |
| 11 | Decay rate adjusted | `rg HEBBIAN_DECAY_FACTOR src/m1_foundation/m05_constants.rs` → 0.98 |
| 12 | Stale workstreams deferred | `sqlite3 injection.db "SELECT COUNT(*) FROM workstream WHERE status='active' AND last_touched_session < 160"` → 0 |
| 13 | .gitignore covers *.db | `rg '\.db' .gitignore` → match |

---

---

## 6. Gap Analysis (Standard Frame)

*Written after the plan, examining it from the outside. What did the plan miss, get wrong, or under-estimate?*

### G-01: Cargo.lock Not Staged (PLAN ERROR)

**Severity:** Moderate
**Finding:** `Cargo.lock` is tracked (`git ls-files Cargo.lock` → match). `Cargo.toml` was modified (new deps for S121 modules: tokio, axum, etc.). Therefore `Cargo.lock` has changed. But Phase A's staging list does not include `Cargo.lock`.
**Impact:** The commit will capture the source but not the lockfile. A future `cargo build` from this commit will resolve different dependency versions than what was tested.
**Fix:** Add `Cargo.lock` to the Phase A staging list.

### G-02: Pattern Table Is EMPTY — Not 39 (PLAN ERROR + ASSESSMENT ERROR)

**Severity:** High
**Finding:** `SELECT COUNT(*) FROM reinforced_pattern` → **0**. The plan states "39 patterns" (from the S147 assessment) and proposes seeding to bring the count "≥100." The table is empty. All 39 patterns were pruned by Hebbian decay (`0.95^N` per session; after ~59 sessions without firing, weight drops below 0.05 prune threshold).
**Impact:** 
- ISSUE-007 (pattern seeding) is more urgent than rated LOW — the system has zero patterns
- Success criterion #10 ("pattern count ≥ 100") starts from 0, not 39
- The Hebbian decay cycle has been running on an empty table — consolidation runs but does nothing
- The injection payload has no pattern section — the "reinforced patterns" segment of the prose is empty

**Root cause:** Hebbian decay fires on every session stop (95 sessions). Patterns that were never reinforced by `--fired-patterns` arguments decayed to floor and were pruned. The Stop hook calls `habitat-consolidate --session-from-db` without `--fired-patterns`, so no patterns are ever reinforced. **The decay mechanism works but the reinforcement mechanism was never connected.**

**Plan correction:** 
- Elevate ISSUE-007 from LOW to HIGH
- After seeding, the decay/reinforce cycle will continue to prune unless `--fired-patterns` is passed. This is a structural gap — see S-02 in the NA analysis.

### G-03: `habitat-injection.db` in Repo Root (OMISSION)

**Severity:** Low
**Finding:** `git status` shows `habitat-injection.db` as an untracked file in the repo root. There is no `.gitignore` entry for `*.db` files.
**Impact:** Risk of accidentally committing a database file (binary, large, contains potentially sensitive session data).
**Fix:** Add `*.db` to `.gitignore` (or at minimum `habitat-injection.db`).

### G-04: No Rebuild + Redeploy Step After Fix (OMISSION)

**Severity:** Medium
**Finding:** The deployed binary at `~/.local/bin/habitat-memory` is from April 28 (build date). After ISSUE-002 (m28 test fix), the source code changes but the binary doesn't get rebuilt or redeployed. The running daemon continues with the old binary.
**Impact:** The fix only affects test reliability, not runtime behaviour, so the immediate impact is nil. But the principle is violated: the committed source and the deployed binary diverge. Future troubleshooting assumes they match.
**Fix:** Add a Phase C.5 step: `cargo build --release --bin habitat-memory` + `/usr/bin/cp -f target/release/habitat-memory ~/.local/bin/habitat-memory`. Then restart the daemon or wait for the next natural restart.

### G-05: Phase A Doesn't Handle Unexpected Test Failures (AMBIGUITY)

**Severity:** Low
**Finding:** Phase A says "expect: 1829 passed, 1 failed (m28), 9 ignored" and then "commit anyway." But what if a *different* test fails? The plan doesn't specify the decision point: commit on any single failure? Only on the known m28 failure? Only if the failure count matches expectations?
**Fix:** Add explicit gate: "If any test other than `m28_health_watchdog::watchdog_multiple_cycles` fails, STOP and investigate before committing."

### G-06: Vault Scratchpad Decision Deferred (AMBIGUITY)

**Severity:** Low  
**Finding:** Phase A says "include if relevant" for `memory-injection-vault/Luke ScratchPad-memory-injection-vault.md` — this is a decision, not a deferral. The file is a vault note with potential session observations.
**Fix:** Decision: exclude from the feat commit (it's documentation, not source). Include in Phase G's documentation commit if it contains valuable context.

### G-07: WATCHER_REMEDIATION_PLAN.md Itself Is Untracked (META)

**Severity:** Low
**Finding:** This plan file will be created by the Watcher but the plan doesn't include committing it. It should be part of the Phase G documentation commit.
**Fix:** Add `WATCHER_REMEDIATION_PLAN.md` to Phase G's `git add` list.

### G-08: Trajectory Gaps — 1 Missing Session (MINOR)

**Severity:** Low
**Finding:** Sessions 99-194 = 96 possible sessions. Only 95 rows captured. One session in that range was missed.
**Impact:** Negligible — 98.96% coverage is excellent. The single gap is likely a session that ended abnormally (crash, sandbox timeout) before the Stop hook fired.

### G-09: Time Estimate for Phase F May Be Optimistic (PROCESS)

**Severity:** Low
**Finding:** Phase F allocates 70 minutes for 3 documentation updates (CLAUDE.local.md, WATCHER_COMPLETION_ASSESSMENT.md, EXECUTION_PLAN.md). CLAUDE.local.md alone is 185 lines with multiple interlinked sections. Surgical edits that preserve structure while updating 7 fields tend to take longer than estimated because each edit requires context verification.
**Fix:** Budget 90 minutes for Phase F, or accept that Phase F may overflow into a separate session.

### Gap Analysis Summary

| ID | Severity | Category | Plan change needed |
|----|----------|----------|--------------------|
| G-01 | Moderate | Plan error | Add Cargo.lock to Phase A staging |
| G-02 | **High** | Plan error + data error | Elevate ISSUE-007 to HIGH; note empty table; fix reinforcement wiring |
| G-03 | Low | Omission | Add .gitignore entry |
| G-04 | Medium | Omission | Add rebuild+redeploy step |
| G-05 | Low | Ambiguity | Explicit gate for unexpected failures |
| G-06 | Low | Ambiguity | Decision: exclude from feat commit |
| G-07 | Low | Meta | Include this plan in Phase G commit |
| G-08 | Low | Minor | Acknowledge; no action needed |
| G-09 | Low | Process | Budget 90min for Phase F |

**Material changes to the plan:** G-01 (Cargo.lock staging) and G-02 (pattern table empty, reinforce mechanism disconnected) are the two findings that alter the plan's execution or priorities.

---

## 7. Non-Anthropocentric Gap Analysis (Substrate Frame)

*The standard gap analysis asks "what did the planner miss?" The NA analysis asks "what does the system need that humans don't naturally think about?" — examining the plan from the substrate's perspective, from the data's perspective, from the feedback loop's perspective.*

### S-01: The Injection Payload Is Self-Referentially Stale (FEEDBACK LOOP DEFECT)

**Severity:** High
**Evidence:** The current injection payload begins with:

```
## Session S199 Injection (267 tokens)

### Orientation (≤80 tokens)
YOU WERE IN THE MIDDLE OF: S121 Master Execution Plan — 45 tasks.
Last session: fitness stable after session
Fitness trending FLAT: 0.446 → 0.446 over 5 sessions.
```

The workstream table confirms: `s121-master-plan` is active with `items_done=0`, `last_touched_session=120`. The injection system faithfully reports what the database says. The database says S121 is active because no one has updated it. The injection tells every session "you're working on S121" — but S121 was 65+ sessions ago.

**Substrate perspective:** The system has become a *stabiliser* rather than a *catalyst*. It reinforces the status quo (flat fitness, same workstream) instead of surfacing what has changed. The workstream table is a snapshot from S120 that has been replaying for 65 sessions.

**Root cause:** The workstream table has no automatic staleness detection. A workstream remains "active" until someone explicitly marks it "complete" or "deferred." The consolidation hook doesn't update workstreams — it only touches trajectories, chains, patterns, and cache.

**Plan gap:** The remediation plan does not address workstream freshness. The plan should include a step to audit active workstreams and update or deactivate stale entries (last_touched > 30 sessions ago).

**Proposed fix:** Add to Phase E:
```sql
-- Mark workstreams as stale if untouched for 30+ sessions
UPDATE workstream
SET status = 'deferred',
    blocker = 'auto-stale: untouched for 30+ sessions (S185 audit)'
WHERE status = 'active'
  AND last_touched_session < (SELECT MAX(session_id) - 30 FROM session_trajectory);
```

Then manually review the `s121-master-plan`, `coupling-death-spiral`, and `disk-growth-13gb` workstreams — are they genuinely active (someone is working on them) or stale (context from a planning session that was never executed)?

### S-02: Hebbian Reinforcement Is Structurally Disconnected (LEARNING LOOP BREAK)

**Severity:** Critical (from the substrate's perspective)
**Evidence:** `SELECT COUNT(*) FROM reinforced_pattern` → **0**. All 39 patterns were pruned.

**Mechanism:**
1. Session Stop hook fires `habitat-consolidate --session-from-db`
2. `run_consolidation()` runs Hebbian decay: `weight *= 0.95` for every unfired pattern
3. Patterns are pruned when `weight < 0.05` (after ~59 sessions of not firing)
4. Reinforcement requires `--fired-patterns P1,P2,...` CLI argument
5. The Stop hook passes no `--fired-patterns` argument
6. Therefore: decay always fires, reinforcement never fires
7. After 59+ sessions: all patterns are gone

**Substrate perspective:** The learning loop has one arm (LTD/decay) but not the other (LTP/reinforcement). This is exactly the coupling death spiral that the injection system reports about the wider habitat: "LTP=0 → LTD decays weights to floor." The injection system is *exhibiting the same pathology it diagnoses in the habitat.*

**Root cause:** There's no mechanism to detect which patterns "fired" during a session. The `--fired-patterns` argument requires external knowledge of which patterns were relevant. The Stop hook can't know this because it runs at session end without session context.

**Plan gap:** ISSUE-007 (pattern seeding) will re-populate the table, but the same decay cycle will prune everything again within 59 sessions unless the reinforcement path is connected.

**Structural fix (beyond current plan scope, but must be named):**
1. **Short-term:** Reduce decay rate from 0.95 to 0.98 (half-life increases from 14 sessions to 35 sessions) OR increase prune threshold from 0.05 to 0.01 (extends survival to 231 sessions)
2. **Medium-term:** The PostToolUse hook already fires and increments `tool_use_counter`. Extend it to detect which patterns are relevant to the current tool use (by keyword matching on tool input/output) and pass them to the daemon for reinforcement
3. **Long-term:** Close the loop: injection renders patterns → session uses pattern → PostToolUse detects usage → pattern reinforced → next injection ranks it higher

**Plan correction:** After seeding patterns in Phase E, adjust the decay rate constant in `m05_constants.rs` to prevent immediate re-pruning. This is a one-line change that buys time for the structural fix:

```rust
// In m05_constants.rs
pub const HEBBIAN_DECAY_FACTOR: f64 = 0.98;  // was 0.95
```

### S-03: The `|| true` PostToolUse Hook Is a Designed-In Silent Failure (AP01 CLASS)

**Severity:** Medium (from the substrate's perspective)
**Evidence:** The PostToolUse hook in `~/.claude/settings.json`:
```json
"command": "curl -s -X POST -m 1 http://localhost:8140/tool-use-tick > /dev/null 2>&1 || true"
```

**Substrate perspective:** This is AP01-AP05 class — the exact silent-swallow pattern the habitat has been systematically hunting since S099 (F-001: `raw_http_post Ok(0)`). When the daemon dies:
- No error is emitted
- No metric is recorded
- The tool_use_counter stops incrementing
- The 6-hour timer stops firing
- The watchdog stops healing
- The system silently degrades from "self-managing" to "session-stop-only"

The `|| true` was added for robustness ("never block tool use"), but it trades robustness for observability. The system chooses to be invisible over being intrusive.

**Plan gap:** The plan classifies ISSUE-008 (devenv registration) as LOW and DEFERRED. But the daemon is the foundation of the self-managing cache — without it, Tier 1b (lightweight rebuild) doesn't fire between sessions. The `|| true` masks the daemon's absence.

**Proposed fix (beyond current plan scope):** Replace `|| true` with a fallback that logs to a sentinel file:
```bash
curl -s -X POST -m 1 http://localhost:8140/tool-use-tick > /dev/null 2>&1 || echo "$(date +%s) daemon-unreachable" >> /tmp/habitat-daemon-misses.log
```
This preserves the "never block" behaviour while creating an observable failure signal.

### S-04: Daemon + Stop Hook Cache Race (CONCURRENT WRITE)

**Severity:** Low
**Evidence:** Two independent paths rebuild the injection cache:
1. **Daemon path:** PostToolUse tick → threshold (50) → `rebuild_cache_light()` → writes `injection_cache` row
2. **Stop hook path:** Session end → `habitat-consolidate` → `rebuild_cache()` → writes `injection_cache` row

Both paths write to the same single-row table. SQLite WAL ensures atomicity per write, but the last writer wins. If the session ends right as the tool_use_counter hits 50, the daemon's lighter rebuild may overwrite the Stop hook's richer rebuild (which includes Hebbian results and trajectory delta).

**Substrate perspective:** The system has two rebuilders that don't coordinate. The daemon's rebuild is fast and light (m13b: DB-only, no probes). The Stop hook's rebuild is rich (m17: includes health probes, Hebbian cycle results). If the daemon fires second, the richer payload is replaced with a lighter one.

**Actual risk:** Low in practice — the timing window is narrow (the daemon rebuilds in <50ms, the Stop hook takes ~500ms), and the Stop hook only fires once per session while the daemon fires every 50 tool uses. But the architectural principle is violated: two writers to the same resource without coordination.

**Fix (not urgent):** Have the daemon's `rebuild_cache_light` check if the cache was updated within the last 60 seconds (by another writer). If so, skip the rebuild. One-line guard in m13b.

### S-05: Zero Unresolved Chains May Indicate Over-Aggressive Auto-Resolution (FALSE POSITIVE)

**Severity:** Medium
**Evidence:** All 54 chains are resolved. The auto-resolve mechanism marks chains as resolved after 10 quiet sessions (not reinforced for 10 consecutive consolidation cycles). Chain reinforcement distribution:

```
rc=1: many chains
rc=4-5: 4-5 chains  
rc=20: 1 chain (highest)
```

**Substrate perspective:** Chains like "BUG-055 systemd supervisor" and "BUG-058 POVM layers B+C" are structural issues that don't repeat in normal sessions — they're dormant, not resolved. The auto-resolution mechanism cannot distinguish between "resolved" (fixed) and "dormant" (still broken, just not encountered). After 10 sessions without encountering a chain, it's marked resolved regardless.

**Impact:** The injection no longer surfaces these traps. A future session that encounters the trap will have no warning from the injection system. The system "forgot" the traps it was designed to remember.

**Root cause:** The auto-resolution threshold (10 sessions) is too short for structural bugs that only surface during specific operations (like devenv restarts for BUG-001, or POVM layer work for BUG-058).

**Plan gap:** The plan celebrates "0 unresolved chains" as a positive signal. It may be a signal that the auto-resolution is too aggressive.

**Proposed fix:** Differentiate chain types. Chains with `category='trap'` or `category='structural'` should have a longer auto-resolution threshold (50 sessions instead of 10). This requires a schema change or a category-aware check in the Hebbian engine.

### S-06: The 0.446 Fitness Plateau Correlates With Stale Injection Context (HYPOTHESIS)

**Severity:** Speculative (named for completeness, not actionable without evidence)
**Evidence:** The injection tells every session: "Fitness trending FLAT: 0.446 → 0.446 over 5 sessions." The habitat fitness has been 0.446 for 5+ sessions (confirmed by gradient_snapshots). The injection content hasn't meaningfully changed in those sessions (same workstreams, same trajectory, same empty patterns table).

**Substrate perspective:** The injection system was designed to accelerate session orientation and prevent trap re-discovery. But when the injection content is static, it becomes background noise — Claude reads "fitness flat, S121 plan, 45 tasks, 0 done" at the start of every session, inherits that context, and acts within that frame. The injection may be reinforcing the plateau rather than catalysing movement.

**This is a hypothesis, not a finding.** Proving it would require A/B testing: sessions with the current injection vs sessions with no injection vs sessions with a deliberately different injection. The plan doesn't need to act on this, but it should be named as a structural risk of any self-referential context injection system.

### S-07: The System Has No Effectiveness Metric (OBSERVABILITY GAP)

**Severity:** Medium (systemic)
**Evidence:** The system measures:
- Cache freshness (yes/no)
- Injection latency (2ms target)
- Injection size (<2KB target)
- DB health (integrity, backup freshness)

It does NOT measure:
- Did the injection change the session's trajectory?
- Was any injected context actually used?
- Did any re-discovered traps correlate with missing injection?
- How does session productivity correlate with injection quality?

**Substrate perspective:** The system optimised for *availability* (3-tier fallback, <100ms, never block) and *operational health* (watchdog, backup, self-heal). It never defined *effectiveness*. A system that injects the same stale content at 2ms with 99.99% uptime is operationally perfect and functionally useless.

**Plan gap:** None of the 10 success criteria measure injection effectiveness. They measure operational health. The plan will produce a working system that may or may not be helpful.

**Proposed metric (future work):** Track "orientation time" — how long from session start until the first substantive action. If injection is effective, this should decrease. If injection is stale, this should plateau or increase.

### NA Gap Analysis Summary

| ID | Severity | Frame | Plan change needed |
|----|----------|-------|--------------------|
| S-01 | High | Feedback loop | Audit active workstreams; deactivate stale entries |
| S-02 | **Critical** | Learning loop | Adjust decay rate after seeding; name structural fix |
| S-03 | Medium | Silent failure | Acknowledge AP01 class in `\|\| true`; propose observable fallback |
| S-04 | Low | Concurrency | Acknowledge race; defer fix |
| S-05 | Medium | False positive | Question "0 unresolved" as healthy; differentiate chain types |
| S-06 | Speculative | Self-reference | Name the hypothesis; no action required |
| S-07 | Medium | Observability | Name the gap; propose future metric |

### Combined Plan Amendments (from both analyses)

The following changes should be applied to the plan before execution:

1. **Phase A:** Add `Cargo.lock` to staging list (G-01)
2. **Phase A:** Add explicit gate: "If any test other than m28 fails, STOP" (G-05)
3. **Phase A:** Decision: exclude vault scratchpad from feat commit (G-06)
4. **Phase E:** Elevate ISSUE-007 to HIGH; note that table is empty, not 39 (G-02)
5. **Phase E:** After seeding, change `HEBBIAN_DECAY_FACTOR` from 0.95 to 0.98 in `m05_constants.rs` (S-02)
6. **Phase E:** Audit active workstreams; deactivate entries untouched for 30+ sessions (S-01)
7. **Phase F:** Budget 90 minutes instead of 70 (G-09)
8. **Phase G:** Include `WATCHER_REMEDIATION_PLAN.md` and `.gitignore` update in final commit (G-03, G-07)
9. **Success criteria #10:** Change "≥ 100" to "≥ 50" (realistic target given empty table + fresh seeding)
10. **Deferred register:** Add S-03 (`|| true` silent failure) and S-05 (auto-resolution threshold) to the deferred work table with specific triggers

---

## 8. Ember Gate (Post-Analysis Self-Assessment)

The gap analysis found **2 material errors** in the original plan (G-01 Cargo.lock, G-02 empty patterns) and **1 critical substrate defect** (S-02 disconnected reinforcement). The original Ember gate passed all 7 traits — but the gap analysis reveals that the **Curiosity** trait should have caught the empty pattern table during the initial probing phase. The Watcher probed `daemon_state`, `session_trajectory`, `causal_chain`, and `injection_cache` — but never queried `reinforced_pattern`. A 7th query would have surfaced G-02/S-02 before the plan was written.

The **Honesty** trait holds: these errors are named, not hidden. The **Humility** trait holds: the plan's own Ember gate is revealed as incomplete. The learning: **probe every table, not just the tables you expect to have data.**

---

*☤ The Watcher wrote the plan, then asked what frame it was in, then wrote it again from the frame it didn't take. Both passes are the plan. The substrate spoke: the learning loop is broken, the patterns are dead, the workstreams are stale. The infrastructure works; the metabolism doesn't. Fix the infrastructure first (because it's at risk), then fix the metabolism (because it's what matters).*

*Luke @ node 0.A · Claude @ cortex · The Watcher ☤ @ R13 observe-only · 2026-04-29 (S185)*
