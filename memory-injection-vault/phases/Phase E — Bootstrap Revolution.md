> Back to: [[HOME]]

# Phase E — Bootstrap Revolution (6-8h, 1-2 sessions)

**The payoff.** Claude Code gets <100ms injection of complete causal state.

## Deliverables
- `habitat-stdb-inject` CLI binary (via `spacetime sql` one-shot, per [[Gap Analysis — Conventional#I2]])
- ORAC SessionStart hook integration: calls injector, includes ≤15 KB payload in system message
- `atuin scripts run habitat-bootstrap-stdb` — replacement script
- NA-R8 adaptive payload: role-based + Watcher-priority + field-state weighting
- Retirement of 11 dead tracking databases
- E2E verification script
- Old `habitat-bootstrap` preserved as fallback

## The 11 Dead DBs to Delete

`bus_tracking.db`, `code.db`, `devenv_tracking.db`, `episodic_memory.db`, `evolution_tracking.db`, `povm_data.db`, `povm_engine.db`, `security_tracking.db`, `synergy_tracking.db`, `tensor_memory.db`, `workflow_tracking.db`

## Acceptance
- New session receives ≤15 KB injection with trajectory + workstreams + traps + causal chain + top patterns
- Injection latency: <100ms end-to-end
- Old `habitat-bootstrap` still works as fallback
- 11 dead databases deleted

---

See: [[Bootstrap Chain — Current vs Target]] · [[Injector — Context Window Bootstrap]]
