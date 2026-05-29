# vb-y13m6 formatting repair evidence

## Scope

Repair current Moon formatting drift.

## Changes

- Ran rustfmt on `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs`.
- No semantic code changes intended; rustfmt-only reflow.

## Command evidence

- `moon run velvet-ballistics:fmt`
  - BEFORE: FAIL, reported formatting drift in
    `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs`.
- `rustup run nightly-2026-04-28 rustfmt --edition 2024 crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs`
  - PASS: completed with no output.
- `moon run velvet-ballistics:fmt`
  - AFTER: PASS, task completed in 1.366s.

## Residual risk

Full `moon ci` still needs a fresh rerun after remaining global blockers are
handled. This bead only covers the formatting lane.
