# Verification Layers

## Boundary
- TLA+ temporal model: `verification/tla/StepBudgetSuspension.tla` and `.cfg` for exhaustion/suspension/resume.
- Verus kernel: existing/future `verification/verus/step_budget.rs` and `verification/verus/run_loop_termination.rs` obligations tied to actual Rust targets.
- Kani/proptest/tests: concrete Rust no-panic, no-underflow, no evidence corruption, and runtime lifecycle behavior.
- Runtime shell: shard scheduler and external event plumbing verified by scoped Rust tests/review, not theorem proof.

## Layer Assignment
- PRE-001 -> TLA+ valid init + Rust resume-state tests.
- PRE-002/PRE-005/INV-001 -> Verus + Kani + TLA+ bounded arithmetic.
- PRE-003/POST-002/INV-003 -> Verus + Kani + TLA+ `TakeStep`.
- PRE-004/POST-001/INV-002 -> TLA+ `ExhaustBudget` + Rust zero-budget tests.
- POST-003/INV-006/INV-007 -> TLA+ preservation invariant + runtime/core tests.
- POST-004/INV-005/INV-009 -> TLA+ liveness/fairness + shard lifecycle tests.
- POST-006/INV-008 -> TLA+ external-suspension distinction + runtime evidence tests.
- INV-010 -> TLA+ invariant forbidding terminal exhaustion + review of legacy mismatch.
- Error taxonomy -> TLA+ explicit signal/error states + Rust enum/signal review.
- Static source governance -> existing `moon ci` / scoped lint gates in later states.

## TLA+ Scope
- Model: `verification/tla/StepBudgetSuspension.tla`.
- Config: `verification/tla/StepBudgetSuspension.cfg`.
- Variables: `pc`, `frame`, `budget`, `run_state`, `last_signal`, `evidence`, `consumed_steps`, `completed_steps`, `reschedule_pending`, `arith_error`.
- Actions: `Init`, `TakeStep`, `CompleteContinue`, `CompleteFinished`, `CompleteTypedError`, `BlockOnAction`, `BlockOnWait`, `BlockOnAsk`, `ExhaustBudget`, `ReplenishBudget`, `ResumeExternal`, `ArithmeticError`.
- Safety invariants: `BudgetWithinBounds`, `NoBudgetUnderflowOrWrap`, `ExhaustionNonTerminal`, `ExhaustionPreservesRunState`, `EvidenceRequiresConsumedBudget`, `NoSucceededOnExternalSuspend`, `LegacyTerminalExhaustionForbidden`.
- Temporal properties: `BudgetSuspensionEventuallyReschedulable`, `FreshBudgetEventuallyProgresses`, `NoDeadlockExceptTerminal`.
- Fairness/deadlock stance: weak fairness for scheduler replenishment and enabled runnable steps; no forced external event resolution unless configured.
- Evidence command: `tla2tools verification/tla/StepBudgetSuspension.tla`.

## Verus Scope
- Targets: `vb_core::StepBudget::new`, `vb_core::StepBudget::try_take`, `vb_core::drive_deterministic`/`run_until_blocked` abstraction, existing `verification/verus/step_budget.rs`, and `verification/verus/run_loop_termination.rs`.
- Spec surface: bounded budget predicate, clamp postcondition, zero-exhaustion postcondition, positive decrement postcondition, no-underflow invariant, no state mutation before successful budget consumption.
- Trusted boundary: validated `StepBudget` constructors and abstraction relation from concrete Rust run/frame to proof model.
- Shell exclusions: storage, external event delivery, wall-clock time, shard scheduling, and evidence transport.
- Evidence command: blocked for exact Verus command until proof planner/writer confirms current local verifier invocation; do not invent module commands beyond discovered files.

## Kani/Proptest/Rust Test Scope
- Kani: prove representative arbitrary/core generated budgets cannot underflow/wrap or panic and that zero budget does not mutate frame.
- Proptest: generate budget/run-state combinations for concrete invariants if existing strategies support them.
- Rust tests: scoped nextest/cargo tests for `vb_core` and `vb_runtime` budget/suspension behavior already identified in State 2.
- Evidence command: exact command deferred to State 4/5 proof/test planning because required verifier modes line was truncated in State 2 input and this state must not invent proof targets.

## Review Scope
- Independent contract verification review must reject any model where `ExhaustBudget` is terminal.
- Review must check that the TLA+ spec uses bounded arithmetic definitions and explicit error/sink transitions.
- Review must check traceability from every contract clause to proof/test/review evidence.

## Waivers
- Lean theorem waiver: optional only; Verus and TLA+ own the necessary proof surface.
- Parser/codec fuzzing waiver: no parser/codec boundary is in bead scope.
- Performance/API/release waiver: no speed, public API compatibility, or release-provenance claim is made by this contract.
