> Back to: [[HOME]] · [[Execution Plan]] · [[Deployment Checklist]]

# CLI Binary Architecture

Canonical Mermaid diagrams at `schematics/CLI_BINARY_ARCHITECTURE.md` (project root).

## Binary → Library Dependency Map

| Binary | L1 | L2 | L3 | L4 | L5 | L6 |
|--------|----|----|----|----|----|----|
| `habitat-init` | config | schema | — | — | — | — |
| `habitat-inject` | config, types | schema | all 4 | cache_builder, atuin | — | — |
| `habitat-consolidate` | config, types | schema | — | all 5 | — | — |
| `habitat-query` | config | schema | — | — | all 4 | — |
| `habitat-seed` | config | schema, CRUD | — | — | — | — |

## Key Design Decisions

1. **`habitat-inject` exits 0 always** — never blocks session start
2. **`habitat-consolidate` uses transactions** — 4-step Hebbian cycle is atomic
3. **`habitat-query` falls back to non-interactive** — works without fzf
4. **`habitat-seed` is idempotent** — safe to re-run

---

See: [[Execution Plan]] · [[Implementation Status]]
