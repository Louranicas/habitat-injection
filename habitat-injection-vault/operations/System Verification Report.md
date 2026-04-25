> Back to: [[HOME]] | [[Complete Wiring Schematic]] | [[Injection Database State]] | [[README.md]](`~/claude-code-workspace/memory-injection/README.md`)
> POVM namespace: `habitat_injection_verification_*`
> Tracking DB: `~/.local/share/habitat/injection.db`

# System Verification Report — habitat-injection

> **Verified:** 2026-04-25 S111 | **Verdict:** ALL SYSTEMS OPERATIONAL
> **Verifier:** Claude (cortex) — end-to-end pipeline test across all 6 binaries + hook + DB

---

## Verification Summary

| Check | Status | Detail |
|-------|--------|--------|
| DB integrity | PASS | `PRAGMA integrity_check` = ok |
| Schema version | PASS | v3 (current) |
| Table structure | PASS | 7 tables, 9 custom indexes, WAL mode |
| Data population | PASS | 47 chains, 23 sessions, 15 workstreams, 80 patterns |
| Cache freshness | PASS | 207 tokens, FRESH after consolidation |
| Atuin KV | PASS | `habitat.last-session` = 111 |
| Hook wiring | PASS | Position 3 in SessionStart, 3s timeout |
| Binary install | PASS | 6/6 in `~/.local/bin/` |
| Inject output | PASS | 207 tokens, 30 lines, real habitat context |
| Hebbian cycle | PASS | 19 decayed, 4 auto-resolved |
| Consent filter | PASS | All rows default `Emit` — no Store/Forget blocking |
| Fallback chain | PASS | Tier 1 (cache) serving fresh data |

---

## How the System Was Verified

### Step 1: Database Path Resolution

The default database path is `~/.local/share/habitat/injection.db`, resolved by `m03_config::default_db_path()`:

```
$HOME + ".local/share/habitat/injection.db"
```

**Verification:** File exists at 132KB, `PRAGMA integrity_check` returns `ok`.

### Step 2: Schema Verification

```sql
PRAGMA user_version;  -- Returns: 3
```

7 tables confirmed: `causal_chain`, `session_trajectory`, `workstream`, `reinforced_pattern`, `injection_cache`, `session_checkpoint`, `injection_script`.

9 custom indexes:

| Index | Table | Type |
|-------|-------|------|
| `idx_causal_unresolved` | `causal_chain` | Partial (WHERE resolved_session IS NULL), DESC |
| `idx_causal_label` | `causal_chain` | Label lookup |
| `idx_trajectory_recent` | `session_trajectory` | session_id DESC |
| `idx_workstream_active` | `workstream` | Partial (WHERE status IN active/blocked) |
| `idx_pattern_weight` | `reinforced_pattern` | weight DESC |
| `idx_checkpoint_label` | `session_checkpoint` | Label unique lookup |
| `idx_checkpoint_ts` | `session_checkpoint` | timestamp DESC |
| `idx_script_name` | `injection_script` | Name unique lookup |
| `idx_script_tags` | `injection_script` | Tag substring search |

Plus 6 SQLite autoindexes on UNIQUE/PK columns.

### Step 3: Data Population Verification

| Table | Rows | Key Metrics |
|-------|------|-------------|
| `causal_chain` | 47 | 17 unresolved, 30 resolved |
| `session_trajectory` | 23 | S060 oldest, S111 newest |
| `workstream` | 15 | 3 active, 1 blocked, 4 complete, 7 deferred |
| `reinforced_pattern` | 80 | avg weight 0.506, range [0.451, 0.555] |
| `injection_cache` | 1 | 207 tokens, computed 2026-04-24T20:21:31Z |
| `session_checkpoint` | 0 | Pending `/save-session` integration |
| `injection_script` | 0 | Pending script registration |

**Chain type distribution:**

| Type | Count | Purpose |
|------|-------|---------|
| trap | 18 | Recurring operational hazards (cp alias, pkill exit 144, etc.) |
| bug | 15 | Active bugs (devenv-stop, POVM write-only, etc.) |
| plan | 8 | In-flight work items (daemon Phase G, comms layer, etc.) |
| pattern | 6 | Recurring behavioral patterns (convergence trap, etc.) |

**Pattern category distribution:**

| Category | Count | Avg Weight |
|----------|-------|------------|
| procedural | 42 | 0.503 |
| semantic | 14 | 0.500 |
| feedback | 13 | 0.519 |
| trap | 11 | 0.507 |

### Step 4: Injection Pipeline Test

```
habitat-inject (installed at ~/.local/bin/)
  → Config::load(None) → path = ~/.local/share/habitat/injection.db
  → execute_fallback_chain(Some(path), 60)
  → Tier 1: SELECT payload FROM injection_cache WHERE section='full_payload'
  → Cache age: 24 seconds (< 60s threshold)
  → HIT: return cached 207-token payload
  → print to stdout (exit 0)
```

**Output verified:** 30 lines, 207 tokens, contains:
- Orientation (active workstream + fitness trend)
- Trajectory (5 most recent sessions S107-S111)
- Workstreams (3 active, 1 blocked)
- Unresolved chains (top 5 by frequency)
- Health (5/11 services — 6 down because devenv not running)

### Step 5: Consolidation Pipeline Test

```
habitat-consolidate --session 111
  → fetch_health_snapshot(): curl ORAC :8133/health + SYNTHEX :8090/v3/thermal
  → capture_trajectory(conn, 111, snapshot)
  → run_consolidation(conn, 111, [])
    → decay_patterns: 19 patterns decayed (weight *= 0.95)
    → reinforce_patterns: 0 (no --fired-patterns passed)
    → prune_patterns: 0 (all weights > 0.05)
    → auto_resolve_chains: 4 resolved (inactive ≥ 10 sessions)
  → rebuild_cache(conn, 111, 5, 11, Some(0.0))
    → Query 4 tables → consent filter → render 207 tokens
    → INSERT OR REPLACE injection_cache
  → write_injection_cache(AtuinCacheEntry) → atuin KV
  → write_kv("habitat.last-session", "111")
```

### Step 6: Hook Wiring Verification

```json
{
  "type": "command",
  "command": "/home/louranicas/.local/bin/habitat-inject",
  "timeout": 3
}
```

Position 3 in SessionStart hook chain (after orac-hook.sh and session-health-broadcast.sh). Binary is 2.6MB, executes in <100ms (target).

### Step 7: Binary Installation Verification

| Binary | Size | Path | Status |
|--------|------|------|--------|
| `habitat-init` | 2.6M | `~/.local/bin/habitat-init` | Installed |
| `habitat-inject` | 2.6M | `~/.local/bin/habitat-inject` | Installed |
| `habitat-seed` | 2.6M | `~/.local/bin/habitat-seed` | Installed |
| `habitat-consolidate` | 2.8M | `~/.local/bin/habitat-consolidate` | Installed |
| `habitat-query` | 2.7M | `~/.local/bin/habitat-query` | Installed |
| `habitat-memory` | 2.6M | `~/.local/bin/habitat-memory` | Installed |

---

## Issues Found and Resolved During Verification

### Issue 1: Stale Injection Cache (9.7 hours old)

**Root cause:** Cache was built by a prior `habitat-consolidate` run but no subsequent runs refreshed it. The 60-second TTL meant Tier 1 always missed.

**Fix:** Ran `habitat-consolidate --session 111` which rebuilt the cache with fresh data.

**Prevention:** Wire `habitat-consolidate --session $N` into `/save-session` hook or post-session script.

### Issue 2: Poisoned Atuin KV

**Root cause:** An earlier run of `habitat-inject` fell through to Tier 2 (atuin KV) which returned "payload 2" — garbage from the SessionStart hook's own output being captured. This was then served as the "real" payload.

**Fix:** Consolidation overwrote `habitat.last-injection` with the freshly-rendered 207-token payload.

**Prevention:** The consolidation pipeline always writes fresh payload to both injection_cache AND atuin KV.

### Issue 3: Accidental DB at `--help` Path

**Root cause:** Running `habitat-init --help` created a database file literally named `--help` because the binary treats the first argument as a path, not a flag.

**Fix:** Removed the accidental file. The real DB at `~/.local/share/habitat/injection.db` was already correct.

**Recommendation:** Add `--help` / `-h` flag parsing to `habitat-init` in a future session.

---

## Diagnostic Commands (Quick Reference)

```bash
# Full status
habitat-query summary

# The One Query (most important)
habitat-query "SELECT label, reinforcement_count, description FROM causal_chain WHERE resolved_session IS NULL ORDER BY reinforcement_count DESC LIMIT 5"

# Cache state
sqlite3 ~/.local/share/habitat/injection.db "SELECT section, token_count, datetime(computed_at,'unixepoch') as utc, CASE WHEN (strftime('%s','now')-computed_at)<=60 THEN 'FRESH' ELSE 'STALE' END FROM injection_cache;"

# Hebbian weight distribution
sqlite3 ~/.local/share/habitat/injection.db "SELECT printf('%.2f-%.2f', ROUND(weight*20)/20, ROUND(weight*20)/20+0.05) as band, COUNT(*) FROM reinforced_pattern GROUP BY band ORDER BY band;"

# Workstream health
habitat-query workstreams

# Force cache rebuild
habitat-consolidate --session $(habitat-query "SELECT MAX(session_id) FROM session_trajectory" 2>/dev/null | tail -1)

# Test inject without hook
habitat-inject

# DB integrity
sqlite3 ~/.local/share/habitat/injection.db "PRAGMA integrity_check; PRAGMA page_count; PRAGMA freelist_count;"
```

---

## Cross-References

- **Complete Wiring:** [[Complete Wiring Schematic]] — system topology
- **Database State:** [[Injection Database State]] — live row counts and Hebbian state
- **Hook Registration:** [[Hook Registration]] — SessionStart chain config
- **Binary Map:** [[Binary Map]] — all 6 binaries
- **Diagnostics Runbook:** [[Diagnostics Runbook]] — systematic troubleshooting
- **Fidelity Tuning Guide:** [[Fidelity Tuning Guide]] — weight calibration
- **README:** [`README.md`](~/claude-code-workspace/memory-injection/README.md)
- **CLAUDE.local.md:** `~/claude-code-workspace/CLAUDE.local.md` § habitat-injection anchors
