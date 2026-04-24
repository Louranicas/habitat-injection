# RUST_CORE_PATTERNS

Patterns used across the habitat-injection codebase.

## Error mapping

All L2 modules use the shared `sqlite_err(e: impl Display) -> SchemaError` helper from `m2_schema::mod`. Avoids per-module helpers or inline closures.

## Row structs

Each table has a corresponding `*Row` struct deriving `Debug, Clone, Serialize, Deserialize`. All fields use Rust-native types (`u32` for session numbers, `Option<T>` for nullable columns). Row parsing is via a private `fn row_to_*(row: &Row) -> rusqlite::Result<*Row>` that uses positional `row.get::<_, T>(index)` with explicit type annotations for non-inferred types.

## Integer conversions

`COUNT(*)` queries return `i64` from SQLite. Convert to `u64` via `i64::cast_unsigned` (Rust 2024 edition). No `unwrap` or `try_from` — `cast_unsigned` is infallible for non-negative counts.

## Hebbian arithmetic

Reinforcement: `weight += 0.1 * (1.0 - weight)` — asymptotic approach to 1.0, never reaches it. Computed in SQL to avoid read-modify-write races. Decay: `weight *= rate` where `last_fired_session IS NOT NULL`. Prune: `DELETE WHERE weight < threshold`.

## Timestamp management

`created_at` and `updated_at` use ISO-8601 TEXT columns with `DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))`. `updated_at` is refreshed in every UPDATE statement via inline `strftime` call.

## Feature gating

The `sqlite` feature gates the `rusqlite` dependency. All L2 modules import `rusqlite` unconditionally — the feature gate operates at the dependency level, not at the module level.
