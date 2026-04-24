# L5_QUERY_&_BROWSER

Interactive memory browser: preset queries, raw SQL passthrough, fzf integration, formatted output, atuin-compatible scripts engine.

## Modules

- `m19_preset_queries` — Named query presets: trajectory, chains, workstreams, patterns, summary. Dispatcher via `query_preset`. Aligned columns with Unicode separators.
- `m20_raw_query` — Raw SQL passthrough with two-layer safety (keyword pre-filter + `Statement::readonly()`). Structured `QueryOutput` with `format_results` renderer.
- `m21_fzf_browser` — fzf-powered browser with `BrowserConfig` builder, `browse_table` entry point, `simple_filter` fallback, `format_for_fzf` + `build_fzf_args` composability.
- `m21b_scripts_engine` — Atuin-compatible scripts engine. CRUD for `injection_script` table. Template variable substitution (`{{VAR}}`, `{{VAR:-default}}`). Auto-injected vars (`__DB_PATH__`, `__TIMESTAMP__`, `__PANE_ID__`). Subprocess execution with signal-aware exit codes.

See `ai_specs/layers/L5_QUERY_&_BROWSER_SPEC.md` for implementation details.
