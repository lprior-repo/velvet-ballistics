STATUS: PASS

# Machine Gate Report

Commands:
- `cargo +nightly fmt --check -- crates/vb_runtime/src/runtime.rs` -> pass.
- `rtk cargo test -p vb_runtime tick_shard_` -> `4 passed, 1526 filtered out`.
- `moon ci --force --summary normal` -> pass after rebase; summary: `Actions: 23 completed`, `Time: 42s 550ms`.
