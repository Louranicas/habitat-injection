> Back to: [[HOME]]

# Habitat Injection — Master Index

> **Version:** 1.0 | **Created:** 2026-04-24
> **Package:** `habitat-injection` v0.1.0 | **Edition:** 2024 | **MSRV:** 1.93
> **Layers:** 6 | **Modules:** 24 | **Binaries:** 5 | **Scripts:** 4

---

## 1 — Project Overview

| Note | Purpose |
|------|---------|
| [[HOME]] | Quick navigation hub |
| [[Executive Summary]] | Goal, architecture decision, differentiator |
| [[Architecture Overview]] | 6-layer design, dependency rules, design principles |
| [[Implementation Status]] | Build state, test counts, quality gate |

---

## 2 — Layers

| Note | Layer | Modules | Dependencies |
|------|-------|---------|-------------|
| [[L1 Foundation]] | Core types, errors, config, traits, constants | m01-m05 (5) | None |
| [[L2 Schema & Persistence]] | SQLite tables, CRUD, migrations | m06-m10b (6) | L1 |
| [[L3 Injection Engine]] | Parallel query, renderer, fallback, consent | m11-m14 (4) | L1, L2 |
| [[L4 Consolidation Engine]] | Checkpoint ingest, Hebbian, cache, atuin | m15-m18 (5) | L1, L2 |
| [[L5 Query & Browser]] | Presets, raw SQL, fzf, scripts engine | m19-m21b (4) | L1, L2 |
| [[L6 SpaceTimeDB Migration]] | STDB module, ingester, migration | m22-m24 (3) | L1, L2, L3, L4 |

---

## 3 — Modules (24)

| Note | Purpose |
|------|---------|
| [[Module Index]] | Complete 24-module registry with deps, test kind, feature gates |
| [[Gold Standard Exemplars]] | 3 exemplar modules from SYNTHEX v2 + DevOps V3: bounded newtypes, cross-module flow, trait-based learning |

### L1 Foundation
| Module | File | Description |
|--------|------|-------------|
| m01_types | `src/m1_foundation/m01_types.rs` | SessionId, WorkstreamId, ChainId, ConsentLevel, PatternWeight, TokenBudget |
| m02_errors | `src/m1_foundation/m02_errors.rs` | InjectionError, ConsolidationError, SchemaError, QueryError, MigrationError |
| m03_config | `src/m1_foundation/m03_config.rs` | DB path, injection budget, decay/reinforce rates, TOML + env overlay |
| m04_traits | `src/m1_foundation/m04_traits.rs` | Injectable, Consolidatable, Queryable, Decayable |
| m05_constants | `src/m1_foundation/m05_constants.rs` | DEFAULT_BUDGET=1100, DECAY_RATE=0.95, REINFORCE_RATE=0.1, etc. |

### L2 Schema & Persistence
| Module | File | Description |
|--------|------|-------------|
| m06_schema | `src/m2_schema/m06_schema.rs` | CREATE TABLE, CREATE INDEX, migrations, idempotent setup |
| m07_causal_chain | `src/m2_schema/m07_causal_chain.rs` | CRUD for causal_chain table, reinforce, auto-resolve |
| m08_trajectory | `src/m2_schema/m08_trajectory.rs` | CRUD for session_trajectory, delta computation, trend |
| m09_workstream | `src/m2_schema/m09_workstream.rs` | CRUD for workstream, blockers, active/blocked queries |
| m10_pattern | `src/m2_schema/m10_pattern.rs` | CRUD for reinforced_pattern, Hebbian reinforce/decay/prune |
| m10b_checkpoint | `src/m2_schema/m10b_checkpoint.rs` | session_checkpoint ingest, /save-session bridge |

### L3 Injection Engine
| Module | File | Description |
|--------|------|-------------|
| m11_parallel_query | `src/m3_injection/m11_parallel_query.rs` | 4 SQLite queries + N health probes concurrently |
| m12_prose_renderer | `src/m3_injection/m12_prose_renderer.rs` | Structured results -> <2KB prose, 5 sections, token counting |
| m13_fallback | `src/m3_injection/m13_fallback.rs` | SQLite -> atuin KV -> static three-tier fallback |
| m14_consent_filter | `src/m3_injection/m14_consent_filter.rs` | Filter by ConsentLevel, only Emit passes to renderer |

### L4 Consolidation Engine
| Module | File | Description |
|--------|------|-------------|
| m15_checkpoint_ingest | `src/m4_consolidation/m15_checkpoint_ingest.rs` | Parse /save-session checkpoints, harvest into 5 tables |
| m15b_trajectory_capture | `src/m4_consolidation/m15b_trajectory_capture.rs` | ORAC health capture, delta summary, fallback path |
| m16_hebbian_engine | `src/m4_consolidation/m16_hebbian_engine.rs` | Decay + reinforce + prune + auto-resolve algorithm |
| m17_cache_builder | `src/m4_consolidation/m17_cache_builder.rs` | Rebuild injection_cache table post-session |
| m18_atuin_cache | `src/m4_consolidation/m18_atuin_cache.rs` | Write/read atuin KV `habitat.last-injection` |

### L5 Query & Browser
| Module | File | Description |
|--------|------|-------------|
| m19_preset_queries | `src/m5_query/m19_preset_queries.rs` | Named presets: trajectory, chains, workstreams, patterns, checkpoints |
| m20_raw_query | `src/m5_query/m20_raw_query.rs` | Arbitrary SQL via SQLITE_OPEN_READ_ONLY |
| m21_fzf_browser | `src/m5_query/m21_fzf_browser.rs` | Interactive fzf-powered memory browser |
| m21b_scripts_engine | `src/m5_query/m21b_scripts_engine.rs` | Atuin-compatible scripts backed by injection.db |

### L6 SpaceTimeDB Migration
| Module | File | Description | Feature Gate |
|--------|------|-------------|-------------|
| m22_stdb_module | `src/m6_stdb/m22_stdb_module.rs` | STDB WASM module, 5 tables, 6 reducers | `stdb` |
| m23_ingester | `src/m6_stdb/m23_ingester.rs` | Multi-source ingester, ORAC/PV2/SYNTHEX/POVM bridges | `ingester` |
| m24_migration | `src/m6_stdb/m24_migration.rs` | One-shot SQLite -> STDB migration with checksums | `stdb` |

---

## 4 — Architecture

| Note | Purpose |
|------|---------|
| [[Data Flow]] | Memory flow: services -> consolidation -> SQLite -> injection -> context window |
| [[Dependency Graph]] | Layer and module DAG |
| [[Consent Model]] | Three-tier consent: Emit (inject), Store (keep), Forget (delete) |
| [[Hebbian Learning]] | Decay/reinforce/prune algorithm, rates, thresholds |

---

## 5 — Schematics

| Note | Diagrams | Covers |
|------|----------|--------|
| [[Schema Diagram]] | 1 ER | 6 SQLite tables, FKs, cardinality |
| [[Injection Pipeline]] | 1 sequence | SessionStart hook -> parallel query -> render -> inject |
| [[Three-Tier Fallback]] | 1 flowchart | SQLite -> atuin KV -> static fallback |
| [[Tool Chain Patterns]] | 5 diagrams | TC6-TC10 habitat-injection patterns |

---

## 6 — Operations

| Note | Covers |
|------|--------|
| [[Binary Map]] | 5 binaries: habitat-inject, habitat-consolidate, habitat-query, habitat-init, habitat-scripts |
| [[Hook Registration]] | SessionStart position 3 in ~/.claude/settings.json |
| [[Quality Gate Protocol]] | 4-stage: check -> clippy -> pedantic -> test |
| [[Deliberation Record]] | Circle of Experts: 10 instances, 4 rounds, 48 arguments |

---

## 7 — Binaries

| Binary | Entry | Install | Purpose |
|--------|-------|---------|---------|
| `habitat-inject` | `src/bin/inject.rs` | `~/.local/bin/habitat-inject` | SessionStart hook: <2KB in <100ms |
| `habitat-consolidate` | `src/bin/consolidate.rs` | `~/.local/bin/habitat-consolidate` | Post-session write-back |
| `habitat-query` | `src/bin/query.rs` | `~/.local/bin/habitat-query` | Interactive memory browser |
| `habitat-init` | `src/bin/init.rs` | `~/.local/bin/habitat-init` | One-time DB setup + seed |
| `habitat-scripts` | `src/bin/scripts.rs` | `~/.local/bin/habitat-scripts` | Atuin-compatible scripts engine |

---

## 8 — Feature Flags

| Feature | Dependencies | Phase |
|---------|-------------|-------|
| `sqlite` (default) | `rusqlite` | 1 |
| `cli` (default) | `clap` | 1 |
| `stdb` | `spacetimedb-sdk`, `tokio`, `reqwest` | 2 |
| `ingester` | `stdb` + `axum` + `tower-http` | 2 |
| `watcher-digest` | `stdb` | 2 |
| `inhibition` | (none) | 3 |
| `substrate-reciprocal` | `reqwest` | 3 |
| `full` | all of the above | meta |

---

## 9 — Cross-References

| External Doc | Location |
|---|---|
| plan.toml | `~/claude-code-workspace/memory-injection/plan.toml` (517 lines) |
| SpaceTimeDB plan vault | `~/claude-code-workspace/memory-injection/memory-injection-vault/` (95 notes) |
| ai_docs/ | `~/claude-code-workspace/memory-injection/ai_docs/` (10 docs + 6 layer docs) |
| ai_specs/ | `~/claude-code-workspace/memory-injection/ai_specs/` (10 specs + 6 layer specs + 7 patterns) |
| Canonical plan | `~/projects/shared-context/SpaceTimeDB Habitat Integration Plan — 2026-04-24.md` |
| Workspace CLAUDE.md | `~/claude-code-workspace/CLAUDE.md` |
| Main vault | `~/projects/claude_code/` ([[ULTRAPLATE Master Index]]) |

---

*Master Index v1.0 | 2026-04-24 | 6 layers, 24 modules, 5 binaries, 4 scripts | All notes carry `> Back to: [[HOME]]`*
