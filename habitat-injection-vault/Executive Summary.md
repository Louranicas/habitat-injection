> Back to: [[HOME]] · [[MASTER INDEX]]

# Executive Summary

## What

`habitat-injection` is a memory injection system for Claude Code. It delivers <2KB of causal state into the context window in <100ms at session start, replacing the current 9KB bootstrap with a compressed, trajectory-aware, Hebbian-reinforced payload.

## Why

Every new Claude Code context window starts with amnesia. The current `habitat-bootstrap` atuin script injects identity + live metrics + patterns + CLI muscle (55ms, ~9KB), but misses trajectory (fitness deltas across sessions), causal chains (why things happened), workstream state (in-flight/blocked/deferred), active traps, and pattern reinforcement.

## How

- **Phase 1 (SQLite):** 6 tables, 5 binaries, SessionStart hook. Consolidates post-session state via /save-session checkpoint harvesting + Hebbian decay/reinforce. Injects via parallel query + prose renderer + three-tier fallback.
- **Phase 2 (SpaceTimeDB):** Migrates to STDB for real-time subscriptions, causal chain queries, and cascade_forget. See [[SpaceTimeDB Plan]].

## Origin

Deliberated by 10 Claude Code instances across 4 rounds (48 argument files, 384 KB total). Consensus: 5 core tables, SQLite first, STDB when justified. Kill criteria: 20 sessions without measurable improvement -> revert.

## Key Differentiator

Causal chains with Hebbian reinforcement. Traps and bugs that keep recurring get reinforced (higher weight = injected sooner). Patterns that stop firing decay and eventually get pruned. The memory actively learns which context matters.
