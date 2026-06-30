# TLA+ Temporal Model Plan: StepBudgetSuspension

## Boundary
- Temporal/workflow behavior: step-budget consumption, zero-budget exhaustion, graceful suspension, fresh-budget resume, evidence emission, terminal/external-suspension separation, and fairness across scheduler slices.
- Rust/core behavior excluded from TLA+ and handled by Verus/Kani/tests: concrete Rust representation, exact enum layout, borrow/memory safety, panic freedom, and production API compilation.
- External systems abstracted: action completion, wait wakeup, ask response, storage durability, wall-clock time, and shard scheduling are finite abstract events.

## TLA+-Owned Clauses
- PRE-001: Valid initial run state.
- PRE-002/PRE-005/INV-001: bounded u64-style budget domain with `MAX_STEP_BUDGET = 10000` and invalid arithmetic sink.
- PRE-003/INV-003: consuming a positive budget starts exactly one deterministic step.
- PRE-004/POST-001/INV-002: zero budget yields non-terminal `StepBudgetExhausted`/`SuspendedBudget`.
- POST-003/INV-006/INV-007: PC/frame preservation on exhaustion; completed consumed steps remain durable.
- POST-004/INV-009: fresh-budget resume eventually progresses or reaches external/terminal outcome under fairness.
- POST-006/INV-008: action/wait/ask suspensions remain distinct and do not falsely succeed.
- INV-010: exhaustion must not be modeled as terminal.

## Model Shape
- Module path: `verification/tla/StepBudgetSuspension.tla`.
- Config path: `verification/tla/StepBudgetSuspension.cfg`.
- Module name: `StepBudgetSuspension`.
- Constants:
  - `MAX_U64` = `18446744073709551615` in the spec arithmetic definition.
  - `MAX_STEP_BUDGET` = `10000` in the spec arithmetic definition.
  - TLC model constants may restrict explored budgets to `{0, 1, 2, 3, MAX_STEP_BUDGET}` or a finite representative set while preserving the exact production bound in predicates.
  - finite `Steps`, `Frames`, `ExternalEvents`, and `Outcomes`.
- Variables:
  - `pc`: abstract program counter.
  - `frame`: abstract frame/hash/state value.
  - `budget`: current bounded budget value.
  - `run_state`: one of `Runnable`, `RunningStep`, `SuspendedBudget`, `SuspendedAction`, `SuspendedWait`, `SuspendedAsk`, `Finished`, `TypedError`, `InvariantViolation`.
  - `last_signal`: one of `None`, `Continue`, `StepBudgetExhausted`, `AwaitingAction`, `AwaitingWait`, `AwaitingAsk`, `FinishedSignal`, `TypedErrorSignal`.
  - `evidence`: sequence or set of abstract events: `StepStarted`, `StepSucceeded`, `SlotWritten`, `DriveContinue`, `Suspended`, `FinishedEvent`, `TypedErrorEvent`.
  - `consumed_steps`: count of successfully consumed budget units in the current slice.
  - `completed_steps`: abstract durable count/history of completed consumed steps.
  - `reschedule_pending`: boolean marking budget exhaustion as scheduler work, not terminal cleanup.
  - `arith_error`: boolean or state marker for overflow/underflow/out-of-bound arithmetic.
- Init action: `Init` creates a finite valid run with bounded budget and non-terminal or terminal state according to config constraints.
- Next/actions:
  - `ClampBudget` or initialization predicate for values above `MAX_STEP_BUDGET`.
  - `TakeStep`: requires `run_state = Runnable` and `budget > 0`; decrements exactly one; emits `StepStarted`; enters `RunningStep`.
  - `CompleteContinue`: from `RunningStep`; advances `pc`/`frame` according to abstract step; emits success evidence; returns `Runnable` with `Continue`.
  - `CompleteFinished`: from `RunningStep`; emits terminal success; enters `Finished`.
  - `CompleteTypedError`: from `RunningStep`; emits explicit typed error; enters `TypedError`.
  - `BlockOnAction`, `BlockOnWait`, `BlockOnAsk`: from `RunningStep`; enter matching external suspension without `StepSucceeded` unless the modeled step completed.
  - `ExhaustBudget`: requires `run_state = Runnable` and `budget = 0`; enters `SuspendedBudget`; preserves `pc`/`frame`; emits scheduler continuation/suspension evidence only.
  - `ReplenishBudget`: from `SuspendedBudget`; supplies fresh positive bounded budget; returns `Runnable` without changing `pc`/`frame`.
  - `ResumeExternal`: from external suspension by matching finite external event; returns `Runnable` or terminal typed error according to model.
  - `ArithmeticError`: catches any invalid budget value, underflow attempt, or value above `MAX_STEP_BUDGET` after initialization.
- State constraints:
  - `budget \in 0..MAX_STEP_BUDGET` unless `run_state = InvariantViolation`.
  - Finite TLC representative budgets include zero and max-bound representatives.
  - Terminal states stutter except allowed observation/cleanup events that do not resurrect runs.
  - `reschedule_pending = TRUE` implies `run_state = SuspendedBudget` and not terminal.
- Symmetry sets: abstract steps and external events may be symmetric if config uses multiple indistinguishable steps/events.
- Bounded model limits: TLC should use small finite step/PC/frame sets and representative budgets, but the spec must define exact bounded arithmetic with `MAX_U64` and `MAX_STEP_BUDGET` and route invalid arithmetic to error/invariant violation.

## Properties
- Safety invariants:
  - `BudgetWithinBounds`: budget is within `0..MAX_STEP_BUDGET` outside `InvariantViolation`.
  - `NoBudgetUnderflowOrWrap`: no transition maps zero to `MAX_U64` or any wrapped value.
  - `ExhaustionNonTerminal`: `StepBudgetExhausted` implies `SuspendedBudget` and not `Finished`/`TypedError`.
  - `ExhaustionPreservesRunState`: `ExhaustBudget` does not change `pc`, `frame`, `completed_steps`, or durable state except suspension markers/evidence.
  - `EvidenceRequiresConsumedBudget`: `StepStarted`, `StepSucceeded`, and `SlotWritten` require a preceding `TakeStep` in the slice.
  - `NoSucceededOnExternalSuspend`: action/wait/ask suspension does not imply `StepSucceeded` unless modeled as a separate completed step before suspension.
  - `LegacyTerminalExhaustionForbidden`: no `ExhaustBudget` transition reaches a terminal state.
- Temporal properties:
  - `BudgetSuspensionEventuallyReschedulable`: budget exhaustion remains in a resumable state until fresh budget or explicit external cancellation/typed error.
  - `FreshBudgetEventuallyProgresses`: under weak fairness on `ReplenishBudget` and enabled step/external actions, a budget-suspended non-terminal run eventually starts a step, externally suspends, finishes, or reaches typed error.
  - `NoDeadlockExceptTerminal`: non-terminal states have an enabled next action or explicit suspension/reschedule action.
- Fairness assumptions:
  - Weak fairness on `ReplenishBudget` when scheduler keeps providing slices.
  - Weak fairness on `TakeStep`/completion actions when runnable and budget positive.
  - No fairness assumption that external actions/waits/asks must resolve unless the config explicitly enables `ResumeExternal`.
- Deadlock freedom: TLC must check no deadlock for non-terminal states; terminal stuttering may be explicitly modeled to avoid false deadlocks.
- Refinement to Rust/runtime behavior:
  - `TakeStep` refines successful `StepBudget::try_take() == Ok(true)` before deterministic execution.
  - `ExhaustBudget` refines `try_take() == Ok(false)` and `EngineSignal/RuntimeSignal::StepBudgetExhausted`.
  - `ArithmeticError` refines `EngineError::StepCounterOverflow` or proof-only invariant violation for impossible internal state.
  - `ReplenishBudget` refines scheduler/shard rescheduling with a fresh `StepBudget`.
  - Terminal and external actions refine existing runtime signal variants.

## Evidence Command
- Required discovered command: `tla2tools verification/tla/StepBudgetSuspension.tla`.
- If the later proof writer creates a config-driven TLC command, it must be recorded as an exact replacement and produce equivalent evidence for `verification/tla/StepBudgetSuspension.cfg`.
- Expected evidence: TLC exits 0 with no invariant violations, no unexpected deadlocks, and temporal properties satisfied under configured fairness/bounds.

## Waivers
- None for TLA+ temporal semantics. This bead is explicitly formal-spec-first and temporal.
