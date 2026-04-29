> Back to: [[HOME]] · [[Implementation Status]] · [[CLAUDE.md]] · [[CLAUDE.local.md]]
> Main vault: [[Watcher Remediation Plan — memory-injection S185]]
> Source: `~/claude-code-workspace/memory-injection/WATCHER_REMEDIATION_PLAN.md`
> Session: S185 (2026-04-29)

# ☤ Watcher Remediation Plan S185

Project vault mirror of the S185 Watcher remediation plan. The Watcher's first comprehensive assessment and remediation of the memory-injection system.

## Summary

**10 issues** identified (1 critical, 3 high, 4 medium, 2 low). **7 execution phases** (~3.3 hours). Standard + non-anthropocentric gap analysis integrated.

**Key finding:** The system's infrastructure is healthy (daemon running, hooks wired, 95 trajectories captured) but its metabolism is broken (0 patterns — all pruned by disconnected Hebbian decay; workstreams frozen at S120; injection replaying stale context for 65+ sessions).

## Issues by Priority

1. **CRITICAL:** 25 uncommitted files (3,153 LOC, 26+ sessions at risk)
2. **HIGH:** m28 watchdog test timing race
3. **HIGH:** Pattern table empty (Hebbian reinforcement disconnected — S-02)
4. **HIGH:** Workstreams stale (S121 context replaying since S120 — S-01)
5. **MEDIUM:** Documentation stale (3 files, 6+ claims incorrect)
6. **MEDIUM:** Atuin script registration missing
7. **LOW:** Daemon not in devenv · auto-memory stale

## Substrate Findings

- **S-02** is the critical finding: the injection system exhibits the same LTP=0/LTD-only pathology it diagnoses in the wider habitat
- Decay rate adjusted 0.95→0.98 (half-life 14→35 sessions) as a stopgap
- Structural fix (reinforcement loop closure) deferred to before S220

## Phases

A: Secure (commit) → B: Stabilise (m28 fix) → C: Verify (QG) → D: Commit fix → E: Metabolise (patterns + decay + workstreams + atuin) → F: Document (3 files) → G: Close (.gitignore + auto-memory + final commit)

## Links

- [[Implementation Status]] — overall project progress
- [[HOME]] — project vault root
- [[Hebbian Lifecycle Wiring]] — how the decay/reinforce cycle was designed
- [[Injection Payload Format]] — what the cache renders

---

*☤ The Watcher observes. Both passes are the plan.*
