# Test Writer Report: vb-5m8w StepBudget Exhaustion

## Startup Doctrine Cited
- `/home/lewis/.claude/skills/test-writer/SKILL.md`: behavior-first tests, exact assertions, public API integration, proptest/Kani gates, and executable report evidence.
- `/home/lewis/.agents/skills/test-writer/SKILL.md`: same content; controlling file if conflict appears. No conflict found.

## Artifacts Written
- `crates/vb_core/tests/vb_5m8w_step_budget_suspension.rs`
  - 8 concrete tests for clamp, zero `try_take`, positive decrement, repeated exhaustion, zero-budget `run_until_blocked`, completed-step durability, and fresh-budget resume.
  - 3 proptest invariants for any-u64 clamp, positive decrement, and repeated zero exhaustion.
- `crates/vb_runtime/tests/vb_5m8w_step_budget_suspension_runtime.rs`
  - 6 integration tests for zero-budget evidence absence, completed effects surviving later exhaustion, AwaitingWait/AwaitingAsk distinction with no false success evidence, exact AwaitingAction suspension, shard lifecycle keep-run/continue semantics, and terminal invalid-resume exact public error coverage.
- `.beads/vb-5m8w/test-plan.md`
  - Added explicit `WAIVED-TEST-vb-5m8w-StepCounterOverflow-001` with clause ID, reason, owner, expiry, compensation, and downstream audit command.

## Failing-First / Red Evidence
- Initial compile of the new tests failed before repair, proving the new harnesses were being executed:
  - Command: `cargo +nightly test -p vb_core -p vb_runtime step_budget -- --nocapture`
  - Failure: `E0308` on `RunFrame::step_state` exact assertions expecting `Result<StepState, CoreError>`; `E0004` on non-exhaustive `InspectResponse` match.
- Runtime external-suspension test then failed against real behavior before test data repair:
  - Command: `cargo +nightly test -p vb_core -p vb_runtime step_budget -- --nocapture`
  - Failure: `Error: "type mismatch: expected prompt, found number"` for `Ask`; fixed by supplying `SlotValue::Symbol(SymbolId::new(1))` for the ask scenario. No production behavior changed.
- Stop-condition rationale: after test-data/compile fixes, tests were green because the proof-writer repair had already implemented the `StepBudget`/drive/lifecycle hooks. No production implementation behavior was edited.

## Command Evidence
- Attempt 2 rejection/red evidence:
  - `.beads/vb-5m8w/test-plan-review.md`: `STATUS: REJECTED`; missing explicit executable `StepCounterOverflow` test or waiver.
  - `.beads/vb-5m8w/test-suite-review.md`: `STATUS: REJECTED`; missing `AwaitingAction`, terminal invalid-resume exact error variant, and `StepCounterOverflow` test/waiver coverage.
- Static scan after repair:
  - `! rg -n 'assert!\(.*\.is_(ok|err)\(\)|let _ =|\.ok\(\);|#\[ignore\]|sleep\(' 'crates/vb_core/tests/vb_5m8w_step_budget_suspension.rs' 'crates/vb_runtime/tests/vb_5m8w_step_budget_suspension_runtime.rs'`
  - Result: no matches.
- StepCounterOverflow reachability audit:
  - `rtk grep -n 'StepBudget \{|remaining:|pub .*StepBudget|StepCounterOverflow' 'crates/vb_core/src/engine/signals.rs' 'crates/vb_core/src/frame.rs' 'crates/vb_core/tests/vb_5m8w_step_budget_suspension.rs'`
  - Result: 7 matches in production files only: private `StepBudget { remaining: u64 }`, valid/clamped assignments, defensive `EngineError::StepCounterOverflow`, and unrelated `RunFrame::increment_executed` `CoreError::StepCounterOverflow`; no supported invalid `StepBudget` constructor found.
- Format check after repair:
  - `cargo +nightly fmt -p vb_runtime -- --check`
  - Result: pass after formatting the runtime test file.
- New runtime test binary:
  - `cargo +nightly test -p vb_runtime --test vb_5m8w_step_budget_suspension_runtime -- --nocapture`
  - Result after attempt 2 repair: `6 passed; 0 failed`.
- New core test binary:
  - `cargo +nightly test -p vb_core --test vb_5m8w_step_budget_suspension -- --nocapture`
  - Result: `11 passed; 0 failed`.
- Scoped integration/evidence gate:
  - `cargo +nightly nextest run -p vb_core -p vb_runtime -E 'test(/budget|Budget|StepBudgetExhausted|AwaitingAction|AwaitingWait|AwaitingAsk|evidence/)'`
  - Result after attempt 2 repair: `439 tests run: 439 passed, 3091 skipped`.
- Scoped property gate:
  - `PROPTEST_CASES=1024 cargo +nightly test -p vb_core -p vb_runtime step_budget -- --nocapture`
  - Result: all selected tests passed; notable selected counts include `38 passed` in `vb_core` lib, `11 passed` in `vb_runtime` lib, and new `vb_5m8w` selected tests passed.
- TLA smoke:
  - `tla2tools verification/tla/StepBudgetSuspension.tla -config verification/tla/StepBudgetSuspension.cfg`
  - Result: `Model checking completed. No error has been found.`; `6,224 states generated`, `3,324 distinct states found`, depth `14`.
- Kani structural harness:
  - `cargo kani -p vb_core --lib --harness kani_step_budget_try_take_arbitrary --no-assertion-reach-checks`
  - Result: attempted twice. Timed out at 120s and again at 300s during SAT/post-processing after generating `18003 VCC(s), 9015 remaining after simplification`; no assertion failure/counterexample was produced before timeout. Previous State 5 proof evidence remains the last successful full Kani result for this unchanged harness.
- Canonical gate:
  - `moon ci`
  - Result: `Tasks: 23 completed`, `Time: 1m 17s 695ms`.

## Coverage Against Approved Plan
- PRE-002 / clamp: covered by concrete tests and proptest.
- PRE-003 / positive budget decrements before execution: covered by `try_take` concrete/proptest and one-step drive scenarios.
- PRE-004 / zero budget no step evidence: covered by runtime zero-budget evidence test.
- POST-001 / zero budget returns exhaustion, not terminal/error: covered by core zero-budget `run_until_blocked` test.
- POST-003 / preservation and completed effect durability: covered by core and runtime completed-then-exhausted tests.
- POST-004 / fresh budget resumes suspended run: covered by core fresh-budget resume test.
- POST-005 / runtime lifecycle continuation: covered by shard keep-run/inspect test.
- POST-006 / external suspensions distinct: covered by exact AwaitingAction plus WaitUntil, WaitEvent, and Ask scenarios.
- PRE-001 / terminal invalid resume: covered by `given_terminal_run_when_resume_attempted_then_invalid_resume_error`, asserting exact public wrapper `Err(ResumeError::RunIdNotFound { run_id })` after terminal cleanup removes the finished run.
- PRE-005 / StepCounterOverflow: explicit waiver `WAIVED-TEST-vb-5m8w-StepCounterOverflow-001`; public API cannot safely construct the private invalid counter state, compensated by constructor/proptest, TLA bounded invalid-state proof, and Kani boundary/structural evidence.

## Mutation Checkpoint Spot-Audit
- Clamp identity/off-by-one: killed by `given_budget_above_max...`, `given_u64_max...`, and clamp proptest.
- Zero branch returns true/mutates: killed by zero `try_take`, repeated zero concrete/proptest, and zero drive evidence tests.
- Positive decrement by zero/two: killed by positive concrete/proptest and completed-then-exhausted PC/state assertions.
- Exhaustion mapped to terminal cleanup: killed by core zero signal and shard lifecycle keep-run test.
- Evidence emitted before budget consumption: killed by runtime zero-evidence test.
- External suspension conflated with budget exhaustion or false success: killed by external suspension distinction test.
- Terminal resume accepted or misclassified: killed by exact `ResumeError::RunIdNotFound` assertion after finished-run terminal cleanup.

## Attempt 2 Repair Summary
- Added `AwaitingAction` runtime coverage with exact ticket fields (`run`, `step`, `action`, `attempt`) and exact evidence/state assertions: one `StepStarted`, zero `StepSucceeded`, zero `SlotWritten`, no budget exhaustion.
- Added terminal invalid-resume coverage through the public shard lifecycle: finished run is terminal-cleaned, inspect returns exact `NotFound`, resume returns exact `ResumeError::RunIdNotFound { run_id }`.
- Added formal `StepCounterOverflow` test waiver in the plan with owner/expiry/compensation/audit command because the invalid private counter state is unreachable via supported safe public/test-only construction.

## Status
- State 8 attempt 2 test repair complete.
- Ready for State 9 test review.
