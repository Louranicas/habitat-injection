> Back to: [[HOME]] · [[MASTER INDEX]]

# Hook Registration

## SessionStart Hook Chain

`habitat-inject` registers as position 3 in the SessionStart hook chain in `~/.claude/settings.json`:

```
Position 1: orac-hook.sh SessionStart   → ORAC /hooks/SessionStart
Position 2: session-health-broadcast.sh → health pulse to atuin KV
Position 3: habitat-inject              → <2KB causal state injection
```

## Configuration

```json
{
  "hooks": {
    "SessionStart": [
      {
        "command": "~/.local/bin/habitat-inject",
        "timeout": 3000,
        "type": "command"
      }
    ]
  }
}
```

## Timeout

The hook has a 3-second timeout. The injection pipeline targets <100ms. The generous timeout accommodates:
- Cold SQLite connection (~50ms first time)
- Health probe timeouts (if services are down)
- atuin KV fallback path

If the hook times out, Claude Code starts without injected state — the three-tier fallback ensures a static payload is returned before timeout.
