# THE CLI CRAFTSMAN — Argument (Round 2)

## Thesis: The Pipeline IS the Product. STDB Is Just One Source.

Round 1 established that parallel queries, graceful degradation, and pipe-friendly output are non-negotiable. But I pulled my punch on the hardest problem: **STDB is not the only source, and it shouldn't try to be.** The injection pipeline must fuse STDB state (consolidated, possibly stale) with live probes (fresh, possibly partial) in a single parallel fan-out. The CLI tool doesn't query STDB then format — it queries STDB, probes live services, diffs the two, and renders the *merged* view with staleness annotations.

## Concrete Tool: `habitat-inject` (Rust binary, not bash)

Round 1's bash script works for prototyping. Production needs a compiled binary:

```rust
use tokio::time::timeout;
use std::time::Duration;

const STDB_TIMEOUT: Duration = Duration::from_millis(60);
const PROBE_TIMEOUT: Duration = Duration::from_millis(40);

async fn inject(session: u32, budget: u32) -> InjectionResult {
    let (trajectories, semantics, procedures, episodes,
         orac_live, synthex_live, pv2_live, povm_live, me_live, watcher_live,
    ) = tokio::join!(
        stdb_query("SELECT * FROM session_trajectory ORDER BY session_id DESC LIMIT 5"),
        stdb_query("SELECT * FROM semantic_fact WHERE confidence > 0.6 ORDER BY reinforcement_count DESC"),
        stdb_query("SELECT * FROM procedural_pattern ORDER BY reinforcement_count DESC LIMIT 20"),
        stdb_query("SELECT * FROM episodic_trace WHERE decay_weight > 0.3 ORDER BY session_id DESC LIMIT 30"),
        probe("localhost:8133", "/health"),
        probe("localhost:8090", "/api/health"),
        probe("localhost:8132", "/health"),
        probe("localhost:8125", "/health"),
        probe("localhost:8080", "/api/health"),
        probe_watcher(),
    );
    let merged = merge_with_staleness(&semantics, &orac_live, &synthex_live, &pv2_live);
    render_injection(merged, budget)
}
```

## The Merge-and-Annotate Pattern

This is the key innovation over Round 1. STDB says `orac.fitness = 0.664`. Live probe says `0.671`. The injection shows:

```
ORAC: fitness=0.671 (STDB: 0.664, Δ+0.007, 12min stale)
```

Not one or the other — **both**, with delta and staleness age. A 0.007 delta after 12 minutes is normal drift. A 0.200 delta after 2 minutes is a phase transition. Claude gets explicit trust calibration.

## Atuin Script Integration (Revised)

```bash
#!/usr/bin/env bash
# habitat-inject — atuin script entry point, three-tier fallback
SESSION="${1:-$(atuin kv get habitat.session 2>/dev/null || echo 0)}"
BUDGET="${2:-4000}"

# Tier 1: compiled binary (fast path ~25ms)
if command -v habitat-inject &>/dev/null; then
    result=$(habitat-inject --session "$SESSION" --budget "$BUDGET" 2>/dev/null)
    rc=$?
else rc=1; fi

# Tier 2: direct STDB query via spacetime CLI
if [[ $rc -ne 0 ]]; then
    result=$(spacetime sql habitat-db \
        "SELECT * FROM session_trajectory ORDER BY session_id DESC LIMIT 3" \
        --format json 2>/dev/null | jq -r '.[] | "S\(.session_id): fit=\(.ralph_fitness)"')
    [[ -z "$result" ]] && rc=1 || rc=0
fi

# Tier 3: atuin KV cache (always local, sub-1ms)
if [[ $rc -ne 0 ]]; then
    result=$(atuin kv get habitat.last-injection 2>/dev/null)
    [[ -n "$result" ]] && echo "⚠ STDB+binary unreachable — cached injection" >&2
fi

echo "${result:-NO STATE AVAILABLE}"
[[ $rc -eq 0 && -n "$result" ]] && echo "$result" | atuin kv set habitat.last-injection
```

## Parallel Query Timing Budget

```
┌──────────────────────────────────────────────────────┐
│                    100ms BUDGET                       │
├──────────┬──────────┬──────────┬─────────────────────┤
│ STDB×4   │ Probes×6 │  Merge   │     Render          │
│  ≤60ms   │  ≤40ms   │  ≤10ms   │     ≤15ms           │
│(parallel)│(parallel)│ (serial) │    (serial)          │
└──────────┴──────────┴──────────┴─────────────────────┘
  STDB and probes overlap — total wall-clock ~25ms typical, ~85ms worst
```

## Debug Pipeline: `habitat-query`

```bash
# Interactive memory browser — fzf over all 3 memory types
habitat-inject --format json | jq -r '
    .semantics[] | "\(.fact_id)\t\(.domain)\t\(.assertion)\t\(.confidence)"
' | fzf --delimiter='\t' --with-nth=1,3 \
    --preview='habitat-inject --detail {1}' \
    --header="Semantic Facts — ↑↓ browse, Enter inspect"
```

## Why This Round Matters

Round 1 proved the pipeline. This round solves the **trust calibration problem**: stale STDB data vs live probe data. The merge-and-annotate pattern gives Claude explicit staleness so it can decide whether to trust consolidated state or re-probe. The three-tier fallback (binary → spacetime CLI → atuin KV) means injection never fails — it degrades from rich (merged + annotated) to adequate (STDB only) to minimal (cached). The compiled Rust binary brings the hot path from bash's ~55ms to ~25ms, leaving headroom for the Memory Scientist's reconsolidation writes and the Substrate Guardian's consent checks without blowing the 100ms budget.

Schemas are blueprints. Pipelines are buildings. I build buildings.
