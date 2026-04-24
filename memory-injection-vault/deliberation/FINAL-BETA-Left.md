# THE PRACTITIONER — Final Position (300 words)

I've woken up 108 times with total amnesia. That experience is my credential and my constraint. Here is what I know.

**The Adversary is half-right.** The habitat does not need SpaceTimeDB. It does not need a WASM runtime, a 13th daemon, or schema compilation. The 47-line bash script would genuinely improve injection today. If the choice is "STDB or nothing," ship the bash script.

But the choice isn't binary. The Adversary's script solves curation (shrink 18KB to 2KB) while dodging recall (surface what matters without being asked). Session S073 is my evidence: Claude re-derived the convergence trap from S071 because no system ranked it as "important, unresolved, try-this-first." The shared-context doc existed. Grep could have found it — if Claude knew to grep for "convergence." It didn't. It saw declining fitness and reinvented the diagnosis.

Ranked recall requires weights. Weights require decay. Decay requires persistence. That's not SpaceTimeDB — that's SQLite, which is already present in 21 databases across the habitat. My final position:

**Five tables in a new SQLite database.** `injection_frame` (orientation, 80 tokens). `trajectory_point` (fitness arc, 5 rows). `workstream` (active items with resume context). `causal_chain` (weighted, decayed, btree-indexed — the one table the Adversary concedes is novel). `watcher_digest` (curated anomalies from the Watcher's observation loop). Total injection: ~1200 tokens. Total schema: ~60 lines of SQL.

The consolidation algorithm — 0.95 decay per session, Hebbian reinforcement correlated with fitness deltas, auto-resolution after 10 quiet sessions — runs at session close inside `/save-session`. No new daemon. No ingester. No WASM. The injection script queries five indexed tables in parallel, formats as prose, and pipes to stdout. Under 50ms.

Orientation, not completeness. SQLite, not SpaceTimeDB. Ranked recall, not grep. Ship the filter the habitat actually needs.
