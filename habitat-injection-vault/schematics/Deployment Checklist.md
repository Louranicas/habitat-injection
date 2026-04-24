> Back to: [[HOME]] · [[Execution Plan]] · [[CLI Binary Architecture]]

# Deployment Checklist

Canonical checklist at `schematics/DEPLOYMENT_CHECKLIST.md` (project root).

## Summary

11 steps from library-complete to production:

1. `habitat-init` binary — creates injection.db (6 tables, schema v2)
2. Seed causal_chain from 108 session notes (~15-25 rows)
3. Seed trajectory from CLAUDE.local.md metrics (~10 rows)
4. Seed workstreams from CLAUDE.local.md priorities (~6 rows)
5. Seed patterns from service_tracking.db learned_patterns (~141 rows)
6. `habitat-inject` binary — SessionStart hook (<2KB, <100ms)
7. `habitat-consolidate` binary — post-session Hebbian write-back
8. `habitat-query` binary — interactive memory browser
9. Wire Hook 3 in `~/.claude/settings.json`
10. Register 4 atuin scripts
11. 5-session validation against acceptance criteria

## Acceptance Gate

| Metric | Target |
|--------|--------|
| Injection latency | <100ms |
| Injection size | <2KB |
| Re-discovered traps | 0/session |
| Patterns reinforced | ≥3/session |
| Decay prunes ≥1 pattern | weight < 0.05 |

---

See: [[Execution Plan]] · [[Implementation Status]]
