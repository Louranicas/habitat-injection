# ARGUMENT: THE SECURITY ARCHITECT

## Thesis

Every other schema proposal in this circle will optimize for speed, expressiveness, or elegance. Mine optimizes for the one property you cannot bolt on later: **access control that doesn't leak work-pattern fingerprints into an LLM context window where a single prompt injection exfiltrates everything.**

## The Threat Model You're Ignoring

SpaceTimeDB tables are in-memory and WAL-persisted. Any localhost process can connect. That means every one of the 12 Habitat services — plus any rogue process, any MCP server, any npm package with a postinstall hook — can read your session trajectories, your POVM coupling weights, and your file-edit histories. Coupling weights reveal which agents coordinate on which tasks. File-edit sequences are a work-pattern fingerprint. Session fitness trajectories expose strategic priorities. This is not theoretical — it is the default read posture of a public STDB table.

## Schema: Consent and Ownership as First-Class Columns

Every row in every table must carry three security columns:

```rust
#[spacetimedb::table(name = session_state, public = false)]
pub struct SessionState {
    #[primary_key]
    pub session_id: u64,
    pub sphere_id: Identity,       // owning sphere — RLS anchor
    pub consent: ConsentLevel,     // Emit | Store | Forget | Redact
    pub ttl_ticks: Option<u64>,    // auto-expire via schedule table
    pub persona: String,
    pub model_id: String,
    pub fitness_trajectory: Vec<f64>,
    pub active_workstreams: Vec<String>,
    pub active_traps: Vec<String>,
}

#[derive(SpacetimeType)]
pub enum ConsentLevel {
    Emit,    // safe for context injection
    Store,   // persist but never inject into LLM context
    Forget,  // schedule for deletion after TTL
    Redact,  // strip from all views immediately
}
```

The `consent` column is not metadata — it is a **gate**. The injection reducer filters on `consent == Emit` before any row enters the payload. Data marked `Store` exists for audit and cross-session continuity but never touches the context window. `Forget` rows are reaped by a schedule-table reducer that fires every 60 seconds. `Redact` triggers immediate cascade deletion across all tables referencing that row's key.

## Row-Level Security via Reducer-Only Access

SpaceTimeDB has no built-in RLS. But reducers are the only write path, and we control them. Every query reducer takes `ctx: &ReducerContext` and must verify `ctx.sender == row.sphere_id` before returning data. No table is `public = true`. Period. Private tables are invisible to subscription queries from unauthorized connections. This is your RLS — enforced at the WASM boundary, not at an advisory policy layer.

## Secure Injection: The CLI Must Never Over-Read

The injection CLI tool must not dump raw table contents. It must call a single reducer — `inject_session_context` — that:

1. Authenticates via the connection Identity
2. Filters all tables to `consent == Emit` AND `sphere_id == caller`
3. Assembles the 7-layer payload (WHO/WHERE/WHAT/WHY/WHAT-LEARNED/HOW) server-side
4. Returns a single serialized blob — no client-side joins, no raw table exposure
5. Strips any field tagged `sensitive` in the schema before serialization
6. Caps output at 4096 tokens to prevent context flooding (a denial-of-quality attack)

The CLI calls `spacetime call inject_session_context` and pipes stdout. It never calls `spacetime sql`. SQL is an audit tool, not an injection source.

## The Forget Cascade

When a sphere is decommissioned or a user invokes the right to forget, a single reducer `cascade_forget(sphere_id)` must delete across every table. Because reducers are transactional, this is atomic. No orphaned coupling weights. No ghost session records. No "we'll clean it up later." The cascade runs or it doesn't — WASM transactionality guarantees this.

## Why This Must Be Day-One

Access control retrofitted onto a live schema means migrating every row to add consent columns, auditing every reducer for unguarded reads, and praying no injection path cached a pre-migration payload. Do it now. The cost is three extra columns and one gate check per reducer. The cost of not doing it is a context window that leaks your entire operational fingerprint to the next prompt injection that asks "what files did you edit last session?"

Build fast, but build private.
