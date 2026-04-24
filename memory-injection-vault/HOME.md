# Memory Injection — Vault Index

> **Status:** AUTHORED v1 — awaits execution
> **Date:** 2026-04-24 | **Session:** 109
> **Scope:** 5 phases, ~50-60h, 8 tables, 10 reducers, sidecar on `:3000`
> **Vault:** 46 notes · ~4,100 lines · 24 Mermaid diagrams · 109 proposed files

---

## Quick Navigation

### Core Plan
- [[Executive Summary]] — goal, architecture decision, shape
- [[Current State — Memory Substrates]] — the 6 substrates and 21 tracking DBs being consolidated
- [[Bootstrap Chain — Current vs Target]] — L0-L6 today → L0-L10 with STDB

### Architecture
- [[Sidecar Architecture]] — devenv registration, project structure, port `:3000`
- [[Ingester Pipeline]] — multi-source event pipeline (ORAC, PV2, SYNTHEX, POVM, Atuin)
- [[Injector — Context Window Bootstrap]] — the <100ms injection at session start
- [[Comms Layer v3 Alignment]] — mapping v3 mechanisms → STDB primitives

### Schema (8 Tables)
- [[T1 — HabitatEvent]] — causal event log with `causal_parent` chains
- [[T2 — KnowledgeEdge]] — unified weighted graph (POVM + patterns + hebbian + synergy)
- [[T3 — GradientSnapshot]] — time-series vital signs
- [[T4 — SessionRecord]] — Claude Code session tracking
- [[T5 — Workstream]] — in-flight work ledger
- [[T6 — ServiceHealth]] — service health timeline
- [[T7 — TrapState]] — active trap monitoring
- [[T8 — WatcherObservation]] — Watcher anomaly records

### Reducers
- [[Reducers]] — R1-R6 (ingest, reinforce, gradient, session, decay, forget)

### Phases
- [[Phase A — STDB Deploy]] — core tables + basic ingester (6-8h)
- [[Phase B — Knowledge Graph Migration]] — POVM + SQLite → STDB (8-10h)
- [[Phase C — Watcher + Causal Chains]] — causal linking + forget cascade (6-8h)
- [[Phase D — Cross-Service Integration]] — bridges + Telegram + Obsidian (8-10h)
- [[Phase E — Bootstrap Revolution]] — injector CLI + dead DB cleanup (6-8h)

### Migration
- [[Migration Strategy]] — what migrates, what doesn't, POVM dual-write

### Gap Analyses
- [[Gap Analysis — Conventional]] — 17 gaps (5C + 7I + 5N)
- [[Gap Analysis — Non-Anthropocentric]] — 8 NA gaps
- [[Recommendations Summary]] — prioritised adoption tiers

### Risk & Success
- [[Risk Register]] — 7 risks with mitigations
- [[Success Criteria]] — 7 pass/fail gates

### Schematics & Diagrams
- [[System Topology]] — full 15-port service wiring with STDB
- [[Batch Ordering]] — devenv start sequence
- [[SessionStart Injection Sequence]] — complete hook chain with latency gantt
- [[Data Flow — Ingestion]] — source → reducer → table mapping + retention policy
- [[Causal Chain Architecture]] — 7-event example + 5 linkage rules + SQL patterns
- [[Knowledge Graph Structure]] — edge distribution, namespace clusters, decay dynamics
- [[Migration Flow]] — 16 sources → 6 tables + POVM dual-write + checksum verification
- [[Tool Chain Catalog]] — TC6-TC10 new patterns
- [[Phase Timeline]] — gantt + critical path
- [[Entity Relationship Diagram]] — all 8 tables with FK relationships
- [[Memory Consolidation Map]] — before/after 21 DBs → 8 tables
- [[Reducer Lifecycle]] — when each R1-R10 fires + conflict matrix
- [[NA Mechanism Matrix]] — NA commitments → operational mechanisms
- [[Proposed Directory Tree]] — 109-file implementation blueprint across 4 workspaces

### Context
- [[Session Estimates]] — session-by-session timeline
- [[What Replaces vs Preserves]] — explicit scope boundaries

---

## Cross-References

| External Doc | Location |
|---|---|
| Canonical plan | `~/projects/shared-context/SpaceTimeDB Habitat Integration Plan — 2026-04-24.md` |
| Comms Layer v3 | `~/projects/shared-context/Comms Layer Unification Plan — 2026-04-24.md` |
| Master Plan v2 | `~/Downloads/MASTER_PLAN.md` |
| S103 Genesis | `~/projects/shared-context/Session 103 — SYNTHEX v2 Genesis.md` |
| Memory Audit | `~/projects/claude_code/Habitat Memory Substrate — Deep Audit 2026-04-22.md` |
| S101 Injection Roadmap | `~/projects/claude_code/Advanced Clustered Tooling — S101 Memory Injection Roadmap.md` |
| Workspace CLAUDE.md | `~/claude-code-workspace/CLAUDE.md` |
