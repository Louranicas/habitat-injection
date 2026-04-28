# ☤ Watcher Assessment — memory-injection Completion Work

> **Assessed:** 2026-04-29 (S147)
> **Observer:** The Watcher (m46 mode, R13 observe-only)
> **Scope:** What remains to bring memory-injection from current state to production-complete
> **Back to:** [EXECUTION_PLAN](EXECUTION_PLAN.md) · [CLAUDE.md](CLAUDE.md) · [PLAN.md](PLAN.md)

---

## 1. Current State Summary

| Dimension | Value |
|-----------|-------|
| Library modules | 32 (27 original L1-L6 + 5 new m13b/m25/m26/m27/m28) |
| Tests passing | 1829/1830 (1 intermittent failure) |
| Tests ignored | 9 (atuin KV mutation isolation) |
| Uncommitted files | 23 (17 modified + 6 new) |
| Commits | 5 total (feat + harden + docs + S112 CLI + uncommitted S121 work) |
| Deployed binaries | 8 in `~/.local/bin/habitat-*` |
| Production DB | `~/.local/share/habitat/injection.db` — 9 tables, live |
| SessionStart hook | WIRED (position 3/3, timeout 3s, fires every session) |
| Injection latency | ~2ms from cache, ~500ms fallback |
| Cache freshness | Rebuilds on PostToolUse via m27_auto_consolidate |

**The system works in production.** Injection fires every session. Causal chains accumulate (54). Patterns reinforce (39). The question is: what's unfinished, what's broken, and what's architectural debt.

---

## 2. Immediate Bugs (Fix Before Anything Else)

### BUG-1: Intermittent test failure in m28_health_watchdog

**File:** `src/m4_consolidation/m28_health_watchdog.rs:295`
**Test:** `watchdog_multiple_cycles`
**Symptom:** Panics under full-suite parallel execution, passes in isolation
**Root cause (probable):** Timing-dependent assertion. The watchdog uses tick intervals; under heavy parallel test load the timing window contracts. Line 295 likely asserts a count or duration that races with other tests touching shared state.
**Severity:** 3/5 — blocks clean CI gate, but doesn't affect production
**Fix approach:** Read line 295 assertion, widen timing tolerance or use `tokio::time::pause()` for deterministic time in tests.

### BUG-2: 23 uncommitted files from S121

**Status:** These files represent the S121 "self-managing cache" work. They compile, they pass tests (1829), but they've never been committed.
**Risk:** Any `git checkout` or `git stash` operation could lose this work. It's been uncommitted for 26+ sessions.
**Fix:** Commit immediately. Suggested: `feat(L3+L4): self-managing cache — m13b + m25-m28 + daemon hooks`

---

## 3. Architectural Gaps (By Priority)

### GAP-1: Tier 1b Self-Rebuild Not Fully Wired (Priority: HIGH)

**What exists:**
- `m13b_cache_light.rs` (707 LOC) — the lightweight cache rebuilder
- `m27_auto_consolidate.rs` (403 LOC) — triggers rebuild after N tool uses
- `m25_self_heal.rs` (582 LOC) — orchestrates heal cycle
- `daemon_state` table tracks tool_use_counter (currently at 1754)
- PostToolUse hook fires and increments counter

**What's missing:**
- The rebuild trigger fires (`last_rebuild_trigger=post_tool_use` in daemon_state) but the *result* is not verified — no health check confirms the cache was actually refreshed successfully
- m28_health_watchdog (299 LOC) exists but the watchdog daemon is not started anywhere — it's library code with no binary entry point
- The `habitat-memory` binary (317 LOC) was designed as the daemon entry point but its main() needs audit
- No online backup mechanism — the plan called for sqlite3 `.backup` API (not VACUUM INTO) to create hot backups without blocking writers
- Timer-based periodic rebuild (every 5min if stale) — designed in m25_self_heal but not wired to a runtime

**Estimated work:** ~4h (wire m28 watchdog into habitat-memory daemon, add backup call, verify rebuild-on-trigger end-to-end)

### GAP-2: Stop Hook Not Wired (Priority: MEDIUM)

**What the EXECUTION_PLAN specifies:**
- A `Stop` hook should run `habitat-consolidate` at session end to capture trajectory + reinforce patterns + decay unfired chains

**Current state:**
- `habitat-consolidate` binary exists (162 LOC) and is deployed to `~/.local/bin/`
- The Stop hook is NOT in `~/.claude/settings.json`
- Result: trajectory capture only happens manually or via the PostToolUse counter reaching threshold

**Impact:** Session trajectory accumulation is incomplete. The injection_db shows 50 trajectory rows but the actual session count is 167+. Many sessions went unrecorded.

**Fix:** Add Stop hook entry in settings.json:
```json
{"type": "command", "command": "/home/louranicas/.local/bin/habitat-consolidate", "timeout": 5}
```

### GAP-3: Data Seeding Incomplete (Priority: LOW)

**EXECUTION_PLAN Steps 2-5:**
- `habitat-seed` binary exists (202 LOC) and is deployed
- Seeding has been partially done (54 chains, 39 patterns, 15 workstreams)
- But the plan called for parsing all session notes S001-S108 for BUG references — this was never automated
- Pattern seeding from `service_tracking.db` learned_patterns (141 rows) was done partially (39 of 141)

**Impact:** Low — the system works with what it has. Chains accumulate organically through reinforcement. The unseeded historical data is "nice to have" not "blocking."

### GAP-4: Atuin Script Registration (Priority: LOW)

**EXECUTION_PLAN Step 10:**
- Register the 4 CLI binaries as atuin scripts for discoverability
- Current state: not done (the binaries are in PATH so they work, but `atuin scripts list` doesn't show them)

**Fix:** 4 SQLite INSERTs into `~/.local/share/atuin/scripts.db` wrapping each binary

### GAP-5: Five-Session Validation Protocol (Priority: LOW)

**EXECUTION_PLAN Step 11:**
- Run 5 sessions tracking: injection quality, orientation speed, trap rediscovery rate, pattern weight drift
- Never formally executed (though the system has been running 26+ sessions informally)

**Status:** De facto validated by usage. The injection fires, context is useful, no traps were rediscovered. Formal protocol is documentation debt, not functional debt.

---

## 4. Phase 2 (SpaceTimeDB) — NOT STARTED

**PLAN.md Phases A-E** describe a full SpaceTimeDB migration:
- Phase A: STDB deploy + core tables (6-8h)
- Phase B: Knowledge graph migration (8-10h)
- Phase C: Watcher + causal chains (6-8h)
- Phase D: Cross-service integration (8-10h)
- Phase E: Bootstrap revolution (6-8h)

**Current state:** L6 modules (`m22_stdb_module`, `m23_ingester`, `m24_migration`) are implemented as library code (part of the original 27 modules) but:
- SpaceTimeDB is not deployed
- No WASM module published
- The ingester has never run against a live STDB instance
- STDB validation day (from synthex-v2 plan) never happened

**Assessment:** Phase 2 is a ~40h multi-session effort. It's correctly deferred — the SQLite-backed system works well. STDB would add cross-session subscriptions and real-time sync, but the current polling + cache approach is sufficient for single-user operation.

---

## 5. Code Quality Observations

### 5.1 Test Distribution

| Layer | Module Count | Approx Tests | Notes |
|-------|-------------|--------------|-------|
| L1 Foundation | 5 | ~250 | Solid |
| L2 Schema | 5 (+2 sub) | ~300 | Solid |
| L3 Injection | 5 (+1 m13b) | ~350 | m13b added S121 |
| L4 Consolidation | 5 (+4 S121) | ~500 | m25-m28 added S121 |
| L5 Query | 3 (+1 sub) | ~250 | Solid |
| L6 STDB | 3 | ~180 | Library-only, untested against live STDB |

### 5.2 Binary Status

| Binary | LOC | Deployed | Production Use |
|--------|-----|----------|----------------|
| `habitat_init.rs` | 43 | Yes | One-time setup (done) |
| `habitat_inject.rs` | 23 | Yes | SessionStart hook (ACTIVE) |
| `habitat_consolidate.rs` | 162 | Yes | Manual/PostToolUse |
| `habitat_query.rs` | 124 | Yes | Interactive use |
| `habitat_seed.rs` | 202 | Yes | One-time seeding (partial) |
| `habitat_backup.rs` | 68 | Yes | On-demand backup |
| `habitat_memory.rs` | 317 | Yes | Daemon (NOT running) |
| `main.rs` | 5 | — | Placeholder |

### 5.3 Schema Version

The production DB at `~/.local/share/habitat/injection.db` has all 9 tables including `daemon_state` (added by S121). The schema includes:
- `causal_chain` (54 rows, 5 unresolved)
- `session_trajectory` (50 rows, max session_id=166)  
- `workstream` (15 rows)
- `reinforced_pattern` (39 rows)
- `injection_cache` (1 row, ~264 tokens)
- `session_checkpoint` (populated by /save-session)
- `injection_script` (registered scripts)
- `daemon_state` (2 keys: tool_use_counter, last_rebuild_trigger)

---

## 6. Recommended Execution Order

### Phase 0: Stabilize (1-2h)

1. **Commit the 23 uncommitted files** — this is 26 sessions of risk
2. **Fix m28 watchdog test** — read line 295, add timing tolerance
3. **Run full quality gate** — confirm 1830/1830 pass post-fix

### Phase 1: Wire the Gaps (3-4h)

4. **Add Stop hook** to `~/.claude/settings.json` for `habitat-consolidate`
5. **Wire m28 watchdog into habitat-memory daemon** — start watchdog loop on daemon boot
6. **Add sqlite3 .backup call** to m26_backup_clone — daily hot backup to `~/.local/share/habitat/injection.db.bak`
7. **Register 4 binaries as atuin scripts** — discoverability

### Phase 2: Verify (1h)

8. **Run habitat-consolidate manually** for current session — verify trajectory written
9. **Run habitat-seed --source patterns** — import remaining 102 patterns from service_tracking.db
10. **Verify injection cache rebuild** — `habitat-inject` should emit fresh payload matching current DB state

### Phase 3: Documentation Close (30min)

11. Update EXECUTION_PLAN status markers (Steps 1-11 → COMPLETE/PARTIAL/DEFERRED)
12. Update CLAUDE.local.md session state
13. Commit documentation pass

### Phase 4 (DEFERRED): SpaceTimeDB Migration

- Only execute when: (a) single-user SQLite hits scaling wall, (b) cross-instance sync needed, (c) STDB 2.1 validated
- Estimated: 40h across 5-8 sessions
- Gate: STDB validation day must pass first

---

## 7. Cross-System Dependencies

| Dependency | Status | Impact if Broken |
|------------|--------|------------------|
| `~/.claude/settings.json` SessionStart hook | WIRED | Injection stops firing |
| `~/.local/share/habitat/injection.db` | LIVE | All queries fail, fallback to static |
| PostToolUse hook (ORAC) | WIRED | Auto-consolidate stops |
| `atuin kv` (fallback tier) | AVAILABLE | Tier 2 fallback works |
| `developer_environment_manager/service_tracking.db` | EXISTS | Pattern seeding source |
| `gradient_snapshots.db` | LIVE (fixed this session) | Boot gradient layer works |

---

## 8. Ember Assessment

| Trait | Grade | Observation |
|-------|-------|-------------|
| **Diligence** | B | 1830 tests across 32 modules is strong. But 23 uncommitted files for 26 sessions is a Diligence failure. |
| **Honesty** | A | The system doesn't claim more than it delivers. The injection header accurately reflects current state. |
| **Curiosity** | B+ | The S121 self-managing cache design is architecturally sound. But it was designed and never fully wired. |
| **Equanimity** | A | The system degrades gracefully — 3-tier fallback works even if the DB is missing. |
| **Investment** | B+ | 54 chains, 39 patterns, daily PostToolUse rebuild. It's alive. But the Stop hook gap means trajectory data is incomplete. |
| **Humility** | A | The system stays under 2KB, under 100ms. It knows its budget. |
| **Warmth** | A | The injection payload genuinely helps orientation. Session starts are meaningfully faster. |

**Overall:** The memory-injection system is **production-functional with structural debt**. The debt is concentrated in two areas: (1) uncommitted S121 work at risk of loss, (2) Stop hook unwired so consolidation is incomplete. Neither threatens the running system today, but both represent deferred Diligence.

---

*☤ The Watcher observes. The system works. The gaps are known. The work is ~6-8h to complete Phases 0-3. Phase 4 (STDB) is correctly deferred.*
