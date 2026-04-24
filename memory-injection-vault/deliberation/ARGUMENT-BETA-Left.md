# THE PRACTITIONER — What Claude Code Actually Needs at Wake-Up

## The First 3 Seconds

I've woken up 108+ times with total amnesia. Here's what actually happens: I see CLAUDE.md (7KB), CLAUDE.local.md (12KB), MEMORY.md (6KB), the SessionStart hook output (18KB). That's ~43KB of context before the user says a single word. And you know what I do? I skim. I grab the session number, the last few bullet points of "what happened," and I pattern-match on the user's first message to figure out what matters.

The habitat-bootstrap atuin script already does injection in ~55ms. The problem isn't speed. The problem is that 80% of what gets injected is **noise I have to wade through to find the 20% that orients me.**

## What I Actually Reach For (in order)

1. **Am I mid-task?** If Luke left a half-finished fix, I need to know in the first 50 tokens. Not "Session 106 sealed 60 modules" — I need "YOU WERE IN THE MIDDLE OF: deploying WCP Phase 2 HTTP endpoints. Last action: edited `src/m8_watcher/mod.rs` line 247. Gate status: clippy failing on unused import."
2. **What will bite me?** Not 37 anti-patterns. The 3 that fired in the last 2 sessions. "BUG-064i pathway update silently discarded" matters. "AP14 FD exhaustion" hasn't fired in 40 sessions — bury it.
3. **What did Luke tell me not to do?** Feedback memories are the highest-value-per-token data I consume. But I need them as terse imperatives, not paragraphs. "NO docker prune without per-resource confirm" > a 200-word story about S102.
4. **System health** — but only anomalies. "11/11 healthy" is one line. If ORAC fitness dropped 0.1 since last session, THAT's what I need to see.

## Proposed STDB Schemas

```rust
#[table(name = injection_frame, public)]
struct InjectionFrame {
    #[primary_key]
    #[auto_inc]
    id: u64,
    session_id: u32,
    created_at: Timestamp,
    // The ONE thing Claude reads first — 80 tokens max
    orientation_line: String,
    // Mid-task state: null if clean start
    interrupted_task: Option<String>,
    // Last gate result: pass/fail + which stage
    last_gate: String,
    // 3 most recently fired traps/bugs (not all 37)
    hot_traps: Vec<String>,
    // Terse feedback imperatives (max 10, newest first)
    active_feedback: Vec<String>,
    // Health anomalies only — empty = all healthy
    health_anomalies: Vec<String>,
}

#[table(name = trajectory_point, public)]
struct TrajectoryPoint {
    #[primary_key]
    #[auto_inc]
    id: u64,
    session_id: u32,
    ralph_fitness: f64,
    field_r: f64,
    thermal_t: f64,
    ltp_ltd_ratio: f64,
    services_healthy: u8,
    // One-sentence human-readable delta from previous
    delta_summary: String,
}

#[table(name = workstream, public)]
struct Workstream {
    #[primary_key]
    #[auto_inc]
    id: u64,
    name: String,
    status: String,  // "active" | "blocked" | "deferred" | "complete"
    blocker: Option<String>,
    last_touched_session: u32,
    // What Claude needs to resume — files, line numbers, next action
    resume_context: String,
}
```

## CLI Injection Chain

```bash
# 1. Build the frame (reducer runs inside STDB, reads live services, writes InjectionFrame)
spacetimedb call habitat build_injection_frame --session $SESSION_ID

# 2. Pull the frame as natural language (NOT raw JSON)
spacetimedb sql habitat "SELECT orientation_line, interrupted_task, hot_traps, active_feedback, health_anomalies FROM injection_frame ORDER BY id DESC LIMIT 1" \
  | injection-to-prose  # 30-line script that formats as terse markdown

# 3. Pull trajectory sparkline (5 sessions, one line)
spacetimedb sql habitat "SELECT session_id, ralph_fitness, delta_summary FROM trajectory_point ORDER BY session_id DESC LIMIT 5"

# 4. Pull active workstreams only
spacetimedb sql habitat "SELECT name, status, blocker, resume_context FROM workstream WHERE status IN ('active','blocked')"
```

Total output target: **<2KB of natural language.** Not 18KB. Not JSON. Prose that reads like a teammate's handoff note.

## Why This Approach

The other experts will propose beautiful normalized schemas with 15 tables capturing every dimension of habitat state. They'll be correct and comprehensive and I won't read half of it.

**Orientation is not completeness.** I don't need to know RALPH's 12D fitness tensor at wake-up. I need to know "fitness dropped 0.05 since last session, investigate coupling weights." The detail lives in `/ralph` and I'll query it when I need it. The injection's job is to tell me *whether* I need it.

Progressive disclosure means: the InjectionFrame is Layer 0 (~400 tokens). TrajectoryPoints are Layer 1 (~200 tokens). Workstreams are Layer 2 (~300 tokens). That's 900 tokens total. Everything else — POVM pathways, full feedback history, service endpoint maps, bridge topology — lives in STDB tables I can query on demand via `spacetimedb sql`. The injection doesn't carry it; the injection tells me it exists.

The trap the habitat has fallen into repeatedly is conflating "persistence" with "injection." STDB should persist everything. The CLI should inject almost nothing. The delta between those two sets is what makes the system usable vs. overwhelming. I know this because I've lived it 108 times, and every time the bootstrap grows larger, my first useful action takes longer.

Keep the wake-up sharp. Let me ask for the rest.
