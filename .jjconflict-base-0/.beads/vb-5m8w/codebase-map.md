bead_id: vb-5m8w
title: Add TLA+ Step Budget Model
state: 2-explore
workspace: /home/lewis/src/go-skill-vb-5m8w
source_checkout_forbidden: /home/lewis/src/velvet-ballistics

# Codebase Map: Step Budget Exhaustion + Graceful Suspension

## Search Evidence

Commands/read targets used in the isolated workspace:

- `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-5m8w --json` from `/home/lewis/src/go-skill-vb-5m8w`: bead exists, `status=in_progress`, `assignee=Lewis`.
- Read existing State 1 artifacts:
  - `/home/lewis/src/go-skill-vb-5m8w/.beads/vb-5m8w/STATE.md`
  - `/home/lewis/src/go-skill-vb-5m8w/.beads/vb-5m8w/baseline-report.md`
- Globbed existing TLA+/CFG files under `/home/lewis/src/go-skill-vb-5m8w`.
- Grepped Rust and specs for `StepBudgetExhausted`, `step_budget_remaining`, `run_until_blocked`, `drive_deterministic`, `try_take`, `AwaitingAction`, `AwaitingWait`, `AwaitingAsk`, `TLA+`, `tlc`, `tla2tools`, `verification`.
- Read the core/runtime/proof files listed below.

## Primary Runtime Semantics

### Core step-budget API

- `/home/lewis/src/go-skill-vb-5m8w/crates/vb_core/src/limits.rs`
  - `MAX_STEP_BUDGET: u64 = 10_000` at line 94.
- `/home/lewis/src/go-skill-vb-5m8w/crates/vb_core/src/engine/signals.rs`
  - `StepBudget { remaining: u64 }` is private.
  - `StepBudget::new(value)` clamps values above `MAX_STEP_BUDGET`.
  - `StepBudget::try_take()`:
    - returns `Err(EngineError::StepCounterOverflow)` if `remaining > MAX_STEP_BUDGET`;
    - returns `Ok(false)` when `remaining == 0`;
    - otherwise decrements by one with `saturating_sub(1)` and returns `Ok(true)`.
  - `EngineSignal` variants include `Continue`, `Finished`, `StepBudgetExhausted`, `AwaitingAction`, `AwaitingWait`, `AwaitingAsk`.
- `/home/lewis/src/go-skill-vb-5m8w/crates/vb_core/src/engine/run_loop.rs`
  - `run_until_blocked(plan, run, budget, store)` delegates to `drive_deterministic`.
  - `drive_deterministic(...)` loops while `budget.try_take()?` and executes `step_once`.
  - Any non-`Continue` signal exits immediately.
  - When `try_take()` returns `Ok(false)`, it returns `Ok(EngineSignal::StepBudgetExhausted)`.

### Runtime full drive loop

- `/home/lewis/src/go-skill-vb-5m8w/crates/vb_runtime/src/engine/types.rs`
  - `RuntimeSignal` mirrors core signal categories: `Continue`, `Finished`, `StepBudgetExhausted`, `AwaitingAction`, `AwaitingWait`, `AwaitingAsk`.
- `/home/lewis/src/go-skill-vb-5m8w/crates/vb_runtime/src/engine/drive.rs`
  - `drive_deterministic_full(...)` calls `begin_drive_step`; if budget cannot be taken, it returns `RuntimeSignal::StepBudgetExhausted`.
  - `begin_drive_step(...)` consumes budget before marking the current PC running and before emitting `StepStarted`.
  - `finish_drive_step(...)` marks state and emits evidence after a node executes.
  - `signal_is_success` treats only `Continue` and `Finished` as success evidence.
- `/home/lewis/src/go-skill-vb-5m8w/crates/vb_runtime/src/engine/helpers.rs`
  - `mark_step_after_signal` maps:
    - `AwaitingWait` -> `run.mark_waiting(step)`;
    - `AwaitingAsk` -> `run.mark_asking(step)`;
    - `AwaitingAction(_) | StepBudgetExhausted` -> no state transition;
    - `Continue | Finished(_)` -> `run.mark_succeeded(step)`.
  - Current `drive_deterministic_full` usually returns budget exhaustion before marking a new step running, because `try_take` happens before `mark_running`; however tests contain black-hat commentary about stale Running state on exhaustion, so the TLA+ model should explicitly rule out corrupt/terminal transitions on exhaustion and document whether exhaustion is a scheduler suspension rather than step completion.
- `/home/lewis/src/go-skill-vb-5m8w/crates/vb_runtime/src/shard/lifecycle/chunk_002.rs`
  - `apply_drive_result` treats `RuntimeSignal::Continue | RuntimeSignal::StepBudgetExhausted` the same: emits `RuntimeEvent::DriveContinue`, keeps the run, and returns `Ok(())`.
  - This is the strongest implementation evidence for graceful suspension/rescheduling semantics: budget exhaustion is not terminal failure.
- `/home/lewis/src/go-skill-vb-5m8w/crates/vb_runtime/src/engine/signal.rs`
  - `runtime_from_core(EngineSignal::StepBudgetExhausted)` maps directly to `RuntimeSignal::StepBudgetExhausted`.

## Existing Tests Around This Scope

- `/home/lewis/src/go-skill-vb-5m8w/crates/vb_core/src/engine/run_loop.rs`
  - Unit tests cover zero budget, one-budget exhaustion after first transition, exact-budget completion, and suspension on `Do` node.
- `/home/lewis/src/go-skill-vb-5m8w/crates/vb_core/src/engine/tests/integration_budget.rs`
  - Integration tests assert zero budget does not execute or mutate frame, one budget executes exactly one transition and leaves next step pending, exact two-step budget finishes, and `try_take` returns false after depletion without underflow.
- `/home/lewis/src/go-skill-vb-5m8w/crates/vb_runtime/src/engine/drive.rs`
  - Runtime tests include budget exhaustion after N transitions and action/wait/ask suspension cases.
- `/home/lewis/src/go-skill-vb-5m8w/crates/vb_runtime/src/engine/tests.rs`
  - `bh_drive_budget_exhausted_does_not_emit_step_succeeded_in_evidence` asserts zero-budget exhaustion emits no evidence.
  - Black-hat comments identify a prior/possible state-machine gap around `StepBudgetExhausted`; use this as a risk seed, not as proof of current bug.
- `/home/lewis/src/go-skill-vb-5m8w/crates/vb_runtime/src/engine/property_tests.rs`
  - RuntimeSignal equality/non-finished distinction includes `StepBudgetExhausted`, `AwaitingWait`, `AwaitingAsk`.

## Existing Formal/Verification Assets

### TLA+

- `/home/lewis/src/go-skill-vb-5m8w/verification/tla/WorkflowBoundedAdmission.tla`
  - Already models finite `StepBudgets == 0..3`.
  - Has `ExecuteStep` decrement and `ExhaustStepBudget` transition from `running/capped` to `blocked`.
  - Invariants include `StepBudgetNeverNegative`; liveness includes `EventuallyBlockedOrTerminal`.
  - Limitation for this bead: bundled with admission/capacity/certificate concerns; not a focused temporal model of budget exhaustion, resume/scheduler slice, and graceful suspension.
- `/home/lewis/src/go-skill-vb-5m8w/specs/vb_qi37_2_5/BoundednessSlice.tla`
  - Focused budget slice with `TakeStep`, `BlockOnAction`, `BlockOnWait`, `Finish`, `TypedError`, `ExhaustBudget`.
  - Limitation for this bead: `ExhaustBudget` sets `workflow_state' = "Terminal"`; that conflicts with the requested graceful suspension semantics and with runtime shard behavior where `StepBudgetExhausted` keeps the run.
- Additional relevant TLA+ context:
  - `/home/lewis/src/go-skill-vb-5m8w/verification/tla/EngineYamlRunLifecycle.tla` models suspension lifecycle generally.
  - `/home/lewis/src/go-skill-vb-5m8w/verification/tla/V1PrimitiveLowering.tla` includes `Suspend` transition for primitives.
  - `/home/lewis/src/go-skill-vb-5m8w/specs/ResumeStateMachine.tla` models resume/suspend state machine.
  - `/home/lewis/src/go-skill-vb-5m8w/specs/tla/BudgetArithmetic.tla` models bounded word arithmetic, overflow/underflow error statuses, and budget fields including `max_step_budget_per_tick`.

### Verus/Kani

- `/home/lewis/src/go-skill-vb-5m8w/verification/verus/run_loop_termination.rs`
  - Verus abstraction for run loop termination within budget.
  - Assumes `try_take` decreases by exactly one and returns false at zero.
  - Ensures exhaustion signal when remaining reaches zero.
  - Limitation: not a temporal/shard lifecycle model and not tied to graceful suspension/resume semantics.
- `/home/lewis/src/go-skill-vb-5m8w/verification/verus/step_budget.rs`
  - Additional Verus step-budget artifact exists and should be considered by proof planning.
- `/home/lewis/src/go-skill-vb-5m8w/crates/vb_core/src/kani_step_budget_zero.rs`
- `/home/lewis/src/go-skill-vb-5m8w/crates/vb_core/src/kani_step_budget_one.rs`
- `/home/lewis/src/go-skill-vb-5m8w/crates/vb_core/src/kani_step_budget.rs`
  - Kani budget arithmetic boundary harnesses exist, but they focus on arithmetic/panic freedom, not temporal suspension semantics.

## Proof Gates / Tooling That Should Consume the Model

- `/home/lewis/src/go-skill-vb-5m8w/xtask/src/proof.rs`
  - `commands_for_obligation` emits `tla2tools <path>` when a proof obligation has `required.tla`.
- `/home/lewis/src/go-skill-vb-5m8w/xtask/src/lanes.rs`
  - `detect_available_lanes` enables `tla` when `verification/tla` exists.
  - Generic `tla_command(crate_name, workspace_root)` currently expects `verification/tla/<crate_name>.tla`; explicit proof obligations are more precise for a named model.
- `/home/lewis/src/go-skill-vb-5m8w/contracts/proof_obligations.yaml`
  - Contains prior TLA commands and notes Kani for StepBudget. State 3 should decide whether to add a formal obligation entry for the new model.
- `/home/lewis/src/go-skill-vb-5m8w/contracts/invariants.yaml`
  - Existing invariant descriptions include `StepBudget::try_take() returns false when budget is 0`, `StepBudget::remaining is always >= 0`, and `Budget 0 causes immediate StepBudgetExhausted`.

## Recommended Delivery Scope

Primary implementation should be a formal spec artifact, not production Rust:

- Add focused model under `/home/lewis/src/go-skill-vb-5m8w/verification/tla/`, suggested names:
  - `/home/lewis/src/go-skill-vb-5m8w/verification/tla/StepBudgetSuspension.tla`
  - `/home/lewis/src/go-skill-vb-5m8w/verification/tla/StepBudgetSuspension.cfg`
- Optional contract/proof-planning follow-up if State 3 requires it:
  - add/extend a proof obligation that runs `tla2tools verification/tla/StepBudgetSuspension.tla` or the cfg-equivalent command used by local tooling.

The model should cover:

1. Bounded budgets, including zero and max small model values.
2. `try_take` semantics: no negative budget, decrement exactly once per started deterministic step, false at zero.
3. Distinct outcomes:
   - deterministic `Continue`;
   - terminal `Finished` / typed failure;
   - external suspension (`AwaitingAction`, `AwaitingWait`, `AwaitingAsk`);
   - budget exhaustion as graceful scheduler suspension/rescheduling, not terminal failure.
4. No evidence/step-completion event when zero budget prevents a step from starting.
5. No `StepSucceeded` for action/wait/ask suspension.
6. Resume/reschedule path: a budget-exhausted run remains resumable/runnable with a fresh slice and preserves PC/frame state.
7. Fairness/liveness under fresh budget: non-terminal resumable runs eventually reach terminal or external suspension if steps are available and budget is replenished.
8. Safety invariant: budget exhaustion never turns into `Failed`, never releases reservation as terminal completion by itself, and never advances PC without a consumed step.

## Public API / Behavior Surface

- `vb_core::StepBudget`
- `vb_core::EngineSignal::StepBudgetExhausted`
- `vb_core::run_until_blocked`
- `vb_core::drive_deterministic`
- `vb_runtime::engine::RuntimeSignal::StepBudgetExhausted`
- `vb_runtime::engine::drive_deterministic_full`
- `vb_runtime::shard` lifecycle behavior for `RuntimeSignal::StepBudgetExhausted`

No dependency changes are indicated for the formal-spec-first delivery.

## Risks / Open Questions

- Existing `specs/vb_qi37_2_5/BoundednessSlice.tla` models budget exhaustion as terminal. The new model must override that behavior for current runtime semantics or explicitly document legacy mismatch.
- Runtime black-hat comments mention a stale Running state on budget exhaustion; current `drive_deterministic_full` consumes budget before marking running, but proof planning should include this as a risk and require model clauses about PC/state preservation.
- TLA tooling integration is lightweight: `xtask` can emit `tla2tools <path>`, but no dedicated Moon task was found for TLA. State 3 should bind exact verifier command.
- Because this bead is formal-spec-first and runtime semantics affect scheduler safety, unknown integration risk should escalate to stricter local verifier mode: TLC/TLA plus scoped Rust tests around `vb_core` and `vb_runtime` budget/suspension behavior.
