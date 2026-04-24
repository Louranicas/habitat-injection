> Back to: [[HOME]] · [[MASTER INDEX]] · [[Sidecar Architecture]] · [[DEPLOYMENT FRAMEWORK]]

# Proposed Directory Tree

Three separate workspaces per [[Gap Analysis — Conventional#C2]]. STDB module compiles to WASM; ingester and injector are native Rust binaries.

---

## Complete Tree

```
memory-injection/
│
├── PLAN.md                                    # Canonical plan (911L)
├── GAP_ANALYSIS.md                            # Conventional gaps (244L)
├── NA_GAP_ANALYSIS.md                         # NA gaps (252L)
├── CLAUDE.md                                  # Project-level instructions + traps
├── CLAUDE.local.md                            # Session state + resume instructions
├── README.md                                  # Quick-start for new contributors
│
├── memory-injection-vault/                    # Obsidian vault (45 notes, 145KB)
│   ├── HOME.md
│   ├── MASTER INDEX.md
│   ├── DEPLOYMENT FRAMEWORK.md
│   ├── *.md                                   # (see vault structure below)
│   ├── schematics/
│   ├── schemas/
│   ├── phases/
│   ├── gaps/
│   └── architecture/
│
│   ┌─────────────────────────────────────────────────────────────────┐
│   │  WORKSPACE 1: STDB Module (compiles to WASM)                   │
│   └─────────────────────────────────────────────────────────────────┘
│
├── module/
│   ├── Cargo.toml                             # [lib] crate-type = ["cdylib"]
│   │                                          # spacetimedb = "2.1"
│   │                                          # target: wasm32-unknown-unknown
│   ├── rust-toolchain.toml                    # stable + wasm32-unknown-unknown target
│   ├── .cargo/
│   │   └── config.toml                        # [build] target = "wasm32-unknown-unknown"
│   │
│   └── src/
│       ├── lib.rs                             # Module entry point
│       │                                      #   - #[spacetimedb::table] re-exports
│       │                                      #   - init reducer (seeds DecaySchedule + GradientSchedule)
│       │                                      #   - client_connected / client_disconnected
│       │
│       ├── tables/
│       │   ├── mod.rs                         # Re-exports all 8+1 tables
│       │   ├── habitat_event.rs               # T1 — causal event log + causal_parent
│       │   ├── knowledge_edge.rs              # T2 — unified weighted graph + NA-R1 per-edge params
│       │   ├── gradient_snapshot.rs            # T3 — time-series vital signs + NA-R6 self-reports
│       │   ├── session_record.rs              # T4 — Claude Code session lifecycle
│       │   ├── workstream.rs                  # T5 — in-flight work ledger
│       │   ├── service_health.rs              # T6 — service health timeline
│       │   ├── trap_state.rs                  # T7 — 18 active trap monitors
│       │   ├── watcher_observation.rs         # T8 — Watcher anomaly records
│       │   └── service_session.rs             # T9 — service lifecycle (NA-R5, proposed)
│       │
│       ├── reducers/
│       │   ├── mod.rs                         # Re-exports all reducers
│       │   ├── ingest.rs                      # R1 ingest_event — primary write path
│       │   │                                  #   consent gate (NA-R2)
│       │   │                                  #   severity ≥ 7 → trigger watcher observation
│       │   ├── reinforce.rs                   # R2 reinforce_edge — pattern reinforcement
│       │   │                                  #   creates edge if absent
│       │   │                                  #   increments reinforcement_count
│       │   ├── gradient.rs                    # R3 capture_gradient — scheduled every 60s
│       │   ├── session.rs                     # R4 register_session / close_session
│       │   ├── decay.rs                       # R5 run_decay — per-edge decay (NA-R1)
│       │   │                                  #   reads decay_rate per edge, not global constant
│       │   │                                  #   Ember-gate: skip Watcher-referenced edges (NA-R4)
│       │   ├── forget.rs                      # R6 forget_sphere — NA-P-13 cascade
│       │   │                                  #   redacts T1 + deletes T2 + scrubs T3
│       │   │                                  #   preserves forget event for causal trace
│       │   ├── compact.rs                     # R7 compact_old_events — retention policy
│       │   │                                  #   30d: strip payload → envelope only
│       │   │                                  #   90d: delete entirely
│       │   │                                  #   gradient downsample: 7d→hourly, 30d→daily
│       │   ├── consolidate.rs                 # R8 consolidate_mature_edges — POVM rhythm
│       │   │                                  #   300-tick interval for povm-origin edges only
│       │   ├── watcher_reinforce.rs           # R9 — Watcher overrides decay on important edges
│       │   └── watcher_annotate.rs            # R10 — Watcher annotates any HabitatEvent
│       │
│       ├── schedules/
│       │   ├── mod.rs
│       │   ├── decay_schedule.rs              # ScheduleAt::interval(6h) → R5
│       │   ├── compact_schedule.rs            # ScheduleAt::interval(24h) → R7
│       │   ├── gradient_schedule.rs           # ScheduleAt::interval(60s) → R3
│       │   └── consolidate_schedule.rs        # ScheduleAt::interval(300 ticks) → R8
│       │
│       └── types.rs                           # Shared newtypes: SphereId, ServiceId, Tick,
│                                              #   SessionId, EventId, ConsentState
│
│   ┌─────────────────────────────────────────────────────────────────┐
│   │  WORKSPACE 2: Ingester (native Rust binary)                     │
│   └─────────────────────────────────────────────────────────────────┘
│
├── ingester/
│   ├── Cargo.toml                             # spacetimedb-sdk, tokio, reqwest,
│   │                                          # tungstenite, serde_json, tracing
│   ├── .cargo/
│   │   └── config.toml                        # [build] target = native (default)
│   │
│   ├── src/
│   │   ├── main.rs                            # Tokio runtime, graceful shutdown,
│   │   │                                      # health server on :3001, KV writer
│   │   │
│   │   ├── config.rs                          # STDB_URL, poll intervals, feature flags
│   │   │                                      # Reads from env or ~/.config/habitat/stdb.toml
│   │   │
│   │   ├── stdb_client.rs                     # SpaceTimeDB SDK connection manager
│   │   │                                      # Reconnect logic, circuit breaker
│   │   │
│   │   ├── bridges/
│   │   │   ├── mod.rs
│   │   │   ├── orac.rs                        # Polls :8133 /health, /emergence, /ralph,
│   │   │   │                                  # /coupling, /thermal every 30s
│   │   │   │                                  # Calls R1 ingest_event + R3 capture_gradient
│   │   │   │                                  # Assigns causal_parent via Rule 1 (triggered_by_tick)
│   │   │   │
│   │   │   ├── pv2.rs                         # WebSocket to :8132/bus/ws
│   │   │   │                                  # client_id = "habitat-stdb-ingester"
│   │   │   │                                  # subscribe: ["emergence.*","sphere.*","field.*","command.*"]
│   │   │   │                                  # Calls R1 per event
│   │   │   │                                  # Assigns causal_parent via Rules 2,4
│   │   │   │
│   │   │   ├── synthex.rs                     # Polls :8090 /v3/thermal every 60s
│   │   │   │                                  # Calls R3 capture_gradient
│   │   │   │                                  # Assigns causal_parent via Rule 3 (threshold crossing)
│   │   │   │
│   │   │   ├── povm.rs                        # Polls :8125 /pathways every 300s
│   │   │   │                                  # Diffs weights vs last poll
│   │   │   │                                  # Calls R2 reinforce_edge for changed pathways
│   │   │   │
│   │   │   └── atuin.rs                       # Receives command.* events via PV2 /bus/ws
│   │   │                                      # Assigns causal_parent via Rule 4 (preexec→postexec)
│   │   │
│   │   ├── consent.rs                         # NA-R2: checks ORAC /consent/{sphere_id}
│   │   │                                      # before calling R1. Cache with 60s TTL.
│   │   │                                      # "full" → ingest verbatim
│   │   │                                      # "minimal" → redact sphere_id
│   │   │                                      # "none" → drop silently, increment counter
│   │   │
│   │   ├── causal.rs                          # Causal parent assignment engine
│   │   │                                      # Implements 5 linkage rules from C4
│   │   │                                      # Maintains in-memory tick→event_id index
│   │   │                                      # for fast causal_parent lookups
│   │   │
│   │   ├── reciprocal/                        # NA-R3: data flows BACK to sources
│   │   │   ├── mod.rs
│   │   │   ├── orac_trajectory.rs             # Queries STDB for fitness Δ across sessions
│   │   │   │                                  # POSTs trajectory hints to ORAC /api/ingest
│   │   │   │                                  # Every 300s
│   │   │   ├── synthex_patterns.rs            # Queries STDB for cross-session thermal patterns
│   │   │   │                                  # POSTs to SYNTHEX /api/ingest
│   │   │   │                                  # Every 600s
│   │   │   └── pv2_coupling.rs                # Queries STDB for historical coupling effectiveness
│   │   │                                      # POSTs to PV2 /bus/events
│   │   │                                      # Every 600s
│   │   │
│   │   ├── health.rs                          # axum server on :3001
│   │   │                                      # GET /health → {"status":"ok","lag_ms":12}
│   │   │                                      # GET /metrics → prometheus text format
│   │   │                                      #   ingester_events_total
│   │   │                                      #   ingester_events_dropped_consent
│   │   │                                      #   ingester_stdb_latency_ms
│   │   │                                      #   ingester_source_last_poll_ms{source="orac"}
│   │   │                                      #   ingester_causal_links_assigned
│   │   │                                      #   ingester_reciprocal_posts_total
│   │   │
│   │   └── kv_writer.rs                       # TC10: writes stdb.* keys to atuin KV
│   │                                          # every 60s, alongside R3 gradient capture
│   │                                          # stdb.events.count, stdb.edges.count,
│   │                                          # stdb.last.fitness, stdb.last.grade,
│   │                                          # stdb.ingester.lag_ms
│   │
│   └── tests/
│       ├── integration/
│       │   ├── orac_bridge_test.rs            # Mock ORAC → verify events land in STDB
│       │   ├── pv2_bridge_test.rs             # Mock PV2 WS → verify events + causal links
│       │   ├── consent_test.rs                # Verify consent gate drops/redacts correctly
│       │   ├── causal_test.rs                 # Verify 5 linkage rules produce correct parents
│       │   └── reconnect_test.rs              # Kill STDB → verify reconnect + zero event loss
│       └── fixtures/
│           ├── orac_health.json
│           ├── orac_emergence.json
│           ├── pv2_bus_event.json
│           └── synthex_thermal.json
│
│   ┌─────────────────────────────────────────────────────────────────┐
│   │  WORKSPACE 3: Injector CLI (native shell script)                │
│   └─────────────────────────────────────────────────────────────────┘
│
├── injector/
│   ├── habitat-stdb-inject.sh                 # The SessionStart hook script
│   │                                          # 7× spacetime sql (parallel) → python3 format
│   │                                          # TC6 chain: fan-out → funnel → stdout
│   │                                          # ≤15KB output, <100ms latency
│   │                                          # Role-adaptive payload (NA-R8)
│   │
│   ├── habitat-stdb-query.sh                  # Ad-hoc query wrapper (atuin script)
│   │                                          # Presets: trajectory, patterns, causal, workstreams
│   │
│   ├── habitat-stdb-health.sh                 # STDB + ingester health check (atuin script)
│   │                                          # Table row counts, ingester metrics
│   │
│   ├── habitat-stdb-migrate.sh                # One-shot migration trigger
│   │                                          # Orchestrates povm_migrator + sqlite_migrator
│   │                                          # Runs verification checksums
│   │
│   └── tests/
│       ├── inject_test.bats                   # bats tests for injector
│       │                                      # Verify ≤15KB, <100ms, sections present
│       ├── query_test.bats                    # bats tests for query presets
│       └── health_test.bats                   # bats tests for health check
│
│   ┌─────────────────────────────────────────────────────────────────┐
│   │  WORKSPACE 4: Migration Scripts                                 │
│   └─────────────────────────────────────────────────────────────────┘
│
├── migration/
│   ├── povm_migrator.rs                       # One-shot: reads POVM /pathways
│   │                                          # → calls R2 reinforce_edge per pathway
│   │                                          # Preserves namespace, co_activations,
│   │                                          # per-edge learning params (NA-R1)
│   │                                          # Verification: count + weight aggregate checksum
│   │
│   ├── sqlite_migrator.rs                     # One-shot: reads 10 live SQLite DBs
│   │                                          # → maps 5 patterns to STDB tables
│   │                                          # Per-source checksum verification (C5)
│   │                                          # Consent check per sphere (NA-R2)
│   │
│   ├── rm_migrator.rs                         # One-shot: reads RM /search → T3
│   │                                          # TSV parse (TRAP: never JSON)
│   │                                          # ~2000 heartbeat entries → gradient snapshots
│   │
│   ├── verify_checksums.sh                    # Post-migration verification
│   │                                          # Compares source COUNT/SUM/AVG vs STDB
│   │                                          # Tolerance: ±0.01 on weight aggregates
│   │                                          # Exit 1 on mismatch → abort
│   │
│   └── Cargo.toml                             # Standalone binary for Rust migrators
│                                              # spacetimedb-sdk, rusqlite, reqwest
│
│   ┌─────────────────────────────────────────────────────────────────┐
│   │  Runtime Data + Config                                          │
│   └─────────────────────────────────────────────────────────────────┘
│
├── data/                                      # STDB runtime data directory
│   ├── .gitkeep                               # Track dir, not contents
│   └── (WAL files, snapshots — .gitignored)
│
├── config/
│   ├── stdb.toml                              # Ingester config
│   │                                          # [stdb]
│   │                                          # url = "http://127.0.0.1:3000"
│   │                                          # database = "habitat"
│   │                                          #
│   │                                          # [sources.orac]
│   │                                          # url = "http://127.0.0.1:8133"
│   │                                          # poll_interval_secs = 30
│   │                                          # endpoints = ["/health","/emergence","/ralph","/coupling"]
│   │                                          #
│   │                                          # [sources.pv2]
│   │                                          # ws_url = "ws://127.0.0.1:8132/bus/ws"
│   │                                          # client_id = "habitat-stdb-ingester"
│   │                                          # subscribe = ["emergence.*","sphere.*","field.*","command.*"]
│   │                                          #
│   │                                          # [sources.synthex]
│   │                                          # url = "http://127.0.0.1:8090"
│   │                                          # poll_interval_secs = 60
│   │                                          #
│   │                                          # [sources.povm]
│   │                                          # url = "http://127.0.0.1:8125"
│   │                                          # poll_interval_secs = 300
│   │                                          #
│   │                                          # [retention]
│   │                                          # event_full_days = 30
│   │                                          # event_envelope_days = 90
│   │                                          # gradient_full_days = 7
│   │                                          # gradient_hourly_days = 30
│   │                                          #
│   │                                          # [reciprocal]
│   │                                          # enabled = true
│   │                                          # orac_interval_secs = 300
│   │                                          # synthex_interval_secs = 600
│   │                                          # pv2_interval_secs = 600
│   │
│   └── devenv-stdb.toml                       # devenv.toml snippet for both services
│                                              # habitat-stdb (:3000, Batch 1)
│                                              # habitat-stdb-ingester (:3001, Batch 2)
│
│   ┌─────────────────────────────────────────────────────────────────┐
│   │  Scripts + CI                                                   │
│   └─────────────────────────────────────────────────────────────────┘
│
├── scripts/
│   ├── deploy.sh                              # Full deploy cycle:
│   │                                          #   1. Build module (cargo build --target wasm32)
│   │                                          #   2. spacetime publish habitat
│   │                                          #   3. Build ingester (cargo build --release)
│   │                                          #   4. \cp -f ingester binary → ~/.local/bin/
│   │                                          #   5. \cp -f injector scripts → ~/.local/bin/
│   │                                          #   6. devenv restart habitat-stdb habitat-stdb-ingester
│   │                                          #   7. Verify: spacetime sql habitat "SELECT COUNT(*) FROM habitat_event"
│   │
│   ├── install-hooks.sh                       # Update ~/.claude/settings.json
│   │                                          # Replace habitat-bootstrap with habitat-stdb-inject
│   │                                          # Preserve legacy as habitat-bootstrap-legacy
│   │
│   ├── register-atuin-scripts.sh              # Register 4 atuin scripts:
│   │                                          #   habitat-stdb-inject
│   │                                          #   habitat-stdb-query
│   │                                          #   habitat-stdb-health
│   │                                          #   habitat-stdb-migrate
│   │
│   ├── verify-e2e.sh                          # End-to-end verification:
│   │                                          #   TC9 chain: inject → size check → section check → latency check
│   │                                          #   Full round-trip: echo cmd → atuin → PV2 → ingester → STDB → query
│   │                                          #   Causal chain: verify causal_parent populated
│   │                                          #   Forget cascade: create test sphere → forget → verify zero rows
│   │
│   └── benchmark.sh                           # Injection latency benchmark
│                                              #   100 runs of habitat-stdb-inject > /dev/null
│                                              #   Report p50, p95, p99 latency
│                                              #   Assert p95 < 100ms
│
├── .gitignore                                 # data/, target/, *.wasm, *.db.pre-stdb
└── .claude/
    └── settings.json                          # Project-level Claude Code settings
                                               # Allow: spacetime, cargo build --target wasm32
```

## File Count Summary

| Directory | Files | Purpose |
|-----------|-------|---------|
| `module/src/` | 18 | STDB WASM module (tables + reducers + schedules + types) |
| `ingester/src/` | 14 | Native ingester binary (bridges + consent + causal + reciprocal + health) |
| `ingester/tests/` | 6 | Integration tests + fixtures |
| `injector/` | 7 | Shell scripts + bats tests |
| `migration/` | 5 | One-shot migrators + verification |
| `config/` | 2 | Runtime config |
| `scripts/` | 5 | Deploy, hooks, atuin, e2e, benchmark |
| `memory-injection-vault/` | 45 | Obsidian vault |
| Root | 7 | Plan docs + CLAUDE.md + README |
| **Total** | **~109** | |

## Build Targets

| Workspace | Target | Binary | Deploy To |
|-----------|--------|--------|-----------|
| `module/` | `wasm32-unknown-unknown` | `habitat_stdb_module.wasm` | `spacetime publish habitat` |
| `ingester/` | native (x86_64-linux) | `habitat-stdb-ingester` | `~/.local/bin/` + devenv |
| `migration/` | native (x86_64-linux) | `habitat-stdb-migrate` | `~/.local/bin/` (one-shot) |
| `injector/` | bash (no compile) | `habitat-stdb-inject` | `~/.local/bin/` + atuin scripts |

## Cargo.toml Dependencies

| Workspace | Key Dependencies |
|-----------|-----------------|
| `module/` | `spacetimedb = "2.1"` |
| `ingester/` | `spacetimedb-sdk`, `tokio = {features=["full"]}`, `reqwest = {features=["json","rustls-tls"]}`, `tokio-tungstenite`, `axum = "0.8"`, `serde_json`, `tracing`, `tracing-subscriber` |
| `migration/` | `spacetimedb-sdk`, `rusqlite = {features=["bundled"]}`, `reqwest`, `serde_json` |

---

See: [[Sidecar Architecture]] · [[Phase A — STDB Deploy]] · [[DEPLOYMENT FRAMEWORK]]
