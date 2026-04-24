# habitat-injection — DATABASE_SPEC

## Database

- **Engine:** SQLite 3 via `rusqlite` (bundled)
- **Location:** `~/.local/share/habitat/injection.db`
- **Schema version:** 3 (tracked via `PRAGMA user_version`)
- **Journal mode:** WAL
- **Busy timeout:** 5000ms
- **Foreign keys:** ON
- **Synchronous:** NORMAL

## Tables (7)

| Table | PK | Row estimate | Write freq | Read freq |
|-------|-----|-------------|------------|-----------|
| `causal_chain` | `id` (autoincrement) | 50–200 | Per consolidation | Per injection |
| `session_trajectory` | `session_id` | ~1 per session | Per consolidation | Per injection |
| `workstream` | `ws_id` (text) | 5–20 active | Per consolidation | Per injection |
| `reinforced_pattern` | `pattern_id` (text) | 50–200 | Per consolidation | Per injection |
| `injection_cache` | `section` (text) | 5 sections | Per consolidation | Per injection |
| `session_checkpoint` | `id` (autoincrement) | ~1 per session | Per /save-session | On-demand query |
| `injection_script` | `id` (UUIDv7 text) | 10–50 | On script create/run | On script list/run |

## Timestamp columns

## Timestamp columns

`causal_chain`, `workstream`, `reinforced_pattern`, and `injection_script` have `created_at` and `updated_at` columns (ISO-8601 TEXT, auto-populated via `strftime` defaults, `updated_at` refreshed on every UPDATE).

## Consent model

All tables except `injection_cache` carry a `consent` column with CHECK constraint: `Emit` (default), `Store`, `Forget`.

## Indexes

| Index | Table | Columns | Condition |
|-------|-------|---------|-----------|
| `idx_causal_unresolved` | `causal_chain` | `reinforcement_count DESC` | `WHERE resolved_session IS NULL` |
| `idx_causal_label` | `causal_chain` | `label` | — |
| `idx_trajectory_recent` | `session_trajectory` | `session_id DESC` | — |
| `idx_workstream_active` | `workstream` | `status` | `WHERE status IN ('active', 'blocked')` |
| `idx_pattern_weight` | `reinforced_pattern` | `weight DESC` | — |
| `idx_checkpoint_label` | `session_checkpoint` | `label` | — |
| `idx_checkpoint_ts` | `session_checkpoint` | `timestamp_utc DESC` | — |
| `idx_script_name` | `injection_script` | `name` | — |
| `idx_script_tags` | `injection_script` | `tags` | — |

## Migration history

| Version | Changes |
|---------|---------|
| 0 → 1 | Initial schema: 6 tables + indexes |
| 1 → 2 | Add `created_at`/`updated_at` to `causal_chain`, `workstream`, `reinforced_pattern` |
| 2 → 3 | Add `injection_script` table (7th table) with name + tags indexes |
