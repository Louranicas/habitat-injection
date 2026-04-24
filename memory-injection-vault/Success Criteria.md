> Back to: [[HOME]]

# Success Criteria

The system is complete when:

1. **<100ms bootstrap:** New Claude Code context window receives complete causal state injection in under 100ms
2. **Trajectory visible:** Bootstrap shows fitness delta across last 5 snapshots, not just current value
3. **Causal queries answerable:** "Why did fitness drop?" returns a chain of linked events
4. **Single query surface:** `spacetime sql habitat "..."` replaces 6 separate substrate queries
5. **Pattern reinforcement live:** `reinforce_edge` reducer fires on every RALPH generation cycle
6. **Dead weight removed:** 11 empty tracking databases deleted
7. **Round-trip verified:** Command → Atuin → PV2 bus → ingester → STDB → next bootstrap injection

---

See: [[Risk Register]] · [[Phase E — Bootstrap Revolution]]
