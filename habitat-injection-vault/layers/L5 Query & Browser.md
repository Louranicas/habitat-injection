> Back to: [[HOME]] · [[MASTER INDEX]] · [[Architecture Overview]]

# L5 Query & Browser

> **Path:** `src/m5_query/` | **Modules:** 4 | **Dependencies:** [[L1 Foundation]], [[L2 Schema & Persistence]]

Interactive memory browser for on-demand exploration. Preset queries, raw SQL passthrough, fzf integration, and an atuin-compatible scripts engine.

---

## Modules

### m19_preset_queries (`m19_preset_queries.rs`)
Named query presets, each returns formatted text:
- `trajectory` — last 10 session trajectory points
- `chains` — unresolved causal chains by reinforcement frequency
- `workstreams` — active + blocked workstreams
- `patterns` — top 20 patterns by weight
- `checkpoints` — last 5 session checkpoint summaries
- `health` — latest health record per service

### m20_raw_query (`m20_raw_query.rs`)
Arbitrary SQL passthrough against `injection.db`. Opens with `SQLITE_OPEN_READ_ONLY` for safety. Returns header + separator + rows formatted output.

### m21_fzf_browser (`m21_fzf_browser.rs`)
Interactive fzf-powered memory browser. Pipes table contents through `fzf` with `--preview` showing related records. Falls back to non-interactive preset display if fzf is not in PATH.

### m21b_scripts_engine (`m21b_scripts_engine.rs`)
Atuin-compatible scripts engine backed by `injection.db`. Scripts table stores name, description, tags, shebang, body, template variables. Commands: `habitat-scripts new/list/run/get/edit/delete`. Template vars via `{{VAR}}` syntax with automatic `__DB_PATH__` injection. Dual-surface: scripts registered here are also auto-registered with `atuin scripts new`.

---

## Spec
See `ai_specs/layers/L5_QUERY_&_BROWSER_SPEC.md` for implementation details.
