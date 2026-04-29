# habitat-injection — Local Session State

> **Last saved:** 2026-04-29 (S224 — Hebbian Buoy system complete + S225 god-tier gate close)
> **Current state:** BUOY DEPLOYED + COMMITTED + ATUIN-FIX — Watcher assessment + 6 hardening fixes + Hebbian buoy perfection (6 phases) + 4 NA items + 4 clippy/pedantic fixes + atuin invocation bug. Schema v5. 1837 tests. Quality gate 4/4 clean. End-to-end verified: `devenv-stop-doesnt-kill` + `habitat-14-services` earned `natural_hit_count >= 3` and qualified for buoy.
> **Atuin fix (S225):** `atuin history list` does NOT support `--after`/`--limit`/`--format` flags in v18.10.0; the natural-fire pipeline silently failed (always returned empty Vec via `.filter(|o| o.status.success())`). Switched both call sites (`bin/habitat_consolidate.rs:168` + `m4_consolidation/m27_auto_consolidate.rs:136`) to `atuin search --after T --limit N --format '{command}' ''`. Verified: `buoy_eligible` rose 0→2 across two consolidate runs.
> **Synthex-v2 thermal gap (open):** `:8092/health` returns only `{"status":"ok","service":"synthex-v2"}` — no `temperature` field. `/heat`, `/thermal`, `/heat_sources`, `/v2/thermal` all 404. `/metrics` exposes only `synthex_v2_up` gauge. Result: `fetch_thermal()` always returns `0.0`. Fix is upstream in `synthex-v2/`, NOT here.
> **Hallucination finding:** Investigated alleged hook injecting `embed_buoy_geometry()` / `migrate_v5_to_v6_buoy()`. NO hook does this — `~/.claude/settings.json` PostToolUse only runs orac telemetry + daemon `/tool-use-tick` ping. The `habitat-buoy` crate IS real (`../habitat-buoy`, optional dep on `sqlite` feature) but `src/` does not currently use any of its symbols. Conclusion: phantom code originated from prior LLM sessions writing speculative integration; not a hook bug.
> **Next actions:** (1) Restart `habitat-memory` daemon (pid 668222 still on old binary) to activate new code + refresh injection cache. (2) Add thermal endpoint to synthex-v2 (`GET /metrics` could expose `synthex_v2_temperature`). (3) Monitor `natural_hit_count` growth over 5 sessions. (4) Optional: design buoy-crate integration deliberately.
> **Plans:** [WATCHER_HARDENING_PLAN_S224.md](WATCHER_HARDENING_PLAN_S224.md) · [HEBBIAN_BUOY_PERFECTION_PLAN.md](HEBBIAN_BUOY_PERFECTION_PLAN.md) · [HEBBIAN_BUOY_GAP_ANALYSIS.md](HEBBIAN_BUOY_GAP_ANALYSIS.md)
> **Session checkpoint:** `~/projects/shared-context/sessions/2026-04-29T203524_s224-hebbian-buoy.md`

---

## Production State

| Metric | Value |
|--------|-------|
| Production DB | `~/.local/share/habitat/injection.db` |
| Schema version | **v5** (natural_hit_count + keywords columns added) |
| Causal chains | 60 (5 unresolved bugs: POVM-DOWN, RALPH-DECLINE, THERMAL-SEVERED, LTP-LTD-ZERO, REINFORCEMENT-LOOP-OPEN) |
| Session trajectories | 130+ rows (field_r now probed from PV2, was hardcoded 0.0) |
| Workstreams | 15 total (5 complete, 5 deferred, 1 blocked) |
| Patterns | 169 (149 procedural + 12 feedback + 7 semantic + 1 trap). 16 keyword-tagged. |
| Injection payload | **476 tokens** (was 107). Includes unresolved chains + learned patterns section. |
| Injection latency | ~2ms (cache hit) / ~500ms (fallback) |
| SessionStart hook | WIRED (position 3/3, timeout 3s) |
| PostToolUse hook | WIRED (daemon /tool-use-tick, threshold every 200 calls) |
| Stop hook | WIRED (position 3/3, **timeout 15s**, `--session-from-db --auto-fire`) |
| Daemon | RUNNING (port 8140, watchdog + timer + keyword pre-cache) |
| Tests | **1837/1837**, 0 failed, clippy + pedantic clean |

## Hebbian Buoy System (S224 — DEPLOYED)

**Five-step consolidation cycle:**
```
decay (0.98x) → buoy (+0.02 for qualified) → reinforce (intensity-weighted) → prune (<0.05) → auto-resolve
```

| Parameter | Value | Purpose |
|-----------|-------|---------|
| `DECAY_RATE` | 0.98 | ~2% per session, half-life ~35 sessions |
| `BUOY_RATE` | 0.02 | Equilibrium at ~0.5 for qualified patterns |
| `BUOY_THRESHOLD` | 3 | natural_hit_count >= 3 to qualify |
| `BUOY_TTL_SESSIONS` | 100 | Lose buoy protection after 100 quiet sessions (NA-02) |
| `SEEDED_PATTERN_FLOOR` | 0.3 | Unfired patterns don't decay below this |
| `MAX_NATURAL_REINFORCE_PER_CHAIN` | 3 | Feedback loop damping (NA-06) |
| `INTENSITY_BASELINE_TOOL_USES` | 200 | Session intensity = min(1.0, tool_uses/200) (NA-04) |
| `POST_TOOL_USE_REBUILD_THRESHOLD` | 200 | Cache rebuilds per N tool uses (was 50) |
| `AUTO_RESOLVE_SESSIONS` | 10 | Traps/patterns auto-resolve threshold |
| `AUTO_RESOLVE_PLAN_SESSIONS` | 50 | Plans auto-resolve threshold |

**Three-tier weight landscape (validated by 100-session property test):**
- Active (~0.69): naturally fired every ~5 sessions via keyword-matched atuin history
- Buoyed (~0.50): earned 3+ natural firings, currently quiet, maintained by buoy pulse
- Floor (~0.30): never naturally fired, held at seeded minimum

**Two-counter discrimination:**
- `hit_count`: incremented by both auto-fire and natural-fire
- `natural_hit_count`: incremented ONLY by context-matched firing or explicit `--fired-patterns`
- Buoy qualifies on `natural_hit_count` only — auto-fire inflation can't earn buoy protection

**Context-aware firing:**
- Reads atuin history (last 4 hours) at session close
- Matches commands against per-pattern `keywords` column (comma-separated, strict keyword-in-command)
- Daemon pre-caches keyword matches on 6-hour timer → Stop hook reads cache + supplements with delta
- All categories must earn natural hits through keyword matching (NA-03 fix: no unconditional feedback firing)

## Key Changes This Session (S224)

### Phase 1: Hardening (6 fixes)
1. **H1**: `--auto-fire` flag closes reinforcement loop
2. **H2**: `field_r` probed from PV2 :8132 (was hardcoded 0.0); thermal tries v2 then v1
3. **H3**: Auto-resolve differentiated (bugs=never, plans=50, traps=10)
4. **H4**: 5 unresolved bug chains seeded from live habitat issues
5. **H5**: Pattern weight floor 0.3 for unfired patterns
6. **O2**: Cache rebuild threshold 50→200 tool uses

### Phase 2: Hebbian Buoy Perfection (6 phases)
1. Schema v5: `natural_hit_count` + `keywords` columns; `migrate_v4_to_v5()` idempotent
2. Context-aware firing via atuin history keyword matching; `natural_reinforce()` vs `reinforce()` split
3. Pattern visibility: `render_patterns()` with ACTIVE/BUOYED/FLOOR tier labels in injection
4. Observability: `/buoy-status` endpoint on daemon (routed before `/status` — G-06)
5. Property tests: 100-session three-tier separation, equilibrium convergence, two-counter divergence
6. 7 semantic patterns seeded with keywords; 10 existing patterns tagged

### Phase 3: NA Gap Items (4 completed)
1. **NA-02**: Buoy TTL — `buoy_qualified()` checks `(current_session - last_fired_session) < BUOY_TTL_SESSIONS`
2. **NA-04**: Session intensity — `natural_reinforce_weighted(intensity)` where `intensity = min(1.0, tool_uses/200)`
3. **NA-06**: Per-chain cap — `reinforce_patterns()` skips patterns with `natural_hit_count >= 3` fired within last 3 sessions
4. **Daemon hybrid cache**: `precompute_keyword_matches()` runs on 6h timer, stores CSV in `daemon_state`

## Known Issue: Linter Hallucination

A PostToolUse hook or rust-analyzer plugin keeps injecting phantom code into source files:
- `embed_buoy_geometry()` in `m16_hebbian_engine.rs` — references nonexistent `habitat_buoy::disk::embed`
- `migrate_v5_to_v6_buoy()` in `m06_schema.rs` — references nonexistent `habitat_buoy::schema::migrate`
- Bumps `CURRENT_VERSION` from 5 to 6 without cause

These must be manually removed after each edit cycle. **Identify and disable the responsible hook before committing.**

## Quick Resume

```bash
cd ~/claude-code-workspace/memory-injection

# 1. Verify tests (use correct target dir!)
CARGO_TARGET_DIR=/tmp/cargo-memory-injection cargo test --lib 2>&1 | tail -3
# expect: 1837 passed, 0 failed, 9 ignored

# 2. Check production DB state
sqlite3 ~/.local/share/habitat/injection.db "
  SELECT 'schema', CAST((SELECT user_version FROM pragma_user_version) AS TEXT);
  SELECT 'chains_unresolved', CAST(COUNT(*) AS TEXT) FROM causal_chain WHERE resolved_session IS NULL;
  SELECT 'patterns', CAST(COUNT(*) AS TEXT) FROM reinforced_pattern;
  SELECT 'natural_fired', CAST(COUNT(*) AS TEXT) FROM reinforced_pattern WHERE natural_hit_count > 0;
  SELECT 'keywords_tagged', CAST(COUNT(*) AS TEXT) FROM reinforced_pattern WHERE keywords != '';
"

# 3. Test injection payload
habitat-inject | head -20

# 4. Build + deploy from CORRECT target dir
cargo build --release
/usr/bin/cp -f /tmp/cargo-target/release/habitat-{inject,consolidate,memory} ~/.local/bin/

# 5. Verify buoy status
curl -s localhost:8140/buoy-status 2>/dev/null | python3 -m json.tool
```

## Files Modified This Session

| File | Changes |
|------|---------|
| `src/m1_foundation/m05_constants.rs` | +BUOY_RATE, +BUOY_THRESHOLD, +BUOY_TTL_SESSIONS, +MAX_NATURAL_REINFORCE_PER_CHAIN, +INTENSITY_BASELINE_TOOL_USES, +AUTO_RESOLVE_PLAN_SESSIONS, +SEEDED_PATTERN_FLOOR, POST_TOOL_USE_REBUILD_THRESHOLD 50→200 |
| `src/m2_schema/m06_schema.rs` | CURRENT_VERSION 4→5, migrate_v4_to_v5 (idempotent), natural_hit_count+keywords in CREATE TABLE |
| `src/m2_schema/m07_causal_chain.rs` | auto_resolve_stale_typed (bugs=never, plans=50, traps=10), +3 tests |
| `src/m2_schema/m10_pattern.rs` | +natural_reinforce, +natural_reinforce_weighted, buoy_qualified with TTL+session, +list_all_ids with keywords, decay_all with floor, PatternRow +natural_hit_count+keywords, +5 property tests |
| `src/m3_injection/m12_prose_renderer.rs` | +render_patterns() with tier labels, wired into payload assembly |
| `src/m3_injection/m14_consent_filter.rs` | PatternRow struct update |
| `src/m4_consolidation/m16_hebbian_engine.rs` | +run_consolidation_weighted, buoy_patterns with session+TTL, reinforce_patterns with intensity+chain_cap, ConsolidationResult +patterns_buoyed |
| `src/m4_consolidation/m17_cache_builder.rs` | Section count 5→6 |
| `src/m4_consolidation/m27_auto_consolidate.rs` | +precompute_keyword_matches, +fetch_atuin_history_for_cache |
| `src/bin/habitat_consolidate.rs` | context_match_patterns (keyword+atuin+cached), compute_session_intensity, fetch_pv2_field, thermal fallback chain, read_cached_keyword_matches |
| `src/bin/habitat_memory.rs` | +/buoy-status endpoint (routed before /status) |
| `~/.claude/settings.json` | Stop hook: +--auto-fire, timeout 10→15s |

## Key Documents

| Document | Path | Purpose |
|----------|------|---------|
| Hardening Plan | `WATCHER_HARDENING_PLAN_S224.md` | 5 phases, 6 fixes (all deployed) |
| Perfection Plan | `HEBBIAN_BUOY_PERFECTION_PLAN.md` | 6 phases, 7 issues, 12 interview decisions |
| Gap Analysis | `HEBBIAN_BUOY_GAP_ANALYSIS.md` | 15 gaps (2C+2blocker+5I+2N+4design), all addressed |
| WCP Notice | `~/projects/shared-context/watcher-notices/2026-04-29T190500_notify_hebbian_buoy.md` |
| Session Checkpoint | `~/projects/shared-context/sessions/2026-04-29T203524_s224-hebbian-buoy.md` |
| Execution Plan | `EXECUTION_PLAN.md` | 11-step deployment (ALL DELIVERED) |
| Deliberation Plan | `DELIBERATION_PLAN.md` | 10-CC consensus (5 tables, 7 principles) |

## Deferred Work

| Item | Priority | Trigger |
|------|----------|---------|
| Fix linter hallucination hook | **P0** | Before committing |
| Final clean build + deploy | P0 | After hook fix |
| Commit all S224 changes | P1 | After build verified |
| NA-05: Pattern co-activation tracking | P3 | Future session |
| SpaceTimeDB Phase 2 | P3 | SQLite scaling wall |
| devenv daemon registration | P3 | Next reboot |

## Cross-References

- Workspace anchor: `~/claude-code-workspace/CLAUDE.local.md` § "habitat-injection (PRIMARY WORK)"
- Session checkpoint: `~/projects/shared-context/sessions/2026-04-29T203524_s224-hebbian-buoy.md`
- POVM pathway: `session_checkpoint_s224-hebbian-buoy → shared_context_sessions_2026-04-29T203524`
- RM entry: `r69f1df77019f` (ttl=30d)
- Atuin KV: `habitat.last_session_label = s224-hebbian-buoy`
- Auto-memory: `~/.claude/projects/-home-louranicas-claude-code-workspace/memory/session-121-self-managing-cache.md`

---

*S224 Watcher ☤ — Hebbian buoy system: from concept to production in one session. Assessed, planned, gap-analyzed (conventional + NA), designed (12 AskUserQuestion decisions), implemented (schema v5, two-counter, context-aware, observable, property-tested, TTL-bounded, intensity-weighted, loop-damped). 1837 tests. The system now earns its maintenance.*
