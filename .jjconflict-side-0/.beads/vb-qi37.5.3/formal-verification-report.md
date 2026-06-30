# Formal Verification Report - vb-qi37.5.3

STATUS: APPROVED

## Commands

```bash
rtk cargo fmt --check
rtk cargo test -p vb_runtime admission::tests::admit_artifact_run
rtk cargo test -p vb_storage admission::tests::submit_artifact
rtk cargo test -p vb_runtime -p vb_storage --lib admission::tests
rtk cargo clippy -p vb_runtime -p vb_storage --lib -- -D warnings -D clippy::unwrap_used -D clippy::panic -D clippy::expect_used
rtk cargo kani -p vb_compile --harness idempotency_gate_parity
moon ci
```

## Results

- `rtk cargo fmt --check`: PASS.
- `rtk cargo test -p vb_runtime admission::tests::admit_artifact_run`: PASS, 7 passed.
- `rtk cargo test -p vb_storage admission::tests::submit_artifact`: PASS, 7 passed.
- `rtk cargo test -p vb_runtime -p vb_storage --lib admission::tests`: PASS, 49 passed.
- Source clippy: PASS, no issues found.
- Kani all-45 idempotency parity: PASS, `VERIFICATION:- SUCCESSFUL`, 1 harness verified.
- `moon ci`: PASS, 20 tasks completed in 4m 48s 923ms.

## Classified Non-Blocking Evidence

- `rtk cargo clippy -p vb_runtime -p vb_storage --all-targets ...`: FAIL_REGRESSION/DEFERRED_GLOBAL due pre-existing test lint debt in `crates/vb_storage/tests/*`, not touched source. Source-only clippy passed and `moon ci` passed.
