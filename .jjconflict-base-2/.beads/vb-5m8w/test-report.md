# Scoped Test Report

STATUS: PASS

- Command: `cargo +nightly nextest run -p vb_core -p vb_runtime -E 'test(/budget|Budget|StepBudgetExhausted|AwaitingAction|AwaitingWait|AwaitingAsk|evidence/)'`
  - Exit status: 0
  - Result: `439 tests run: 439 passed, 3091 skipped`.
- Command: `PROPTEST_CASES=1024 cargo +nightly test -p vb_core -p vb_runtime step_budget -- --nocapture`
  - Exit status: 0
  - Result: selected `step_budget` unit/property tests passed across `vb_core` and `vb_runtime`.
