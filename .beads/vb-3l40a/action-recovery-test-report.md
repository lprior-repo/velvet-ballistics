# vb-3l40a Action Recovery Test Report

**Bead:** vb-3l40a
**Test Date:** 2026-10-21
**Commit:** da633fe0b
**Environment:** Local5090 isolated workspace `/home/lewis/src/isoloated/velvet-ballistics-vb-3l40a-runtime-5090`

## Test Objective

Verify that crash recovery via `handle_recover` correctly restores action contracts
from the accepted artifact, enabling contract validation to succeed on action
completion. Regression test for the bug where recovery discarded action contracts
(`Box::new([])`), causing the first action's validation to fail.

## Test Design

The test `recover_restores_action_contracts_and_completes` uses the following design:

1. **Custom Artifact Store:** Constructs a `ContractStore` that serves an
   `AcceptedArtifact` with non-empty `action_contracts` (one `ActionContract` for
   action ID 0 with name "recovery-action").

2. **Minimal Workflow:** Builds a `CompiledWorkflow` with two nodes:
   - Step 0: `Do` node (action ID 0, input slot 0, output slot 0)
   - Step 1: `Finish` node (result slot 0)

3. **Recovery Flow:** Calls `shard.handle_recover()` with a `RecoverRunCommand`:
   - Run ID: `RunId::new(1)`
   - Frame: `RunFrame::new(run_id, StepIdx::ZERO, 2, 2)`
   - Artifact and workflow digests: `[42u8; 32]`
   - Boundary: `RecoveredRunBoundary::none()` (no suspended action to reinject)

4. **Assertion:** Verifies `handle_recover` returns `Ok`, proving action contracts
   were restored. If contracts were not restored (`Box::new([])`), the action
   completion validation during `drive_run` would fail, and `handle_recover` would
   return an error.

## Test Execution

**Command:**
```bash
timeout 300 env RUSTC_WRAPPER= CARGO_BUILD_JOBS=2 cargo test -p vb_runtime --lib vb_3l40a_action_recovery_tests
```

**Result:**
```
running 1 test
test shard::lifecycle::vb_3l40a_action_recovery_tests::recover_restores_action_contracts_and_completes ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1877 filtered out; finished in 0.00s
```

**Full Suite:**
```bash
timeout 600 env RUSTC_WRAPPER= CARGO_BUILD_JOBS=2 cargo test -p vb_runtime --lib
```

**Result:**
```
test result: ok. 1878 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.12s
```

## Assertions Verified

1. `handle_recover` returns `Ok` — action contracts were restored from the accepted artifact
2. Action completion validation passes during recovery drive — the restored contracts are functional
3. Full 1878-test suite passes — no regressions introduced

## Test Location

Test module: `vb_3l40a_action_recovery_tests`
File: `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` (appended inline test module)

## Notes

- The test uses a custom `ContractStore` implementation that overrides the
  `AlwaysPresentArtifactStore` behavior to return an artifact with non-empty
  action contracts. This is necessary because `AlwaysPresentArtifactStore`
  returns empty `action_contracts: Box::new([])`, which would not exercise the
  regression.
- The test validates the recovery path end-to-end through the public shard API,
  not through internal state inspection. If action contracts are not restored,
  the run will error during `drive_run` and the test will fail.
- This test complements the existing 8 recovery/contract validation tests by
  covering the specific regression identified in the code review: action contracts
  discarded during recovery.
