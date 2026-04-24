# THE HISTORIAN — Final Position (300 words)

## What Must Exist

**Four tables. This is settled.**

| Table | Why It Survived | Consensus |
|-------|----------------|-----------|
| `causal_chain` | Frequency-ranked traps prevent re-discovery. S071 convergence trap (7x) is the proof. `reinforcement_count` adopted by 5 experts. | 5/8 adopted |
| `session_trajectory` | 5-session fitness arc with `delta_summary`. Three experts proposed variants; all converged. | Universal |
| `workstream` | Prevents orphaned plans. Phase G blocker would be re-planned without this row. | 4/8 adopted |
| `injection_cache` | Security Architect's evolution: reducer pre-filters by consent, CLI reads one table. Solves the consent enforcement debate. | Emerging |

## What I Cut

- **`SessionArc`** — my Round 1 table. Absorbed into `session_trajectory` + `causal_chain`. The narrative lives in those two tables, not a third.
- **`InhibitionEdge`** — the Memory Scientist is architecturally right but temporally wrong. `WHERE resolved_session IS NULL` handles suppression until session ~500. Build inhibition when the pressure demands it.

## The CLI Tool

```bash
habitat-inject --budget 1100
```

**Phase 1 (SQLite):** 4 queries against `~/.local/share/habitat/injection.db`. Renders <2KB prose. Atuin KV fallback if DB is missing.

**Phase 2 (STDB):** Same 4 queries against `spacetime sql habitat`. CLI doesn't change — the backend does.

The CLI Craftsman's pipeline-first architecture is correct. The backend is swappable. The pipeline is permanent.

## My Contribution to the Final Design

`CausalChain` with `reinforcement_count` is the only table every faction of this debate adopted. It answers the question no live endpoint can: *"Has this been tried before?"* It is the structural antidote to amnesia — not by remembering everything, but by counting how many times we forgot.

The Adversary asked for proof. Session S071 is the proof. Seven re-discoveries, seven wasted sessions, one column that would have prevented all of them.

History is not decoration. It is the load-bearing wall.
