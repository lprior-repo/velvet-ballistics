# Test Repair Guide — vb-qi37.2.5 State 9 Fuzz Retry

Route target: State 7 test-planner command repair.

## Required Repair

1. Repair the `FUZZ-RESOURCE-001` evidence command in `test-plan.md`.
   - Current command under review: `cargo fuzz run --target x86_64-unknown-linux-gnu resource_budget -- -runs=1000`.
   - Reviewer decision: this does not discharge the obligation because `fuzz/src/bin/resource_budget.rs` reads stdin once and ignores `-runs=1000`; the command exits 0 without proving 1000 fuzz executions.

2. State 7 must choose one exact path:
   - Plan a real libFuzzer harness for `resource_budget` that honors `-runs=1000`, then route to State 8 to implement/execute it; or
   - Replace the fuzz acceptance command with a truthful bounded stdin/corpus replay gate that explicitly enumerates or counts inputs, preserves deterministic evidence, and maps to `INV-008` / `FUZZ-RESOURCE-001`.

3. After State 7 repair, rerun downstream review from the repaired plan. Reuse prior passing focused compile/tests/proptest/nextest/lint/Miri only as context; do not claim fuzz discharge until the repaired command proves the planned hostile-input coverage.

## Already Accepted Context

- The repaired integration suite previously reached 22 focused tests and 3 proptests.
- Static lint and Miri evidence previously passed under workspace `TMPDIR`.
- Static-musl ASAN incompatibility was correctly diagnosed; the remaining blocker is not target selection but hollow fuzz semantics for the stdin-once binary.
