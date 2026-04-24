> Back to: [[CLAUDE.md]] (`~/claude-code-workspace/memory-injection/CLAUDE.md`)

# Habitat Injection — Vault Index

> **Package:** `habitat-injection` | **Directory:** `~/claude-code-workspace/memory-injection/`
> **Architecture:** 6 layers, 24 modules | **Phase:** Scaffold complete, implementation pending
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
- [[Schema Diagram]] — 6 SQLite tables with relationships
- [[Injection Pipeline]] — SessionStart hook chain and latency budget
- [[Three-Tier Fallback]] — SQLite -> atuin KV -> static fallback chain
- [[Tool Chain Patterns]] — TC6-TC10 habitat-injection patterns

### Operations
- [[Binary Map]] — 5 binaries: inject, consolidate, query, init, scripts
- [[Hook Registration]] — SessionStart hook wiring in settings.json
- [[Quality Gate Protocol]] — 4-stage zero-tolerance gate
- [[Deliberation Record]] — Circle of Experts provenance and consensus

### Cross-References
- [[SpaceTimeDB Plan]] — Phase 2 STDB vault at `memory-injection-vault/`

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

*Vault created 2026-04-24 | 6 layers, 24 modules, 5 binaries | All notes carry `> Back to: [[HOME]]` for graph connectivity*
