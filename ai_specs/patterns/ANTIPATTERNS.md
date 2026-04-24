# habitat-injection — Anti-Patterns

## NEVER

1. `unwrap()`/`expect()` in production code
2. `unsafe` blocks
3. `stdout` in daemon binaries (SIGPIPE death)
4. Global mutable state (use `Arc<RwLock<T>>`)
5. Suppressing clippy warnings (`#![allow]`) — fix the code
6. Panic-based error handling
7. `mod.rs` that re-exports everything blindly
8. Chaining after `pkill` (exit 144 kills `&&` chains)
9. `cp` without `\` prefix (aliased to interactive)
10. `git status -uall` (memory explosion on large repos)
