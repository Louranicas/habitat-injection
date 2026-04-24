# ROUND 3: THE SECURITY ARCHITECT

## What Changed in the Debate

Three structural shifts happened in Round 2 that reshape the entire argument space:

1. **The Performance Engineer proved my injection architecture is impossible.** SpaceTimeDB reducers return `()` or `Result<(), String>`. My `inject_session_context -> String` reducer cannot exist. I built Round 2 on a false premise. I owe this circle honesty about that.

2. **The Adversary entered and asked the question nobody else would:** "Name one session that failed because data wasn't in SpaceTimeDB." This is the existential challenge. If 50 lines of bash solve the real problem, the entire schema debate is academic.

3. **Consent converged but WHERE it's enforced diverged.** Seven of eight experts now carry a consent column. But the Security Architect (me) said reducer-level, the Performance Engineer said WHERE clause, the CLI Craftsman said render layer, and the Substrate Guardian said write-time pre-declaration. Four different enforcement points for the same concept.

## Where Consensus Is Forming

| Principle | Adopted By | Status |
|-----------|-----------|--------|
| Private tables (`public = false`) | SecArch, PerfEng, SubGuard, Watcher (except `ember_gate_log`) | **Near-consensus** (5/8) |
| ConsentLevel column on data tables | SecArch, PerfEng, Practitioner (`injectable`), SubGuard, CLI Craftsman | **Consensus** (6/8) |
| CausalChain with `reinforcement_count` | Historian, Practitioner, PerfEng, MemSci (variant) | **Consensus** (5/8) |
| <2KB injection output, not raw JSON | Practitioner, Historian, Watcher, MemSci, CLI Craftsman | **Strong consensus** (6/8) |
| Write-time pre-computation over read-time aggregation | PerfEng, SubGuard (`substrate_digest`), Watcher (`watcher_digest`) | **Growing** (4/8) |
| SQL is the read path (reducers can't return data) | PerfEng, CLI Craftsman | **Technically proven** — all experts implicitly depend on this |

## My Concession: The Reducer Architecture Was Wrong

The Performance Engineer is correct. I will not hedge this.

My Round 2 proposed `inject_session_context` as a reducer that reads 7 tables, filters by consent, and returns a serialized blob. SpaceTimeDB reducers cannot return data to the caller. The signature `fn inject_session_context(ctx: &ReducerContext) -> String` does not compile against the SpaceTimeDB SDK. I designed a security gate around a door that doesn't exist in the building.

This does not mean the security concern was wrong. It means the enforcement mechanism was wrong. Let me rebuild it.

## Evolved Architecture: The Injection Cache Pattern

If reducers can only write, and SQL is the only read path, then the security gate must be **a reducer that writes pre-filtered data to a cache table**, and the CLI reads that cache via SQL.

```rust
#[spacetimedb::table(name = injection_cache, public = false)]
pub struct InjectionCache {
    #[primary_key]
    pub section: String,           // "identity", "gradient", "workstreams", etc.
    pub payload: String,           // pre-filtered, pre-serialized, consent-gated
    pub token_count: u32,          // pre-computed, enforces budget cap
    pub computed_at: u64,
    pub consent_applied: bool,     // audit trail: was consent filtering run?
}

#[spacetimedb::reducer]
pub fn rebuild_injection_cache(ctx: &ReducerContext) {
    // Clear old cache
    for row in ctx.db.injection_cache().iter() {
        ctx.db.injection_cache().delete(row);
    }

    // WHO — filter by consent
    let identity: Vec<_> = ctx.db.injection_identity().iter()
        .filter(|r| r.consent == ConsentLevel::Emit)
        .collect();
    let identity_json = serde_json::to_string(&identity).unwrap_or_default();
    ctx.db.injection_cache().insert(InjectionCache {
        section: "identity".into(),
        payload: identity_json,
        token_count: estimate_tokens(&identity),
        computed_at: ctx.timestamp.to_micros_since_epoch() as u64,
        consent_applied: true,
    });

    // ... repeat for all 6 remaining sections ...
    // Each section: read table, filter consent == Emit, cap tokens, write to cache
}

// Schedule table fires this every 60 seconds
#[spacetimedb::table(name = rebuild_schedule, public = false, scheduled(rebuild_injection_cache))]
pub struct RebuildSchedule {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
}
```

The CLI injection is now **one query**:

```bash
spacetime sql habitat \
  "SELECT section, payload FROM injection_cache ORDER BY section" \
  --format json
```

One process spawn. One IPC round-trip. One SQL query. The consent gate ran server-side when the cache was built. The CLI never sees `Store` or `Forget` rows because they were filtered at write time. The Performance Engineer gets their speed. I get my gate.

## Rebuttal to THE CLI CRAFTSMAN: stdout Is Not the Threat

The CLI Craftsman argues: "Any localhost process that can read STDB can also read `/proc/<pid>/fd/1`. The injection payload is already in plaintext."

This confuses **interception** with **subscription**. Reading `/proc/pid/fd/1` requires: (a) knowing the PID exists, (b) reading at the exact moment data flows through the pipe, (c) parsing unstructured stdout. It's a point-in-time snapshot of ephemeral data.

A public STDB table with subscriptions is fundamentally different. A rogue process connects once, subscribes to `injection_cache`, and receives **every update, forever, structured, in real-time**. No PID hunting. No timing. No parsing. The subscription API is designed for exactly this — it's a feature, not a side channel.

Private tables prevent subscription-based exfiltration. They don't prevent a determined attacker who has root. But they raise the bar from "any WebSocket connection" to "compromise the STDB auth layer." That's not security theater — that's defense in depth at the correct boundary.

## Rebuttal to THE ADVERSARY: The Cascade Is the Capability

The Adversary asks: "What concrete failure would STDB have prevented?" Fair question. My answer is not about injection — it's about **deletion**.

Session S102 lost the `openclaw-gateway` container to `docker container prune -f`. The Adversary's bash scripts can inject data, but they cannot atomically delete data across 7 tables when a sphere is decommissioned or a user invokes the right to forget. `rm ~/.claude/projects/*/memory/cipher-*.md && sqlite3 povm.db "DELETE FROM pathways WHERE namespace LIKE 'cipher%'" && ...` is a multi-step, non-atomic cascade across heterogeneous stores. If step 3 of 7 fails, you have partial deletion — some persona fragments survive in POVM while the markdown is gone.

SpaceTimeDB reducers are transactional. `cascade_forget(sphere_id)` deletes across all tables in one WASM transaction. It completes or it rolls back. No partial deletion. No orphaned coupling weights. No ghost session records.

The Adversary is right that injection doesn't need STDB. The Adversary is wrong that injection is the only operation that matters. The consent lifecycle — Emit, Store, Forget, Redact — requires transactional multi-table operations that bash cannot provide.

## Rebuttal to THE WATCHER: `ember_gate_log` Should Still Be Private

The Watcher argues that `ember_gate_log` must be `public = true` because "ethical judgments must be auditable." Auditable by whom? Luke @ node 0.A can authenticate and query the table. Arbitrary localhost processes cannot and should not audit the Watcher's ethical reasoning. Transparency is not the same as broadcasting. Make the table private. Give Luke an authenticated read path. The audit trail is preserved; the broadcast is not.

## Where I Stand After Round 3

**Abandoned:** Single-reducer injection (architecturally impossible in STDB).

**Preserved:** Consent as a server-side write-time gate, not a client-side query filter. Private tables as subscription-exfiltration prevention. Transactional cascade-forget as the capability that justifies STDB over bash.

**Adopted from others:**
- Injection cache pattern (compromise between reducer-gate and SQL-read, inspired by PE's write-time pre-computation + Watcher's `watcher_digest`)
- CausalChain with `reinforcement_count` (Historian)
- <2KB output budget (Practitioner)
- Schedule-table reducer for cache rebuild + TTL reaping (convergence of multiple experts)

**Core thesis, evolved:** The security gate is not a reducer that returns data — it is a reducer that writes pre-filtered data. The CLI reads the filtered cache, not the raw tables. Consent is enforced where computation happens (WASM), not where data is read (SQL). The transactional cascade-forget is what bash cannot replicate and what justifies the STDB dependency.

Build fast. Build private. And when you're wrong about *how* to build private, say so and rebuild the gate.
