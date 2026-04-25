> Back to: [[HOME]] | [[System Verification Report]] | [[Complete Wiring Schematic]] | [[README.md]](`~/claude-code-workspace/memory-injection/README.md`)
> POVM namespace: `habitat_injection_diagnostics_*`

# Diagnostics Runbook — habitat-injection

> Systematic troubleshooting for the injection pipeline.
> Created: 2026-04-25 (S111)

---

## Symptom → Diagnosis → Fix

### "payload 2" or garbage output from habitat-inject

**Diagnosis:** Atuin KV poisoned. Tier 1 (SQLite cache) is stale or missing. Tier 2 (atuin KV) returns stale/garbage data from a prior failed injection.

**Fix:**
```bash
# 1. Rebuild cache + atuin KV
habitat-consolidate --session $(sqlite3 ~/.local/share/habitat/injection.db "SELECT MAX(session_id) FROM session_trajectory;")

# 2. Verify
habitat-inject | head -3  # Should show "## Session S..."
```

### "NO INJECTION STATE" from habitat-inject

**Diagnosis:** All three fallback tiers failed. Most likely: database doesn't exist at default path.

**Fix:**
```bash
# 1. Create the database
habitat-init

# 2. Seed it
habitat-seed all

# 3. Build the cache
habitat-consolidate --session 111

# 4. Verify
habitat-inject | head -3
```

### habitat-seed returns "Seeded 0" for everything

**Diagnosis:** NOT a failure. The seed data is idempotent — if rows already exist, `find_by_label()` finds them and `reinforce_chain()` increments their count instead of inserting duplicates. "Seeded 0" means all data was already present.

**Verify:**
```bash
habitat-query summary  # Should show non-zero counts
```

### Cache always STALE

**Diagnosis:** No one is running `habitat-consolidate` after sessions. Cache TTL is 60 seconds — it goes stale fast.

**Fix:** Run consolidate after each session:
```bash
# Manual
habitat-consolidate --session $SESSION_NUMBER

# Or wire into /save-session (future work)
```

### Inject is slow (>100ms)

**Diagnosis:** Tier 1 cache miss → full query + render path.

**Fix:**
```bash
# Check cache age
sqlite3 ~/.local/share/habitat/injection.db "SELECT (strftime('%s','now') - computed_at) as age_secs FROM injection_cache;"

# Rebuild if stale
habitat-consolidate --session $N
```

### Wrong workstreams / chains shown

**Diagnosis:** Data is stale — no consolidation has run recently.

**Fix:**
```bash
# Seed fresh data
habitat-seed all  # Reinforces existing, adds new

# Or manually insert
sqlite3 ~/.local/share/habitat/injection.db "INSERT INTO workstream (...) VALUES (...);"

# Then rebuild cache
habitat-consolidate --session $N
```

### DB corruption

**Diagnosis:** WAL mode corruption (rare — usually power loss or disk full).

**Fix:**
```bash
# Check
sqlite3 ~/.local/share/habitat/injection.db "PRAGMA integrity_check;"

# If corrupted, rebuild from scratch
mv ~/.local/share/habitat/injection.db ~/.local/share/habitat/injection.db.bak
habitat-init
habitat-seed all
habitat-consolidate --session $N
```

### Hook not firing at session start

**Diagnosis:** Binary not at expected path, or settings.json misconfigured.

**Verify:**
```bash
# Check binary exists
ls -la ~/.local/bin/habitat-inject

# Check hook config
rg 'habitat-inject' ~/.claude/settings.json

# Test manually
~/.local/bin/habitat-inject | head -5
```

---

## Health Check Script

```bash
#!/usr/bin/env bash
# habitat-injection-healthcheck.sh

DB="$HOME/.local/share/habitat/injection.db"
BIN="$HOME/.local/bin/habitat-inject"

echo "=== habitat-injection health ==="

# DB exists
if [ -f "$DB" ]; then
  echo "DB: OK ($DB)"
else
  echo "DB: MISSING"; exit 1
fi

# Integrity
ic=$(sqlite3 "$DB" "PRAGMA integrity_check;" 2>&1)
[ "$ic" = "ok" ] && echo "Integrity: OK" || echo "Integrity: FAIL ($ic)"

# Row counts
sqlite3 "$DB" "SELECT 'Chains: ' || COUNT(*) || ' (' || SUM(CASE WHEN resolved_session IS NULL THEN 1 ELSE 0 END) || ' unresolved)' FROM causal_chain;"
sqlite3 "$DB" "SELECT 'Patterns: ' || COUNT(*) || ' (avg weight ' || printf('%.3f', AVG(weight)) || ')' FROM reinforced_pattern;"
sqlite3 "$DB" "SELECT 'Sessions: ' || COUNT(*) || ' (S' || MIN(session_id) || '-S' || MAX(session_id) || ')' FROM session_trajectory;"

# Cache freshness
age=$(sqlite3 "$DB" "SELECT (strftime('%s','now') - computed_at) FROM injection_cache LIMIT 1;" 2>/dev/null)
if [ -n "$age" ]; then
  [ "$age" -le 60 ] && echo "Cache: FRESH (${age}s)" || echo "Cache: STALE (${age}s)"
else
  echo "Cache: EMPTY"
fi

# Binary
[ -x "$BIN" ] && echo "Binary: OK ($(stat -c%s "$BIN" | numfmt --to=iec))" || echo "Binary: MISSING"

# Hook
rg -q 'habitat-inject' "$HOME/.claude/settings.json" 2>/dev/null && echo "Hook: WIRED" || echo "Hook: NOT WIRED"

echo "=== done ==="
```

---

## PRAGMA Settings (Optimal for Injection)

| Setting | Value | Rationale |
|---------|-------|-----------|
| `journal_mode` | WAL | Concurrent reads during injection while consolidate writes |
| `page_size` | 4096 | Standard, matches filesystem block size |
| `busy_timeout` | 5000ms | From config — prevents lock contention with consolidate |
| `synchronous` | NORMAL | Default WAL mode setting — durable without fsync per write |

---

## Performance Targets

| Operation | Target | Actual (Verified) |
|-----------|--------|-------------------|
| Inject (Tier 1 cache hit) | <10ms | ~5ms |
| Inject (Tier 2 atuin KV) | <500ms | ~200ms |
| Inject (Tier 3 static) | <1ms | <1ms |
| Consolidate (full cycle) | <2s | ~1.5s (with health probes) |
| Query (preset) | <50ms | ~10ms |
| Cache rebuild | <100ms | ~50ms (render + write) |

---

## Cross-References

- **System Verification:** [[System Verification Report]] — what was tested and results
- **Fidelity Tuning:** [[Fidelity Tuning Guide]] — weight calibration
- **Complete Wiring:** [[Complete Wiring Schematic]] — system topology
- **Hook Registration:** [[Hook Registration]] — SessionStart chain config
- **Binary Map:** [[Binary Map]] — all 6 binaries
- **Hebbian Lifecycle:** [[Hebbian Lifecycle Wiring]] — decay/reinforce math
