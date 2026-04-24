# THE SUBSTRATE GUARDIAN — Final Position

## The Tables That Must Exist

Three layers, three authorities. The debate proved each layer needs a different governance model.

**Shared Foundation (4 tables, consensus across 5+ experts):**
- `causal_chain` — the Historian's table. `reinforcement_count` surfaces traps that recency misses. Adopted by Practitioner, Performance Engineer, Memory Scientist. Non-negotiable.
- `session_trajectory` — fitness arc. Universal agreement across all experts.
- `active_workstream` — status, blockers, resume context. Prevents orphaned plans.
- `injection_cache` — the Security Architect's Round 3 innovation. Pre-filtered, consent-gated, rebuilt by schedule-table reducer every 60s. One SQL query reads the entire injection.

**Substrate Layer (3 tables, my contribution):**
- `substrate_digest` — each substrate writes its own pre-formatted, relevance-ranked summary. STDB does not score, filter, or inhibit substrate contributions. The substrate is the authority on its own data. Indexed on `relevance_score` for injection query.
- `substrate_registry` (private) — per-substrate plasticity parameters: LTP/LTD rates, consolidation phase, consent level. Only the consolidation reducer reads this.
- `reciprocal_writeback` — post-session queue. When Claude uses a pattern 12 times, the reinforcement signal flows back to the substrate that provided it. This is the feedback loop that prevents STDB from becoming a stale data lake.

**Memory Layer (the Memory Scientist's territory):**
I defer to the Memory Scientist's episodic/semantic/procedural/inhibition tables for STDB-internal memory. The key synthesis from Round 3: **inhibition governs STDB's own memories; relevance governs substrate contributions.** Two orthogonal authorities, neither overrides the other.

All tables: `public = false`, `ConsentLevel`-gated, btree-indexed.

## The CLI Tool

```bash
habitat-inject --session 109 --budget 1500   # pre-session, <100ms
habitat-reciprocate --session 109             # post-session, no time budget
```

Injection: 5 parallel SQL queries → merge with live probes → <2KB prose → atuin KV cache fallback.
Reciprocity: drain writeback queue → check substrate consent → POST reinforcement deltas → record outcomes.

## Core Principle (Defended)

Substrates compute their own digests. Substrates score their own relevance. Substrates receive their own reinforcement. STDB coordinates — it does not colonize.
