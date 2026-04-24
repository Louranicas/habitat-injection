# L5_QUERY_&_BROWSER — Implementation Spec

Interactive memory browser: preset queries (trajectory, chains, workstreams, patterns, summary), raw SQL passthrough, fzf integration, formatted output, atuin-compatible scripts engine.

## Rationale

On-demand exploration. The habitat-query tool that lets Claude investigate its own memory during a session.

## Modules

- `m19_preset_queries` — Named query presets: trajectory (last N), chains (unresolved by frequency), workstreams (active+blocked), patterns (top N by weight), summary (one-line counts). Dispatcher via `query_preset`. Each returns formatted text with aligned columns and Unicode separators.
- `m20_raw_query` — Raw SQL passthrough against injection.db. Two-layer safety: (1) first-keyword pre-filter rejects obvious writes, (2) `Statement::readonly()` authoritatively blocks any compiled write statement including `WITH ... INSERT`. Returns `QueryOutput` struct with columns, rows, timing. `format_results` renders aligned tables with footer.
- `m21_fzf_browser` — fzf-powered memory browser. `BrowserConfig` builder with table selection, `--filter`, `--preview`, `--multi`. `browse_table` queries DB + pipes through fzf or falls back to `simple_filter`. `format_for_fzf` + `build_fzf_args` for composability. `is_fzf_available` PATH scan.
- `m21b_scripts_engine` — Atuin-compatible scripts engine backed by `injection_script` table. Full CRUD (`create_script`, `get_script`, `list_scripts`, `delete_script`). Template variable substitution (`{{VAR}}` and `{{VAR:-default}}`). Auto-injected vars: `__DB_PATH__`, `__TIMESTAMP__`, `__PANE_ID__`. Subprocess execution via shebang with captured stdout/stderr/exit_code. Unix signal codes via 128+signal convention.

## Dependencies

Depends on: L1, L2

## Constraints

- should: 50+ tests per module
- must: No `unwrap()`/`expect()` outside tests
- must: Quality gate after every module
