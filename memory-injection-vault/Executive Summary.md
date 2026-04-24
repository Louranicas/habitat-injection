> Back to: [[HOME]]

# Executive Summary

**Goal:** Deploy SpaceTimeDB as a sidecar service in the Habitat, consolidating 21+ fragmented SQLite databases, 3554 POVM pathways, and 6 memory substrates into a single real-time causal memory substrate. At new context window start, Claude Code gets <100ms injection of *complete* Habitat state — trajectory, causation chains, workstream ledger, synergy graph, and active trap state.

## Architecture Decision

**Sidecar (91/100) — not Zellij Plugin (22/100)**

Per S103 ADR-002. The WASM sandbox cannot run the STDB SDK, open sockets, or persist data beyond the terminal session. Every facet favours the sidecar:

| Facet | Sidecar | Plugin |
|---|---|---|
| Data Ingestion | 95 | 10 |
| Query Capability | 92 | 35 |
| Architecture Fit | 95 | 20 |
| Reliability | 93 | 18 |
| Cross-Service Integration | 90 | 15 |
| Resource Efficiency | 70 | 45 |
| Development Velocity | 88 | 30 |

## Shape

- **5 phases**, ~50-60h across 10-12 sessions
- **8 STDB tables** consolidating 5 canonical data patterns
- **6 reducers** including causal chain construction and Hebbian decay
- **Ingester** (multi-source Rust binary) + **Injector** (bootstrap CLI)
- Phase A independently valuable — each phase additive

## Key Insight

The Habitat currently has 21 tracking databases, of which 11 are dead (zero data). The 10 live ones contain 5 distinct data patterns that map cleanly onto STDB tables. The fragmentation is accidental, not architectural — consolidation is net simplification.

## Key Differentiator

`causal_parent: Option<u64>` on every [[T1 — HabitatEvent]] row. Links effect to cause. Makes "why did fitness drop?" answerable from a single query instead of manual session note archaeology.

---

See: [[Sidecar Architecture]] · [[Phase A — STDB Deploy]] · [[Migration Strategy]]
