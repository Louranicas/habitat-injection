> Back to: [[HOME]] · [[MASTER INDEX]] · [[Architecture Overview]]

# L1 Foundation

> **Path:** `src/m1_foundation/` | **Modules:** 5 | **Dependencies:** None

The foundation layer defines all core types, error taxonomy, configuration, traits, and constants. Every other layer imports from L1. L1 imports from nothing.

---

## Modules

### m01_types (`m01_types.rs`)
Newtypes with strong typing:
- `SessionId(u32)` — monotonic session counter
- `WorkstreamId(String)` — human-readable workstream label
- `ChainId(u64)` — causal chain identifier
- `PatternId(String)` — pattern name key
- `ConsentLevel` — `Emit | Store | Forget`
- `PatternWeight(f64)` — clamped 0.0-1.0
- `TokenBudget(u32)` — max tokens for injection payload

All implement `Display + Serialize + Deserialize`.

### m02_errors (`m02_errors.rs`)
`thiserror`-derived error enums:
- `InjectionError` — query/render/fallback failures
- `ConsolidationError` — ingest/decay/cache failures
- `SchemaError` — table/migration failures
- `QueryError` — SQL execution failures
- `MigrationError` — STDB migration failures

All `#[non_exhaustive]` for future-proof public API.

### m03_config (`m03_config.rs`)
Configuration with TOML file + env overlay:
- DB path: `~/.local/share/habitat/injection.db`
- Injection budget: 1100 tokens (default)
- Decay rate: 0.95
- Reinforce rate: 0.1
- Auto-resolve threshold: 10 sessions
- Cache rebuild interval: 60s

### m04_traits (`m04_traits.rs`)
Core trait definitions:
- `Injectable` — `fn inject(&self, budget: TokenBudget) -> Result<String>`
- `Consolidatable` — `fn consolidate(&mut self, session: SessionId) -> Result<()>`
- `Queryable` — `fn query(&self, sql: &str) -> Result<Vec<Row>>`
- `Decayable` — `fn decay(&mut self, rate: f64) -> Result<u32>`

### m05_constants (`m05_constants.rs`)
Named constants (no magic numbers):
- `DEFAULT_BUDGET = 1100`
- `DECAY_RATE = 0.95`
- `REINFORCE_RATE = 0.1`
- `AUTO_RESOLVE_SESSIONS = 10`
- `CACHE_REBUILD_SECS = 60`
- `MAX_CHAINS_INJECTED = 5`
- `MAX_PATTERNS_INJECTED = 10`
- `MAX_TRAJECTORY_POINTS = 5`
- `MAX_WORKSTREAMS = 10`
- `PRUNE_THRESHOLD = 0.05`

---

## Spec
See `ai_specs/layers/L1_FOUNDATION_SPEC.md` for implementation details.
