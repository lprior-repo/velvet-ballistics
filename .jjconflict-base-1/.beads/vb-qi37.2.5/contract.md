# Contract Specification - vb-qi37.2.5

## Context
- Bead: `vb-qi37.2.5`.
- Title: `quality: Boundedness adversarial tests`.
- Acceptance: adversarial tests cover runaway loops, fanout, value growth, nested composition, step ceilings, typed bounded failures, no panic, no OOM.
- State 2 scope sources: `codebase-map.md`, `delivery-scope.jsonl`, and `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-qi37.2.5 --json`.

## Domain Terms
- Whole-workflow budget: `WholeWorkflowBudget` from `crates/vb_core/src/budget.rs`, computed from compiled IR and `ResourceContract`.
- Boundedness policy: `BoundednessPolicy::validate`, which rejects budget dimensions above policy ceilings.
- Step budget: `StepBudget` from `crates/vb_core/src/engine/signals.rs`, clamped to `MAX_STEP_BUDGET` and consumed by `drive_deterministic`.
- Value arena cap: `ValueStore::with_max_slots` and `check_arena_cap`, where cap exhaustion returns `CoreError::BudgetExceeded { budget: "max_slots", limit }`.
- Nested composition: collect/reduce/repeat/together/branching structures whose aggregate budget can multiply or maximize resource dimensions.
- Fail closed: admission, validation, or runtime returns a typed bounded error/signal before unbounded CPU, allocation, panic, or process exhaustion.

## Assumptions
- Dependency `vb-qi37.2.2` is closed and provides per-run value arena cap behavior.
- Dependency `vb-qi37.2.4` is still in progress; tests for nested composition must either consume its final verifier API or mark missing verifier hooks as blocked until that dependency lands.
- Pre-existing `vb_runtime` missing `runtime/chunk_001.rs` is `DEFERRED_GLOBAL` and must not block bead-local boundedness contract artifacts.
- This contract plans verification and executable scenarios only; it does not implement production code, tests, TLA+, Verus, Kani, fuzz, or other proof code.

## State 3 Repair Transition After State 11 Blocker
- Repair trigger: State 11 rejected `FUZZ-RESOURCE-001` because the exact approved command `cargo fuzz run resource_budget -- -runs=1000` selected static `x86_64-unknown-linux-musl` with ASAN and failed before executing any malformed-input cases.
- Contract repair: `FUZZ-RESOURCE-001` / `INV-008` no longer requires or claims evidence from the invalid cargo-fuzz `-runs=1000` invocation for the current stdin-once `fuzz/src/bin/resource_budget.rs` driver.
- Valid alternative obligation: build the current stdin driver with `cargo build --manifest-path fuzz/Cargo.toml --features fuzz --bin resource_budget`, execute exactly 1000 deterministic bounded stdin cases, require exact output `resource_budget stdin replay PASS cases=1000`, and pair it with the focused malformed-byte/property test evidence approved by State 7/8/9.
- Waiver boundary: this is not a waiver of `INV-008`; it is a waiver of the invalid cargo-fuzz command as evidence for this driver until a true `libfuzzer_sys::fuzz_target!` harness exists.

## Open Questions
- OQ-001: Which final public constructor/API from `vb-qi37.2.4` will expose nested collect/reduce/repeat/together verifier diagnostics?
- OQ-002: Should adversarial tests live only in existing module tests/integration tests, or should a dedicated `vb_qi37_2_5_*` integration test module be added by test-writer?

## Preconditions
- PRE-001: Adversarial fixtures must build workflows through validated public constructors (`CompiledWorkflow::try_from_parts`, `ResourceContract::validate`, or equivalent), not private invalid states.
- PRE-002: Every generated adversarial size parameter must be bounded before allocation; no scenario may require allocating more than the configured cap to prove rejection.
- PRE-003: Step-budget scenarios must pass an explicit `StepBudget` and assert the exact `EngineSignal::StepBudgetExhausted` or typed error boundary.
- PRE-004: Value-growth scenarios must construct `ValueStore::with_max_slots(max_slots)` with a finite nonzero cap and must observe `total_arena_count() <= max_arena_entries()` after rejection.
- PRE-005: Nested-composition scenarios must define finite fanout, loop counts, repeat counts, gather pages/items, and branch counts before exercising budget computation or verifier admission.
- PRE-006: Acceptance evidence must exclude `vb_runtime` workspace build failure from bead-local failure classification and record it only as `DEFERRED_GLOBAL`.

## Postconditions
- POST-001: Runaway or cyclic-looking deterministic execution consumes at most the configured step budget and returns `EngineSignal::StepBudgetExhausted` or a typed budget error without panic.
- POST-002: Fanout above `BoundednessPolicy::max_fanout` is rejected with `BudgetError::FanoutExceeded { actual, limit }`, where `actual > limit`.
- POST-003: Nesting above `BoundednessPolicy::max_nesting_depth` is rejected with `BudgetError::NestingDepthExceeded { actual, limit }`, where `actual > limit`.
- POST-004: Value arena growth beyond `ValueStore::max_arena_entries()` is rejected with `CoreError::BudgetExceeded { budget: "max_slots", limit }` before the store count exceeds the cap.
- POST-005: Overlarge list/object/blob/symbol payloads are rejected with `CoreError::ResourceLimitExceeded { resource }` matching the exceeded payload dimension.
- POST-006: Nested repeat/together/collect composition that exceeds total steps or executable-step ceilings is rejected with `BudgetError::TotalStepsExceeded`, `BudgetError::StepsExecutableExceeded`, or the verifier's exact boundedness diagnostic.
- POST-007: Accepted bounded workflows preserve computed resource dimensions within `BoundednessPolicy::DEFAULT` and `ResourceContract` hard limits.
- POST-008: All failure paths are typed (`BudgetError`, `CoreError`, `EngineError`, `WorkflowError`, `ValidationError`, or `EngineSignal`) and no adversarial path relies on panic, OOM, timeout, or process kill as success.

## Invariants
- INV-001: `StepBudget::new(x).remaining() <= MAX_STEP_BUDGET` for all `u64` inputs, and `try_take` monotonically decreases or reports no budget.
- INV-002: `drive_deterministic` performs no more deterministic transitions than the mutable `StepBudget` allows and terminates with a blocking signal, finish signal, typed error, or `StepBudgetExhausted`.
- INV-003: For capped `ValueStore`, every successful arena insertion increases `total_arena_count()` by at most one; every rejected insertion leaves `total_arena_count() <= max_arena_entries()`.
- INV-004: `WholeWorkflowBudget::compute` never admits an entry outside `nodes`, detects step-count overflow, and computes all dimensions without unchecked panic.
- INV-005: `BoundednessPolicy::validate` is exact: each budget dimension above its limit maps to the corresponding semantic `BudgetError` variant.
- INV-006: Nested composition accounting is monotonic with respect to adding steps, branches, loops, repeats, gather pages/items, and slot writes; adversarial growth cannot reduce the computed bound.
- INV-007: Hard limits in `crates/vb_core/src/limits.rs` remain nonzero and internally compatible with compact runtime representations.
- INV-008: Parser/codec/hostile inputs for resource budget and compiled IR are bounded and never panic under malformed bytes; for the current stdin-once `resource_budget` driver this is discharged by deterministic stdin replay plus companion malformed-byte/property tests, not by cargo-fuzz `-runs=1000`.

## Error Taxonomy
- `BudgetError::TotalStepsExceeded { actual, limit }` - computed total steps exceed policy.
- `BudgetError::TotalSlotsExceeded { actual, limit }` - computed total slots exceed policy.
- `BudgetError::FanoutExceeded { actual, limit }` - fanout exceeds policy.
- `BudgetError::NestingDepthExceeded { actual, limit }` - nesting depth exceeds policy.
- `BudgetError::ParallelExceeded { actual, limit }` - parallel in-flight exceeds policy.
- `BudgetError::ActionTicketsExceeded { actual, limit }` - action tickets exceed policy.
- `BudgetError::RunTimeExceeded { actual, limit }` - run time exceeds policy.
- `BudgetError::ResultBytesExceeded { actual, limit }` - result bytes exceed policy.
- `BudgetError::StepsExecutableExceeded { actual, limit }` - executable steps exceed policy.
- `CoreError::BudgetExceeded { budget: "max_slots", limit }` - value arena cap rejects insertion.
- `CoreError::ResourceLimitExceeded { resource }` - payload length or ID-space resource exceeds hard limit.
- `EngineError::StepCounterOverflow` - private step counter invariant is violated.
- `EngineSignal::StepBudgetExhausted` - deterministic run slice ran out of fuel.
- `WorkflowError::EntryOutOfBounds { entry }` - workflow budget entry is outside node slice.
- `WorkflowError::StepCountOverflow { actual }` - total step count cannot fit executable compact representation.

## Contract Signatures
- `WholeWorkflowBudget::compute(nodes: &[CompiledNode], entry: StepIdx, contract: &ResourceContract) -> Result<WholeWorkflowBudget, WorkflowError>`.
- `BoundednessPolicy::validate(&self, budget: &WholeWorkflowBudget) -> Result<(), BudgetError>`.
- `StepBudget::new(value: u64) -> StepBudget`.
- `StepBudget::try_take(&mut self) -> Result<bool, EngineError>`.
- `StepBudget::remaining(&self) -> u64`.
- `drive_deterministic(plan: &CompiledWorkflow, run: &mut RunFrame, budget: &mut StepBudget, store: &mut ValueStore) -> Result<EngineSignal, EngineError>`.
- `run_until_blocked(plan: &CompiledWorkflow, run: &mut RunFrame, budget: StepBudget, store: &mut ValueStore) -> Result<EngineSignal, EngineError>`.
- `ValueStore::with_max_slots(max_slots: u16) -> ValueStore`.
- `ValueStore::{insert_symbol,insert_list,insert_list_with_taint,insert_object,insert_blob}(...) -> CoreResult<...>`.
- `ResourceContract::validate(&self) -> ValidationResult<()>`.

## Verus-Owned Clauses
- INV-001: step budget monotonicity and underflow freedom, target `verification/verus/step_budget.rs`.
- INV-006: composition arithmetic boundedness and monotonicity, target `verification/verus/resource_budget.rs`.
- INV-007: hard-limit compatibility where expressible as pure arithmetic.

## TLA+-Owned Clauses
- INV-002, POST-001: temporal execution slice behavior from `Run` through repeated `TakeStep`/`Block`/`Exhaust` transitions.
- POST-006, INV-006: nested composition lifecycle from admitted bounded workflow to rejected over-limit workflow.

## Theorem-Owned Clauses
- None at State 3. Existing Verus kernels are sufficient for Rust-local arithmetic/monotonicity. Lean/Aeneas/Hax is explicitly a non-goal unless future proof review finds a smaller algebraic theorem kernel that Verus cannot express.

## Non-goals
- No production code changes.
- No test implementation in State 3.
- No proof/model code implementation in State 3.
- No full-workspace release certification while `vb_runtime` has the pre-existing missing chunk artifact.
