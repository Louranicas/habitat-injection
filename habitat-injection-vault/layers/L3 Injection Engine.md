> Back to: [[HOME]] · [[MASTER INDEX]] · [[Architecture Overview]]

# L3 Injection Engine

> **Path:** `src/m3_injection/` | **Modules:** 4 | **Dependencies:** [[L1 Foundation]], [[L2 Schema & Persistence]]

The core deliverable: SessionStart injection pipeline that delivers <2KB of causal state into the context window in <100ms.

---

## Pipeline

```
SessionStart hook fires
  -> m14_consent_filter (only Emit-consented data passes)
  -> m11_parallel_query (4 SQLite queries + N health probes)
  -> m12_prose_renderer (structured results -> <2KB prose)
  -> m13_fallback (SQLite -> atuin KV -> static if needed)
  -> output: system message injected into context window
```

## Token Budget

| Section | Max Tokens |
|---------|-----------|
| Orientation | 80 |
| Trajectory | 200 |
| Workstreams | 300 |
| Causal chains | 200 |
| Health | 100 |
| **Total** | **≤1100** (configurable via m03_config) |

---

## Modules

### m11_parallel_query (`m11_parallel_query.rs`)
Runs 4 SQLite queries + N curl health probes concurrently. Thread pool for SQLite (local file, no async needed). Returns structured results with staleness annotations.

### m12_prose_renderer (`m12_prose_renderer.rs`)
Converts structured query results into <2KB prose. 5 sections with per-section token budgets. Token counting via whitespace split.

### m13_fallback (`m13_fallback.rs`)
Three-tier fallback chain:
1. **Tier 1:** SQLite query (primary)
2. **Tier 2:** atuin KV `habitat.last-injection` (stale but available)
3. **Tier 3:** Static "NO STATE" (never fails)

First `Some()` wins.

### m14_consent_filter (`m14_consent_filter.rs`)
Filters all query results by ConsentLevel. Only rows with `consent = 'Emit'` pass to the renderer. Rows with `Store` or `Forget` are logged via tracing and dropped.

---

## Spec
See `ai_specs/layers/L3_INJECTION_ENGINE_SPEC.md` for implementation details.
