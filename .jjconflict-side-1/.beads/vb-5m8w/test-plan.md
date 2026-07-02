# Test Plan: vb-5m8w Step Budget Suspension

## Startup Doctrine Cited
- `/home/lewis/.claude/skills/test-planner/SKILL.md`: requires behavior-first public API plans, Testing Trophy allocation, Given/When/Then scenarios, proptest/Kani/fuzz/mutation checkpoints, exact value/error assertions, and one artifact: `test-plan.md`.
- `/home/lewis/.agents/skills/test-planner/SKILL.md`: same content and controlling if conflicts appear; no conflict found.
- `/home/lewis/.agents/skills/test-planner/references/testing-philosophy.md`: tests must verify behavior/state, prefer real implementations over mocks, use DAMP scenarios, reject `is_ok()`/`is_err()` only assertions, and make every cared-about behavior executable.

## Summary
- Behaviors identified: 12.
- Trophy allocation: 11 unit/calc or property, 15 integration/BDD, 1 E2E/smoke, 4 static/formal gates. Integration is intentionally widest because the contract spans `vb_core`, `vb_runtime`, lifecycle continuation, and proof artifacts.
- Proptest invariants: 6.
- Fuzz targets: 0; contract scope has no parser/deserialization boundary and explicitly excludes parser/codec/network claims.
- Kani harness groups: 2 command groups covering boundary arithmetic and production-bound zero-budget frame preservation.
- Mutation threshold: scoped `cargo-mutants` kill rate >= 90%; all listed critical mutants must be killed.
- Approved proof inputs consumed: `contract-verification-review.md` status APPROVED; `proof-review.md` status APPROVED; `proof-evidence.md` PASS for PO-001..PO-006, PO-008..PO-011, Verus waived, PO-012 not run here.

## 1. Behavior Inventory
1. `StepBudget` clamps supplied budget to `MAX_STEP_BUDGET` when input exceeds the production cap.
2. `StepBudget::try_take` returns false and preserves observable budget/run state when remaining budget is zero.
3. `StepBudget::try_take` returns true and decrements exactly once when remaining budget is positive.
4. Repeated zero-budget `try_take` calls never underflow, wrap, panic, or change observables.
5. `run_until_blocked` returns `EngineSignal::StepBudgetExhausted` as graceful suspension when zero budget is supplied before a deterministic step starts.
6. Zero-budget drive emits no `StepStarted`, `StepSucceeded`, or `SlotWritten` evidence.
7. Zero-budget exhaustion preserves PC/frame/run state and does not mark running/succeeded or write slots.
8. Completed consumed step effects remain durable when a later slice exhausts budget.
9. Budget-exhausted runs remain eligible for reschedule/resume with fresh positive budget.
10. Runtime lifecycle maps `RuntimeSignal::StepBudgetExhausted` to continue/reschedule semantics, not terminal cleanup or workflow loss.
11. `AwaitingAction`, `AwaitingWait`, and `AwaitingAsk` remain distinct from budget exhaustion and do not create false success evidence.
12. Formal artifacts prove bounded arithmetic, non-terminal exhaustion, evidence ordering, preservation, liveness under fresh budget, and legacy terminal-exhaustion rejection.

## 2. Trophy Allocation

| Behavior | Layer | Planned artifact/command | Rationale |
|---|---|---|---|
| Clamp above max | Unit + proptest | `PROPTEST_CASES=1024 cargo +nightly test -p vb_core -p vb_runtime step_budget -- --nocapture` | Pure bounded arithmetic with generated u64 cases. |
| Zero `try_take` false/no mutation | Unit + Kani | unit tests; `cargo kani -p vb_core --lib --harness kani_step_budget_try_take_arbitrary --no-assertion-reach-checks` | Critical invariant needs exact assertions and bounded model checking. |
| Positive `try_take` decrements once | Unit + proptest + Kani | scoped step_budget tests; boundary Kani command chain | Pure calc behavior; all edge values must be generated/proved. |
| No underflow/wrap after exhaustion | Unit + proptest + Kani | repeated zero tests; boundary Kani harnesses | Arithmetic safety must be regression-proof. |
| Core zero-budget suspension | Integration | `cargo +nightly nextest run -p vb_core -p vb_runtime -E 'test(/budget|Budget|StepBudgetExhausted|AwaitingAction|AwaitingWait|AwaitingAsk|evidence/)'` | Tests public engine behavior with real runtime/core code. |
| No false step evidence | Integration | same scoped nextest selection | Evidence integrity crosses run loop and event collection. |
| PC/frame/run preservation | Integration + Kani | scoped nextest; structural Kani harness | Runtime state preservation must be observed and formally bounded. |
| Completed step durability | Integration | nextest scenario over two drive slices | Requires real step execution then exhaustion. |
| Reschedule/resume | Integration | nextest lifecycle scenario | Scheduler/lifecycle contract, not pure method behavior. |
| Runtime continue semantics | Integration | runtime lifecycle test | Prevents terminal cleanup regression. |
| External suspension distinction | Integration | AwaitingAction/Wait/Ask scenarios | Prevents conflating external suspension with budget exhaustion. |
| TLA model smoke/evidence | Static/formal + E2E smoke | `tla2tools verification/tla/StepBudgetSuspension.tla -config verification/tla/StepBudgetSuspension.cfg`; `moon ci` | Formal model is a first-class deliverable and CI must gate it. |

## 3. Requirement-to-Test/Proof Matrix

| Clause | Required tests | Proof obligations/evidence |
|---|---|---|
| PRE-001 | `given_budget_suspended_run_when_fresh_budget_scheduled_then_run_resumes_from_same_pc`; `given_terminal_run_when_resume_attempted_then_invalid_resume_error` | PO-005 / TLA-BUDGET-005 PASS |
| PRE-002 | `given_budget_above_max_when_constructed_then_clamped_to_max_step_budget` | PO-001, PO-008, PO-011 PASS; Verus waived |
| PRE-003 | `given_positive_budget_when_step_starts_then_budget_decrements_before_execution` | PO-003 PASS; Verus waived |
| PRE-004 | `given_zero_budget_when_drive_runs_then_no_step_started_or_succeeded_evidence` | PO-002, PO-010 PASS |
| PRE-005 | WAIVED-TEST-vb-5m8w-StepCounterOverflow-001: no supported public or test-only safe constructor can create a `StepBudget` with private `remaining > MAX_STEP_BUDGET`; exact executable tests cover public clamp/decrement behavior instead | PO-001 PASS; PO-008/PO-009 Kani PASS; TLA bounded arithmetic PASS |
| POST-001 | `given_zero_budget_when_run_until_blocked_then_signal_is_step_budget_exhausted_not_finished_or_error` | PO-002, PO-010 PASS |
| POST-002 | `given_one_budget_when_one_step_executes_then_next_slice_is_exhausted` | PO-003, PO-008, PO-011 PASS; Verus waived |
| POST-003 | `given_zero_budget_when_drive_runs_then_pc_and_frame_are_unchanged`; `given_completed_step_then_later_exhaustion_does_not_rollback_completed_effects` | PO-004, PO-009, PO-010 PASS |
| POST-004 | `given_budget_exhausted_run_when_new_budget_slice_arrives_then_execution_can_continue` | PO-005, PO-010 PASS |
| POST-005 | `given_runtime_step_budget_exhausted_when_apply_drive_result_then_run_is_kept_and_drive_continue_emitted` | PO-005, PO-010 PASS |
| POST-006 | `given_action_wait_or_ask_suspension_when_drive_returns_then_signal_is_not_step_budget_exhausted_and_no_false_success` | PO-006, PO-010 PASS |
| INV-001 | `given_try_take_repeated_after_zero_then_budget_does_not_underflow` | PO-001, PO-008, PO-011 PASS; Verus waived |
| INV-002 | `given_zero_budget_when_try_take_called_then_returns_false_without_mutation` | PO-002, PO-008, PO-009 PASS; Verus waived |
| INV-003 | `given_positive_budget_when_try_take_called_then_remaining_decrements_by_one` | PO-003, PO-008, PO-011 PASS; Verus waived |
| INV-004 | `given_zero_budget_runtime_drive_when_inspecting_evidence_then_no_step_started_succeeded_or_slot_written` | PO-003, PO-010 PASS |
| INV-005 | `given_step_budget_exhausted_when_lifecycle_applies_result_then_not_terminal_cleanup` | PO-002 PASS |
| INV-006 | `given_zero_budget_when_drive_attempts_step_then_pc_frame_and_step_status_unchanged` | PO-004, PO-009, PO-010 PASS |
| INV-007 | `given_one_step_completed_when_next_budget_exhausts_then_completed_step_remains_succeeded` | PO-004 PASS |
| INV-008 | `given_awaiting_action_wait_or_ask_when_mark_step_after_signal_then_no_step_succeeded` | PO-006, PO-010 PASS |
| INV-009 | `given_resumable_budget_exhausted_run_when_budget_replenished_repeatedly_then_progress_or_explicit_block_occurs` | PO-005 PASS |
| INV-010 | `given_step_budget_exhausted_when_modeled_then_state_is_suspended_budget_not_terminal` | PO-002; proof review APPROVED |

## 4. BDD Scenarios

### Behavior: budget construction clamps to max
- Test name: `given_budget_above_max_when_constructed_then_clamped_to_max_step_budget`
- Given: a requested budget greater than `MAX_STEP_BUDGET`.
- When: `StepBudget::new(value)` is constructed through the public API.
- Then: remaining budget equals exactly `MAX_STEP_BUDGET`, not the requested value.

### Behavior: zero budget refuses consumption without mutation
- Test name: `given_zero_budget_when_try_take_called_then_returns_false_without_mutation`
- Given: `StepBudget::new(0)` and a captured observable state.
- When: `try_take` is called once.
- Then: result is exactly `Ok(false)` and remaining budget/state observables are unchanged.

### Behavior: positive budget consumes exactly once
- Test name: `given_positive_budget_when_try_take_called_then_remaining_decrements_by_one`
- Given: any positive budget `n` in `1..=MAX_STEP_BUDGET`.
- When: `try_take` is called once.
- Then: result is exactly `Ok(true)` and remaining budget equals `n - 1`.

### Behavior: repeated exhaustion is stable
- Test name: `given_try_take_repeated_after_zero_then_budget_does_not_underflow`
- Given: `StepBudget::new(0)`.
- When: `try_take` is called repeatedly.
- Then: every call returns exactly `Ok(false)`, remaining budget stays zero, and no panic/underflow/wrap occurs.

### Behavior: zero budget blocks core drive gracefully
- Test name: `given_zero_budget_when_run_until_blocked_then_signal_is_step_budget_exhausted_not_finished_or_error`
- Given: a valid runnable workflow/run frame and zero budget before the next deterministic step.
- When: the core drive API runs until blocked.
- Then: the result is exactly `EngineSignal::StepBudgetExhausted` or the documented runtime refinement, not `Finished`, not `TypedStepError`, not panic.

### Behavior: zero budget emits no step evidence
- Test name: `given_zero_budget_when_drive_runs_then_no_step_started_or_succeeded_evidence`
- Given: a valid runnable workflow/run frame and zero budget.
- When: the drive slice runs.
- Then: evidence contains exactly no `StepStarted`, no `StepSucceeded`, and no `SlotWritten` entries.

### Behavior: zero exhaustion preserves run observables
- Test name: `given_zero_budget_when_drive_attempts_step_then_pc_frame_and_step_status_unchanged`
- Given: a captured PC/frame/run status and zero budget before step start.
- When: the drive slice runs.
- Then: PC, frame observables, running flag, succeeded flag, and slots equal their pre-drive values; only the suspension signal/reschedule marker may change.

### Behavior: completed consumed work survives later exhaustion
- Test name: `given_one_step_completed_when_next_budget_exhausts_then_completed_step_remains_succeeded`
- Given: one slice with positive budget has completed a deterministic step.
- When: a later slice enters with zero budget.
- Then: the prior completed step and its durable slot/evidence remain present and unchanged.

### Behavior: fresh budget resumes budget-suspended run
- Test name: `given_budget_exhausted_run_when_new_budget_slice_arrives_then_execution_can_continue`
- Given: a run suspended only for budget exhaustion.
- When: a fresh positive budget slice is scheduled.
- Then: the run is eligible to start the next deterministic step, externally suspend, finish, or return a typed error; it must not be rejected as terminal.

### Behavior: runtime lifecycle continues on exhaustion
- Test name: `given_runtime_step_budget_exhausted_when_apply_drive_result_then_run_is_kept_and_drive_continue_emitted`
- Given: a persisted/non-terminal run and `RuntimeSignal::StepBudgetExhausted`.
- When: lifecycle applies the drive result.
- Then: the run remains stored/eligible and the observable lifecycle outcome is continuation/reschedule (`DriveContinue`-like), not terminal cleanup/deletion.

### Behavior: external suspensions stay distinct
- Test name: `given_action_wait_or_ask_suspension_when_drive_returns_then_signal_is_not_step_budget_exhausted_and_no_false_success`
- Given: workflows that block on action, wait, and ask.
- When: drive returns each external suspension.
- Then: the signal is the matching external suspension, never budget exhaustion; no false `StepSucceeded` evidence is emitted unless a consumed step actually completed.

### Behavior: terminal resume is rejected explicitly
- Test name: `given_terminal_run_when_resume_attempted_then_invalid_resume_error`
- Given: a terminal finished or typed-error run.
- When: caller attempts to resume it as budget-suspended.
- Then: result is exactly `InvalidResumeState` or the concrete public error variant mapped to invalid terminal resume.

### Waiver: StepCounterOverflow exact executable test
- Waiver ID: `WAIVED-TEST-vb-5m8w-StepCounterOverflow-001`.
- Clause ID: PRE-005 plus Error Taxonomy `StepCounterOverflow`.
- Reason: `StepBudget::remaining` is private; `StepBudget::new` clamps every `u64` to `MAX_STEP_BUDGET`; `StepBudget::MAX` is valid; `try_take` only decreases through `saturating_sub`. No supported public API or safe test-only constructor can create the invalid internal state (`remaining > MAX_STEP_BUDGET`) needed to return exact `EngineError::StepCounterOverflow` without editing production behavior or using forbidden unsafe/private-field mutation.
- Owner: State 8 test-writer for `vb-5m8w`; State 9 test-reviewer must re-check waiver validity.
- Expiry: 2026-06-18 or immediately when any public/test-only constructor/deserializer/fixture can construct invalid `StepBudget` internals, whichever occurs first.
- Compensation: exact clamp tests for `MAX_STEP_BUDGET + 1` and `u64::MAX`; proptest for all `u64` constructor inputs; zero/positive `try_take` tests; TLA bounded arithmetic invalid/out-of-range sink proof; Kani boundary and structural StepBudget harness evidence from State 5.
- Command evidence: `rtk grep -n 'StepBudget \{|remaining:|pub .*StepBudget|StepCounterOverflow' 'crates/vb_core/src/engine/signals.rs' 'crates/vb_core/src/frame.rs' 'crates/vb_core/tests/vb_5m8w_step_budget_suspension.rs'` returned only the private `StepBudget { remaining: u64 }` definition, the clamping constructor assignment, the defensive `EngineError::StepCounterOverflow` branch, and unrelated `RunFrame::increment_executed` overflow mapping; no supported test constructor for an invalid `StepBudget` internal state was found.
- Downstream audit command: rerun the command evidence above before State 9 approval to ensure no reachable exact-variant path was missed.

## 5. Proptest Invariants

### Proptest: `StepBudget::new`
- Invariant: for all `u64` inputs, constructed remaining budget is in `0..=MAX_STEP_BUDGET` and equals `min(input, MAX_STEP_BUDGET)`.
- Strategy: any `u64`, with explicit seeds `0`, `1`, `MAX_STEP_BUDGET - 1`, `MAX_STEP_BUDGET`, `MAX_STEP_BUDGET + 1`, `u64::MAX`.
- Anti-invariant: any constructed value above `MAX_STEP_BUDGET` must fail the test.

### Proptest: `StepBudget::try_take` positive path
- Invariant: for all `n in 1..=MAX_STEP_BUDGET`, one `try_take` returns exactly `Ok(true)` and leaves `n - 1`.
- Strategy: bounded `u64` in production range.
- Anti-invariant: returning false or decrementing by any amount other than one must fail.

### Proptest: `StepBudget::try_take` zero path
- Invariant: for zero budget, any finite repetition of `try_take` returns exactly `Ok(false)` and leaves zero.
- Strategy: repetition count `0..=MAX_STEP_BUDGET.min(1024)` for fast property runs; include 1024-case CI seed.
- Anti-invariant: underflow, wrap, `Ok(true)`, or state mutation must fail.

### Proptest: drive budget/evidence consistency
- Invariant: total `StepStarted`, `StepSucceeded`, and `SlotWritten` evidence for a slice never exceeds consumed budget; with zero budget all are zero.
- Strategy: generated small workflows/run frames and budget `0..=3` matching Kani/TLA finite representative bounds.
- Anti-invariant: evidence without prior consumed budget must fail.

### Proptest: exhaustion preservation
- Invariant: zero-budget drive preserves public PC/frame/run observables.
- Strategy: generated valid `RunFrame` shapes within existing test builders; include minimal one-step and two-step frames.
- Anti-invariant: PC advance, frame mutation, status success, slot write, or terminalization must fail.

### Proptest: resume fairness finite analogue
- Invariant: a budget-suspended non-terminal run supplied recurring fresh positive budgets either progresses, externally suspends, finishes, or returns typed error within the modeled finite workflow length.
- Strategy: finite generated workflows with max steps bounded to keep tests deterministic.
- Anti-invariant: stuck budget-suspended state despite fresh positive budgets must fail.

## 6. Fuzz Targets
- None planned. This bead covers deterministic step-budget state transitions and formal artifacts, not byte/string parsing, serde, network, file format, or codec surfaces. If test writing introduces any raw input parser or deserializer, add a `cargo-fuzz` target before State 8 exits.

## 7. Kani Harnesses

### Kani Harness Group: arithmetic boundary harnesses
- Property: selected budget arithmetic boundaries do not panic, underflow, wrap, or violate expected clamp/decrement results.
- Bound: package/lib target harnesses already listed in proof evidence.
- Command:
  ```bash
  cargo kani -p vb_core --lib --harness kani_budget_sub_dim_zero --no-assertion-reach-checks && cargo kani -p vb_core --lib --harness kani_budget_sub_one_minus_one --no-assertion-reach-checks && cargo kani -p vb_core --lib --harness kani_budget_sub_one_minus_two_underflow --no-assertion-reach-checks && cargo kani -p vb_core --lib --harness kani_sub_dim_zero_minus_one_underflow --no-assertion-reach-checks && cargo kani -p vb_core --lib --harness kani_sub_dim_max_minus_max --no-assertion-reach-checks && cargo kani -p vb_core --lib --harness kani_sub_dim_max_minus_max_minus_one --no-assertion-reach-checks
  ```
- Rationale: arithmetic boundary safety is too important for sampled tests only.

### Kani Harness: structural zero-budget preservation
- Property: generated actual `RunFrame` observables are unchanged around production `StepBudget::new(0)`/`try_take`; no dummy fixed shapes.
- Bound: existing harness bounds `step_count=1..=2`, `slot_count<=2`, and `first_step < step_count` per proof evidence.
- Command:
  ```bash
  cargo kani -p vb_core --lib --harness kani_step_budget_try_take_arbitrary --no-assertion-reach-checks
  ```
- Rationale: proves contract-critical zero-budget preservation through production-bound code, complementing integration tests.

## 8. TLA+ Model Smoke and Evidence Gate
- Smoke command:
  ```bash
  tla2tools verification/tla/StepBudgetSuspension.tla -config verification/tla/StepBudgetSuspension.cfg
  ```
- Required assertions from evidence:
  - TLC exits 0 with `Model checking completed. No error has been found.`
  - Evidence records state/depth summary. Approved evidence currently records 6,224 states generated, 3,324 distinct states, 0 states left, depth 14.
  - Invariants/properties include bounded arithmetic, no underflow/wrap, non-terminal exhaustion, evidence requires consumed budget, exhaustion preserves run state, external suspension distinction, fair fresh-budget progress, no terminal legacy exhaustion.
- Test-writer must not weaken the model/config to make the smoke pass.

## 9. CI Gates
1. Scoped integration/evidence gate:
   ```bash
   cargo +nightly nextest run -p vb_core -p vb_runtime -E 'test(/budget|Budget|StepBudgetExhausted|AwaitingAction|AwaitingWait|AwaitingAsk|evidence/)'
   ```
   Required evidence: all selected tests pass; prior proof evidence records 426 passed.
2. Scoped property gate:
   ```bash
   PROPTEST_CASES=1024 cargo +nightly test -p vb_core -p vb_runtime step_budget -- --nocapture
   ```
   Required evidence: all selected tests/proptests pass.
3. TLA smoke gate: command in section 8 exits 0 and evidence summary is recorded.
4. Kani gates: both command groups in section 7 pass or produce raw failure evidence for implementation repair, not harness weakening.
5. Canonical project gate:
   ```bash
   moon ci
   ```
   Required evidence: canonical CI completes cleanly or failures are classified with raw output. PO-012 was not run in proof evidence and remains mandatory downstream.

## 10. Mutation Checkpoints
- Threshold: scoped `cargo-mutants` kill rate >= 90%; any survivor in the listed critical branches blocks acceptance even if global percentage passes.
- Critical mutants that must die:
  - `StepBudget::new` clamp changed from `min(MAX_STEP_BUDGET)` to identity or off-by-one; killed by clamp unit/proptest.
  - `try_take` zero branch returns `Ok(true)`; killed by zero unit/proptest/Kani.
  - `try_take` positive branch decrements by zero or two; killed by positive unit/proptest/Kani.
  - Exhaustion path maps to `Finished`/typed error/terminal cleanup; killed by core/runtime lifecycle integration tests and TLA smoke.
  - Evidence emission occurs before budget consumption; killed by evidence consistency integration/proptest and TLA `EvidenceRequiresConsumedBudget`.
  - PC/frame preservation branch mutates on exhaustion; killed by zero preservation integration/proptest/Kani.
  - External suspension is classified as budget exhaustion or emits false success; killed by AwaitingAction/Wait/Ask BDD scenarios.

## 11. Combinatorial Coverage Matrix

| Scenario | Input class | Expected output | Layer |
|---|---|---|---|
| construct clamp zero | `0` | remaining `0` | unit |
| construct below max | `1..MAX_STEP_BUDGET-1` | remaining input | unit/proptest |
| construct max | `MAX_STEP_BUDGET` | remaining `MAX_STEP_BUDGET` | unit/proptest |
| construct above max | `MAX_STEP_BUDGET+1..u64::MAX` | remaining `MAX_STEP_BUDGET` | unit/proptest |
| try_take zero once | budget `0` | exactly `Ok(false)`, remaining `0` | unit/Kani |
| try_take zero repeated | budget `0`, repetitions > 1 | every call exactly `Ok(false)`, remaining `0` | unit/proptest/Kani |
| try_take one | budget `1` | exactly `Ok(true)`, remaining `0`; next call `Ok(false)` | unit/Kani |
| try_take positive middle | `2..MAX_STEP_BUDGET-1` | exactly `Ok(true)`, remaining `n-1` | unit/proptest |
| try_take max | `MAX_STEP_BUDGET` | exactly `Ok(true)`, remaining `9999` | unit/proptest/Kani |
| core zero drive | runnable frame + zero budget | exact `StepBudgetExhausted`, no terminal/error | integration |
| core one-budget drive | runnable frame + budget `1` | one consumed step; next slice can exhaust | integration |
| evidence zero | runnable frame + zero budget | no `StepStarted`/`StepSucceeded`/`SlotWritten` | integration/proptest |
| preservation zero | generated valid frame + zero budget | PC/frame/status/slots unchanged | integration/proptest/Kani |
| completed then exhausted | positive slice then zero slice | completed effects retained | integration |
| reschedule exhausted | suspended budget + fresh positive budget | run continues or explicitly blocks/finishes/errors | integration/TLA |
| lifecycle exhausted | runtime signal exhausted | run kept, continuation/reschedule emitted | integration |
| external action/wait/ask | external blockers | matching external signal, no false success | integration |
| terminal resume | terminal run | exact invalid resume error variant | integration |
| TLA smoke | model/config | TLC pass, expected invariants/properties checked | formal/static |
| Kani structural | bounded generated frame | successful proof, no hardcoded dummy shape | formal/static |

## 12. Assertion Rules for Test Writer
- Do not assert only `is_ok()` or `is_err()`; assert exact success values, signal variants, error variants, and state/evidence deltas.
- Use public APIs and observable state; do not test private implementation details except existing `cfg(test)`/`cfg(kani)` observability needed to prove the contract.
- Prefer real `vb_core`/`vb_runtime` integration paths. Use fakes only for deterministic workflow construction, not for replacing the drive/lifecycle code under contract.
- Any proof/test failure from these scenarios means repair implementation or proof artifact; do not weaken the contract, TLA model, Kani assumptions, or property strategies.

## Open Questions
- Whether `InvalidResumeState` is exposed as that exact public variant or mapped through a runtime/core error wrapper; test writer must assert the concrete public variant used by the current API.
- Whether mutation testing should be scoped by crate/package filters available in this workspace; if unavailable, record the exact blocker and still run the listed critical tests.
