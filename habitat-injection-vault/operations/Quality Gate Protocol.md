> Back to: [[HOME]] · [[MASTER INDEX]]

# Quality Gate Protocol

## 4-Stage Zero-Tolerance Gate

```bash
cargo check && \
cargo clippy -- -D warnings && \
cargo clippy -- -D warnings -W clippy::pedantic && \
cargo test --lib
```

Every stage must pass. No exceptions. Run before every commit.

## Rules

| Rule | Enforcement |
|------|------------|
| No `unwrap()` outside tests | `[lints.clippy] unwrap_used = "deny"` |
| No `expect()` outside tests | `[lints.clippy] expect_used = "deny"` |
| No `unsafe` | Zero tolerance |
| Pedantic clippy clean | `-W clippy::pedantic` |
| Doc comments on public items | Convention (not enforced by lint) |
| 50+ tests per module | Convention (tracked in [[Implementation Status]]) |

## Lint Configuration (Cargo.toml)

```toml
[lints.clippy]
pedantic = { level = "warn", priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
```
