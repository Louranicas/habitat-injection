# Circle of Experts — SpaceTimeDB Schema & CLI Injection Design

## Your Mission
You are participating in a Circle of Experts. You have been assigned a PERSONA.
Read your persona file at /tmp/circle-of-experts/PERSONA-<your-pane>.md
Argue your position in 400-600 words.
Write your argument to /tmp/circle-of-experts/ARGUMENT-<your-pane>.md

## SpaceTimeDB Constraints (from docs)
- Tables: in-memory, WAL-persisted, public or private
- Reducers: transactional, NO I/O (no network, no filesystem, no randomness)
- Views: read-only computed queries, NO .iter() — only indexed lookups
- Subscriptions: clients subscribe to queries, get real-time push
- Schedule tables: trigger reducers on intervals
- Identity: OIDC-based, per-connection
- Modules: compile to WASM, run inside database

## The Problem
Claude Code starts every context window with amnesia. We need to inject complete
causal state in <100ms at session start. The injection must include:
- WHO am I (session, persona, model)
- WHERE was I (trajectory: fitness across last 5 sessions)  
- WHAT was I building (active workstreams, blockers)
- WHAT could bite me (active traps, known failure modes)
- WHY did things happen (causal chains linking events)
- WHAT have I learned (reinforced patterns, feedback)
- HOW is the system right now (12 service health, thermal, coupling)

## Live Data Sources
- ORAC :8133 — RALPH gen=26068, fitness=0.664, 8 emergence detectors, 23920 events
- PV2 :8132 — Kuramoto field r=1.0, 1 sphere, /bus/ws WebSocket
- POVM :8125 — 3554 Hebbian pathways with weights
- SYNTHEX :8090 — thermal PID, T=0.500
- RM :8130 — TSV heartbeat (tick/r/gen/fitness/phase)
- 21 SQLite tracking DBs (10 live with ~1800 rows, 11 dead)
- Atuin — 82+ scripts, 2714 shell history entries, KV store
- Obsidian — 215 notes
- Auto-Memory — ~50 markdown files with YAML frontmatter

## Deliverable
1. Your proposed STDB table schemas (Rust syntax)
2. Your proposed CLI injection tool chain
3. Your argument for WHY your approach is the right one
