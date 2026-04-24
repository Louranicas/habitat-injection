> Back to: [[HOME]]

# T7 — TrapState

**Active trap monitoring.** Implements the S101 Roadmap's `habitat-traps-live` proposal.

## Schema

```rust
#[spacetimedb::table(accessor = trap_state, public)]
pub struct TrapState {
    #[primary_key]
    trap_name: String,          // "cp-alias", "pkill-exit-144", "rm-tsv-only", etc.
    is_active: bool,
    last_checked: spacetimedb::Timestamp,
    last_triggered: Option<spacetimedb::Timestamp>,
    trigger_count: u32,
    description: String,
}
```

## The 18 Known Traps

| Trap | Description |
|------|-------------|
| cp-alias | `cp` aliased to interactive — use `\cp -f` |
| pkill-exit-144 | pkill returns 144 on success in some contexts |
| rm-tsv-only | RM accepts TSV only, never JSON |
| povm-hydrate-broken | `/hydrate` returns summary only |
| bridge-url-prefix | No `http://` in bridge URLs |
| pswarm-port-10002 | Not 10001 |
| synthex-api-health | `/api/health` not `/health` |
| me-port-8180 | Not 8080 |
| zellij-wasm-no-http | `run_command(curl)` only |
| pv2-ipc-socket | Unix socket, not HTTP WS |
| synthex-v2-no-v3 | No `/v3/*` endpoints yet |
| povm-pathways-plural | `/pathways` not `/pathway` |
| unwrap-in-wasm | Panics kill plugin |
| timer-5s-minimum | Sub-1s not proven |
| focus-next-pane | Wraps unpredictably |
| synthex-ws-collision | v1 and v2 both on `:8091` |
| orac-breakers-cascade | Circuit breakers can open |
| pv2-governance-gated | Feature-gated, may not compile |

Powers the ACTIVE TRAPS section of [[Bootstrap Chain — Current vs Target|bootstrap injection]].

---

See: [[T8 — WatcherObservation]] · [[Phase B — Knowledge Graph Migration]]
