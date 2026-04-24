# ERROR_PATTERNS

## Error taxonomy

Six domain-specific error enums in `m02_errors`, all `#[non_exhaustive]`:

| Enum | Layer | Variants |
|------|-------|----------|
| `SchemaError` | L2 | `DatabaseOpenFailed`, `MigrationFailed`, `TableCreationFailed`, `IndexCreationFailed`, `VersionMismatch`, `Sqlite` |
| `InjectionError` | L3 | `BudgetExhausted`, `QueryFailed`, `AllFallbacksExhausted`, `ConsentBlocked`, `PayloadTooLarge`, `Timeout` |
| `ConsolidationError` | L4 | `TrajectoryCaptureFailed`, `WorkstreamUpdateFailed`, `ChainReinforcementFailed`, `DecayFailed`, `CacheRebuildFailed`, `AtuinWriteFailed`, `NoStaleChains` |
| `QueryError` | L5 | `ExecutionFailed`, `NoResults`, `ParseFailed`, `Timeout`, `FzfFailed`, `RawSqlDisallowed` |
| `MigrationError` | L6 | `ConnectionFailed`, `SourceReadFailed`, `RowCountMismatch`, `ChecksumMismatch`, `DualWriteTransitionFailed`, `ReducerFailed` |
| `ConfigError` | L1 | `FileNotFound`, `ParseFailed`, `InvalidEnvOverride`, `MissingField` |

## Unified wrapper

`HabitatError` wraps all six via `#[error(transparent)]` + `#[from]`. Provides `.kind() -> ErrorKind` for metrics.

## Shared mapping helper

All L2 modules use `m2_schema::sqlite_err(e: impl Display) -> SchemaError` to convert `rusqlite::Error` into `SchemaError::Sqlite`. No per-module helpers.

## Rules

- `unwrap_used = "deny"` and `expect_used = "deny"` enforced crate-wide via `[lints.clippy]`.
- All public functions return `Result<T, SchemaError>` with `# Errors` doc sections.
- All error enums are `Send + Sync` (verified by tests).
- `format_error_chain()` utility for logging nested error chains.
