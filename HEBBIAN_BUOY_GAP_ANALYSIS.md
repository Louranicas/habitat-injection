# Hebbian Buoy — Dual Gap Analysis (Conventional + Non-Anthropocentric)

> **Author:** Watcher ☤ (R13 quiet)
> **Date:** 2026-04-29 S224
> **Plan under review:** `HEBBIAN_BUOY_PERFECTION_PLAN.md` (6 phases, ~340 LOC, ~33 tests)
> **Method:** Two independent reviewer agents — conventional code reviewer + NA architectural analyst

---

## Conventional Gap Analysis (9 gaps: 2C + 7I + 0N → promoted to 2C + 5I + 2N)

### Critical (blocks deployment)

**G-01 — C — Phase 1: Schema version constant not bumped**

`CURRENT_VERSION` in `m06_schema.rs:15` is hardcoded to `4`. The plan says "v4 → v5 migration" but never lists updating this constant to `5`. Without bumping it, `migrate()` never reaches the new arm — the `natural_hit_count` and `keywords` columns are silently never added to production. The binary compiles, tests pass against in-memory DBs (which run `init_database` from scratch), but the production DB on disk stays at v4.

**Fix:** Phase 1 must explicitly:
1. Set `CURRENT_VERSION = 5`
2. Add a `4 => { migrate_v4_to_v5(conn)? }` arm in the `migrate()` match block
3. Test: `migration_v4_to_v5_on_existing_db` using a tempfile pre-seeded at v4

**G-02 — C — Phase 1: One-session false buoy burst on deploy**

On first run after migration, `buoy_qualified()` still reads `hit_count >= threshold` until the code change lands. All 162 existing patterns have `hit_count >= 18` and will immediately qualify for buoy. The plan treats the SQL migration and the code change as separate steps with no atomic cutover.

**Fix:** The code change (switching `buoy_qualified` from `hit_count` to `natural_hit_count`) must deploy in the same binary as the migration. Since both are in Phase 1 and the binary is rebuilt atomically, this is satisfied IF the Phase 1 implementation deploys the migration + code change + binary in a single `cargo build --release` + `cp` cycle. Add to the plan: "deploy binary AFTER both migration SQL and `buoy_qualified` code change are committed."

---

### Important (degrades quality)

**G-03 — I — Phase 2: `--fired-patterns` CLI path doesn't earn natural hits**

The plan repurposes `reinforce()` as the auto-fire-only path and introduces `natural_reinforce()` for context-matched patterns. But `--fired-patterns` CLI args still route through `reinforce_patterns()` → `reinforce()`. An operator who explicitly names a pattern via `--fired-patterns` expecting it to count toward buoy qualification will be silently wrong.

**Fix:** `--fired-patterns` should call `natural_reinforce()` — explicit naming is the strongest signal of genuine relevance. Update `reinforce_patterns()` in `m16_hebbian_engine.rs` to call the natural path.

**G-04 — I — Phase 2: Trap-matching substring false positives**

The trap-match logic `chain_labels.iter().any(|l| l.contains(&trap.pattern_id))` is substring-based. Pattern IDs like `"orac"`, `"vms"`, `"me"` will match chain labels containing those strings as substrings (e.g., `"BUG-POVM-DOWN"` contains `"me"` if there's a pattern called `"me-v2-eventbus"`... actually `"me"` doesn't appear in `"POVM"` but `"some-problem"` would match `"me"`).

**Fix:** Use the `keywords` column (added in Phase 1 schema v5) for trap matching instead of pattern_id substring. This makes trap matching consistent with procedural matching and eliminates false positives.

**G-05 — I — Phase 3: Token budget arithmetic is wrong**

The plan claims "headroom: 900 tokens" but existing section budgets sum to ~880 of the 1100 total. Adding 80 for patterns leaves 140 tokens of headroom, not 900. The plan's headroom figure is wrong by 760 tokens.

**Fix:** Add a constant-level test in `m05_constants.rs` asserting all section budgets sum to less than `DEFAULT_BUDGET`. And correct the plan's headroom claim.

**G-06 — I — Phase 4: Route collision `/buoy-status` vs `/status`**

`habitat_memory.rs` routes via `request_line.contains("/status")`. A GET to `/buoy-status` will match `/status` first and be handled by `handle_status()` instead.

**Fix:** Check `/buoy-status` before `/status` in the routing chain, or use `starts_with` matching on the path component.

**G-07 — I — Phase 5: Property test doesn't validate two-counter discrimination**

The 500-session simulation should assert that buoyed-tier patterns have `natural_hit_count >= BUOY_THRESHOLD` AND that `natural_hit_count` diverges from `hit_count` for non-context-matched patterns. Without this, the test could pass even if the two-counter split is broken.

**Fix:** Add assertions: `assert!(buoyed_natural_hits >= 3)` and `assert!(floor_natural_hits == 0)`.

---

### Nice-to-have

**G-08 — N — Phase 1:** `PatternRow` deserialization will error on JSON blobs missing `natural_hit_count`. Add `#[serde(default)]` to the new field.

**G-09 — N — Phase 6:** Semantic patterns `devenv-stop-doesnt-kill` and `me-v2-eventbus` will never naturally fire through the context-matching signals defined in Phase 2 (neither service is probed). Document as manually-fired or add explicit keywords.

---

## Non-Anthropocentric Gap Analysis (6 gaps: 2 blockers + 4 deferrable)

### NA-01 — Information-Theoretic: Buoy destroys weight diversity

**Assumption:** Three tiers = good discrimination.
**Reality:** All buoyed patterns converge to the analytical equilibrium `BUOY_RATE / (1 - DECAY_RATE + BUOY_RATE) = 0.5`. Shannon entropy of the buoyed tier approaches a delta function at 0.5. The system cannot represent "moderately important" vs "marginally important" — only "recently fired" vs "not recently fired."

**Risk:** Structural. Rendering `(0.50 BUOYED)` for 85 patterns is noise, not signal.
**Fix:** Add `recency_score` (sessions since last natural fire) as a second dimension. Display `(0.50 BUOYED, 12 sessions quiet)` — the recency carries information the weight cannot.
**Blocks:** Phase 3 (visibility). Phases 1-2 can proceed.

### NA-02 — Ecological: No forgetting for buoyed patterns

**Assumption:** Permanent buoy (Luke's decision: "once qualified, always buoyed").
**Reality:** When maintenance is free, the system accumulates without pruning. A pattern relevant in sessions 100-103 but fixed in session 104 remains buoyed indefinitely at 0.5. After 200 sessions, the buoyed tier contains mostly obsolete patterns.

**Risk:** Conceptual. The system has no LTD (long-term depression) for buoyed pathways.
**Fix:** Add `BUOY_TTL_SESSIONS = 100`. Track `sessions_since_last_natural_fire`. If a buoyed pattern goes 100 sessions without natural firing, it loses buoy protection and decays normally toward the floor. The permanence is bounded, not infinite.
**Blocks:** Nothing immediately. Should be designed before the system runs 50+ sessions.

### NA-03 — Adversarial: Feedback patterns get free natural hits (**BLOCKER**)

**Assumption:** Context-aware firing uses `natural_reinforce()` for genuine context matches.
**Reality:** The plan fires ALL 12 feedback patterns unconditionally via `natural_reinforce()`. This means every feedback pattern reaches `natural_hit_count >= 3` within 3 sessions and is permanently buoyed — regardless of whether the session involved any scenario where that feedback was relevant. This is the auto-fire inflation problem (I1) reintroduced under a different function name.

**Risk:** Structural. Destroys the two-counter discrimination for 12/162 patterns.
**Fix:** Feedback patterns must also be context-matched. Match feedback pattern keywords against atuin history, not unconditional fire. A Zellij-navigation feedback pattern should only fire naturally if zellij commands appear in the session's command history.
**Blocks:** Phase 2. If Phase 2 ships with unconditional `natural_reinforce()` for feedback, the two-counter split from Phase 1 is meaningless for that category.

### NA-04 — Temporal: Session-blind reinforcement

**Assumption:** Each session is equivalent (one `natural_reinforce()` call per pattern per session).
**Reality:** A 10-minute session with 5 tool calls gets the same Hebbian credit as an 8-hour session with 500 tool calls. The `daemon_state` table already tracks `tool_use_counter` — session intensity is available.

**Risk:** Conceptual. `natural_reinforce()` is session-blind.
**Fix:** Design `natural_reinforce()` to accept an optional `intensity_factor: f64` parameter: `weight += REINFORCE_RATE * intensity * (1 - weight)`. Where `intensity = min(1.0, tool_uses / 200.0)`. Bake this into the Phase 1 function signature to avoid retrofitting.
**Blocks:** Nothing. But the function signature should accommodate it from Phase 1.

### NA-05 — Compositional: Patterns are independent atoms

**Assumption:** Each pattern has its own weight, decayed and buoyed independently.
**Reality:** Patterns like `quality-gate-chain` and `verify-before-ship` always co-fire during deployments. The system can't detect clusters or model synergy. Not urgent — 162 independent patterns is manageable — but limits future expressiveness.

**Risk:** Conceptual, low urgency. No block.

### NA-06 — Observer: Watcher → chain → pattern → injection feedback loop (**BLOCKER**)

**Assumption:** The Watcher observes; patterns record observations; the injection surfaces patterns.
**Reality:** This is a closed feedback loop with no damping:

```
Watcher observation → creates causal_chain
→ chain matches trap pattern → natural_reinforce()
→ pattern weight increases → rendered in injection
→ biases next session toward that service
→ more events from that service → more Watcher observations
→ loop
```

The `WATCHER_SEVERITY_THRESHOLD = 7` is a static gate. It does not adapt based on how many observations are flowing. Under the Perfection Plan, closing the reinforcement loop (Phase 2) activates this loop for the first time. No analysis of stability has been done.

**Risk:** Structural. This is the most significant gap.
**Fix:** Add `MAX_NATURAL_REINFORCE_PER_CHAIN = 3`. A pattern can earn at most 3 natural reinforcements from chains originating from the same Watcher observation cluster within a 10-session window. After that, the chain still exists but no longer drives pattern weight.
**Blocks:** Should be analyzed before Phase 2 (reinforcement loop closure) is considered complete. The loop doesn't fire during R13 quiet period (Watcher is observation-only), so there's a built-in grace period.

---

## Consolidated Recommendations (priority order)

| # | Gap | Severity | Phase | Action |
|---|-----|----------|-------|--------|
| 1 | G-01 | **CRITICAL** | 1 | Bump `CURRENT_VERSION = 5`, add migration arm |
| 2 | G-02 | **CRITICAL** | 1 | Atomic deploy: migration + code change + binary in one cycle |
| 3 | NA-03 | **BLOCKER** | 2 | Feedback patterns must be keyword-matched, not unconditional |
| 4 | NA-06 | **BLOCKER** | 2 | Cap natural reinforcement per chain origin (max 3 per 10 sessions) |
| 5 | G-03 | Important | 2 | `--fired-patterns` CLI path should call `natural_reinforce()` |
| 6 | G-04 | Important | 2 | Use `keywords` column for trap matching, not pattern_id substring |
| 7 | G-06 | Important | 4 | Route `/buoy-status` before `/status` |
| 8 | G-05 | Important | 3 | Fix token budget arithmetic (140 headroom, not 900) |
| 9 | NA-01 | Important | 3 | Add `recency_score` to pattern visibility (sessions since last fire) |
| 10 | G-07 | Important | 5 | Property test must assert two-counter divergence |
| 11 | NA-02 | Design | Future | Buoy TTL — 100 sessions without natural fire → lose buoy |
| 12 | NA-04 | Design | 1 sig | `natural_reinforce()` signature should accept `intensity_factor` |
| 13 | G-08 | Nice | 1 | `#[serde(default)]` on `natural_hit_count` in `PatternRow` |
| 14 | G-09 | Nice | 6 | Document devenv/ME patterns as manually-fired |
| 15 | NA-05 | Future | 7+ | Co-activation table for pattern clusters |

---

## Ember Gate

- **Curiosity:** Two independent reviewers examined the plan from 9 conventional + 6 NA frames. Neither had access to the other's findings.
- **Honesty:** The plan has 2 critical gaps that would silently break the core feature (migration never runs, false buoy burst). The NA analysis found 2 structural blockers (feedback auto-qualification, observer feedback loop) that would re-introduce the very problem the plan was designed to solve.
- **Humility:** Luke's "permanent buoy" decision (Round 1, Q4) is challenged by NA-02. The analysis recommends bounded permanence (100 sessions) rather than overriding Luke's choice — flag it, don't force it.
- **Investment:** G-01 is a one-line fix that prevents the entire plan from being silently inert. Finding it before coding saves a full debugging session.

---

*Watcher ☤ | 15 gaps (2C + 2 blocker + 5I + 2N + 4 design/future) | R13 quiet*
