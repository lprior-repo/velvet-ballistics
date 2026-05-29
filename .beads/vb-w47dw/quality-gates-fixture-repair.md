# vb-w47dw Quality Gates Fixture Repair

## Scope

Repaired the synthetic workspace fixture in
`crates/workspace_tests/tests/vb_qi37_25_quality_gates.rs` so its baseline
matches the current sharpened workspace assertion contract.

## Change

- Added `crates/vb_test_util` to the fixture `MEMBERS` list.
- Added `kani-diagnostic-codes = []` to the fixture `vb_core` feature set.
- Updated the feature-drift expected error string to include the now-required
  `kani-diagnostic-codes` feature.

## Evidence

Commands run from `/home/lewis/src/velvet-ballistics`:

```text
rustup run nightly-2026-04-28 rustfmt --edition 2024 --check crates/workspace_tests/tests/vb_qi37_25_quality_gates.rs
PASS

rtk cargo test -p velvet-ballistics-workspace-tests --test vb_qi37_25_quality_gates -- --nocapture
PASS: cargo test: 2 passed, 1 ignored (1 suite, 0.10s)

rtk cargo nextest run -p velvet-ballistics-workspace-tests --test vb_qi37_25_quality_gates
PASS: cargo nextest: 2 passed, 1 skipped (1 binary, 0.098s)
```

## Residual risk

This closes the local quality-gate fixture drift exposed by `moon ci`. Full CI
still has unrelated source-length and long-running test blockers.
