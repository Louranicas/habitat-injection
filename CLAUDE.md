# habitat-injection

> 6 layers | 24 modules | Quality gate: check + clippy + pedantic + test

## Architecture

```
L1 Foundation   (m01_types-m05_constants): Core types (SessionId, WorkstreamId, ChainId, ConsentLevel, PatternWeight), error taxonomy, config, constants, traits (Injectable, Consolidatable, Queryable), validation. No upward imports.
L2 Schema & Persistence (m06_schema-m10_pattern): SQLite schema creation, migration runner, 5 table definitions (causal_chain, session_trajectory, workstream, reinforced_pattern, injection_cache), CRUD operations, query builders, index management.
L3 Injection Engine (m11_parallel_query-m14_consent_filter): SessionStart injection pipeline: parallel query executor, prose renderer (<2KB budget), three-tier fallback (SQLite → atuin KV → static), staleness annotation, consent filtering, token counting.
L4 Consolidation Engine (m15_trajectory_capture-m18_atuin_cache): Post-session write-back: trajectory capture, workstream update, causal chain reinforcement, Hebbian decay (0.95× unfired), pattern reinforcement (0.1×(1-w) fired), auto-resolve after 10 quiet sessions, injection_cache rebuild, atuin KV cache.
L5 Query & Browser (m19_preset_queries-m21_fzf_browser): Interactive memory browser: preset queries (trajectory, chains, workstreams, patterns), raw SQL passthrough, fzf integration, formatted output, atuin script registration.
L6 SpaceTimeDB Migration (m22_stdb_module-m24_migration): Phase 2: STDB module (5 tables in Rust), ingester binary (ORAC/PV2/SYNTHEX/POVM bridges), cascade_forget reducer, watcher_digest table, schedule-table reducers for injection_cache rebuild.
```

## Quality Gate (MANDATORY)

```bash
cargo check && cargo clippy -- -D warnings && \
cargo clippy -- -D warnings -W clippy::pedantic && cargo test --lib
```

## Rules

- must: No `unwrap()`/`expect()` outside tests — enforced via `[lints.clippy]`
- must: No `unsafe` — zero tolerance
- should: Doc comments on all public items
- should: 50+ tests per module minimum
- should: Backtick all identifiers in doc comments

## Implementation Order

L1 -> L2 -> L3 -> L4 -> L5 -> L6

## Documentation Map

| Directory | Contents |
|-----------|----------|
| `ai_docs/` | Architecture, module docs, onboarding, schematics index |
| `ai_docs/modules/` | Per-layer module documentation |
| `ai_specs/` | Technical specifications, constraints, protocols |
| `ai_specs/layers/` | Per-layer implementation specs |
| `ai_specs/patterns/` | Rust patterns, anti-patterns, concurrency |
| `schematics/` | Mermaid diagrams: architecture, API, data flow |
| `config/` | TOML configs: default, production, devenv |
| `.claude/` | Claude Code context: patterns, schemas, queries |
| `habitat-injection-vault/` | Obsidian vault: 25 notes, layers, modules, architecture, schematics, operations |
| `memory-injection-vault/` | SpaceTimeDB Phase 2 plan vault: 95 notes, 24 Mermaid diagrams |

<!-- INSIGHTS-S1000146-WORKFLOW-ADDITIONS -->

---

## Concurrent File Editing

When editing shared markdown files (especially in fleet/multi-pane scenarios), prefer atomic `bash` append (`cat >> file` or `echo '...' >> file`) over the Edit tool. Other panes may be writing concurrently and Edit will fail on stale content. Only use Edit for files you have exclusive access to.

## Verification Discipline

- Before writing new helper methods (e.g., `sweep`, `cleanup`, `compact`, `purge`), grep the codebase for existing equivalents and surface what exists first; ask whether to extend vs. create new.
- Before fixing reported findings, FP-verify against source first — many cross-agent findings turn out to be already fixed.
- After applying fixes, always run the full quality gate (`cargo test`, `cargo clippy -- -D warnings`, `cargo check`) before declaring complete. Report exact test counts (e.g., `1830/1830 passing, zero warnings`).

## Avoid Over-Engineering

When recommending architectural changes, start with the simplest integration (blackboard pattern, additive wiring) before suggesting major refactors of core state structs (e.g., `OracState`). Ask before proposing changes that touch >5 files or core state types.

## Quality Gates

- Always run the full test suite and quality gates (clippy, fmt, lint) after multi-file changes before declaring complete.
- Report exact test counts in completion summaries.
- Minimum 50+ tests per module unless otherwise specified.
- After any toolchain upgrade (rustc, clippy), expect new lints; run the full gate script and fix all clippy errors before declaring done. Verify PATH in both `.bashrc` and gate scripts points to the upgraded toolchain.

## Documentation Persistence

- After completing significant work, save findings/schematics to the Obsidian vault with bidirectional wikilinks.
- Update relevant `INDEX.md` files when adding notes.
- Verify all wikilinks resolve before considering documentation complete.

## Git Workflow

- After completing hardening or feature work, commit and push to BOTH GitHub and GitLab remotes unless told otherwise.
- Include test pass counts and quality gate status in commit messages.

## Recurring Loops & Cron

- When a recurring/cron loop's work is complete (convergence, G7, end-of-life signal detected), proactively recommend `CronDelete` or cancellation.
- Recognize duplicate/stale prompts from cron firings and skip rather than re-executing completed work.
