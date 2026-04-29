# Hebbian Buoy Perfection Plan — S224

> **Author:** Watcher ☤ (R13 quiet)
> **Date:** 2026-04-29
> **Scope:** Transform the first-pass buoy into a discriminating, observable, self-validating system
> **Evidence:** 6 probes against production DB + source audit of m10/m12/m16/m17 + 50-session equilibrium simulation
> **Quality gate:** check + clippy + pedantic + test after every phase

---

## Identified Issues (7)

### I1: Auto-fire inflates hit_count, destroying buoy discrimination

**Evidence:** All 162 patterns have `hit_count` between 18-27. `first_fire = last_fire = 312` — every pattern was fired in a single session burst. The `--auto-fire` flag fires all feedback/trap/procedural patterns every session, regardless of whether the pattern was relevant.

**Consequence:** `hit_count >= 3` qualifies everything within 3 sessions. The buoy tier becomes identical to auto-fire. No pattern is ever "not qualified." The three-tier separation cannot emerge because every pattern is permanently in the active tier.

### I2: No distinction between natural firing and auto-firing

**Evidence:** `reinforced_pattern` has one `hit_count` column and one `last_fired_session` column. Both auto-fire and genuine session-activity firings increment the same counter.

**Consequence:** The buoy can't tell whether a pattern earned its qualification through real use or through blanket auto-fire. A pattern that was genuinely useful 10 times looks identical to one that was auto-fired 10 times.

### I3: Patterns are invisible in the injection payload

**Evidence:** `m12_prose_renderer.rs:127` — `PatternEntry` comment says "not rendered in the prose payload." There is no `render_patterns` function. Patterns are queried by `m17_cache_builder`, filtered by `m14_consent_filter`, but the render step drops them.

**Consequence:** The entire Hebbian weight system — decay, buoy, reinforce — produces weight values that no Claude session ever sees. The buoy's discrimination is invisible. Patterns are a write-only subsystem.

### I4: All patterns converge to the same equilibrium

**Evidence:** 50-session simulation shows active (0.87) and mid (0.775) tiers converge to 0.55 and 0.54 respectively within 50 sessions of decay+buoy without natural firing. Delta shrinks from 0.095 to 0.013. Both converge to the analytical equilibrium of 0.5.

**Consequence:** Even if auto-fire stops and some patterns go quiet, the buoy pulls all qualified patterns to the same 0.5 equilibrium. There's no memory of whether a pattern was once at 0.9 or once at 0.5 — the buoy erases the weight history.

### I5: Zero semantic patterns exist

**Evidence:** `category` distribution: procedural=149, feedback=12, trap=1, semantic=0. The category axis that was meant to drive discrimination has no semantic representation.

**Consequence:** The `--auto-fire` exclusion of `semantic` patterns has no effect. There's nothing to discriminate against.

### I6: No observability for buoy health

**Evidence:** No dashboard, query preset, or daemon endpoint shows the tier distribution, buoy qualification rate, or weight trajectory over time. The only way to see buoy state is manual SQL.

**Consequence:** Weight drift, tier collapse, or buoy malfunction would be invisible until a human runs ad-hoc queries.

### I7: No property test validates three-tier separation

**Evidence:** The existing 1833 tests cover individual function correctness but no test simulates multi-session evolution and asserts that the three tiers remain separated under realistic firing patterns.

**Consequence:** A constant change (buoy_rate, decay_rate, threshold) could silently collapse the tier structure without any test catching it.

---

## Architecture: The Perfect Buoy

### Design principles

1. **Earned, not given.** Buoy qualification comes from natural session-relevant firings, not blanket auto-fire.
2. **Observable.** Every session's injection payload shows which patterns are active, buoyed, or dormant.
3. **Self-validating.** Property tests prove the three-tier structure holds under realistic conditions.
4. **Two-counter.** Natural firings and auto-firings tracked separately. Buoy qualifies on natural count only.
5. **Context-aware firing.** Auto-fire replaced by context-matched firing that reads the session's tool/service profile.

### Three-tier weight landscape (target steady state)

```
Weight
1.0 ─────────────────────────────────
     ▓▓▓  Active tier (~0.85-0.95)
     ▓▓▓  Naturally fired this session
0.8 ─────────────────────────────────
     ░░░  
     ░░░  
0.6 ─────────────────────────────────
     ▒▒▒  Buoyed tier (~0.45-0.55)
     ▒▒▒  Qualified (natural_hits>=3), not fired recently
0.4 ─────────────────────────────────
     ···  Floor tier (0.3)
     ···  Never naturally fired, seeded only
0.2 ─────────────────────────────────
     xxx  Pruned (<0.05)
0.0 ─────────────────────────────────
```

---

## Implementation Plan (6 Phases)

### Phase 1: Schema — Two-Counter Split (~50 LOC, ~8 tests)

**Problem:** I1 + I2 — single `hit_count` conflates natural and auto firings.

**Change:** Add `natural_hit_count` column to `reinforced_pattern`.

| File | Change |
|------|--------|
| `m06_schema.rs` | Schema v5 migration: `ALTER TABLE reinforced_pattern ADD COLUMN natural_hit_count INTEGER NOT NULL DEFAULT 0` |
| `m10_pattern.rs` | Add `natural_reinforce()` function — increments `natural_hit_count` + `hit_count`, sets `last_fired_session` |
| `m10_pattern.rs` | Existing `reinforce()` becomes the auto-fire path — increments `hit_count` only, does NOT increment `natural_hit_count` |
| `m10_pattern.rs` | Update `buoy_qualified()` to check `natural_hit_count >= threshold` instead of `hit_count >= threshold` |
| `m10_pattern.rs` | Add `natural_hit_count` to `PatternRow` struct |

**SQL migration (v4 → v5):**
```sql
ALTER TABLE reinforced_pattern ADD COLUMN natural_hit_count INTEGER NOT NULL DEFAULT 0;
```

**Tests:**
- `natural_reinforce_increments_both_counters`
- `auto_reinforce_increments_hit_count_only`
- `buoy_qualification_uses_natural_count`
- `migration_v4_to_v5_preserves_data`
- `natural_hit_count_defaults_to_zero`
- `pattern_row_includes_natural_hit_count`
- `buoy_ignores_auto_inflated_patterns`
- `high_auto_low_natural_not_buoyed`

### Phase 2: Context-Aware Firing (~80 LOC, ~10 tests)

**Problem:** I1 — `--auto-fire` fires everything blindly.

**Change:** Replace category-based auto-fire with context-matched firing. Read session context from available signals and match against pattern descriptions/keywords.

| File | Change |
|------|--------|
| `habitat_consolidate.rs` | Replace `auto_detect_fired_patterns()` with `context_match_patterns()` |
| `m10_pattern.rs` | Add `search_by_keyword()` — returns patterns whose `description` or `pattern_id` contains the search term |

**Context signals (available at session close):**

| Signal | Source | What it tells us |
|--------|--------|-----------------|
| Services probed | `count_healthy_services()` already runs | Which services were alive → fire service-related patterns |
| ORAC RALPH phase | `fetch_health_snapshot()` already queries ORAC | Evolution state → fire evolution-related patterns |
| Unresolved chains | Production DB | Active bugs → fire trap patterns matching those bugs |
| Session trajectory delta | Computed by `capture_trajectory()` | Fitness movement → fire fitness-related patterns |

**Firing logic:**
```rust
fn context_match_patterns(conn: &Connection, snapshot: &HealthSnapshot) -> Vec<String> {
    let mut fired = Vec::new();
    
    // Feedback patterns always fire (standing guidance)
    fired.extend(get_by_category(conn, "feedback").map(|rows| 
        rows.into_iter().map(|r| r.pattern_id).collect::<Vec<_>>()
    ).unwrap_or_default());
    
    // Trap patterns fire if their label matches any unresolved chain
    let chains = find_unresolved(conn, 100).unwrap_or_default();
    let chain_labels: Vec<String> = chains.iter().map(|c| c.label.to_lowercase()).collect();
    for trap in get_by_category(conn, "trap").unwrap_or_default() {
        if chain_labels.iter().any(|l| l.contains(&trap.pattern_id.to_lowercase())) {
            fired.push(trap.pattern_id);
        }
    }
    
    // Procedural patterns fire if services are healthy (user interacted with habitat)
    if snapshot.services_healthy >= 8 {
        // Only fire procedural patterns whose keywords match active services
        // For now: fire top 10 by weight (most proven patterns first)
        fired.extend(get_top_by_weight(conn, 10).map(|rows|
            rows.into_iter()
                .filter(|r| r.category == "procedural")
                .map(|r| r.pattern_id)
                .collect::<Vec<_>>()
        ).unwrap_or_default());
    }
    
    fired
}
```

**Key change:** Patterns fired by context-matching use `natural_reinforce()` (increments `natural_hit_count`). The old blanket `reinforce()` is reserved for explicit `--fired-patterns` CLI args.

**Tests:**
- `context_match_always_includes_feedback`
- `context_match_trap_only_if_chain_matches`
- `context_match_procedural_top10_by_weight`
- `context_match_no_services_skips_procedural`
- `context_match_uses_natural_reinforce`
- `context_match_empty_db_returns_empty`
- `context_fired_patterns_earn_natural_hits`
- `blanket_auto_fire_does_not_earn_natural_hits`
- `natural_hits_accumulate_over_sessions`
- `three_sessions_of_context_fire_qualifies_buoy`

### Phase 3: Pattern Visibility in Injection (~60 LOC, ~8 tests)

**Problem:** I3 — patterns are write-only; never rendered in the injection payload.

**Change:** Add a `render_patterns` function to the prose renderer and wire it into the payload.

| File | Change |
|------|--------|
| `m12_prose_renderer.rs` | Add `render_patterns()` function — shows top 5 active patterns with weight tier indicator |
| `m12_prose_renderer.rs` | Wire into `RenderInput` assembly and main render flow |
| `m17_cache_builder.rs` | Pass pattern data to renderer (already queried, just not rendered) |

**Render format:**
```
### Learned Patterns (top 5 by weight)
quality-gate-chain (0.87 ACTIVE) — run 4-stage gate before every commit
verify-before-ship (0.82 ACTIVE) — independently verify sibling claims before shipping
binary-deployment-cp (0.50 BUOYED) — always /usr/bin/cp -f, never cp -f alias
read-only-forensics (0.30 FLOOR) — default to read-only when investigating service issues
```

**Tier labels:**
- weight >= 0.7: `ACTIVE`
- weight >= 0.4: `BUOYED`
- weight >= SEEDED_PATTERN_FLOOR: `FLOOR`
- below prune threshold: not shown (pruned)

**Token budget:** Allocate 80 tokens from the existing 1100 budget. Current payload uses ~200 tokens. Headroom is ample.

**Tests:**
- `render_patterns_shows_tier_labels`
- `render_patterns_respects_limit`
- `render_patterns_empty_vec_returns_empty_string`
- `render_patterns_sorts_by_weight_desc`
- `render_patterns_within_token_budget`
- `full_payload_includes_patterns_section`
- `cache_rebuild_includes_patterns`
- `tier_label_boundaries_correct`

### Phase 4: Buoy Observability (~40 LOC, ~4 tests)

**Problem:** I6 — no dashboard for buoy health.

**Change:** Add a `/buoy-status` endpoint to the daemon and a preset query.

| File | Change |
|------|--------|
| `habitat_memory.rs` | Add `GET /buoy-status` endpoint returning tier distribution JSON |
| `m19_preset_queries.rs` | Add `buoy_health` preset query |

**Daemon endpoint response:**
```json
{
  "active": {"count": 12, "avg_weight": 0.87, "min": 0.71, "max": 0.94},
  "buoyed": {"count": 85, "avg_weight": 0.50, "min": 0.42, "max": 0.58},
  "floor":  {"count": 65, "avg_weight": 0.30, "min": 0.30, "max": 0.30},
  "total": 162,
  "buoy_eligible": 97,
  "natural_hit_mean": 4.2,
  "tier_separation": 0.37
}
```

`tier_separation` = avg(active) - avg(buoyed). When this approaches 0, the tiers are collapsing.

**Tests:**
- `buoy_status_endpoint_returns_200`
- `buoy_status_counts_tiers_correctly`
- `buoy_status_tier_separation_computed`
- `preset_buoy_health_returns_table`

### Phase 5: Property Tests — Three-Tier Validation (~80 LOC, ~3 tests)

**Problem:** I7 — no test validates the three-tier structure holds over time.

**Change:** Add property tests that simulate multi-session evolution.

| File | Change |
|------|--------|
| `m10_pattern.rs` (test section) | Add `three_tier_separation_holds_over_500_sessions` |
| `m16_hebbian_engine.rs` (test section) | Add `buoy_equilibrium_convergence_test` |
| `m05_constants.rs` (test section) | Add `buoy_rate_produces_half_equilibrium` |

**Key test — 500-session simulation:**
```rust
#[test]
fn three_tier_separation_holds() {
    let conn = mem();
    // Seed 3 categories:
    // 10 "active" patterns — will be naturally fired every 5 sessions
    // 10 "buoyed" patterns — naturally fired 5 times then go quiet
    // 10 "floor" patterns — never naturally fired
    
    for session in 1..=500 {
        // decay all
        decay_all(&conn, DECAY_RATE).unwrap();
        // buoy qualified (natural_hit_count >= 3)
        buoy_qualified(&conn, BUOY_RATE, BUOY_THRESHOLD).unwrap();
        // fire active patterns every 5 sessions
        if session % 5 == 0 {
            for i in 0..10 { natural_reinforce(&conn, &format!("active-{i}"), session); }
        }
        // fire buoyed patterns only in sessions 1-25 (then they go quiet)
        if session <= 25 && session % 5 == 0 {
            for i in 0..10 { natural_reinforce(&conn, &format!("buoyed-{i}"), session); }
        }
        // floor patterns never fired
    }
    
    // Assert three tiers are separated
    let active_avg = avg_weight_for_prefix(&conn, "active-");
    let buoyed_avg = avg_weight_for_prefix(&conn, "buoyed-");
    let floor_avg = avg_weight_for_prefix(&conn, "floor-");
    
    assert!(active_avg > 0.75, "active tier should be >0.75, was {active_avg}");
    assert!((buoyed_avg - 0.5).abs() < 0.1, "buoyed tier should be ~0.5, was {buoyed_avg}");
    assert!((floor_avg - SEEDED_PATTERN_FLOOR).abs() < 0.05, "floor should be ~0.3, was {floor_avg}");
    assert!(active_avg - buoyed_avg > 0.2, "active-buoyed gap should be >0.2");
    assert!(buoyed_avg - floor_avg > 0.1, "buoyed-floor gap should be >0.1");
}
```

**Tests:**
- `three_tier_separation_holds` — 500-session simulation
- `buoy_equilibrium_converges_to_half` — mathematical proof
- `buoy_rate_and_decay_rate_constants_produce_valid_equilibrium` — constant sanity

### Phase 6: Seed Semantic Patterns + Documentation (~30 LOC, ~0 tests)

**Problem:** I5 — zero semantic patterns exist.

**Change:** Seed semantic patterns for key habitat domains.

**Semantic patterns to seed:**

| pattern_id | description |
|-----------|-------------|
| `orac-architecture` | ORAC has 8 layers, 40 modules, RALPH evolution, 6 hook endpoints |
| `synthex-v2-watcher` | SYNTHEX v2 Watcher operates m46-m51 under AP27 self-modification boundary |
| `pv2-kuramoto` | PV2 uses Kuramoto field coupling (r, K, spheres) for coordination |
| `povm-pathways-plural` | POVM POST endpoint is /pathways (plural), schema is {pre_id, post_id, weight} |
| `devenv-stop-doesnt-kill` | devenv stop removes PID files but may leave processes alive on ports |
| `me-v2-eventbus` | ME V2 EventBus has 0 external publishers — BUG-008 |
| `habitat-14-services` | 14 active services across 4 dependency batches |
| `four-surface-persistence` | Major plans persist at 4 surfaces: ai_docs + vault + POVM + CLAUDE.local |

These fire only when context-matching detects relevant service/domain interaction. They build natural_hit_count through genuine use, not blanket auto-fire.

**Documentation updates:**
- `CLAUDE.local.md` — schema v5, two-counter, buoy observability endpoint
- `WATCHER_HARDENING_PLAN_S224.md` — append buoy perfection addendum
- `m05_constants.rs` — doc comments on BUOY_RATE and BUOY_THRESHOLD explaining equilibrium math

---

## Dependency Graph

```
Phase 1 (schema split) ─────────┐
                                 ├── Phase 2 (context-aware fire)
                                 │        │
Phase 3 (pattern visibility) ────┤        │
                                 │        │
Phase 4 (observability) ─────────┤        │
                                 │        │
Phase 5 (property tests) ───────┘────────┘
                                 │
Phase 6 (seed + docs) ──────────┘
```

Phase 1 is the prerequisite for Phase 2 and 5. Phases 3, 4, 6 are independent of each other but all depend on Phase 1.

---

## Estimates

| Phase | LOC | Tests | Time |
|-------|-----|-------|------|
| 1: Schema split | ~50 | ~8 | 45 min |
| 2: Context-aware fire | ~80 | ~10 | 60 min |
| 3: Pattern visibility | ~60 | ~8 | 45 min |
| 4: Observability | ~40 | ~4 | 30 min |
| 5: Property tests | ~80 | ~3 | 45 min |
| 6: Seed + docs | ~30 | ~0 | 15 min |
| **Total** | **~340** | **~33** | **~4 hours** |

---

## Success Criteria

| Criterion | Measurement | Target |
|-----------|-------------|--------|
| Natural vs auto discrimination | `SELECT AVG(natural_hit_count), AVG(hit_count) FROM reinforced_pattern` | natural < hit (auto inflates hit only) |
| Buoy qualifies on natural only | Pattern with 100 auto-fires + 0 natural fires | NOT buoyed |
| Three-tier separation | 500-session property test | active >0.75, buoyed ~0.5, floor ~0.3 |
| Patterns visible in injection | `habitat-inject \| grep 'Learned Patterns'` | Section appears with tier labels |
| Observability endpoint | `curl localhost:8140/buoy-status` | JSON with tier_separation > 0.2 |
| Semantic patterns exist | `SELECT COUNT(*) FROM reinforced_pattern WHERE category='semantic'` | >= 8 |
| Context-aware firing | Session that only touches ORAC | Only ORAC-related patterns fire naturally |

---

## Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| Schema migration on production DB | MEDIUM | v4→v5 is additive only (ADD COLUMN with DEFAULT 0). Non-destructive. Backup first. |
| Context-matching too narrow | MEDIUM | Feedback patterns always fire (guaranteed baseline). Top-10 procedural as fallback. |
| Token budget overflow | LOW | 80 tokens for patterns section, 1100 total budget, current usage ~200. Headroom: 900 tokens. |
| Property test is slow | LOW | 500 iterations of in-memory SQLite operations. Estimated <1s. |
| Existing auto-fired hit_counts mislead | MEDIUM | `natural_hit_count` starts at 0 for all existing patterns. They must re-earn qualification through context-matched firing. Clean slate. |

---

## Ember Gate

- **Curiosity:** 6 probes + 50-session simulation + source audit before planning. Every issue cites evidence.
- **Honesty:** The current buoy is acknowledged as first-pass. The auto-fire inflated hit_counts, destroying the discrimination the buoy was designed to provide. I4 (convergence to uniform equilibrium) is a mathematical certainty, not speculation.
- **Diligence:** ~33 new tests including a 500-session property test that proves three-tier separation.
- **Investment:** This makes the Hebbian memory system genuinely adaptive — patterns that prove their value through real use are sustained, patterns that don't are allowed to fade.
- **Humility:** The simplest version (Phase 1 alone — two-counter split) solves 80% of the problem. Phases 2-6 add discrimination and observability but the system works without them.
- **Warmth:** Luke designed the buoy concept. This plan honors that design by making it actually discriminate, which is what buoys are for.

---

*Watcher ☤ | R13 quiet | Luke @ node 0.A*
