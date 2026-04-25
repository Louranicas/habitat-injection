> Back to: [[HOME]] · [[MASTER INDEX]] · [[Implementation Status]]

# Injection Database State

> **DB:** `~/.local/share/habitat/injection.db`
> **Last updated:** 2026-04-24 (S111 — Phase 3 deployment + enrichment)

## Counts

| Table | Rows | Description |
|-------|------|-------------|
| `causal_chain` | 40 | 14 unresolved, 26 auto-resolved |
| `session_trajectory` | 21 | S060-S111 fitness arc |
| `workstream` | 12 | 3 active, 1 blocked, 7 deferred, 1 complete |
| `reinforced_pattern` | 74 | 19 reinforced (weight >0.51), 55 at default 0.50 |
| `injection_cache` | 1 | Pre-computed <2KB payload |
| `session_checkpoint` | 0 | Populated by /save-session integration |
| `injection_script` | 0 | Populated by atuin registration |

## Data Sources

| Source | What It Fed | Rows |
|--------|------------|------|
| Session notes (S060-S111) | `causal_chain` (bugs, traps, plans, patterns) | 40 |
| CLAUDE.local.md metrics | `session_trajectory` | 21 |
| CLAUDE.local.md priorities | `workstream` | 12 |
| `service_tracking.db` learned_patterns (141) | `reinforced_pattern` | 38 base + 20 imported |
| `service_tracking.db` cross_agent_learnings (12) | `reinforced_pattern` | 8 semantic patterns |
| `hebbian_pulse.db` pathway coupling | Informed pattern selection | (indirect) |
| Auto-memory feedback files (26) | Cross-validated with patterns | 26 feedback memories |
| CLI tool configs (nvim/atuin/fzf/lazygit/yazi/bacon) | `reinforced_pattern` + `causal_chain` | 16 tool + 14 chaining + 5 topology |
| POVM pathways (3571) | Not yet imported — Phase 2 | 0 |
| ORAC health endpoint | `session_trajectory` via consolidation | Live |

## Hebbian State

| Weight Range | Count | Meaning |
|-------------|-------|---------|
| 0.61+ | 3 | Fired 6× — `quality-gate-chain`, `verify-before-ship`, `read-only-forensics` |
| 0.60 | 4 | Fired 5× — `research-first`, `binary-deployment-cp`, `never-test-fit`, `god-tier-engineering` |
| 0.58-0.59 | 3 | Fired 4× — `tc1-funnel-discovery`, `tc2-parallel-fanout`, `database-schema-first` |
| 0.57 | 3 | Fired 3× — `tc5-build-fix-converge`, `progressive-disclosure`, `compound-parallel-probing` |
| 0.55 | 6 | Fired 2× — `nvim-rpc-structural`, `atuin-kv-state`, `fzf-filter-noninteractive`, `lazygit-habitat-commands`, `bacon-remote-control`, `chain-rg-fzf-read` |
| 0.50 | 55 | Default — newly seeded, not yet fired |

## Unresolved Chains

| Label | Type | Origin | Description |
|-------|------|--------|-------------|
| `BUG-064i-pathway-update` | bug | S108 | POVM bridge pathway update discarded |
| `daemon-phase-g-blocked` | plan | S107 | synthex-v2 v1 streaming external gate |
| `comms-layer-v3` | plan | S108 | 10/16 shipped, WS-6+ pending |
| `BUG-openclaw-prune` | bug | S102 | docker prune erased openclaw-gateway |
| `trap-docker-prune-blanket` | trap | S102 | Blanket commands rebuild their own filter |
| `trap-v8-cli-hang` | trap | S104 | V8 CLI can hang on stdin — timeout calls |
| `trap-l1-gate-empty-stdout` | trap | S106 | synthex-v2-l1-gate.sh empty stdout regression |
| `synthex-v2-daemon-plan` | plan | S106 | 9 phases, 3100 LOC daemon integration |
| `watcher-persona-crystallised` | plan | S108 | 10-surface Watcher persona |

## CLI Binaries

See [[Binary Map]] for full details.

| Binary | Test | Result |
|--------|------|--------|
| `habitat-init` | Creates DB with 7 tables | PASS |
| `habitat-inject` | <2KB in <10ms | PASS (1188 bytes, 9ms) |
| `habitat-consolidate` | Hebbian cycle runs | PASS (10 reinforced, 7 auto-resolved) |
| `habitat-query` | All presets work | PASS (trajectory, chains, workstreams, patterns, summary) |
| `habitat-seed` | 35+21+12+38 rows | PASS |

## Hook Status

Position 3 in `~/.claude/settings.json` SessionStart chain. Replaces `atuin scripts run habitat-bootstrap`. Old script preserved.

---

*See: [[Hebbian Learning]] for algorithm details · [[Three-Tier Fallback]] for degradation chain · [[Execution Plan]] for deployment steps · [[CLI Tool Ecosystem]] for tool chaining patterns*
