# Regression Diff — vb-f7k6 State 11 Retry

STATUS: NO_REGRESSION

## Baseline

- `.beads/vb-f7k6/baseline-report.md` records canonical `moon ci` at shared parent `ysnxntql cc80fac3` with exit 0, `Tasks: 23 completed`.

## Current Rerun

- `tlc -config verification/tla/TimerWheel.cfg verification/tla/TimerWheel.tla`: PASS
- `cargo xtask loom --model timer_fired_cancel`: PASS
- `/usr/bin/env cargo test -p vb_runtime timer`: PASS
- `/usr/bin/env cargo check --workspace --all-targets --all-features`: PASS
- `/usr/bin/env moon ci`: PASS, `Tasks: 23 completed`, `Time: 36s 977ms`.

## New Failure Versus Baseline

None. The prior `lint-src` panic regression in `crates/vb_runtime/src/shard/tests/chunk_001.rs` is repaired and canonical CI passes.
