> Back to: [[HOME]]

# Sidecar Architecture

## devenv.toml Registration

- **Service ID:** `habitat-stdb`
- **Port:** `:3000` (STDB default)
- **Batch:** 1 (no dependencies — other services depend on STDB)
- **Health check:** `http://localhost:3000/v1/identity`
- **Resource limits:** 1024 MB RAM, 50% CPU
- **Auto-restart:** yes, max 5 attempts, 5s delay

## Project Structure (per [[Gap Analysis — Conventional#C2]] — split workspaces)

```
habitat-stdb-module/      # WASM module → spacetime publish
  └── src/lib.rs           # Tables T1-T8, Reducers R1-R6, Views

habitat-stdb-ingester/    # Native Rust binary → ~/.local/bin/
  └── src/
      ├── main.rs          # Tokio runtime
      ├── orac_bridge.rs   # Polls ORAC every 30s
      ├── pv2_bridge.rs    # Subscribes PV2 /bus/ws
      ├── synthex_bridge.rs
      ├── povm_migrator.rs # One-shot POVM migration
      ├── sqlite_migrator.rs
      └── atuin_bridge.rs

habitat-stdb-injector/    # Native CLI → ~/.local/bin/
  └── src/main.rs          # spacetime sql → format → stdout
```

Three separate workspaces because STDB modules compile to WASM; ingester/injector need native tokio/reqwest.

## Port Map

| Port | Service | Purpose |
|------|---------|---------|
| `:3000` | STDB standalone | WebSocket subscriptions + SQL + publish |
| `:3001` | Ingester health | `/health` + `/metrics` (per [[Gap Analysis — Conventional#I5]]) |

---

See: [[Ingester Pipeline]] · [[Injector — Context Window Bootstrap]] · [[Phase A — STDB Deploy]]
