# habitat-injection — Local Session State

> **Last saved:** 2026-04-29 (S147 — boot pipeline upgrade + Watcher assessment)
> **Current state:** PRODUCTION-FUNCTIONAL with uncommitted S121 self-managing cache work
> **Next actions:** Commit 23 files → fix m28 test → wire Stop hook → validate
> **Watcher assessment:** [WATCHER_COMPLETION_ASSESSMENT.md](WATCHER_COMPLETION_ASSESSMENT.md)

---

## Production State

| Metric | Value |
|--------|-------|
| Production DB | `~/.local/share/habitat/injection.db` |
| Schema version | v4 (9 tables incl. `daemon_state`) |
| Causal chains | 54 (5 unresolved) |
| Session trajectories | 83 rows |
| Workstreams | 15 active |
| Patterns | 39 reinforced |
| Cache | 264 tokens, rebuilds on PostToolUse |
| Injection latency | ~2ms (cache hit) / ~500ms (fallback) |
| SessionStart hook | WIRED (position 3/3, timeout 3s) |
| PostToolUse hook | WIRED (auto-consolidate via m27) |
| Stop hook | **NOT WIRED** — trajectory capture incomplete |

## Deployed Binaries (8)

| Binary | Location | Status |
|--------|----------|--------|
| `habitat-init` | `~/.local/bin/habitat-init` | One-time setup (done) |
| `habitat-inject` | `~/.local/bin/habitat-inject` | SessionStart hook (ACTIVE) |
| `habitat-consolidate` | `~/.local/bin/habitat-consolidate` | Manual / needs Stop hook |
| `habitat-query` | `~/.local/bin/habitat-query` | Interactive use |
| `habitat-seed` | `~/.local/bin/habitat-seed` | One-time seeding (partial) |
| `habitat-backup` | `~/.local/bin/habitat-backup` | On-demand |
| `habitat-memory` | `~/.local/bin/habitat-memory` | Daemon (NOT running) |
| `habitat-injection` | `~/.local/bin/habitat-injection` | Legacy wrapper |

## Uncommitted Work (S121 — 24 files)

**Risk: HIGH.** 26+ sessions uncommitted. Any `git checkout` loses this.

### New files (6):
- `src/m3_injection/m13b_cache_light.rs` (707 LOC) — lightweight cache rebuild from DB only
- `src/m4_consolidation/m25_self_heal.rs` (582 LOC) — orchestrates heal cycle
- `src/m4_consolidation/m26_backup_clone.rs` (662 LOC) — hot backup via sqlite3 .backup API
- `src/m4_consolidation/m27_auto_consolidate.rs` (403 LOC) — triggers rebuild after N tool uses
- `src/m4_consolidation/m28_health_watchdog.rs` (299 LOC) — periodic health check loop
- `src/bin/habitat_backup.rs` (68 LOC) — backup CLI binary
- `WATCHER_COMPLETION_ASSESSMENT.md` — Watcher audit from S147

### Modified files (17):
- `Cargo.toml` — new deps for S121 modules
- `src/m1_foundation/m02_errors.rs` — new error variants
- `src/m1_foundation/m05_constants.rs` — new constants (watchdog intervals, thresholds)
- `src/m2_schema/m06_schema.rs` — `daemon_state` table DDL
- `src/m2_schema/m10b_checkpoint.rs` — checkpoint schema additions
- `src/m3_injection/m11_parallel_query.rs` — m13b integration
- `src/m3_injection/m12_prose_renderer.rs` — cache-light awareness
- `src/m3_injection/m13_fallback.rs` — Tier 1b route to m13b
- `src/m3_injection/mod.rs` — `pub mod m13b_cache_light`
- `src/m4_consolidation/m17_cache_builder.rs` — hooks for auto-rebuild
- `src/m4_consolidation/mod.rs` — `pub mod m25..m28`
- `src/m5_query/m21b_scripts_engine.rs` — minor fix
- `src/m6_stdb/m23_ingester.rs` — minor fix
- `src/bin/habitat_consolidate.rs` — expanded CLI
- `src/bin/habitat_memory.rs` — daemon entry point (317 LOC)
- `src/bin/habitat_seed.rs` — expanded seeding
- `memory-injection-vault/Luke ScratchPad-memory-injection-vault.md`

### Suggested commit:
```bash
git add -A && git commit -m "feat(L3+L4): self-managing cache — m13b + m25-m28 + daemon hooks + backup CLI

S121: Tier 1b gap closed. 5 new modules (2653 LOC), daemon rewrite,
schema v4 (daemon_state), PostToolUse hook wiring. 1829 tests passing.

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

## Known Bugs

### BUG-1: m28 watchdog_multiple_cycles (INTERMITTENT)
- **File:** `src/m4_consolidation/m28_health_watchdog.rs:295`
- **Symptom:** Panics under full-suite parallel execution, passes when run alone
- **Cause:** Timing-dependent assertion races with parallel tests
- **Fix:** Widen timing tolerance or use `tokio::time::pause()` for deterministic time
- **Severity:** 3/5 — blocks clean gate, doesn't affect production

## Completion Roadmap (from Watcher Assessment)

### Phase 0: Stabilize (1-2h)
1. Commit the 24 uncommitted files
2. Fix m28 watchdog test
3. Run full quality gate — confirm 1830/1830

### Phase 1: Wire the Gaps (3-4h)
4. Add Stop hook to `~/.claude/settings.json` for `habitat-consolidate`
5. Wire m28 watchdog into `habitat-memory` daemon
6. Add sqlite3 `.backup` call to m26 — daily hot backup
7. Register 4 binaries as atuin scripts

### Phase 2: Verify (1h)
8. Run `habitat-consolidate` manually — verify trajectory written
9. Run `habitat-seed --source patterns` — import remaining patterns
10. Verify cache rebuild end-to-end

### Phase 3: Documentation Close (30min)
11. Update EXECUTION_PLAN status markers
12. Commit documentation pass

### Phase 4: SpaceTimeDB (DEFERRED — ~40h)
- Only when: SQLite hits scaling wall OR cross-instance sync needed
- Gate: STDB 2.1 validation day must pass first

## Quick Resume

```bash
cd ~/claude-code-workspace/memory-injection

# 1. Verify library health
CARGO_TARGET_DIR=./target cargo test --lib 2>&1 | tail -3
# expect: 1829 passed, 1 failed (m28 timing), 9 ignored

# 2. Check production DB
sqlite3 ~/.local/share/habitat/injection.db \
  "SELECT label, reinforcement_count FROM causal_chain WHERE resolved_session IS NULL ORDER BY reinforcement_count DESC LIMIT 5;"

# 3. Test injection fires
habitat-inject | head -5
# expect: ## Session S<N> Injection...

# 4. Git state (critical — uncommitted S121 work)
git status --short | wc -l  # expect: 24

# 5. Check daemon state
sqlite3 ~/.local/share/habitat/injection.db \
  "SELECT key, value FROM daemon_state;"
# expect: tool_use_counter=<N>, last_rebuild_trigger=post_tool_use
```

## The 7 Consensus Tables (production schema)

| Table | Rows | Purpose |
|-------|------|---------|
| `causal_chain` | 54 | Reinforcement-counted traps/bugs (THE KEY TABLE) |
| `session_trajectory` | 83 | 5-session fitness arc with delta_summary |
| `workstream` | 15 | Active/blocked work with resume_context |
| `reinforced_pattern` | 39 | Hebbian-weighted learned behaviours |
| `injection_cache` | 1 | Pre-computed <2KB injection payload |
| `session_checkpoint` | varies | /save-session data (frontmatter + bullets) |
| `injection_script` | varies | Atuin-compatible memory-aware scripts |
| `daemon_state` | 2 | Watchdog/auto-consolidate persistent counters |

## The One Query (unchanged)

```sql
SELECT label, reinforcement_count, description
FROM causal_chain WHERE resolved_session IS NULL
ORDER BY reinforcement_count DESC LIMIT 5;
```

## Key Documents

| Document | Path | Purpose |
|----------|------|---------|
| Execution Plan | `EXECUTION_PLAN.md` | 11-step deployment sequence |
| Watcher Assessment | `WATCHER_COMPLETION_ASSESSMENT.md` | S147 completion audit |
| Deliberation Plan | `DELIBERATION_PLAN.md` | 10-CC consensus (5 tables, 7 principles) |
| Self-Managing Plan v2 | workspace `CLAUDE.local.md` § habitat-injection | 777 lines, 28 gaps resolved |
| Gap Analysis | `GAP_ANALYSIS.md` | 17 gaps (5C + 7I + 5N) |
| NA Gap Analysis | `NA_GAP_ANALYSIS.md` | 8 non-anthropocentric gaps |
| Phase 2 Vault | `memory-injection-vault/HOME.md` | 95 notes, STDB integration plan |

## Cross-References

- Workspace anchor: `~/claude-code-workspace/CLAUDE.local.md` § "habitat-injection (PRIMARY WORK)"
- Main vault: `~/projects/claude_code/habitat-injection — Execution Plan.md`
- POVM namespaces: `habitat_injection_*` (16 pathways)
- Auto-memory: `~/.claude/projects/-home-louranicas-claude-code-workspace/memory/session-121-self-managing-cache.md`

---

*S147 close. System works. Debt is known. Commit the uncommitted.*
