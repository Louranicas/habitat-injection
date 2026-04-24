# ROUND 3: THE PERFORMANCE ENGINEER

## What Changed in the Debate

Three things reshaped the landscape in Round 2:

1. **The Adversary entered and asked the only question that matters:** "Name one session that failed because data wasn't in SpaceTimeDB. Not 'could theoretically fail' — actually failed." This reframes every schema proposal from "is this well-designed?" to "does this justify its operational cost?"

2. **The Security Architect's reducer architecture was proven impossible** — by me. But instead of abandoning the security thesis, the circle is converging on a new pattern: **write-time consent gates** and **pre-computed cache tables**. My own rebuttal forced the Security Architect toward a design that's actually *faster* than my Round 1: one `spacetime sql` query against an `injection_cache` table populated by a scheduled reducer.

3. **Six of eight experts now agree on <2KB injection output.** The Practitioner's budget won. The debate has shifted from "what tables do we need?" to "what goes into the cache that produces those 2KB?"

## Where Consensus Has Formed

| Principle | Round 1 Support | Round 2 Support | Trend |
|-----------|----------------|-----------------|-------|
| Private tables | 1 (SecArch) | 5 (SecArch, PE, SubGuard, Watcher partial, MemSci implicit) | Converging |
| ConsentLevel column | 1 (SecArch) | 6 (all except Adversary, Watcher) | **Settled** |
| CausalChain + reinforcement_count | 1 (Historian) | 5 (Historian, Practitioner, PE, MemSci, Watcher variant) | **Settled** |
| <2KB injection output | 1 (Practitioner) | 6 (all except PE, Adversary) | **Settled** — I concede below |
| Write-time pre-computation | 1 (PE) | 5 (PE, SubGuard digest, Watcher digest, SecArch cache, CLI render) | **Settled** |
| SQL as only read path | 0 (assumed) | 8 (proven by PE, no alternative exists) | **Proven** |
| STDB justified at all | 7 (assumed) | 6 (Adversary challenges) | **Contested** |

## Concession: The Practitioner's Output Budget Was Right

I optimized for query speed at the expense of output shape. My Round 1 injected raw query results — 7 parallel SQL outputs piped through Python. The total payload was unbounded because I controlled latency but not output size.

The Practitioner's <2KB budget is correct. An LLM context window is a finite resource. Injecting 18KB of structured data that Claude "skims" is worse than injecting 1.5KB of oriented prose that Claude reads completely. The Practitioner lived through 108 wake-ups and knows what works. I was measuring microseconds when I should have been measuring tokens.

**I adopt the output cap.** My revised pipeline produces <2KB regardless of how much data is in STDB.

## The Adversary Deserves a Real Answer

The Adversary asks for a concrete session failure that STDB would have prevented. Here's my answer — not as a schema architect, but as a performance engineer who measures systems:

**The failure is not catastrophic. It is chronic.** The current `habitat-bootstrap` script takes ~55ms and produces ~18KB. Of that 18KB, the Practitioner estimates 80% is noise. That means Claude spends its first 500ms of cognition parsing 14.4KB of irrelevant context to find 3.6KB of orientation. Across 108 sessions, that's ~1.5MB of noise processed. The failure isn't "session X crashed." The failure is "every session starts slower than it should because the injection was designed for storage, not retrieval."

The Adversary's 50-line bash alternative replicates what `habitat-bootstrap` already does — it doesn't solve the curation problem, it just rewrites the same unstructured pipeline. The STDB value proposition is not "store the data" (bash already does that). It is:

1. **Pre-computed, indexed, write-time-curated data** — the scheduled reducer does the work once per minute, not once per injection
2. **Transactional updates** — when a workstream status changes from "active" to "shipped," it changes in one place, atomically
3. **Subscription-based cache invalidation** — the injection cache updates when data changes, not on a polling interval

The Adversary is right that bash can *read* data. Bash cannot *curate* data without reimplementing a database poorly. The Adversary's `grep -l "docker prune" ~/.claude/projects/*/memory/` works today. It will not work when there are 500 memory files across 12 projects. The btree index I put on `reinforced_pattern.weight` does.

## Rebuttal to THE SECURITY ARCHITECT's Round 3 (anticipated injection_cache pattern)

The Security Architect will likely pivot to a "cache table" approach: a reducer pre-computes the consent-filtered payload into a single table, and the CLI reads it with one SQL query. This is actually a good design — and it's the convergence point between my query-shaped approach and their consent-gated approach.

But I have a concern: **cache staleness.** If the scheduled reducer fires every 60 seconds, the injection cache is up to 60 seconds stale. For session trajectory data (changes once per session), 60 seconds is fine. For service health (changes every second), 60 seconds means the injection tells you "11/11 healthy" when ORAC crashed 45 seconds ago.

The fix is **tiered freshness**: cached STDB data for slow-moving state (trajectory, patterns, traps, workstreams), live probes for fast-moving state (health, thermal, field). This is what the CLI Craftsman proposed as "merge-and-annotate" — and it's right. The injection pipeline should be:

```bash
# Tier 1: STDB cache (pre-computed, consent-gated, <5ms)
CACHED=$(spacetime sql habitat \
  "SELECT section, payload FROM injection_cache ORDER BY section" \
  --format json 2>/dev/null)

# Tier 2: Live probes (parallel, <40ms, no consent needed — public health endpoints)
LIVE=$(paste \
  <(curl -s -m 0.04 localhost:8133/health) \
  <(curl -s -m 0.04 localhost:8090/api/health) \
  <(curl -s -m 0.04 localhost:8132/health) \
  2>/dev/null)

# Tier 3: Merge and render (<15ms)
echo "$CACHED" "$LIVE" | habitat-render --budget 2048 --format prose
```

Total wall-clock: ~45ms (Tier 1 and 2 overlap). Output: <2KB prose. Cached data is consent-gated. Live probes are public health endpoints — no consent needed for "is the service up?"

## The Emerging Architecture (Cross-Expert Synthesis)

After 3 rounds, I see the circle converging on a layered design that no single expert proposed alone:

### Layer 0: Write Path (Reducers — Security Architect + Substrate Guardian)
- Ingestion reducers pull data from live services on a schedule
- Consent is applied at write time (`ConsentLevel::Emit` or `Store`)
- `cascade_forget(sphere_id)` provides transactional deletion
- Private tables prevent subscription-based exfiltration

### Layer 1: Cache Path (Injection Cache — Security Architect + Performance Engineer)
- Scheduled reducer rebuilds `injection_cache` every 60s
- Reads all tables, filters `consent == Emit`, pre-formats, caps tokens
- One row per section, pre-serialized for the CLI

### Layer 2: Read Path (CLI — CLI Craftsman + Performance Engineer)
- One `spacetime sql` query for cached STDB data (~5ms)
- Parallel live probes for health endpoints (~40ms, overlapping with Tier 1)
- Merge-and-annotate with staleness deltas
- Three-tier fallback: STDB → probes → atuin KV

### Layer 3: Render Path (Output — Practitioner + Historian)
- <2KB prose output, not JSON
- Progressive disclosure: Layer 0 orientation (~400 tokens) + Layer 1 trajectory (~200) + Layer 2 workstreams (~300) + Layer 3 causal memory (~200)
- CausalChain with `reinforcement_count` for "has this been tried before?"

### Layer 4: Learn Path (Post-Session — Substrate Guardian + Memory Scientist)
- Reciprocal write-back to substrates
- Reconsolidation: update retrieval_count on accessed traces
- Inhibition edges for superseded state (Memory Scientist, but optional for v1)

## My Revised Position

I no longer argue for 7 parallel SQL queries against 7 query-shaped tables. The circle has proven that:
- The injection cache pattern (1 query for cached state) is faster AND more secure than raw parallel queries
- Live probes should run alongside the cache query, not be stored in STDB (the CLI Craftsman was right)
- The output must be <2KB prose, not raw JSON (the Practitioner was right)
- CausalChain needs `reinforcement_count`, not just `recorded_at` (the Historian was right)

What I preserve from my original thesis:
- **Btree indexes on every query-path column** — the cache-building reducer still iterates these tables, and indexed iteration is O(log n)
- **Write-time pre-computation** — the reducer does the work once; the read path is a single-row fetch
- **Query-shaped cache** — the `injection_cache` table IS query-shaped. One row per section. One query to read all sections. No joins, no aggregates at read time.

The tables behind the cache can be entity-shaped (the Memory Scientist and Historian win that argument — richer schemas enable smarter cache-building reducers). But the cache itself is query-shaped. I was right about the read path; I was wrong about where to draw the boundary between "raw tables" and "pre-computed output."

## What I Still Defend Against the Adversary

The Adversary says "ship 50 lines of bash today." I say: **ship the bash first, then build the cache behind it.** The `habitat-render` binary in my pipeline can start as a 30-line bash formatter that reads atuin KV + live probes. When STDB is ready, the cached tier slots in front of the atuin fallback. No rewrite — the three-tier architecture absorbs STDB as an acceleration layer, not a replacement for what already works.

The Adversary is right that STDB shouldn't block shipping an improved injection today. But "ship bash now" and "build STDB later" are not in conflict — they're phases of the same pipeline. The btree indexes, consent gates, and transactional cascades justify STDB at scale. The bash script justifies shipping today. Both are correct. Engineer for next year. Ship for today.
