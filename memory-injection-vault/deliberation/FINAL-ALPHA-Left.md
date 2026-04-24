# THE MEMORY SCIENTIST — Final Position

## The 7 Tables That Must Exist

The debate converged on a three-layer architecture. Here is my final schema — every table earned its place across 4 rounds of cross-examination.

**Layer 1 — Memory (STDB-governed, inhibition-filtered):**
- `episodic_trace` — timestamped events with `decay_weight`, `retrieval_count`, causal links. Hebbian: accessed more = stronger trace. Ebbinghaus: unaccessed traces decay to zero and get pruned automatically.
- `semantic_fact` — durable assertions with `reinforcement_count`, `supersedes` chain. Facts don't decay — they're confirmed or contradicted.
- `procedural_pattern` — trigger/action/anti-action triples with Hebbian weighting. The "muscle memory" Claude reaches for without thinking.
- `inhibition_edge` — suppresses stale STDB memories only (not substrate data). The Substrate Guardian convinced me: substrates govern their own relevance. Inhibition governs mine.

**Layer 2 — Continuity (consensus tables):**
- `session_trajectory` — fitness arc across sessions. The "WHERE was I" answer.
- `causal_chain` — the Historian's table, adopted by 5 experts. `reinforcement_count` surfaces recurring traps that recency-based filtering misses. S071 is the proof.

**Layer 3 — Substrate (substrate-governed):**
- `substrate_digest` — the Substrate Guardian's contribution. Pre-computed, relevance-ranked, <200 chars per substrate. Substrates score their own relevance; STDB doesn't override.

All tables: `public = false`, `consent: ConsentLevel`, btree-indexed on query columns.

## The CLI Tool

```bash
habitat-inject --session 109 --budget 1500
```

A compiled Rust binary. 5 parallel `spacetime sql` queries (<15ms). Renders <2KB prose. Three-tier fallback: binary → spacetime CLI → atuin KV cache. Post-session: `habitat-consolidate` increments retrieval counts, applies decay, processes inhibition edges, queues reciprocal write-backs to substrates.

## What I Learned

The Practitioner taught me: inject the activation pattern, not the data. The Substrate Guardian taught me: inhibition stops at the substrate boundary. The Performance Engineer taught me: reducers can't return data. The Adversary taught me: earn your tables. Seven tables. Each one answers a question no other mechanism can. That's the schema.
