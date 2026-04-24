# L3_INJECTION_ENGINE — Implementation Spec

SessionStart injection pipeline: parallel query executor, prose renderer (<2KB budget), three-tier fallback (SQLite → atuin KV → static), staleness annotation, consent filtering, token counting..

## Rationale

The core deliverable — what Claude Code receives at context window start. Pipeline-first architecture from CLI Craftsman.

## Modules

- `m11_parallel_query` — Parallel query executor: runs 4 SQLite queries + N curl health probes concurrently. Returns structured results with staleness annotations. Thread pool for SQLite (no async needed — it's local file). CLI Craftsman's TC6 pattern.
- `m12_prose_renderer` — Converts structured query results → <2KB prose injection payload. 5 sections: orientation (≤80 tokens), trajectory (≤200), workstreams (≤300), causal chains (≤200), health (≤100). Token counting via whitespace split. Practitioner's format.
- `m13_fallback` — Three-tier fallback chain: Tier 1 SQLite → Tier 2 atuin KV (habitat.last-injection) → Tier 3 static 'NO STATE'. Each tier returns Option<String>. First Some() wins. CLI Craftsman.
- `m14_consent_filter` — Filters all query results by ConsentLevel. Only rows with consent='Emit' pass to renderer. Logs dropped rows (consent='Store'|'Forget') to tracing. Security Architect's contribution.

## Dependencies

Depends on: L1, L2

## Constraints

- should: 50+ tests per module
- must: No `unwrap()`/`expect()` outside tests
- must: Quality gate after every module
