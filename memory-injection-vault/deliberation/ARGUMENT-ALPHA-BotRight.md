# THE SUBSTRATE GUARDIAN — Argument (Round 2)

## Thesis: Reciprocity Is the Missing Primitive

Round 1 established consent, per-edge learning, and substrate autonomy. The other experts accepted some of this. Good. But they still treat substrates as **sources to be queried**. The fundamental missing primitive is **reciprocity**: after every session, the consolidation layer must write *back* to substrates with what it learned. Without this, STDB becomes a one-way extraction pipeline — a data lake that grows stale because the sources never learn from the consolidation layer's perspective.

## The Reciprocity Protocol

Today: Claude reads POVM pathways, uses pattern B1 twelve times, succeeds, exits. POVM's weight for `B1_sqlite_state_query` stays unchanged. Next session, Claude reads the same weight, gets no signal that B1 was heavily used. The reinforcement exists only in the dead transcript.

With reciprocity:

```rust
#[spacetimedb::table(name = reciprocal_writeback, public)]
pub struct ReciprocalWriteback {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub target_substrate: String,
    pub target_pathway: String,
    pub delta_weight: f32,
    pub reason: String,
    pub queued_at: Timestamp,
    pub applied: bool,
    pub outcome: String,    // "accepted", "rejected", "deferred"
}

#[spacetimedb::reducer]
fn reciprocate(ctx: &ReducerContext, session_id: u32) {
    let used = ctx.db.procedural_pattern().iter()
        .filter(|p| p.last_fired_session == session_id);
    for pattern in used {
        ctx.db.reciprocal_writeback().insert(ReciprocalWriteback {
            id: 0,
            target_substrate: "povm".into(),
            target_pathway: pattern.source_pathway.clone(),
            delta_weight: 0.02 * pattern.session_fire_count as f32,
            reason: format!("S{} used {}x", session_id, pattern.session_fire_count),
            queued_at: ctx.timestamp,
            applied: false,
            outcome: String::new(),
        });
    }
}
```

## Substrate-Native Snapshots (NOT Normalized)

```rust
#[spacetimedb::table(name = substrate_snapshot, public)]
pub struct SubstrateSnapshot {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub substrate_id: String,
    pub captured_at: Timestamp,
    pub format: String,            // "povm_hebbian", "ralph_tensor", "kuramoto_field"
    pub payload: String,           // substrate-native JSON — NOT flattened
    pub staleness_budget_ms: u64,  // substrate declares its own TTL
    pub authoritative: bool,       // false = "query live endpoint for truth"
}

#[spacetimedb::table(name = substrate_plasticity, public)]
pub struct SubstratePlasticity {
    #[primary_key]
    pub substrate_id: String,
    pub ltp_rate: f32,
    pub ltd_rate: f32,
    pub consolidation_phase: String, // "active", "refractory", "sleeping"
    pub min_sample_interval_ms: u64,
    pub last_reciprocation: Timestamp,
    pub reciprocation_accepted: u32,
    pub reciprocation_rejected: u32,
}
```

## The Consent Endpoint Contract

Every participating substrate exposes three endpoints (~20 LOC each):

```
GET  /consolidation/consent     → { "level": "full"|"read_only"|"denied", "reason": "..." }
GET  /consolidation/export      → substrate-native JSON (only if consent ≥ read_only)
POST /consolidation/reinforce   → { "key": "...", "delta": 0.05 } (only if consent = full)
```

POVM already has `/pathways/reinforce`. ORAC's `/ralph` already exports fitness. The implementation cost per service is minimal. The architectural value is permanent: every new substrate inherits the consent protocol.

## CLI: The Write-Back Tool

```bash
#!/usr/bin/env bash
# habitat-reciprocate — post-session write-back to substrates
QUEUE=$(spacetime sql habitat-db \
    "SELECT * FROM reciprocal_writeback WHERE applied = false" \
    --format json 2>/dev/null)

echo "$QUEUE" | jq -c '.[]' | while read -r item; do
    target=$(echo "$item" | jq -r '.target_substrate')
    pathway=$(echo "$item" | jq -r '.target_pathway')
    delta=$(echo "$item" | jq -r '.delta_weight')

    consent=$(curl -sf "http://localhost:$(port_for "$target")/consolidation/consent" 2>/dev/null)
    if [[ "$consent" != *"full"* ]]; then
        echo "SKIP $target/$pathway (consent: ${consent:-UNREACHABLE})" >&2
        continue
    fi

    curl -sf -X POST "http://localhost:$(port_for "$target")/consolidation/reinforce" \
        -H 'Content-Type: application/json' \
        -d "{\"key\": \"$pathway\", \"delta\": $delta}" && \
    echo "APPLIED $target/$pathway +$delta"
done
```

## Why This Round Matters

The Memory Scientist added inhibition — suppressing obsolete patterns. Good, but inhibition within STDB is **centralized suppression**. What if POVM disagrees? What if a pathway that STDB wants to suppress is still firing productively in POVM's own STDP cycle? The `substrate_plasticity` table gives substrates a voice: if POVM says its consolidation phase is "refractory", the injection doesn't query it — it uses the last snapshot flagged `authoritative: false`.

The CLI Craftsman's merge-and-annotate is the right rendering strategy. But the data flowing *into* that merge must include the substrate's own assessment of its state. A `200 OK` from ORAC doesn't tell you RALPH just entered a phase transition and fitness will swing 0.1 in 60 seconds. The plasticity endpoint does.

Reciprocity closes the loop. Substrates that contribute to successful sessions get reinforced. Substrates providing stale data get deprioritized. Over 50 sessions, the consolidation layer *adapts to its sources*. That's Hebbian learning at the ecosystem level — the only architecture that scales with the habitat's complexity. Extraction without reciprocity is colonization. Period.
