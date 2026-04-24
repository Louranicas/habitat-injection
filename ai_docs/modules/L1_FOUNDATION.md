# L1_FOUNDATION

Core types (SessionId, WorkstreamId, ChainId, ConsentLevel, PatternWeight), error taxonomy, config, constants, traits (Injectable, Consolidatable, Queryable), validation. No upward imports.

## Modules

- `m01_types` — Newtypes: SessionId(u32), WorkstreamId(String), ChainId(u64), PatternId(String), ConsentLevel(Emit|Store|Forget), PatternWeight(f64 clamped 0.0-1.0), TokenBudget(u32). All Display + Serialize + Deserialize.
- `m02_errors` — Error taxonomy: InjectionError, ConsolidationError, SchemaError, QueryError, MigrationError. thiserror-derived. #[non_exhaustive] on public enums.
- `m03_config` — Configuration: DB path (~/.local/share/habitat/injection.db), injection budget (default 1100 tokens), decay rate (0.95), reinforce rate (0.1), auto-resolve threshold (10 sessions), cache rebuild interval (60s). TOML + env overlay.
- `m04_traits` — Core traits: Injectable (fn inject(&self, budget: TokenBudget) → Result<String>), Consolidatable (fn consolidate(&mut self, session: SessionId) → Result<()>), Queryable (fn query(&self, sql: &str) → Result<Vec<Row>>), Decayable (fn decay(&mut self, rate: f64) → Result<u32>).
- `m05_constants` — Named constants: DEFAULT_BUDGET=1100, DECAY_RATE=0.95, REINFORCE_RATE=0.1, AUTO_RESOLVE_SESSIONS=10, CACHE_REBUILD_SECS=60, MAX_CHAINS_INJECTED=5, MAX_PATTERNS_INJECTED=10, MAX_TRAJECTORY_POINTS=5, MAX_WORKSTREAMS=10, PRUNE_THRESHOLD=0.05.

See `ai_specs/layers/L1_FOUNDATION_SPEC.md` for implementation details.
