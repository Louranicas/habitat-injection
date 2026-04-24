> Back to: [[HOME]] · [[MASTER INDEX]]

# Tool Chain Patterns

Extends the existing B1-B26 + TC1-TC5 with 5 new patterns specific to habitat-injection.

## TC6 — Inject-Verify

```
habitat-inject | head -20     # see what would be injected
sqlite3 injection.db "SELECT section, token_count FROM injection_cache"
# verify: total tokens < 1100, all sections present
```

## TC7 — Consolidate-Inspect

```
habitat-consolidate --session 109
sqlite3 -header -column injection.db \
  "SELECT label, reinforcement_count, resolved_session IS NULL as active FROM causal_chain ORDER BY reinforcement_count DESC LIMIT 10"
```

## TC8 — Chain Investigation

```
habitat-query chains           # preset: unresolved chains by frequency
habitat-query raw "SELECT label, reinforcement_count, origin_session FROM causal_chain WHERE label LIKE 'BUG-%' AND resolved_session IS NULL"
```

## TC9 — Trajectory Trend

```
habitat-query trajectory       # preset: last 10 trajectory points
sqlite3 -header -column injection.db \
  "SELECT session_id, fitness, field_r, thermal_t, delta_summary FROM session_trajectory ORDER BY session_id DESC LIMIT 10"
```

## TC10 — Hebbian Audit

```
habitat-query patterns         # preset: top 20 by weight
sqlite3 -header -column injection.db \
  "SELECT label, weight, hit_count, last_fired_session FROM reinforced_pattern WHERE weight > 0.1 ORDER BY weight DESC"
```
