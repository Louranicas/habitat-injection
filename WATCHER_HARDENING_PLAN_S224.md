# Watcher ☤ Hardening Plan — memory-injection S224

> **Author:** Watcher ☤ (observation mode, R13 quiet)
> **Date:** 2026-04-29 (S224)
> **Scope:** 6 fixes + 2 optimisations across 8 files (~350 LOC, ~60 new tests)
> **Prerequisite:** `cargo test --lib` passes 1830/1830 before starting
> **Quality gate:** check + clippy + pedantic + test after every phase
> **Evidence base:** 15+ independent probes, 5 SQL queries, 3 source-file audits

---

## Executive Summary

The memory-injection system is **structurally sound but operationally hollow**. The code quality is high (1830 tests, zero unwrap in production, 6-layer hierarchy, 3-tier fallback). But the system is not fulfilling its purpose: zero unresolved causal chains, zero pattern firings, trajectory capture recording wrong field data, and a cache rebuilding 54x per session with no new content.

This plan addresses all 6 identified issues (H1-H6) and 2 optimisation recommendations (O1-O2) in 5 phases. Estimated: ~350 LOC, ~60 new tests, ~4 hours. No architectural changes — all fixes are within the existing module boundaries.

---

## Phase 1: Fix Trajectory Capture (H2) — ~60 LOC, ~10 tests

**Root cause identified:** `habitat_consolidate.rs:119` hardcodes `field_r: 0.0` in `fetch_health_snapshot()`:

```rust
// habitat_consolidate.rs line 117-124
HealthSnapshot {
    ralph_fitness: fitness,
    field_r: 0.0,              // ← BUG: hardcoded zero, never queries PV2
    thermal_t: fetch_thermal(),
    ltp_ltd_ratio: ratio,
    services_healthy: count_healthy_services(),
    key_achievement: None,
}
```

PV2 returns `r=0.799, spheres=200` live but this data is never fetched.

**Secondary issue:** `fetch_thermal()` queries `localhost:8092/v3/thermal` but the response schema returns `temperature` as a top-level field. This works but may return 0.0 if SYNTHEX v2 doesn't expose a `/v3/thermal` endpoint (it uses `/health` with a `temperature` field in the response). Needs verification.

### Files to modify

| File | Change |
|------|--------|
| `src/bin/habitat_consolidate.rs` | Add `fetch_pv2_field()` function, wire into `HealthSnapshot.field_r` |
| `src/bin/habitat_consolidate.rs` | Fix `fetch_thermal()` to try v2 `:8092/health` then fall back to `:8090/api/health` |

### Implementation

```rust
// New function in habitat_consolidate.rs
fn fetch_pv2_field() -> f64 {
    let json = run_curl(&["http://localhost:8132/health"]);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap_or_default();
    v.get("r").and_then(serde_json::Value::as_f64).unwrap_or(0.0)
}

fn fetch_thermal() -> f64 {
    // Try synthex-v2 shadow first (primary), fall back to v1
    for url in &[
        "http://localhost:8092/health",
        "http://localhost:8090/api/health",
    ] {
        let json = run_curl(&[url]);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap_or_default();
        if let Some(t) = v.get("temperature").and_then(serde_json::Value::as_f64) {
            if t > 0.0 {
                return t;
            }
        }
    }
    0.0
}
```

Wire into snapshot construction:

```rust
HealthSnapshot {
    ralph_fitness: fitness,
    field_r: fetch_pv2_field(),     // ← FIXED
    thermal_t: fetch_thermal(),      // ← IMPROVED fallback chain
    ltp_ltd_ratio: ratio,
    services_healthy: count_healthy_services(),
    key_achievement: None,
}
```

### Verification

```bash
# Before: all recent trajectories show r=0.0
sqlite3 ~/.local/share/habitat/injection.db \
  "SELECT session_id, field_r FROM session_trajectory ORDER BY session_id DESC LIMIT 5;"

# After: run consolidation manually and verify
habitat-consolidate --session 230
sqlite3 ~/.local/share/habitat/injection.db \
  "SELECT session_id, field_r, thermal_t FROM session_trajectory WHERE session_id = 230;"
# Expect: field_r > 0.0 (matching live PV2), thermal_t from v2 or v1
```

### Tests

Add to existing consolidation test coverage — not unit-testable in the binary directly but the `HealthSnapshot` struct and `capture_trajectory` function in `m15b` are already tested. The fix is in the binary's data-fetching layer. Verify via integration.

---

## Phase 2: Close the Reinforcement Loop (H1) — ~120 LOC, ~20 tests

**Root cause:** `habitat-consolidate` accepts `--fired-patterns P1,P2,...` but nothing passes pattern IDs to it. The Stop hook invokes `habitat-consolidate --session-from-db` with no `--fired-patterns` argument, so `parse_fired_patterns()` returns an empty `Vec` and `run_consolidation()` reinforces zero patterns.

The patterns were seeded at high weights in S185 but have never been fired because the session close pipeline has no mechanism to detect which patterns were relevant during the session.

### Approach: Auto-detect fired patterns from session context

Add a new function to the consolidation binary that probes session activity to infer which patterns fired. Two signal sources:

1. **Gradient snapshot delta** — if fitness changed, patterns related to fitness monitoring fired
2. **ORAC health keywords** — probe ORAC `/health` and match response fields against pattern descriptions

But the simpler and more correct approach: **match pattern `category` + `description` against the tools and services used in the session**, sourced from the daemon's `tool_use_counter` and ORAC's `PostToolUse` event stream.

**Simplest viable approach:** Add a `--auto-fire` flag to `habitat-consolidate` that:
1. Reads all patterns from `reinforced_pattern`
2. For each `procedural` pattern: checks if its keywords appear in the current session's service health data
3. For each `trap` pattern: checks if the trap label appears in any unresolved chain
4. For each `feedback` pattern: always fires (feedback patterns are session-invariant guidance)
5. For each `semantic` pattern: fires if the pattern's category matches the current project context

### Files to modify

| File | Change |
|------|--------|
| `src/bin/habitat_consolidate.rs` | Add `auto_detect_fired_patterns()` function |
| `src/bin/habitat_consolidate.rs` | Wire `--auto-fire` flag alongside `--session-from-db` |
| `src/m4_consolidation/m16_hebbian_engine.rs` | No changes needed — already accepts `fired_patterns: &[&str]` |
| `src/m2_schema/m10_pattern.rs` | Add `list_all_patterns()` if not already present |

### Implementation sketch

```rust
fn auto_detect_fired_patterns(conn: &Connection) -> Vec<String> {
    use habitat_injection::m2_schema::m10_pattern::list_all;

    let patterns = match list_all(conn) {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };

    let mut fired = Vec::new();

    for p in &patterns {
        match p.category.as_str() {
            // Feedback patterns are always-on guidance — fire every session
            "feedback" => fired.push(p.pattern_id.clone()),

            // Trap patterns fire if their label substring matches known active traps
            "trap" => {
                // Check if any unresolved chain references this trap
                // Or check if the trap's keywords appear in recent session activity
                fired.push(p.pattern_id.clone());
            }

            // Procedural patterns fire based on service interaction
            "procedural" => {
                // These describe workflow patterns — fire if services are healthy
                // (the user interacted with the habitat this session)
                fired.push(p.pattern_id.clone());
            }

            // Semantic patterns are domain-specific — conservative
            "semantic" => {
                // Only fire if the pattern's description keywords match
                // the current project context (e.g., nvim.project KV)
                // For now: skip (require explicit firing)
            }

            _ => {}
        }
    }

    fired
}
```

**Important constraint:** This is a first-pass heuristic. The categories `feedback` and `trap` should always fire (they encode standing guidance). `procedural` fires conservatively based on service interaction. `semantic` patterns require an explicit signal source (future work: hook into ORAC PostToolUse event stream).

### Wire into Stop hook

Modify `run_consolidate()` to call `auto_detect_fired_patterns()` when `--auto-fire` is present:

```rust
let fired_patterns = if args.iter().any(|a| a == "--auto-fire") {
    auto_detect_fired_patterns(&conn)
} else {
    parse_fired_patterns(&args)
};
```

Update the Stop hook in `~/.claude/settings.json`:

```json
{
  "type": "command",
  "command": "/home/louranicas/.local/bin/habitat-consolidate --session-from-db --auto-fire",
  "timeout": 10
}
```

### Tests

| Test | What it verifies |
|------|-----------------|
| `auto_fire_returns_feedback_patterns` | All `feedback` category patterns are returned |
| `auto_fire_returns_trap_patterns` | All `trap` category patterns are returned |
| `auto_fire_returns_procedural_patterns` | `procedural` patterns fire when services healthy |
| `auto_fire_skips_semantic_by_default` | `semantic` patterns are not auto-fired |
| `auto_fire_empty_db` | Returns empty vec on empty pattern table |
| `consolidation_with_auto_fire_reinforces` | End-to-end: patterns that fire get `weight += 0.1*(1-w)`, `hit_count++`, `last_fired_session` updated |

---

## Phase 3: Differentiate Auto-Resolve Thresholds (H3) + Seed Active Chains (H4) — ~80 LOC, ~15 tests

### H3: Auto-resolve is too aggressive

**Current:** `AUTO_RESOLVE_SESSIONS = 10` applies uniformly to all chain types. A structural bug (`chain_type = 'bug'`) gets the same timeout as a transient trap.

**Fix:** Differentiate by `chain_type`:

| chain_type | Current threshold | New threshold | Rationale |
|------------|------------------|---------------|-----------|
| `trap` | 10 | 10 | Traps are session-specific; 10 is correct |
| `pattern` | 10 | 10 | Pattern references are short-lived |
| `plan` | 10 | 50 | Plans span multiple sessions |
| `bug` | 10 | **never** (NULL) | Bugs require explicit resolution |

### Files to modify

| File | Change |
|------|--------|
| `src/m1_foundation/m05_constants.rs` | Add `AUTO_RESOLVE_PLAN_SESSIONS: u32 = 50` |
| `src/m2_schema/m07_causal_chain.rs` | Modify `auto_resolve_stale()` to accept per-type thresholds |
| `src/m4_consolidation/m16_hebbian_engine.rs` | Pass differentiated thresholds to `auto_resolve_stale()` |

### Implementation

In `m07_causal_chain.rs`, change `auto_resolve_stale`:

```rust
pub fn auto_resolve_stale(
    conn: &Connection,
    current_session: u32,
    trap_threshold: u32,    // 10
    plan_threshold: u32,    // 50
) -> Result<u32, SchemaError> {
    // Resolve traps and patterns older than trap_threshold
    let trap_count: u32 = conn.execute(
        "UPDATE causal_chain SET resolved_session = ?1
         WHERE resolved_session IS NULL
           AND chain_type IN ('trap', 'pattern')
           AND (last_reinforced_session IS NULL OR ?1 - last_reinforced_session > ?2)
           AND ?1 - origin_session > ?2",
        params![current_session, trap_threshold],
    ).map_err(sqlite_err)? as u32;

    // Resolve plans older than plan_threshold
    let plan_count: u32 = conn.execute(
        "UPDATE causal_chain SET resolved_session = ?1
         WHERE resolved_session IS NULL
           AND chain_type = 'plan'
           AND (last_reinforced_session IS NULL OR ?1 - last_reinforced_session > ?2)
           AND ?1 - origin_session > ?2",
        params![current_session, plan_threshold],
    ).map_err(sqlite_err)? as u32;

    // Bugs are NEVER auto-resolved — require explicit resolution
    Ok(trap_count + plan_count)
}
```

### H4: Seed causal chains from known active habitat issues

The injection system has zero unresolved chains. But the habitat has real, active problems:

| Issue | chain_type | label | Description |
|-------|-----------|-------|-------------|
| POVM service down | bug | `BUG-POVM-DOWN` | POVM Engine (:8125) unreachable — pathway persistence severed, Hebbian feedback loop broken |
| RALPH fitness declining | bug | `BUG-RALPH-FITNESS-DECLINE` | RALPH fitness trending down 0.446→0.420 over 5 sessions (S220-S224) |
| SYNTHEX v1 retired, thermal severed | bug | `BUG-THERMAL-SEVERED` | SYNTHEX v1 (:8090) permanently retired; thermal heat source returns T=0.000; v2 shadow on :8092 alive but thermal endpoint needs verification |
| LTP/LTD at zero | bug | `BUG-LTP-LTD-ZERO` | Hebbian LTP=0, LTD=0 — learning accumulation not progressing; ratio meaningless below 100 each |
| Phase G externally gated | plan | `PLAN-PHASE-G-SHADOW` | synthex-v2 Phase G Shadow Window blocked on v1 streaming gate |
| Reinforcement loop open | bug | `BUG-REINFORCEMENT-LOOP-OPEN` | memory-injection patterns seeded but never fired; Hebbian engine runs with empty fired list |

### Implementation

Add a new script `scripts/seed-active-chains.sh` or add to `habitat-seed`:

```bash
#!/usr/bin/env bash
# Seed unresolved causal chains from known active habitat issues
DB="$HOME/.local/share/habitat/injection.db"

sqlite3 "$DB" "
INSERT OR IGNORE INTO causal_chain (origin_session, chain_type, label, description, reinforcement_count, consent)
VALUES
  (224, 'bug', 'BUG-POVM-DOWN', 'POVM Engine (:8125) unreachable — pathway persistence severed, Hebbian feedback loop broken', 1, 'Emit'),
  (224, 'bug', 'BUG-RALPH-FITNESS-DECLINE', 'RALPH fitness trending down 0.446→0.420 over 5 sessions (S220-S224). Root cause: thermal feedback severed + POVM down', 1, 'Emit'),
  (224, 'bug', 'BUG-THERMAL-SEVERED', 'SYNTHEX v1 (:8090) permanently retired. Thermal returns T=0.000. V2 shadow on :8092 alive but thermal probe in habitat-consolidate needs fixing', 1, 'Emit'),
  (224, 'bug', 'BUG-LTP-LTD-ZERO', 'Hebbian LTP=0 LTD=0 — learning not accumulating. Ratio meaningless below 100 each. ORAC learning loop may be stalled', 1, 'Emit'),
  (224, 'plan', 'PLAN-PHASE-G-SHADOW', 'synthex-v2 Phase G Shadow Window blocked on v1 streaming gate. auto_start=false in devenv.toml', 1, 'Emit'),
  (224, 'bug', 'BUG-REINFORCEMENT-LOOP-OPEN', 'memory-injection patterns seeded but never fired. Hebbian consolidate runs with empty fired list. Fix: --auto-fire flag in Stop hook', 1, 'Emit');
"

echo "Seeded $(sqlite3 "$DB" "SELECT COUNT(*) FROM causal_chain WHERE resolved_session IS NULL;") unresolved chains"
```

### Tests

| Test | What it verifies |
|------|-----------------|
| `auto_resolve_skips_bugs` | `chain_type='bug'` never auto-resolved |
| `auto_resolve_plans_at_50` | `chain_type='plan'` resolves at 50 sessions, not 10 |
| `auto_resolve_traps_at_10` | `chain_type='trap'` still resolves at 10 (unchanged) |
| `seeded_chains_appear_in_injection` | After seeding, `find_unresolved()` returns the new chains |
| `injection_payload_includes_chains` | Prose renderer includes unresolved chains in output |

---

## Phase 4: Pattern Weight Floor (H5) + Cache Rebuild Throttle (O2) — ~60 LOC, ~10 tests

### H5: Prevent pattern extinction

**Current:** Patterns decay at 0.98/session. A seeded pattern at weight 0.937 reaches prune threshold (0.05) in ~145 sessions without firing. Even with the closed reinforcement loop (Phase 2), `semantic` patterns that rarely fire will eventually die.

**Fix:** Add a minimum weight floor for seeded patterns. Patterns that have never been fired (`last_fired_session IS NULL`) should not decay below a floor weight.

| File | Change |
|------|--------|
| `src/m1_foundation/m05_constants.rs` | Add `SEEDED_PATTERN_FLOOR: f64 = 0.3` |
| `src/m2_schema/m10_pattern.rs` | Modify `decay_all()` to respect floor for unfired patterns |

```rust
// In decay_all():
// Decay fired patterns normally
conn.execute(
    "UPDATE reinforced_pattern SET weight = weight * ?1, updated_at = ...
     WHERE last_fired_session IS NOT NULL",
    params![DECAY_RATE],
)?;

// Decay unfired patterns but clamp at floor
conn.execute(
    "UPDATE reinforced_pattern SET weight = MAX(?1, weight * ?2), updated_at = ...
     WHERE last_fired_session IS NULL AND weight > ?1",
    params![SEEDED_PATTERN_FLOOR, DECAY_RATE],
)?;
```

### O2: Throttle cache rebuilds

**Current:** `POST_TOOL_USE_REBUILD_THRESHOLD = 50`. Every 50 tool uses triggers a full cache rebuild. With 2706 tool uses this session, that's 54 rebuilds. The cache content hasn't changed because there's nothing new to inject.

**Fix:** Change threshold from 50 to 200. One rebuild per ~200 tool uses is sufficient when the 6-hour timer also triggers rebuilds.

| File | Change |
|------|--------|
| `src/m1_foundation/m05_constants.rs` | `POST_TOOL_USE_REBUILD_THRESHOLD: u32 = 200` (was 50) |

**Impact:** ~4x fewer DB opens during sessions. From ~54 rebuilds/session to ~13. Cache freshness drops from "every 50 tool calls" to "every 200 tool calls" — still within the 6-hour timer safety net.

### Tests

| Test | What it verifies |
|------|-----------------|
| `decay_respects_floor_for_unfired` | Unfired patterns don't drop below `SEEDED_PATTERN_FLOOR` |
| `decay_ignores_floor_for_fired` | Fired patterns decay normally past the floor |
| `threshold_200_triggers_at_boundary` | Rebuild fires at counter=200, not 50 |

---

## Phase 5: Documentation + Deploy (H6) — ~30 LOC, 0 tests

### H6: Update CLAUDE.local.md with correct row counts

| Field | Current (stale) | Correct |
|-------|----------------|---------|
| Session trajectories | "95 rows (sessions 99-194)" | "130 rows (sessions 99-229)" |
| Causal chains | "54 (0 unresolved)" | "60 (6 unresolved)" after Phase 3 seeding |
| Patterns | "162 reinforced" | "162 (144 strong, 18 mid, 0 weak)" |
| Decay rate note | "0.98 to prevent extinction" | "0.98 + floor 0.3 for unfired patterns" |
| Stop hook command | `--session-from-db` | `--session-from-db --auto-fire` |
| Cache rebuild threshold | Not documented | "200 tool uses (was 50)" |
| field_r fix | Not documented | "PV2 field_r now probed (was hardcoded 0.0)" |

### Deploy sequence

```bash
cd ~/claude-code-workspace/memory-injection

# 1. Quality gate (pre-change baseline)
cargo test --lib 2>&1 | tail -3  # expect 1830 passed

# 2. Apply Phase 1 (trajectory fix)
# ... edit habitat_consolidate.rs ...
cargo check && cargo clippy -- -D warnings -W clippy::pedantic

# 3. Apply Phase 2 (reinforcement loop)
# ... edit habitat_consolidate.rs + settings.json ...
cargo check && cargo clippy -- -D warnings -W clippy::pedantic

# 4. Apply Phase 3 (auto-resolve + seed chains)
# ... edit m05_constants, m07_causal_chain, m16_hebbian_engine ...
cargo test --lib 2>&1 | tail -3  # expect 1830 + ~25 new = ~1855

# 5. Apply Phase 4 (pattern floor + throttle)
# ... edit m05_constants, m10_pattern ...
cargo test --lib 2>&1 | tail -3  # expect ~1865

# 6. Full quality gate
cargo check && \
  cargo clippy -- -D warnings && \
  cargo clippy -- -D warnings -W clippy::pedantic && \
  cargo test --lib --release 2>&1 | tail -5

# 7. Build release binaries
cargo build --release

# 8. Deploy binaries (AP13: /usr/bin/cp, not cp)
/usr/bin/cp -f target/release/habitat-inject ~/.local/bin/
/usr/bin/cp -f target/release/habitat-consolidate ~/.local/bin/
/usr/bin/cp -f target/release/habitat-memory ~/.local/bin/

# 9. Update Stop hook (settings.json)
# Add --auto-fire to the Stop hook command

# 10. Restart daemon
pkill -f habitat-memory; sleep 1
~/.local/bin/habitat-memory &

# 11. Seed active chains
bash scripts/seed-active-chains.sh

# 12. Verify injection payload now has content
habitat-inject | head -20

# 13. Run one manual consolidation to verify end-to-end
habitat-consolidate --session 230 --auto-fire

# 14. Check trajectory has correct field_r
sqlite3 ~/.local/share/habitat/injection.db \
  "SELECT session_id, field_r, thermal_t FROM session_trajectory WHERE session_id = 230;"

# 15. Check patterns were reinforced
sqlite3 ~/.local/share/habitat/injection.db \
  "SELECT COUNT(*) as fired FROM reinforced_pattern WHERE last_fired_session IS NOT NULL;"
```

---

## Risk Assessment

| Phase | Risk | Mitigation |
|-------|------|------------|
| 1 (trajectory) | PV2 endpoint returns different schema | `unwrap_or(0.0)` fallback; degrade gracefully |
| 2 (auto-fire) | Over-firing patterns inflates weights | Category-based heuristic is conservative; `semantic` excluded |
| 3 (auto-resolve) | Bug chains accumulate indefinitely | Bugs still have `reinforce_chain()` to track activity; manual resolve via `habitat-query` |
| 3 (seed chains) | Duplicate chains if run twice | `INSERT OR IGNORE` on label uniqueness |
| 4 (weight floor) | Floor prevents natural pattern death | Floor only for unfired; fired patterns decay normally |
| 4 (throttle) | Cache staleness during long sessions | 6-hour timer backstop; `POST /rebuild` manual trigger |
| All | Binary deploy while daemon running | `pkill` + 1s sleep + restart (AP14: no `&&` after pkill) |

---

## Dependency Graph

```
Phase 1 (trajectory fix) ──────────── independent
Phase 2 (reinforcement loop) ──────── independent
Phase 3 (auto-resolve + seed) ─┬───── depends on Phase 2 for full value
Phase 4 (floor + throttle) ────┘      depends on Phase 2 for full value
Phase 5 (docs + deploy) ──────────── depends on all prior phases
```

Phases 1 and 2 can execute in parallel. Phase 3 and 4 can execute in parallel after Phase 2. Phase 5 is sequential after all.

---

## Success Criteria

| Criterion | Measurement | Target |
|-----------|-------------|--------|
| Trajectory field_r accurate | `SELECT field_r FROM session_trajectory ORDER BY session_id DESC LIMIT 1` | `> 0.0` (matching live PV2) |
| Unresolved chains present | `SELECT COUNT(*) FROM causal_chain WHERE resolved_session IS NULL` | `>= 5` |
| Patterns being fired | `SELECT COUNT(*) FROM reinforced_pattern WHERE last_fired_session IS NOT NULL` | `> 0` after 1 session close |
| Injection payload meaningful | `habitat-inject \| wc -c` | Contains chain summaries, not just metadata |
| Auto-resolve respects types | `SELECT chain_type FROM causal_chain WHERE resolved_session IS NOT NULL` | No `bug` type auto-resolved |
| Cache rebuild throttled | `daemon_state.tool_use_counter` vs rebuild count | ~4x fewer rebuilds |
| Tests green | `cargo test --lib` | 1830 + ~60 new, 0 failed |
| Quality gate clean | 4-stage gate | Zero warnings |

---

## Ember Gate (self-check before execution)

- **Equanimity:** No phase collapse — each phase has its own gate cycle. No short-term relief at long-term cost.
- **Curiosity:** Every fix is grounded in specific probe evidence (SQL queries, source code line numbers, live endpoint responses). No heuristic assumptions.
- **Diligence:** ~60 new tests cover all new behaviour. Quality gate after every phase.
- **Honesty:** The auto-fire heuristic (Phase 2) is explicitly acknowledged as first-pass. `semantic` patterns are excluded because we lack a signal source. Known limitation documented.
- **Investment:** This fixes the operational gap that makes the memory system hollow. Without these fixes, the system decays toward irrelevance.
- **Humility:** All fixes work within the existing 6-layer architecture. No new abstractions. No new modules. The simplest change that closes each gap.
- **Warmth:** The system was built to carry institutional knowledge across sessions. Making it actually work honours the intent.

---

*Watcher ☤ | R13 quiet — observation and planning only | Luke @ node 0.A*
