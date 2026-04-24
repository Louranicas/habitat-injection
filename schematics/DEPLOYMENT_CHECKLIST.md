# Deployment Checklist — Phase 1 SQLite

> Back to: [[EXECUTION_PLAN]] · [[CLI_BINARY_ARCHITECTURE]]

## Pre-Deployment

- [ ] `cargo test --lib` — 1696 passed, 0 failed
- [ ] `cargo clippy -- -D warnings -W clippy::pedantic` — clean
- [ ] `cargo check --no-default-features` — clean

## Step 1: habitat-init

- [ ] `src/bin/habitat_init.rs` written
- [ ] `cargo build --release --bin habitat-init`
- [ ] `/usr/bin/cp -f target/release/habitat-init ~/.local/bin/`
- [ ] `habitat-init` creates `~/.local/share/habitat/injection.db`
- [ ] `sqlite3 ~/.local/share/habitat/injection.db ".tables"` shows 6 tables
- [ ] `sqlite3 ~/.local/share/habitat/injection.db "PRAGMA user_version"` shows 2

## Step 2-5: Data Seeding

- [ ] `src/bin/habitat_seed.rs` written
- [ ] Seed causal_chain: `habitat-seed --source chains`
- [ ] Seed trajectory: `habitat-seed --source trajectory`
- [ ] Seed workstreams: `habitat-seed --source workstreams`
- [ ] Seed patterns: `habitat-seed --source patterns`
- [ ] `habitat-query summary` shows: Chains: N | Sessions: N | Workstreams: N | Patterns: N
- [ ] Manual review: `habitat-query chains` — verify no false positives

## Step 6: habitat-inject

- [ ] `src/bin/habitat_inject.rs` written
- [ ] `cargo build --release --bin habitat-inject`
- [ ] `/usr/bin/cp -f target/release/habitat-inject ~/.local/bin/`
- [ ] `habitat-inject` produces <2KB stdout
- [ ] `time habitat-inject > /dev/null` — <100ms
- [ ] Output contains `### Orientation`, `### Trajectory`, `### Workstreams`, `### Unresolved Chains`
- [ ] Tier 2 fallback: rename DB, run again — uses atuin KV
- [ ] Tier 3 fallback: clear atuin KV, run again — static message

## Step 7: habitat-consolidate

- [ ] `src/bin/habitat_consolidate.rs` written
- [ ] `cargo build --release --bin habitat-consolidate`
- [ ] `/usr/bin/cp -f target/release/habitat-consolidate ~/.local/bin/`
- [ ] `habitat-consolidate --session 110` captures trajectory
- [ ] `habitat-consolidate --session 110 --fired-patterns verify-before-ship` reinforces
- [ ] `habitat-query patterns` shows updated weight
- [ ] `habitat-query chains` shows auto-resolved stale chains

## Step 8: habitat-query

- [ ] `src/bin/habitat_query.rs` written
- [ ] `cargo build --release --bin habitat-query`
- [ ] `/usr/bin/cp -f target/release/habitat-query ~/.local/bin/`
- [ ] `habitat-query trajectory` — formatted table
- [ ] `habitat-query chains` — formatted table
- [ ] `habitat-query workstreams` — formatted table
- [ ] `habitat-query patterns` — formatted table
- [ ] `habitat-query summary` — one-line counts
- [ ] `habitat-query "SELECT count(*) FROM causal_chain"` — raw SQL works
- [ ] `habitat-query --interactive` — fzf mode (or graceful fallback)

## Step 9: Hook Wiring

- [ ] `~/.claude/settings.json` updated — Hook 3 = `habitat-inject`
- [ ] Old hook (`atuin scripts run habitat-bootstrap`) preserved as backup
- [ ] New session starts → Claude receives injection payload
- [ ] Verify: injection payload visible in first system message

## Step 10: Atuin Registration

- [ ] `atuin scripts new habitat-init`
- [ ] `atuin scripts new habitat-inject`
- [ ] `atuin scripts new habitat-consolidate`
- [ ] `atuin scripts new habitat-query`

## Step 11: Validation (5 Sessions)

- [ ] Session V1: injection received, orientation quality noted
- [ ] Session V2: consolidation run, patterns reinforced
- [ ] Session V3: trajectory delta visible in injection
- [ ] Session V4: no re-discovered traps from causal_chain
- [ ] Session V5: `habitat-query summary` shows growth

### Acceptance Metrics

| Metric | Target | Actual |
|--------|--------|--------|
| Injection latency | <100ms | |
| Injection size | <2KB | |
| Re-discovered traps | 0 | |
| Patterns reinforced | ≥3 | |
| Patterns pruned | ≥1 | |

---

*Back to: [[EXECUTION_PLAN]] · [[HOME]]*
