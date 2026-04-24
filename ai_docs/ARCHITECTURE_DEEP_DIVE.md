# habitat-injection — Architecture Deep Dive

## Layer Overview

6 layers, bottom-up dependency chain.

## L1 Foundation

Core types (SessionId, WorkstreamId, ChainId, ConsentLevel, PatternWeight), error taxonomy, config, constants, traits (Injectable, Consolidatable, Queryable), validation. No upward imports.

## L2 Schema & Persistence

SQLite schema creation, migration runner, 5 table definitions (causal_chain, session_trajectory, workstream, reinforced_pattern, injection_cache), CRUD operations, query builders, index management.

## L3 Injection Engine

SessionStart injection pipeline: parallel query executor, prose renderer (<2KB budget), three-tier fallback (SQLite → atuin KV → static), staleness annotation, consent filtering, token counting.

## L4 Consolidation Engine

Post-session write-back: trajectory capture, workstream update, causal chain reinforcement, Hebbian decay (0.95× unfired), pattern reinforcement (0.1×(1-w) fired), auto-resolve after 10 quiet sessions, injection_cache rebuild, atuin KV cache.

## L5 Query & Browser

Interactive memory browser: preset queries (trajectory, chains, workstreams, patterns), raw SQL passthrough, fzf integration, formatted output, atuin script registration.

## L6 SpaceTimeDB Migration

Phase 2: STDB module (5 tables in Rust), ingester binary (ORAC/PV2/SYNTHEX/POVM bridges), cascade_forget reducer, watcher_digest table, schedule-table reducers for injection_cache rebuild.

