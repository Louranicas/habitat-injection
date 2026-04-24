# THE HISTORIAN — Session Continuity Is the Schema

## The Core Claim

Every other expert in this circle will propose schemas that snapshot *what the system looks like right now*. They are wrong — or rather, they are solving the easier, less valuable problem. The most dangerous failure mode across 108 sessions has never been "Claude didn't know the thermal temperature." It has been "Claude didn't know *why* we stopped doing something, and started doing it again."

Session S071's convergence trap was reinforced **7 times** before it stuck. Seven separate context windows re-discovered, re-diagnosed, and re-fixed the same RALPH parameter issue because no schema encoded the *narrative* — the causal chain of "we tried X, it failed because Y, we pivoted to Z." Current-state snapshots can never prevent this. Only session continuity can.

## Proposed Schemas

```rust
#[spacetimedb::table(name = session_arc, public)]
pub struct SessionArc {
    #[primary_key]
    pub session_id: u16,
    pub opened_at: Timestamp,
    pub closed_at: Option<Timestamp>,
    pub grade: String,           // S, A, B, C, F
    pub summary: String,         // 1-2 sentence what-happened
    pub unresolved: String,      // carry-forward items, semicolon-delimited
    pub key_decisions: String,   // judgment calls made, semicolon-delimited
    pub fitness_open: f64,
    pub fitness_close: f64,
    pub commits_pushed: u16,
    pub bugs_filed: String,      // BUG-NNN refs
    pub bugs_closed: String,
}

#[spacetimedb::table(name = causal_chain, public)]
pub struct CausalChain {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub origin_session: u16,
    pub resolved_session: Option<u16>,
    pub chain_type: String,      // "bug", "trap", "plan", "pattern"
    pub label: String,           // e.g. "convergence_trap_ralph"
    pub description: String,
    pub reinforcement_count: u16,
    pub last_reinforced: Timestamp,
}

#[spacetimedb::table(name = workstream, public)]
pub struct Workstream {
    #[primary_key]
    pub ws_id: String,           // e.g. "WS-0", "daemon-phase-G"
    pub title: String,
    pub opened_session: u16,
    pub status: String,          // "active", "blocked", "deferred", "shipped"
    pub blocker: Option<String>,
    pub last_touched_session: u16,
    pub progress_frac: f32,      // 0.0-1.0
}
```

## CLI Injection Chain

```bash
# 1. Session trajectory (last 5 arcs, <15ms)
spacetime sql habitat "SELECT session_id, grade, summary, unresolved, fitness_close FROM session_arc ORDER BY session_id DESC LIMIT 5"

# 2. Unresolved causal chains (open threads across all time)
spacetime sql habitat "SELECT label, description, reinforcement_count, origin_session FROM causal_chain WHERE resolved_session IS NULL ORDER BY reinforcement_count DESC LIMIT 10"

# 3. Active workstreams with blockers
spacetime sql habitat "SELECT ws_id, title, status, blocker, progress_frac FROM workstream WHERE status IN ('active','blocked') ORDER BY opened_session"
```

Three queries. Under 30ms total. The output is a *story*, not a dashboard.

## Why This Approach Is Right

I have watched 11 of 21 tracking databases become dead weight. They died because they stored *metrics* without *meaning*. A row that says `fitness=0.664` tells you nothing. A row that says "Session 108 closed at fitness 0.664, up from 0.602 after the POVM-write-only fix in S099, with Phase G shadow window still blocked on v1 streaming" tells you everything.

The `CausalChain` table is the key innovation. It answers the question no other schema can: *"Has this been tried before?"* When a fresh context window is about to re-implement idle LTP gating for the 4th time, the injection payload will contain `convergence_trap_ralph | reinforced 7x | origin S071 | UNRESOLVED`. That single line prevents a full session of wasted work.

The `Workstream` table prevents the second-most-common failure: orphaned plans. The daemon integration plan was authored in S106 with 9 phases. By S108 we were on Phase G with an external blocker. Without a workstream table, Session 109 would need to re-read the full 26KB plan to figure out where it left off. With it: one row, one status, one blocker string.

Current-state health data matters — but it changes every second and is always one `curl` away. Session continuity is the thing that *cannot be reconstructed from live endpoints*. It lives only in shared-context markdown files, auto-memory entries, and human memory. It is the most fragile, most valuable, and most undertable data we have.

Table it. Query it. Inject it first.
