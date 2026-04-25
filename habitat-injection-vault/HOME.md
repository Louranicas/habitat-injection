> Back to: [[CLAUDE.md]] (`~/claude-code-workspace/memory-injection/CLAUDE.md`)

# Habitat Injection — Vault Index

> **Package:** `habitat-injection` | **Directory:** `~/claude-code-workspace/memory-injection/`
> **Architecture:** 6 layers, 27 modules | **Phase:** PIPELINE LIVE — injection firing at session start
> **Database:** `~/.local/share/habitat/injection.db` — 47 chains, 23 sessions, 80 patterns, 15 workstreams
> **Quality gate:** `cargo check` + `clippy -D warnings` + `clippy pedantic` + `cargo test --lib`
> **Origin:** Circle of Experts deliberation (10 CC instances, 4 rounds, 48 arguments, 384 KB)

---

## Quick Navigation

### Project Overview
- [[Executive Summary]] — what this system does and why it exists
- [[Architecture Overview]] — 6-layer dependency chain, design principles
- [[Implementation Status]] — what's built, what's pending, quality gate state

### Layers (L1-L6)
- [[L1 Foundation]] — core types, errors, config, traits, constants (m01-m05)
- [[L2 Schema & Persistence]] — SQLite tables, CRUD, migrations (m06-m10b)
- [[L3 Injection Engine]] — SessionStart pipeline, renderer, fallback (m11-m14)
- [[L4 Consolidation Engine]] — post-session write-back, Hebbian decay (m15-m18)
- [[L5 Query & Browser]] — preset queries, raw SQL, fzf, scripts engine (m19-m21b)
- [[L6 SpaceTimeDB Migration]] — Phase 2 STDB module, ingester, migration (m22-m24)

### Modules (24 total)
- [[Module Index]] — all 24 modules with dependencies and test status
- [[Gold Standard Exemplars]] — 3 exemplar modules from SYNTHEX v2 + DevOps V3 with code patterns

### Architecture
- [[Data Flow]] — how memory flows from services through consolidation to injection
- [[Dependency Graph]] — layer and module dependency DAG
- [[Consent Model]] — Emit / Store / Forget three-tier consent filtering
- [[Hebbian Learning]] — decay (0.95x unfired) + reinforce (0.1x(1-w) fired) + prune (<0.05)

### Schematics
- [[Complete Wiring Schematic]] — **MASTER** — full system topology, all binaries, services, databases, hook chain, consent wiring (20 Mermaid diagrams)
- [[API Endpoints Map]] — every HTTP endpoint consumed, served, and planned with port registry
- [[Hebbian Lifecycle Wiring]] — 4-step consolidation cycle, weight trajectories, chain discovery pipeline
- [[Injection Payload Format]] — exact <2KB output format, token budgets, overflow strategy, render pipeline
- [[STDB Phase 2 Wiring]] — 8 table mirrors, 5 ingester sources, 11 reducers, migration pipeline
- [[Schema Diagram]] — 6 SQLite tables with relationships
- [[Injection Pipeline]] — SessionStart hook chain and latency budget
- [[Three-Tier Fallback]] — SQLite -> atuin KV -> static fallback chain
- [[Tool Chain Patterns]] — TC6-TC10 habitat-injection patterns

### Execution (Phase 3 — CLI + Deployment)
- [[Execution Plan]] — 11 steps from library-complete to production
- [[CLI Binary Architecture]] — 4 binaries, dependency map, sequence diagrams
- [[Deployment Checklist]] — step-by-step checklist with acceptance criteria

### Operations & Diagnostics
- [[System Verification Report]] — **S111 VERIFIED** — full pipeline test: DB integrity, cache, hook, all 6 binaries, Hebbian cycle
- [[Diagnostics Runbook]] — symptom → diagnosis → fix for every failure mode, health check script, PRAGMA settings
- [[Fidelity Tuning Guide]] — Hebbian weight calibration, convergence math, tuning scenarios, quality metrics
- [[Habitat Assimilation Guide]] — ecosystem integration, POVM namespaces, memory substrate map, bidirectional bridge map
- [[Binary Map]] — 6 binaries: init, inject, seed, consolidate, query, memory
- [[Hook Registration]] — SessionStart hook wiring in settings.json (position 3)
- [[Quality Gate Protocol]] — 5-stage zero-tolerance gate (incl. no-default-features)
- [[Deliberation Record]] — Circle of Experts provenance and consensus
- [[Injection Database State]] — live DB: 47 chains, 23 sessions, 80 patterns, 15 workstreams
- [[CLI Tool Ecosystem]] — nvim/atuin/fzf/lazygit/yazi/bacon + 14 chaining patterns + workspace topology

### Cross-References
- [[SpaceTimeDB Plan]] — Phase 2 STDB vault at `memory-injection-vault/`
- Main vault: `~/projects/claude_code/habitat-injection — Complete Wiring Schematics.md`
- synthex-v2 vault: `~/claude-code-workspace/synthex-v2/obsidian-synthex-v2/synthex-v2/habitat-injection — Cross-Project Bridge.md`

---

## External Links

| Doc | Location |
|---|---|
| Project CLAUDE.md | `~/claude-code-workspace/memory-injection/CLAUDE.md` |
| plan.toml | `~/claude-code-workspace/memory-injection/plan.toml` |
| ai_docs/ | `~/claude-code-workspace/memory-injection/ai_docs/` |
| ai_specs/ | `~/claude-code-workspace/memory-injection/ai_specs/` |
| SpaceTimeDB vault | `~/claude-code-workspace/memory-injection/memory-injection-vault/` |
| Workspace CLAUDE.md | `~/claude-code-workspace/CLAUDE.md` |
| ULTRAPLATE Master Index | `~/projects/claude_code/` (main vault) |

---

*Vault created 2026-04-24, schematics + operations expanded 2026-04-25 | 6 layers, 27 modules, 6 binaries, PIPELINE LIVE | All notes carry `> Back to: [[HOME]]` for graph connectivity*
