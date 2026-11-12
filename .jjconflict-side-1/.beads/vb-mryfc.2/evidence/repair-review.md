# Lifecycle Test Repair Evidence

## Candidate and scope

- **Bead:** `vb-mryfc.2`
- **Parent candidate:** `fa5e084c2ff3e25226b002f7bbb727f5530023f1`
- **Workspace:** `/home/lewis/src/isoloated/velvet-ballistics-vb-mryfc-lifecycle-luna`
- **Scope:** integration-test oracle repairs only; production behavior was not edited.

## Changes

1. Added a five-step deterministic `SetConst -> SetConst -> SetConst -> SetConst -> Finish` workflow. The public engine drive test runs it with step budgets 1 and 2, asserting exact per-drive successful-step counts (`[1,1,1,1,1]` and `[2,2,1]`), `StepBudgetExhausted` before completion, and final `Finished(I64(99))` plus exact final slot value.
2. Replaced idle-tick terminal checks with stale genuine action-ticket scenarios. One ticket is issued before cancellation and another before successful completion. Replaying each after terminal cleanup is processed as exact `RunNotFound`; counters and trace history remain unchanged.
3. Kept the existing runtime action-budget scenario and its canonical ticket completion intact.

The counterfactual changed the test's supplied `StepBudget` to `u64::MAX`. The test failed as intended at budget 1: it observed five successful steps instead of one. The source was restored before the final focused runs.

## Verification

- Orchestration integration: **30 passed, 0 failed, 0 ignored**.
- Lifecycle integration: **40 passed, 0 failed, 0 ignored**.
- Scoped rustfmt check and `git diff --check`: pass.
- Package-wide `cargo fmt -p velvet-ballistics-workspace-tests -- --check`: reports formatting drift in unrelated existing integration tests; those files were not changed.

Raw command output is stored in this directory. The bead remains open for independent review of the updated exact candidate SHA.
