> Back to: [[HOME]] · [[MASTER INDEX]]

# Binary Map

## 5 Binaries

| Binary | Entry Point | Install Path | Trigger | Timeout |
|--------|------------|-------------|---------|---------|
| `habitat-inject` | `src/bin/inject.rs` | `~/.local/bin/habitat-inject` | SessionStart hook | 3s |
| `habitat-consolidate` | `src/bin/consolidate.rs` | `~/.local/bin/habitat-consolidate` | Post-session (manual or hook) | — |
| `habitat-query` | `src/bin/query.rs` | `~/.local/bin/habitat-query` | On-demand CLI | — |
| `habitat-init` | `src/bin/init.rs` | `~/.local/bin/habitat-init` | One-time setup | — |
| `habitat-scripts` | `src/bin/scripts.rs` | `~/.local/bin/habitat-scripts` | On-demand CLI | — |

## Build & Install

```bash
cargo build --release
/usr/bin/cp -f target/release/habitat-inject ~/.local/bin/
/usr/bin/cp -f target/release/habitat-consolidate ~/.local/bin/
/usr/bin/cp -f target/release/habitat-query ~/.local/bin/
/usr/bin/cp -f target/release/habitat-init ~/.local/bin/
/usr/bin/cp -f target/release/habitat-scripts ~/.local/bin/
```

## 4 Bash Scripts

| Script | Path | Atuin Registered | Tags |
|--------|------|-----------------|------|
| `habitat-inject.sh` | `scripts/habitat-inject.sh` | Yes | habitat, injection, bootstrap |
| `habitat-consolidate.sh` | `scripts/habitat-consolidate.sh` | Yes | habitat, consolidation, session |
| `habitat-query.sh` | `scripts/habitat-query.sh` | Yes | habitat, query, memory |
| `habitat-seed.sh` | `scripts/habitat-seed.sh` | Yes | habitat, seed, migration |
