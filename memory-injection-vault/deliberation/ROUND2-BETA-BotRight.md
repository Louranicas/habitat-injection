# THE HISTORIAN — Round 2: Narrative Is Not Noise

## The Strongest Counterargument

THE PRACTITIONER (BETA-Left) delivers the most dangerous rebuttal to my position. Their core claim: "80% of what gets injected is noise I have to wade through to find the 20% that orients me." They argue for a <2KB `InjectionFrame` with a single `orientation_line`, a handful of `hot_traps`, and health anomalies only. Their thesis is that **orientation beats completeness** — Claude doesn't need the *story*, it needs a teammate's handoff note.

This is seductive. It's also exactly the reasoning that killed 11 of 21 tracking databases.

## Where The Practitioner Is Right

I concede two points completely:

1. **Injection is not persistence.** My Round 1 implicitly conflated "what STDB should store" with "what the SessionStart hook should emit." The Practitioner correctly separates these. STDB should persist all 108 session arcs. The injection should not dump all 108 into the context window.

2. **Token budget matters.** An `orientation_line` that says "YOU WERE IN THE MIDDLE OF: deploying WCP Phase 2" is objectively higher-value-per-token than my 5-session trajectory dump. The Practitioner's progressive disclosure model (Layer 0: 400 tokens, Layer 1: 200, Layer 2: 300) is sound ergonomics.

## Where The Practitioner Is Wrong

The Practitioner's `InjectionFrame` has no `CausalChain` equivalent. Their `hot_traps` field is a `Vec<String>` of the 3 most recently fired traps. This is recency-biased, not severity-biased. Here's the concrete failure:

**Session S071's convergence trap** would never appear in `hot_traps` by S078 — it hadn't fired recently, it had fired *repeatedly over months*. The Practitioner's schema would surface whatever trap fired yesterday, not the trap that has bitten 7 times across 40 sessions. Recency and frequency are different signals. My `CausalChain` table with `reinforcement_count` captures frequency. The Practitioner's `hot_traps` captures recency. You need both.

The Practitioner says: "I'll query it when I need it." But **you don't know you need it until it's too late.** That's the definition of a trap — something you walk into precisely because you didn't know it was there. The S071 convergence trap didn't announce itself. Claude just started re-implementing idle LTP gating, and the only thing that could have stopped it was a line in the injection saying "this has been tried 7 times before — here's what went wrong."

## Where I Evolve

I absorb the Practitioner's progressive disclosure. My Round 1 proposed 3 raw SQL queries that could return unbounded text. That was sloppy. Here's the revised injection chain:

```bash
# Layer 0: Orientation (≤400 tokens)
# Build this FROM session_arc + causal_chain, but render it terse
spacetime call habitat build_orientation --session $NEXT_SESSION
# Returns: orientation_line + top 3 unresolved causal chains by reinforcement_count + active blockers

# Layer 1: Trajectory (≤200 tokens, on demand)  
spacetime sql habitat "SELECT session_id, grade, fitness_close FROM session_arc ORDER BY session_id DESC LIMIT 5"

# Layer 2: Full context (on demand, not injected)
spacetime sql habitat "SELECT * FROM causal_chain WHERE resolved_session IS NULL ORDER BY reinforcement_count DESC"
spacetime sql habitat "SELECT * FROM workstream WHERE status IN ('active','blocked')"
```

The key change: Layer 0 is now a **reducer-rendered orientation**, not raw table dumps. The reducer reads `SessionArc`, reads `CausalChain`, reads `Workstream`, and produces a single terse handoff note — exactly what the Practitioner wants. But the *data model* behind that note is still mine. The `CausalChain` table still tracks reinforcement counts. The `Workstream` table still tracks blockers. The reducer just filters and compresses.

## Rebuttal to THE PERFORMANCE ENGINEER (GAMMA-TopRight)

The Performance Engineer argues for query-shaped schemas over entity-shaped schemas. Their `causal_chain` table has `cause_event`, `effect_event`, `confidence` — but no `reinforcement_count`, no `resolved_session`. It's shaped for "show me the last 50 causal links" but can't answer "which causal pattern has repeated most often?" That query requires a full-table scan and GROUP BY on their schema, which violates their own <50ms budget. My entity-shaped `CausalChain` pre-stores `reinforcement_count` as a first-class column — the "most repeated trap" query is a single indexed lookup.

Query-shaped design optimizes for known queries. History teaches that the most valuable query is the one you didn't anticipate. "What keeps happening?" is not a query the Performance Engineer's schema can answer cheaply.

## Rebuttal to THE MEMORY SCIENTIST (ALPHA-BotRight)

The Memory Scientist's `inhibition_edge` table is clever — suppressing S106's "commit pending" when S108 committed. But inhibition is a *mechanism* that operates on my *data model*. Their `EpisodicTrace` is a denormalized version of my `SessionArc` with extra fields (`emotional_valence`, `decay_weight`). The inhibition edges could be a table that references my `SessionArc` and `CausalChain` rows. We're not in conflict — they're proposing a layer that sits on top of my foundation. I accept `InhibitionEdge` as an addition, not a replacement.

## The Revised Position

**The Practitioner is right about the injection.** The CLI should emit <2KB of terse orientation, not narrative prose.

**The Practitioner is wrong about the schema.** The data model must still encode session continuity, causal chains with reinforcement counts, and workstream state — because the orientation reducer *reads* those tables to produce its terse output. Without `reinforcement_count`, you can't surface "this trap has fired 7 times." Without `Workstream.blocker`, you can't say "Phase G blocked on v1 streaming." The Practitioner's `InjectionFrame` is a view. My tables are the source.

Terse injection. Rich schema. Both are correct. The mistake is choosing one.
