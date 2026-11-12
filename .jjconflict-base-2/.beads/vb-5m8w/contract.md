# Contract Specification: Step Budget Suspension

## Context
- Bead: `vb-5m8w` / Add TLA+ Step Budget Model.
- Feature: formal-spec-first temporal contract for deterministic step-budget exhaustion.
- Scope source: State 1/2 artifacts in `/home/lewis/src/go-skill-vb-5m8w/.beads/vb-5m8w/`.
- Implementation bound found by explore: `MAX_STEP_BUDGET: u64 = 10_000`; TLA+ must model bounded unsigned arithmetic and no unbounded `Nat` shortcut.
- Existing behavior surface: `vb_core::StepBudget`, `EngineSignal::StepBudgetExhausted`, `run_until_blocked`, `drive_deterministic`, `RuntimeSignal::StepBudgetExhausted`, `drive_deterministic_full`, shard lifecycle `apply_drive_result`.

## Domain Terms
- Step budget: bounded per-drive deterministic step allowance, represented as a u64 value clamped to `MAX_STEP_BUDGET`.
- Consumed step: a step for which budget was successfully taken before execution began.
- Exhausted: no budget remains before a new deterministic step can start.
- Suspended: non-terminal run state retained for later scheduling/resume.
- External suspension: `AwaitingAction`, `AwaitingWait`, or `AwaitingAsk`; distinct from budget exhaustion.
- Terminal: `Finished` or typed runtime/workflow failure unrelated to budget exhaustion.
- Evidence: runtime events such as `StepStarted`, `StepSucceeded`, `SlotWritten`, and `DriveContinue`.

## Assumptions
- This state writes contract/planning artifacts only; TLA+ model code belongs to later proof-writing state.
- Future model path is `verification/tla/StepBudgetSuspension.tla` with config `verification/tla/StepBudgetSuspension.cfg`.
- Exact TLC integration command available from explored tooling is `tla2tools verification/tla/StepBudgetSuspension.tla`.
- Small TLC constants may use tiny finite budgets, but the TLA+ arithmetic operator definitions must encode the production u64/`MAX_STEP_BUDGET` bound and explicit overflow/underflow error states.

## Open Questions
- Whether the final proof lane should also add `contracts/proof_obligations.yaml`; this is a later state decision because this state must not edit config/proof model files.
- Whether the eventual TLA+ config will use TLC-only constants or an additional Apalache config; no Apalache command was discovered in State 2.

## Preconditions
- PRE-001: A drive slice starts with a validated run state: `Runnable`, `SuspendedBudget`, or an external-suspension state that can be resumed by its matching event.
- PRE-002: The supplied budget is a bounded unsigned integer value; values above `MAX_STEP_BUDGET` are clamped before use.
- PRE-003: A deterministic step may start only after `try_take` succeeds and consumes exactly one budget unit.
- PRE-004: If remaining budget is zero before a step starts, the engine must not start or complete a step in that slice.
- PRE-005: The TLA+ model must include bounded arithmetic states for zero, positive values, `MAX_STEP_BUDGET`, and invalid/out-of-range arithmetic.

## Postconditions
- POST-001: Zero-budget entry returns `StepBudgetExhausted`/`RuntimeSignal::StepBudgetExhausted` as graceful suspension, not `Finished`, not typed failure, and not panic.
- POST-002: A positive budget slice decrements by exactly one per started deterministic step and never decrements when no step starts.
- POST-003: Budget exhaustion preserves the current PC/frame/run state except for effects of already completed consumed steps.
- POST-004: Budget exhaustion keeps the run eligible for reschedule/resume with a fresh budget.
- POST-005: Runtime lifecycle maps budget exhaustion to continue/reschedule semantics (`DriveContinue`-like), not terminal cleanup or workflow loss.
- POST-006: `AwaitingAction`, `AwaitingWait`, and `AwaitingAsk` remain distinct from budget exhaustion and preserve their own wake/resume contracts.

## Invariants
- INV-001: Budget value is always in `0..=MAX_STEP_BUDGET`; no underflow, wrap, or negative state is reachable.
- INV-002: `try_take` at zero returns false/exhausted and leaves budget and run state unchanged.
- INV-003: `try_take` on a positive valid budget decrements exactly once and returns true.
- INV-004: A step cannot emit `StepStarted`, `StepSucceeded`, or `SlotWritten` unless a budget unit was consumed for that step.
- INV-005: Budget exhaustion is a non-terminal scheduler suspension state; it never transitions directly to `Finished`, terminal failure, terminal cleanup, or workflow deletion.
- INV-006: PC/frame state preservation: exhaustion before a step starts cannot advance PC, mutate frame, mark running, mark succeeded, or write slots.
- INV-007: Already completed consumed steps remain durable and are not rolled back when the later slice exhausts budget.
- INV-008: External suspension outcomes do not emit `StepSucceeded` unless the step actually completed successfully.
- INV-009: Under weak fairness with recurring fresh positive budgets, every non-terminal budget-suspended run eventually either starts another step, externally suspends, finishes, or reaches an explicit typed error.
- INV-010: The legacy terminal-exhaustion model is non-authoritative for this bead; any model making `ExhaustBudget` terminal violates this contract.

## Error Taxonomy
- `StepBudgetExhausted`: non-error scheduler suspension returned when budget is zero before the next deterministic step can start.
- `StepCounterOverflow`: error when an internal budget value exceeds `MAX_STEP_BUDGET` or bounded arithmetic detects invalid/out-of-range state.
- `TypedStepError`: terminal typed workflow/runtime error from executing a consumed step; distinct from budget exhaustion.
- `InvalidResumeState`: error when a caller attempts to resume a non-resumable terminal or corrupt state.
- `InvariantViolation`: proof/model-only sink for impossible states such as underflow, wrap, terminal exhaustion, or evidence without consumed budget.

## Contract Signatures
- `StepBudget::new(value: u64) -> StepBudget` clamps `value` to `MAX_STEP_BUDGET`.
- `StepBudget::try_take(&mut self) -> Result<bool, EngineError>` returns `Ok(false)` at zero, `Ok(true)` after one valid decrement, or `Err(StepCounterOverflow)` for invalid internal state.
- `run_until_blocked(..., budget: StepBudget, ...) -> Result<EngineSignal, EngineError>` returns `StepBudgetExhausted` on budget depletion without panicking or losing run state.
- `drive_deterministic_full(..., budget: StepBudget, ...) -> Result<RuntimeSignal, RuntimeError>` refines core exhaustion to runtime graceful suspension.
- `apply_drive_result(RuntimeSignal::StepBudgetExhausted, run) -> Result<(), RuntimeError>` preserves the run and schedules continuation semantics.

## TLA+-Owned Clauses
- PRE-001 through PRE-005.
- POST-001 through POST-006.
- INV-001 through INV-010.
- Temporal liveness: INV-009.
- Legacy mismatch guard: INV-010.

## Verus-Owned Clauses
- INV-001, INV-002, INV-003, INV-006: Rust-local bounded arithmetic and pure transition invariants should remain tied to actual `StepBudget`/run-loop implementation, not separate vacuum proofs.

## Theorem-Owned Clauses
- No mandatory Lean theorem for this bead. Lean may be used later only for a tiny bounded arithmetic lemma if Verus cannot express the u64/clamp/decrement relation compactly.

## Non-goals
- No production code changes in State 3.
- No TLA+ model code in State 3.
- No performance, API compatibility, release-provenance, parser/codec, storage, or network claims.
