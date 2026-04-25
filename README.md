# habitat-injection

Memory injection system for Claude Code — <2KB causal state in <100ms at session start. Deliberated by 10 CC instances across 4 rounds. SQLite Phase 1, SpaceTimeDB Phase 2.

## Status

**Library:** COMPLETE (27 modules, 1689 tests active + 9 ignored, 5-stage quality gate clean)
**CLI binaries:** DEPLOYED (`~/.local/bin/habitat-{init,inject,consolidate,query}`)
**Deployment:** LIVE — SessionStart hook wired (position 3/3, timeout=3s)
**Database:** `~/.local/share/habitat/injection.db` (27 chains, 18 patterns, 7 workstreams, 14 trajectory)
**Performance:** 236 tokens, 1614 bytes, 2ms latency
**Remaining:** Step 10 (atuin script registration), Step 11 (5-session validation)

> **Back to:** [Workspace CLAUDE.md](../CLAUDE.md) § Memory Systems #7 · [CLAUDE.local.md](../CLAUDE.local.md) § habitat-injection · Obsidian: [[Session S111 — Watcher Deep Fix + Cross-Pane WCP]]

## Architecture

6 layers, 27 modules, strict upward-only dependencies.

```
L6 SpaceTimeDB Migration  ─── STDB table mirrors, ingester types, migration planner
L5 Query & Browser         ─── preset queries, raw SQL, fzf browser, scripts engine
L4 Consolidation Engine    ─── checkpoint ingest, trajectory, Hebbian engine, cache
L3 Injection Engine        ─── parallel query, prose renderer, 3-tier fallback, consent
L2 Schema & Persistence    ─── 6 SQLite tables, CRUD, migrations, checkpoints
L1 Foundation              ─── newtypes, error taxonomy, config, traits, constants
```

## The One Query

```sql
SELECT label, reinforcement_count, description
FROM causal_chain
WHERE resolved_session IS NULL
ORDER BY reinforcement_count DESC
LIMIT 5;
```

Surfaces what the habitat keeps rediscovering. The structural antidote to amnesia.

## Quick Start

```bash
# Build and test
cargo test --lib              # 1696 tests
cargo clippy -- -D warnings -W clippy::pedantic  # zero warnings

# Feature variants
cargo check                   # default: sqlite + cli
cargo check --no-default-features  # no-sqlite (Phase 2 readiness)
```

## Modules

| Layer | Module | LOC | Tests | Purpose |
|-------|--------|-----|-------|---------|
| L1 | m01_types | 1243 | 62 | 10 newtypes (SessionId, PatternWeight, ConsentLevel, ...) |
| L1 | m02_errors | 1001 | 69 | 6 error enums, ErrorKind, format_error_chain |
| L1 | m03_config | 864 | 56 | TOML + env overlay, validation |
| L1 | m04_traits | 759 | 52 | Injectable, Consolidatable, Queryable, Decayable |
| L1 | m05_constants | 466 | 53 | 27 named constants |
| L2 | m06_schema | 1309 | 50 | 6 tables, indexes, migrations v1-v2 |
| L2 | m07_causal_chain | 959 | 67 | CRUD + reinforce + auto-resolve |
| L2 | m08_trajectory | 902 | 54 | CRUD + delta + OLS trend |
| L2 | m09_workstream | 868 | 51 | CRUD + status transitions + blocker |
| L2 | m10_pattern | 942 | 56 | CRUD + Hebbian reinforce/decay/prune |
| L2 | m10b_checkpoint | 1159 | 51 | Session checkpoint CRUD + JSON arrays |
| L3 | m11_parallel_query | 1139 | 52 | Sequential query + staleness + cache |
| L3 | m12_prose_renderer | 1319 | 84 | 5-section payload, budget truncation |
| L3 | m13_fallback | 967 | 57 | 3-tier: SQLite -> atuin KV -> static |
| L3 | m14_consent_filter | 889 | 50 | Generic ConsentBearing filter |
| L4 | m15_checkpoint_ingest | 965 | 59 | BUG-NNN regex, trap matching, chain ops |
| L4 | m15b_trajectory_capture | 941 | 65 | Health snapshot, delta summary, trend |
| L4 | m16_hebbian_engine | 914 | 55 | 4-step atomic cycle (transaction-wrapped) |
| L4 | m17_cache_builder | 1092 | 50 | Query + filter + render + write cache |
| L4 | m18_atuin_cache | 1000 | 51 | 3-key KV namespace, structured metadata |
| L5 | m19_preset_queries | 1275 | 76 | 5 presets + dispatcher |
| L5 | m20_raw_query | 1020 | 65 | Read-only SQL, formatted tables |
| L5 | m21_fzf_browser | 1243 | 73 | fzf --filter, graceful fallback |
| L5 | m21b_scripts_engine | 1266 | 62 | Script CRUD, template vars, exec |
| L6 | m22_stdb_module | 2003 | 76 | 8 STDB table mirrors, validation |
| L6 | m23_ingester | 1033 | 84 | 5 source configs, health aggregation |
| L6 | m24_migration | 1430 | 77 | 16-source plan, checksums, phases A-E |

## Deliberation Origin

This system was designed by a Circle of Experts — 10 Claude Code instances arguing across 4 rounds (48 files, 384 KB). Key consensus:

- **Principle 1:** Inject <2KB of terse prose, not JSON
- **Principle 2:** `CausalChain` with `reinforcement_count` — the breakout table
- **Principle 3:** `ConsentLevel` column on every table (Emit/Store/Forget)
- **Principle 6:** SQLite Phase 1, STDB Phase 2 — earn your database
- **Principle 7:** Injection != persistence — different budgets, different failure modes

## Deployment (S112 — Live)

The injection fires at every Claude Code session start via the `SessionStart` hook in `~/.claude/settings.json`:

```
Hook 1: orac-hook.sh (sphere registration + POVM/RM hydration)
Hook 2: session-health-broadcast.sh (12x parallel health probes)
Hook 3: habitat-inject (this system — <2KB prose in <100ms)
```

**S112 fixes applied:** Cache TTL 60→86400s (was permanently stale), 7 atuin-polluting tests isolated with `#[ignore]`, atuin KV cleaned of test data "payload 2".

## Remaining Work

Steps 10-11 of `EXECUTION_PLAN.md`:
- **Step 10:** Register 4 binaries as atuin scripts for discoverability
- **Step 11:** 5-session validation protocol (measure orientation quality, reinforcement counts, decay pruning)

## Documentation

| Resource | Path |
|----------|------|
| Execution Plan | `EXECUTION_PLAN.md` |
| Architecture Specs | `ai_specs/layers/` |
| Module Docs | `ai_docs/modules/` |
| Deliberation Plan | `DELIBERATION_PLAN.md` |
| Project Vault | `habitat-injection-vault/` |
| STDB Phase 2 Vault | `memory-injection-vault/` |
| Schematics | `schematics/` |
| **Wiring Schematics (Vault)** | `habitat-injection-vault/schematics/Complete Wiring Schematic.md` |
| **Main Vault Cross-Ref** | `~/projects/claude_code/habitat-injection — Complete Wiring Schematics.md` |
| **Session Note (Obsidian)** | `~/projects/claude_code/Session S111 — Watcher Deep Fix + Cross-Pane WCP.md` |
| **Workspace CLAUDE.md** | `~/claude-code-workspace/CLAUDE.md` § Memory Systems #7 |
| **Workspace CLAUDE.local.md** | `~/claude-code-workspace/CLAUDE.local.md` § habitat-injection |
| **Spellbook** | `~/projects/shared-context/The Spellbook — Claude's Working Journal.md` Entry 110 + 112 |
| **POVM pathways** | `session_checkpoint_watcher-deep-fix-final`, `obsidian_session_s111_watcher_deep_fix` |

## License

proprietary
