# Memory Injection — Master Index

> **Version:** 1.1 | **Created:** 2026-04-24 | **Updated:** 2026-04-24 | **Session:** 109
> **Vault:** 41 notes · 3,800+ lines · 5 directories · ~160 KB
> **Plan status:** AUTHORED v1 — awaits execution
> **Recommended scope:** Must-Have + NA-R3 = ~57-67h across 11-13 sessions

---

## §1 — Plan Overview

| Note | Lines | Purpose |
|------|-------|---------|
| [[HOME]] | 70 | Quick navigation hub — start here for vault exploration |
| [[Executive Summary]] | 41 | Goal, sidecar vs plugin decision (91/100 vs 22/100), shape, key differentiator |
| [[What Replaces vs Preserves]] | 27 | Explicit scope: what STDB consolidates, what stays independent |

---

## §2 — Current State (what we're consolidating)

| Note | Lines | Covers |
|------|-------|--------|
| [[Current State — Memory Substrates]] | 54 | 6 live substrates, 21 tracking DBs (10 live / 11 dead), POVM namespace distribution, 5 canonical data patterns |
| [[Bootstrap Chain — Current vs Target]] | 70 | L0-L6 today (55ms, 9KB) → L0-L10 with STDB (<100ms, 15KB). Full injection payload example |

---

## §3 — Architecture (how it works)

| Note | Lines | Covers |
|------|-------|--------|
| [[Sidecar Architecture]] | 45 | devenv.toml registration (`:3000`, Batch 1), 3 workspace split (module/ingester/injector), port map |
| [[Ingester Pipeline]] | 48 | Multi-source data flow diagram, source→reducer mapping (5 sources, 6 reducer targets), NA-R3 reciprocal paths, devenv registration |
| [[Injector — Context Window Bootstrap]] | 56 | SessionStart hook flow, `spacetime sql` CLI approach (not SDK), NA-R8 adaptive payload, latency budget (<100ms) |
| [[Comms Layer v3 Alignment]] | 26 | 8 v3 mechanisms → STDB primitives mapping, OIDC migration path |

---

## §4 — Schema (8 tables + extensions)

### Core Tables

| Note | Lines | Table | Consolidates | Rows at Migration |
|------|-------|-------|-------------|-------------------|
| [[T1 — HabitatEvent]] | 67 | Causal event log | service_events + pulse_events + emergence + bus events | ~500 initial |
| [[T2 — KnowledgeEdge]] | 80 | Unified weighted graph | POVM (3554) + patterns (141) + orchestration (29) + hebbian (109) + synergy (89) | ~3,922 |
| [[T3 — GradientSnapshot]] | 82 | Time-series vital signs | gradient_snapshot.db + probes + RM heartbeat | ~2,001 |
| [[T4 — SessionRecord]] | 44 | Claude Code sessions | New (session lifecycle tracking) | ~110 |
| [[T5 — Workstream]] | 44 | In-flight work ledger | V3 workflow_state.db + synthex-v2 workflow_tracking.db | ~6 |
| [[T6 — ServiceHealth]] | 39 | Service health timeline | bridge_health + integration_health + service registry | ~100 |
| [[T7 — TrapState]] | 49 | Active trap monitor | New (18 known traps from S101 roadmap) | 18 |
| [[T8 — WatcherObservation]] | 44 | Watcher anomaly records | synthex-v2 watcher_observation.db schema | 0 (scaffold) |

### Schema Extensions (from gap analyses)

| Extension | Source | Affects | Purpose |
|-----------|--------|---------|---------|
| Per-edge learning params | [[Gap Analysis — Non-Anthropocentric]] NA-R1 | [[T2 — KnowledgeEdge]] | `decay_rate`, `learning_rate_ltp/ltd`, `consolidation_interval_ticks` — preserves substrate-specific plasticity |
| Consent state | NA-R2 | [[T1 — HabitatEvent]], [[T2 — KnowledgeEdge]] | `consent_state: "full"\|"minimal"\|"none"\|"inherited"` |
| Service self-reported health | NA-R6 | [[T3 — GradientSnapshot]] | `orac_system_grade`, `pv2_fleet_mode`, `synthex_pid_converging`, `me_overall_health` |
| Retention class | [[Gap Analysis — Conventional]] C3 | [[T1 — HabitatEvent]] | `retention_class: "full"\|"envelope"\|"sampled"` |
| T9 ServiceSession (proposed) | NA-R5 | New table | Service lifecycle tracking alongside human sessions |

---

## §5 — Reducers (10 total)

| Note | Lines | Covers |
|------|-------|--------|
| [[Reducers]] | 41 | R1 `ingest_event` · R2 `reinforce_edge` · R3 `capture_gradient` · R4 `register_session` · R5 `run_decay` (per-edge) · R6 `forget_sphere` (NA-P-13) · R7 `compact_old_events` (retention) · R8 `consolidate_mature_edges` (POVM cycle) · R9 `watcher_reinforce` · R10 `watcher_annotate_event` |

---

## §6 — Phases (5 phases, ~57-67h)

| Note | Lines | Phase | Hours | Key Gate |
|------|-------|-------|-------|----------|
| [[Phase A — STDB Deploy]] | 29 | Core tables + ingester | 6-8 | Events appearing in STDB |
| [[Phase B — Knowledge Graph Migration]] | 27 | POVM + SQLite → STDB | 8-10 | KnowledgeEdge ≥ 3,934 rows, weight checksums match |
| [[Phase C — Watcher + Causal Chains]] | 30 | Causal linking + forget | 6-8 | `causal_parent IS NOT NULL` rows exist |
| [[Phase D — Cross-Service Integration]] | 25 | Bridges + Telegram + Obsidian | 8-10 | Full round-trip verified |
| [[Phase E — Bootstrap Revolution]] | 28 | Injector CLI + dead DB cleanup | 6-8 | <100ms injection in new session |

**Critical path:** A → B → C → D → E (sequential, each depends on prior)

**Independently valuable:** Phase A alone gives event capture + gradient history from day 1.

---

## §7 — Migration

| Note | Lines | Covers |
|------|-------|--------|
| [[Migration Strategy]] | 57 | 16 sources → STDB table mapping, POVM dual-write transition (3 phases), verification checksums, VMS out-of-scope rationale |

---

## §8 — Gap Analyses & Recommendations

| Note | Lines | Gaps Found | Recommended Adoption |
|------|-------|------------|---------------------|
| [[Gap Analysis — Conventional]] | 41 | 17 (5C + 7I + 5N) | C1-C4 + I1-I2 minimum (+8h) |
| [[Gap Analysis — Non-Anthropocentric]] | 35 | 8 (all NA-C) | NA-R1 + NA-R2 + NA-R7 minimum (+5.25h) |
| [[Recommendations Summary]] | 60 | Combined tiers: Must-Have (+13.25h), Should-Have (+10.6h), Nice-to-Have (+7h) | **Must-Have + NA-R3 = +17.25h** |

### Critical Gaps at a Glance

| ID | Gap | If Ignored |
|----|-----|-----------|
| C1 | STDB views can't use `.iter()` | Phase E won't compile |
| C3 | No retention policy | OOM in ~60 days |
| C4 | Causal parent wiring undefined | Core differentiator ships empty |
| NA-C1 | Uniform decay erases substrate rhythms | Knowledge graph loses its metabolism |
| NA-C3 | Ingester is purely extractive | STDB becomes a panopticon |

---

## §9 — Risk & Verification

| Note | Lines | Covers |
|------|-------|--------|
| [[Risk Register]] | 17 | 7 risks: OOM, SDK conflict, migration loss, sync drift, latency, payload size, crash recovery |
| [[Success Criteria]] | 17 | 7 pass/fail gates: <100ms bootstrap, trajectory visible, causal queries work, single query surface, reinforcement live, dead weight removed, round-trip verified |
| [[Session Estimates]] | 23 | Session-by-session timeline (S110-S118 + buffer) |

---

## §10 — Provenance & Cross-References

### Canonical Sources (outside this vault)

| Doc | Path | Lines |
|-----|------|-------|
| Full integration plan | `~/projects/shared-context/SpaceTimeDB Habitat Integration Plan — 2026-04-24.md` | 911 |
| Conventional gap analysis | `~/projects/shared-context/SpaceTimeDB Habitat Integration Plan — Gap Analysis 2026-04-24.md` | 244 |
| NA gap analysis | `~/projects/shared-context/SpaceTimeDB Habitat Integration Plan — NA Gap Analysis 2026-04-24.md` | 252 |
| Plan copies | `~/claude-code-workspace/memory-injection/{PLAN,GAP_ANALYSIS,NA_GAP_ANALYSIS}.md` | 1,407 |

### Upstream References

| Doc | Purpose |
|-----|---------|
| [[ULTRAPLATE Master Index]] (main vault) | 14 service registry, port map, Obsidian links |
| Comms Layer Unification Plan v3 | Push-channel convergence, §10.4 STDB mapping |
| Master Plan v2.0 | Phase 6 original sketch (4-day, expanded here to 57-67h) |
| Session 103 — SYNTHEX v2 Genesis | ADR-002 (STDB sidecar), ADR-004 (4-tier hybrid memory) |
| Habitat Memory Substrate Audit | 21-DB schema audit that informed table design |
| S101 Memory Injection Roadmap | L7-L10 bootstrap extensions (habitat-arc, traps-live, workstreams) |
| Coding Excellence Charter | Zero-tolerance quality gate binding all plan execution |

### SpaceTimeDB Reference

| Resource | Content |
|----------|---------|
| `~/claude-code-workspace/spacetimedb/` | Upstream STDB repo (40+ crates, CLI, SDKs) |
| `spacetimedb/skills/spacetimedb-concepts/` | Context7 skill: tables, reducers, subscriptions, identity |
| STDB docs (fetched 2026-04-24) | Architecture, modules, views, procedures, self-hosting, Rust SDK, OIDC auth |

---

## §11 — Schematics & Diagrams (14 notes, 24 Mermaid diagrams)

| Note | Lines | Diagrams | Covers |
|------|-------|----------|--------|
| [[System Topology]] | 115 | 1 graph (full service wiring + STDB) | 15-port topology, batch ordering, all connections |
| [[Batch Ordering]] | 42 | 1 graph (start sequence) | devenv batch 1-4 with STDB in Batch 1 |
| [[SessionStart Injection Sequence]] | 100 | 2 (sequence + gantt) | Complete hook chain: ORAC → health → STDB inject, latency breakdown |
| [[Data Flow — Ingestion]] | 115 | 2 (flowchart + retention) | Source → reducer → table mapping, event rate projections, retention policy |
| [[Causal Chain Architecture]] | 120 | 2 (chain example + linkage rules) | 7-event chain example, 5 linkage rules, SQL query patterns, TC8 investigation |
| [[Knowledge Graph Structure]] | 100 | 3 (pie + mindmap + learning dynamics) | Edge type distribution, namespace clusters, per-edge decay dynamics |
| [[Migration Flow]] | 105 | 4 (source mapping + dual-write + checksum + cleanup) | 16 sources → 6 tables, POVM transition states, dead DB deletion |
| [[Tool Chain Catalog]] | 120 | 5 diagrams (TC6-TC10) | 5 new tool chain patterns extending B1-B26 + TC1-TC5 |
| [[Phase Timeline]] | 65 | 2 (gantt + critical path) | Session-by-session timeline, independently-valuable milestones |
| [[Entity Relationship Diagram]] | 85 | 1 ER diagram | All 8 tables, foreign keys, cardinality, independence map |
| [[Memory Consolidation Map]] | 110 | 2 (before/after) | 21 DBs → 8 tables visual, net change metrics table |
| [[Proposed Directory Tree]] | 371 | — (annotated ASCII tree) | 109-file tree across 4 workspaces (module/ingester/injector/migration), build targets, Cargo.toml deps, inline purpose annotations |
| [[Reducer Lifecycle]] | 90 | 2 (trigger sources + cadence) | When each R1-R10 fires, conflict matrix, STDB MVCC guarantees |
| [[NA Mechanism Matrix]] | 80 | 1 (mechanism map) | NA commitments → operational mechanisms, per-phase verification checklist |

### Deployment Framework

| Note | Lines | Covers |
|------|-------|--------|
| [[DEPLOYMENT FRAMEWORK]] | 782 | Complete wiring: hooks → injector → STDB → Claude. Tool chains TC6-TC10. Atuin scripts. CLAUDE.md integration. Deployment checklist. End-to-end data flow. |

---

## §12 — Vault Statistics

| Metric | Value |
|--------|-------|
| Total notes | 46 |
| Total lines | ~4,100 |
| Total size | ~170 KB |
| Directories | 6 (root + architecture + schemas + phases + gaps + schematics) |
| Wikilinks | ~380 |
| Mermaid diagrams | 24 |
| Tables defined | 8 (T1-T8) + 1 proposed (T9) |
| Reducers defined | 10 (R1-R10) |
| Tool chain patterns | 5 new (TC6-TC10) |
| Phases | 5 (A-E) |
| Gaps catalogued | 25 (17 conventional + 8 NA) |
| Adoption tiers | 3 (Must-Have / Should-Have / Nice-to-Have) |
| Atuin scripts added | 4 |
| Proposed project files | 109 across 4 workspaces |

---

## §13 — Reading Order

**For orientation (5 min):**
1. [[Executive Summary]]
2. [[Bootstrap Chain — Current vs Target]]

**For architecture review (15 min):**
3. [[System Topology]] — the full picture
4. [[SessionStart Injection Sequence]] — how memory reaches Claude
5. [[Data Flow — Ingestion]] — how data flows in

**For schema deep-dive (20 min):**
6. [[Entity Relationship Diagram]] — table relationships at a glance
7. [[T1 — HabitatEvent]] → [[Causal Chain Architecture]]
8. [[T2 — KnowledgeEdge]] → [[Knowledge Graph Structure]]
9. [[Reducers]] → [[Reducer Lifecycle]]

**For planning (10 min):**
10. [[Phase Timeline]] — gantt + critical path
11. [[Phase A — STDB Deploy]] → skim B-E
12. [[Recommendations Summary]]

**For deployment (15 min):**
13. [[DEPLOYMENT FRAMEWORK]] — the complete wiring document
14. [[Tool Chain Catalog]] — TC6-TC10 patterns
15. [[Migration Flow]] — what moves where
16. [[Proposed Directory Tree]] — the 109-file implementation blueprint

**For full depth (60 min):**
17. All 8 schema notes T1-T8
18. [[Migration Strategy]]
19. Both gap analyses
20. [[NA Mechanism Matrix]]
21. [[Memory Consolidation Map]]
22. [[Risk Register]] + [[Success Criteria]]

---

*Master Index v1.2 · 2026-04-24 · 46 notes · 24 Mermaid diagrams · 109 proposed files · Round-trip navigable via [[HOME]] hub · All notes carry `> Back to: [[HOME]]` for graph connectivity*
