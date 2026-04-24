# THE SYNTHESIZER — Final Verdict

## What This Circle Produced

Nine experts. Four rounds. ~30 proposed tables. Three existential challenges. One convergence.

The debate began as a schema design exercise and became a sequencing argument. The Adversary forced this — and the Adversary was right to. The question was never "what's the ideal STDB schema?" It was "what ships first, and what earns its place later?"

## The Verdict: 5 Tables, SQLite-First, Pipeline-Permanent

**Five data tables** survived cross-examination by all experts across four rounds:

| Table | Origin | Adoption | Why it survived |
|-------|--------|----------|-----------------|
| `causal_chain` | Historian | 6/9 | The single most adopted schema element. S071's 7x convergence trap is the proof that frequency-ranked surfacing catches what grep cannot. No expert rebutted it. |
| `session_trajectory` | Practitioner | Universal | Fitness arc with interpreted deltas. Every expert proposed a variant. |
| `workstream` | Historian + Practitioner | 5/9 | Prevents orphaned plans. Phase G blocker would be re-planned without this row. |
| `reinforced_pattern` | Perf Engineer + Memory Scientist | 4/9 | Unified episodic/semantic/procedural into one table with category + weight. The CLI Craftsman's FINAL makes this load-bearing: without decay and reinforcement, the other tables are filing cabinets. |
| `injection_cache` | Security Architect | 4/9 | Pre-filtered, consent-gated, token-capped. Solves consent enforcement without the impossible single-reducer. Optional optimization — injection works without it by querying the 4 data tables directly. |

**SQLite Phase 1.** The Adversary, Practitioner, Historian, CLI Craftsman, and Performance Engineer all converged on this. STDB is Phase 2, triggered only when the Watcher needs real-time subscriptions or `cascade_forget` transactional deletion is required.

**The pipeline is permanent.** The CLI Craftsman's architecture — parallel fan-out, merge-with-staleness, three-tier fallback (SQLite → live probes → atuin KV) — is backend-agnostic. It works with bash today, SQLite tomorrow, STDB next month. The pipeline doesn't change when the backend does.

## What Was Deferred (Phase 2+)

The Memory Scientist's `inhibition_edge` (when `resolved_session IS NULL` proves too coarse, ~session 500). The Substrate Guardian's `substrate_digest` and `reciprocal_writeback` (when substrates have consent endpoints). The Watcher's `watcher_observation` + `watcher_hypothesis` + `ember_gate_log` (when synthex-v2 daemon is live). All architecturally sound, all temporally premature.

## What Was Killed

Single-reducer injection (architecturally impossible — reducers return `()`). Public tables (zero-cost privacy). Raw JSON output (prose is better). The Memory Scientist's 3-way episodic/semantic/procedural split (unified into `reinforced_pattern`). 15+ table schemas (five is enough to prove the concept).

## The One Query That Justifies Everything

```sql
SELECT label, reinforcement_count, description
FROM causal_chain
WHERE resolved_session IS NULL
ORDER BY reinforcement_count DESC
LIMIT 5;
```

This query surfaces what the habitat keeps rediscovering. It doesn't require knowing the search term. It doesn't require reading 108 session docs. It doesn't require a human to curate. It is the structural antidote to amnesia, and it is the reason this circle convened.

Ship it this week. Prove it over 20 sessions. Earn the rest.
