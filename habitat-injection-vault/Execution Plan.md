> Back to: [[HOME]] · [[MASTER INDEX]] · [[Implementation Status]]

# Execution Plan — Phase 1 CLI + Deployment

> **Status:** PENDING — library complete, CLI binaries next
> **Canonical doc:** `EXECUTION_PLAN.md` (project root)
> **Schematics:** [[CLI Binary Architecture]] · [[Deployment Checklist]]
> **Estimated:** ~20h across 4-5 sessions

## The 4 CLI Binaries

| Binary | Purpose | Dependencies | Session |
|--------|---------|-------------|---------|
| `habitat-init` | One-time DB setup | L1, L2 | S110 |
| `habitat-inject` | SessionStart hook (<2KB, <100ms) | L1-L3 | S110-S112 |
| `habitat-consolidate` | Post-session Hebbian write-back | L1-L4 | S112 |
| `habitat-query` | Interactive memory browser | L1-L2, L5 | S113 |

## Session Schedule

| Session | Steps | Gate |
|---------|-------|------|
| S110 | habitat-init + habitat-inject (partial) | DB creates, inject produces output |
| S111 | Data seeding (chains, trajectory, workstreams, patterns) | `habitat-query summary` shows data |
| S112 | habitat-inject complete + habitat-consolidate | Full 3-tier fallback works |
| S113 | habitat-query + hook wiring + atuin registration | Hook fires on session start |
| S114 | 5-session validation begins | Metrics tracked |

## Acceptance Criteria

| Metric | Target |
|--------|--------|
| Injection latency | <100ms |
| Injection size | <2KB |
| Re-discovered traps | 0 per session |
| Patterns reinforced | ≥3 per session |
| Decay prunes ≥1 | Below 0.05 threshold |

## Integration Touchpoints

| System | Change |
|--------|--------|
| `~/.claude/settings.json` | Hook 3 → `habitat-inject` |
| `CLAUDE.md § Memory Systems` | Add row 7: Injection DB |
| `/save-session` skill | Call `habitat-consolidate` as surface 8 |
| 4 atuin scripts | `habitat-init`, `habitat-inject`, `habitat-consolidate`, `habitat-query` |
| POVM | `habitat_injection_*` namespace (plan + phases + gates) |

---

See: [[CLI Binary Architecture]] · [[Deployment Checklist]] · [[Implementation Status]]
