# VB_CORE TEST PLAN

**Crate:** `vb_core`
**Date:** 2026-05-10
**Status:** REJECTED — remediation required

---

## SECTION 1 — LETHAL FINDINGS (MUST FIX)

### Finding 1: section36_mandatory_coverage.rs:860 — Bare `assert!(is_err())`

**Location:** `crates/vb_core/tests/section36_mandatory_coverage.rs:860`

**Current Code:**
```rust
let result = step_once(&workflow, &mut frame, &mut store);
assert!(result.is_err());
```

**Problem:** Tests only that `step_once` failed, not WHY it failed. If the error type
changes (e.g., refactor introduces a different error variant), this test passes
incorrectly.

**Required Fix:**
```rust
let result = step_once(&workflow, &mut frame, &mut store);
assert_eq!(
    result,
    Err(CoreError::MissingOutputSlot { step: StepIdx::new(0) }),
    "Copy node with no output slot must fail with MissingOutputSlot"
);
```

**Rationale:** The Copy node at index 0 has `output: None`, which causes
`node_helpers::copy_slot()` to return `EngineError::MissingOutputSlot { step:
StepIdx::new(0) }` per `node_helpers.rs:37`.

**Layer:** Unit (#[cfg(test)] in same module)
**Test Name:** `fn step_once_returns_missing_output_slot_when_copy_node_has_no_output()`

---

### Finding 2: section36_mandatory_coverage.rs:1220 — Bare `assert!(is_ok())`

**Location:** `crates/vb_core/tests/section36_mandatory_coverage.rs:1220`

**Current Code:**
```rust
let result = vb_core::validate_resource_contract(&parts);
assert!(result.is_ok());
```

**Problem:** Tests only that validation succeeded, not the exact return value.
`validate_resource_contract` returns `Result<(), WorkflowError>` so `Ok(())` is
the only success value — asserting `Ok(())` explicitly documents intent.

**Required Fix:**
```rust
let result = vb_core::validate_resource_contract(&parts);
assert_eq!(
    result,
    Ok(()),
    "max_constants at u16::MAX ({MAX_CONSTANTS}) must be accepted"
);
```

**Rationale:** The contract has `max_constants: u16::MAX` (65_535) which equals
`MAX_CONSTANTS`. The validation logic at `validate.rs:28` checks `usize::from(contract.max_constants)
> MAX_CONSTANTS`, so equality passes.

**Layer:** Unit
**Test Name:** `fn validate_resource_contract_accepts_max_constants_at_hard_limit()`

---

### Finding 3: section38_behavioral_properties.rs:411 — Silent discard of step_once result

**Location:** `crates/vb_core/tests/section38_behavioral_properties.rs:411`

**Current Code:**
```rust
let _ = step_once(&workflow, &mut frame, &mut store).map_err(|e| e.to_string())?;
```

**Problem:** `step_once` result is silently discarded. The `.map_err` only converts
the error to a string before discarding — the success path is completely unverified.

**Required Fix:**
```rust
let result = step_once(&workflow, &mut frame, &mut store).map_err(|e| e.to_string())?;
assert_eq!(
    result,
    Ok(EngineSignal::Continue),
    "SetConst step must produce Continue"
)?;
```

**Context:** This is Step 1 of `simple_workflow()`. After SetConst executes,
the signal should be `Continue` and PC should advance to step 1.

**Layer:** Integration (executes workflow step-by-step)
**Test Name:** `fn linear_workflow_step1_produces_continue_and_advances_pc()`

---

### Finding 4: section38_behavioral_properties.rs:549 — Silent discard of step_once result

**Location:** `crates/vb_core/tests/section38_behavioral_properties.rs:549`

**Current Code:**
```rust
let _ = step_once(&workflow, &mut frame, &mut store).map_err(|e| e.to_string())?;
```

**Problem:** Same silent discard pattern as Finding 3.

**Required Fix:**
```rust
let result = step_once(&workflow, &mut frame, &mut store).map_err(|e| e.to_string())?;
assert_eq!(
    result,
    Ok(EngineSignal::Continue),
    "first step must produce Continue"
)?;
```

**Layer:** Integration
**Test Name:** `fn ordering_invariants_pc_advances_monotonically_in_linear_workflow()`

---

### Finding 5: section38_behavioral_properties.rs:646 — Silent discard of run_until_blocked result

**Location:** `crates/vb_core/tests/section38_behavioral_properties.rs:646`

**Current Code:**
```rust
let _ = run_until_blocked(&workflow, &mut frame, StepBudget::MAX, &mut store)
    .map_err(|e| e.to_string())?;
```

**Problem:** `run_until_blocked` result is silently discarded. The workflow
should finish with `EngineSignal::Finished(SlotValue::I64(42), Taint::Clean)`.

**Required Fix:**
```rust
let result = run_until_blocked(&workflow, &mut frame, StepBudget::MAX, &mut store)
    .map_err(|e| e.to_string())?;
assert_eq!(
    result,
    EngineSignal::Finished(SlotValue::I64(42), Taint::Clean),
    "simple_workflow must finish with I64(42)"
)?;
```

**Layer:** Integration
**Test Name:** `fn snapshot_equivalence_step_states_consistent_after_completion()`

---

## SECTION 2 — BEHAVIOR INVENTORY

### 2.1 Core Engine Behaviors

| Behavior | Subject | Action | Outcome when | Layer |
|----------|---------|--------|--------------|-------|
| step_once_copy_no_output | step_once | executes Copy node with output=None | Err(MissingOutputSlot) | unit |
| validate_resource_contract_accepts_max_constants | validate_resource_contract | validates contract with max_constants=65535 | Ok(()) | unit |
| linear_workflow_step_continue | step_once | executes SetConst | Ok(Continue) | integration |
| pc_advances_after_step | step_once | completes step | PC increments | integration |
| workflow_finishes_with_value | run_until_blocked | runs simple_workflow | Finished(I64(42), Clean) | integration |
| step_states_succeeded_after_finish | run_until_blocked | completes workflow | steps[0,1]=Succeeded | integration |
| terminal_state_rejects_running | RunFrame | mark_running on Succeeded | Err | unit |
| failed_rejects_mark_succeeded | RunFrame | mark_succeeded on Failed | Err | unit |
| budget_exhaustion_blocks | run_until_blocked | run with StepBudget(0) | StepBudgetExhausted | unit |

### 2.2 Error Variant Exhaustiveness

Every `CoreError` variant must have a test that produces it:

| Variant | Test Function | Line |
|---------|--------------|------|
| InvalidProgramCounter | `core_error_invalid_program_counter_exact_variant` | errors.rs:861 |
| MissingNextStep | `core_error_missing_next_step_exact_variant` | errors.rs:875 |
| SlotOutOfBounds | `core_error_slot_out_of_bounds_exact_variant` | errors.rs:889 |
| SlotUninitialized | `engine_error_slot_uninitialized_display` | errors.rs:1222 |
| ExprOutOfBounds | `core_error_expr_out_of_bounds_exact_variant` | errors.rs:903 |
| ConstOutOfBounds | `core_error_const_out_of_bounds_exact_variant` | errors.rs:917 |
| MissingOutputSlot | `core_error_missing_output_slot_exact_variant` | errors.rs:931 |
| StepStateOutOfBounds | `core_error_step_state_out_of_bounds_exact_variant` | errors.rs:945 |
| TypeMismatch | `core_error_type_mismatch_exact_variant` | errors.rs:959 |
| NonBoolCondition | `core_error_non_bool_condition_exact_variant` | errors.rs:974 |
| DivisionByZero | `finite_f64_division_by_zero_returns_division_by_zero_error` | section36:80 |
| NonFiniteNumber | existing in expr_eval tests | — |
| StepBudgetExhausted | `budget_exhaustion_does_not_advance_pc` | section36:880 |
| StepCounterOverflow | existing in frame tests | — |
| QueueFull | existing in replay tests | — |
| ResourceLimitExceeded | existing in validation tests | — |
| AllocationFailed | existing in value store tests | — |
| ExpressionStackOverflow | existing in expr_eval tests | — |
| ExpressionStackUnderflow | existing in expr_eval tests | — |
| InvalidCompiledWorkflow | existing in workflow tests | — |
| UnsupportedPrimitive | `step_once_returns_unsupported_primitive_error` | section36 (new) |
| UnsupportedAccessorTraversal | existing in accessor tests | — |
| ObjectFieldNotFound | existing in accessor tests | — |
| ListIndexOutOfBounds | existing in accessor tests | — |
| InternalInvariantViolation | `terminal_state_finished_run_rejects_new_steps` | section38:76 |
| SymbolOutOfBounds | existing in symbols tests | — |
| ListOutOfBounds | existing in list tests | — |
| ObjectOutOfBounds | existing in object tests | — |
| BlobOutOfBounds | existing in blob tests | — |
| IterationLimitExceeded | existing in for_each tests | — |
| RepeatExhausted | existing in repeat tests | — |
| CollectPageLimitExceeded | existing in collect tests | — |
| CollectItemLimitExceeded | existing in collect tests | — |
| CollectTimeLimitExceeded | existing in collect tests | — |
| TogetherBranchLimitExceeded | existing in together tests | — |
| ParallelLimitExceeded | existing in parallel tests | — |
| CapabilityDenied | existing in capability tests | — |
| BudgetExceeded | existing in budget tests | — |

---

## SECTION 3 — TESTING TROPHY ALLOCATION

Target: ~60% integration, ~30% unit, ~5% e2e, ~5% static analysis

### 3.1 Current State
- **Line coverage:** 84.76% (target: ≥90%)
- **Branch coverage:** 72.16% (target: ≥90%)
- **1598 tests passing** across 10 suites

### 3.2 Allocation by Layer

| Layer | Current | Target | Gap |
|-------|---------|--------|-----|
| Unit (#[cfg(test)] in src/) | ~35% | 30% | reduce |
| Integration (tests/) | ~50% | 60% | +10% |
| E2E (separate binary) | ~0% | 5% | +5% |
| Static (clippy, fmt, doc) | ~15% | 5% | reduce |

### 3.3 Coverage Gap Plan

To reach 90% line and branch coverage, the following uncovered branches must be exercised:

1. **Engine signal branches** — `EngineSignal` enum has 8 variants. Cover:
   - `AwaitingAction` (Do node)
   - `AwaitingWait` (WaitUntil/WaitEvent node)
   - `AwaitingAsk` (Ask node)
   - `Continue` (already covered)

2. **Error routing branches** — `ErrorHandlerOutcome` enum:
   - `Routed` path (error handler exists)
   - `NoHandler` path (already covered via MissingOutputSlot)

3. **Frame state transitions** — RunFrame transitions:
   - `Pending → Running → Succeeded` (happy path)
   - `Pending → Running → Failed` (error path)
   - `Pending → Running → Cancelled`
   - `Running → Asking`
   - `Asking → Running`

4. **Accessor evaluation branches** — `eval_accessor` and `eval_accessor_with_store`:
   - Object field access
   - List index access
   - Nested accessor paths

---

## SECTION 4 — BDD GIVEN-WHEN-THEN SCENARIOS

### Scenario 1: step_once fails with MissingOutputSlot when Copy node has no output

**Behavior:** step_once returns MissingOutputSlot when Copy node lacks output slot

```
Given a compiled workflow with a Copy node at step 0 that has no output slot defined
And a run frame with slot 0 initialized to I64(1)
When step_once is called
Then the result must be Err(CoreError::MissingOutputSlot { step: StepIdx(0) })
And step 0 state must be Failed
```

**Test function:** `fn step_once_returns_missing_output_slot_when_copy_node_has_no_output()`

---

### Scenario 2: validate_resource_contract accepts max_constants at hard limit

**Behavior:** Resource contract validation passes when max_constants equals u16::MAX

```
Given a WorkflowParts with ResourceContract max_constants = 65535
When validate_resource_contract is called
Then the result must be Ok(())
```

**Test function:** `fn validate_resource_contract_accepts_max_constants_at_hard_limit()`

---

### Scenario 3: SetConst step produces Continue and advances PC

**Behavior:** After executing a SetConst node, PC advances and signal is Continue

```
Given a simple_workflow (SetConst 42 → Finish)
And a run frame at PC 0
When step_once is called
Then the result must be Ok(EngineSignal::Continue)
And the frame PC must be StepIdx(1)
And step 0 state must be Succeeded
```

**Test function:** `fn linear_workflow_step1_produces_continue_and_advances_pc()`

---

### Scenario 4: PC advances monotonically in linear workflow

**Behavior:** Each step execution advances the program counter

```
Given a simple_workflow
And a run frame at PC 0
When step_once is called (first step)
Then the new PC must be greater than the previous PC
And the new PC must equal StepIdx(1)
```

**Test function:** `fn ordering_invariants_pc_advances_monotonically_in_linear_workflow()`

---

### Scenario 5: Workflow finishes with expected value

**Behavior:** A complete workflow run returns the finished signal with correct value

```
Given a simple_workflow that sets constant 42
And a run frame initialized for this workflow
When run_until_blocked is called with StepBudget::MAX
Then the result must be EngineSignal::Finished(SlotValue::I64(42), Taint::Clean)
```

**Test function:** `fn workflow_finishes_with_expected_value()`

---

### Scenario 6: Step states are Succeeded after workflow completion

**Behavior:** After workflow finishes, all steps are in terminal Succeeded state

```
Given a simple_workflow that completes successfully
When run_until_blocked completes
Then step 0 state must be StepState::Succeeded
And step 1 state must be StepState::Succeeded
```

**Test function:** `fn snapshot_equivalence_step_states_consistent_after_completion()`

---

## SECTION 5 — PROPTEST INVARIANTS

### 5.1 Pure Functions Requiring Invariants

| Function | Input Strategy | Invariant |
|----------|---------------|-----------|
| `FiniteF64::from_i64` | i64 values | Result is finite (not NaN/Inf) |
| `FiniteF64::to_i64` | FiniteF64 values | Round-trip preserves value |
| `SlotValue::from_const_value` | all ConstValue variants | All variants produce Some |
| `StepIdx::new(u16)` | u16 values | If valid (≤MAX), conversion round-trips |
| `SlotIdx::new(u16)` | u16 values | Same invariant |
| `CoreError::diagnostic_code` | all CoreError variants | Code is stable and unique |

### 5.2 Input Classes

**Valid:** Values within defined limits (e.g., i64 within FiniteF64 range)
**Invalid:** NaN, Infinity, values exceeding MAX_* limits
**Boundary:** Exact limit values (e.g., u16::MAX, 0, 1)

### 5.3 Specific Invariants

```rust
// FiniteF64 round-trip
proptest! {
    #[test]
    fn finite_f64_i64_roundtrip(i: i64) {
        let f = FiniteF64::from_i64(i);
        let back = f.to_i64();
        prop_assert_eq!(back, i);
    }
}

// CoreError diagnostic code uniqueness
#[test]
fn core_error_diagnostic_codes_unique() {
    let variants = /* all 38 CoreError variants */;
    let codes: Vec<_> = variants.iter().map(|e| e.diagnostic_code()).collect();
    assert_eq!(codes.len(), codes.iter().collect::<std::collections::HashSet<_>>().len());
}
```

---

## SECTION 6 — FUZZ TARGETS

### 6.1 Parser/Deserializer Targets

| Target | Input Type | Risk Class | Corpus Seeds |
|--------|-----------|------------|--------------|
| `CompiledWorkflow::try_from_parts` | WorkflowParts (JSON) | HIGH | workflow fixtures in tests/ |
| `eval_expr` | Vec\<ExprOp\> + Vec\<ConstValue\> | HIGH | expr_eval fixtures |
| `eval_accessor` | AccessorProgram + ValueStore | MEDIUM | accessor fixtures |

### 6.2 Fuzz Target Specification

```rust
// fuzz/targets/compiled_workflow_parse.rs
#[export_name = "LLVMFuzzerTestOneInput"]
pub extern "C" fn test_one_input(data: &[u8]) -> bool {
    // Deserialize WorkflowParts from JSON
    // Call validate_resource_contract
    // Call step_once if valid
    true
}
```

### 6.3 Corpus Requirements

- Minimum 10 seed inputs per target
- Seeds must cover: happy path, boundary values, error paths
- Update corpus on CI failure to reproduce bugs

---

## SECTION 7 — KANI HARNESSES

### 7.1 Critical Sections for Formal Verification

| Property | Bound | Rationale |
|----------|-------|-----------|
| `step_once` no panic | All valid inputs | Hot path, called per step |
| `RunFrame::pc()` returns valid index | node_count bounds | Used in tight loop |
| `SlotValue::to_const_value()` round-trip | All variants | Persistence boundary |
| Index arithmetic no overflow | MAX_* limits | u16 → usize conversions |

### 7.2 Harness Template

```rust
#[cfg(kani)]
#[kani::proof]
fn step_once_no_panic_on_valid_workflow() {
    // Given: any valid CompiledWorkflow, RunFrame, ValueStore
    // When: step_once is called
    // Then: must not panic
    kani::cover!(true); // exhaustiveness check
}
```

---

## SECTION 8 — MUTATION TESTING CHECKPOINTS

**Target kill rate:** ≥90%

### 8.1 Checkpoint Matrix

| Mutation | Surviving Test | Kill Mechanism |
|----------|---------------|----------------|
| Replace `MissingOutputSlot` with `InvalidProgramCounter` | Finding 1 fix | Exact variant assertion |
| Replace `Ok(())` with `Ok(Some(...))` | Finding 2 fix | Exact value assertion |
| Replace `EngineSignal::Continue` with `Finished` | Findings 3,4 fix | Signal equality |
| Remove `mark_failed` call | All error-path tests | State assertion |
| Replace `StepBudget::MAX` with `StepBudget(0)` | Finding 5 fix | Budget exhaustion test |

### 8.2 Mutation Operators to Test

1. **Conditionals:** Replace `>` with `>=`, `<` with `<=`
2. **Return values:** Replace `Ok(x)` with `Err(x)` on success paths
3. **Boolean constants:** Replace `true` with `false` in guards
4. **Loop bounds:** Modify iteration limits by ±1

---

## SECTION 9 — COMBINATORIAL COVERAGE MATRIX

| Scenario | Input Class | Expected Output | Layer | Priority |
|----------|-------------|-----------------|-------|----------|
| step_once Copy no output | CopyNode {output: None} | Err(MissingOutputSlot) | unit | P0 |
| step_once Copy with output | CopyNode {output: Some(0)} | Ok(Continue) | unit | P1 |
| validate_resource_contract max_constants | u16::MAX | Ok(()) | unit | P0 |
| validate_resource_contract over | u16::MAX + 1 | Err(WorkflowError) | unit | P1 |
| SetConst execution | SetConst { value: 0 } | Ok(Continue) + slot write | unit | P1 |
| Finish execution | Finish { result: 0 } | Ok(Finished) | unit | P1 |
| ExprEval division | (10, 0) | Err(DivisionByZero) | unit | P1 |
| ExprEval overflow | (i64::MAX, 1, Add) | Err(NonFiniteNumber) | unit | P2 |
| RunFrame PC bounds | step_idx >= node_count | Err(InvalidPC) | unit | P2 |
| linear workflow happy path | simple_workflow | Finished(I64(42)) | integration | P0 |
| budget exhaustion | StepBudget(0) | StepBudgetExhausted | integration | P1 |
| error handler routed | workflow with on_error | Continue | integration | P2 |
| PC monotonic advance | any multi-step workflow | PC increases | integration | P1 |

---

## SECTION 10 — EXACT FIXES SUMMARY

| File | Line | Current | Required |
|------|------|---------|----------|
| section36_mandatory_coverage.rs | 860 | `assert!(result.is_err())` | `assert_eq!(result, Err(CoreError::MissingOutputSlot { step: StepIdx::new(0) }))` |
| section36_mandatory_coverage.rs | 1220 | `assert!(result.is_ok())` | `assert_eq!(result, Ok(()))` |
| section38_behavioral_properties.rs | 411 | `let _ = step_once(...).map_err(...)` | `let result = step_once(...).map_err...?; assert_eq!(result, Ok(EngineSignal::Continue))` |
| section38_behavioral_properties.rs | 549 | `let _ = step_once(...).map_err(...)` | `let result = step_once(...).map_err...?; assert_eq!(result, Ok(EngineSignal::Continue))` |
| section38_behavioral_properties.rs | 646 | `let _ = run_until_blocked(...).map_err(...)` | `let result = run_until_blocked(...).map_err...?; assert_eq!(result, EngineSignal::Finished(...))` |

---

## SECTION 11 — COVERAGE REMEDIATION PLAN

### 11.1 Uncovered Code Identification

Run `cargo llvm-cov` to identify:
- Lines 84.76% → 90% = ~200 more lines needed
- Branches 72.16% → 90% = ~100 more branches needed

### 11.2 Priority Areas for Coverage Improvement

1. **EngineSignal variants** (AwaitingAction, AwaitingWait, AwaitingAsk) — require `Do`, `WaitUntil`, `Ask` nodes in test workflows
2. **Error routing** — test `ErrorHandlerOutcome::Routed` path
3. **Accessor evaluation** — test object/list field access paths
4. **Frame state transitions** — test `Pending→Running→Asking→Running` cycle

### 11.3 New Tests Required

```rust
// 1. Do node test (AwaitingAction coverage)
#[test]
fn do_node_produces_awaiting_action() -> Result<(), String> {
    // Workflow with Do node
    // step_once returns Ok(AwaitingAction)
}

// 2. Error handler routed test
#[test]
fn error_handler_routes_successfully() -> Result<(), String> {
    // Workflow with on_error handler
    // Trigger error, verify Routed outcome
}

// 3. Ask node test
#[test]
fn ask_node_produces_awaiting_ask() -> Result<(), String> {
    // Workflow with Ask node
    // step_once returns Ok(AwaitingAsk)
}
```

---

## APPENDIX A — ERROR ENUM REFERENCE

`CoreError` variants (38 total):
```
InvalidProgramCounter, MissingNextStep, SlotOutOfBounds, SlotUninitialized,
ExprOutOfBounds, ConstOutOfBounds, MissingOutputSlot, StepStateOutOfBounds,
TypeMismatch, NonBoolCondition, DivisionByZero, NonFiniteNumber,
StepBudgetExhausted, StepCounterOverflow, QueueFull, ResourceLimitExceeded,
AllocationFailed, ExpressionStackOverflow, ExpressionStackUnderflow,
InvalidCompiledWorkflow, UnsupportedPrimitive, UnsupportedAccessorTraversal,
ObjectFieldNotFound, ListIndexOutOfBounds, InternalInvariantViolation,
SymbolOutOfBounds, ListOutOfBounds, ObjectOutOfBounds, BlobOutOfBounds,
IterationLimitExceeded, RepeatExhausted, CollectPageLimitExceeded,
CollectItemLimitExceeded, CollectTimeLimitExceeded, TogetherBranchLimitExceeded,
ParallelLimitExceeded, CapabilityDenied, BudgetExceeded
```

---

## APPENDIX B — ENGINE SIGNAL REFERENCE

`EngineSignal` variants (8 total):
```
Continue, Finished(Value, Taint), StepBudgetExhausted,
AwaitingAction, AwaitingWait, AwaitingAsk,
Blocked, Panicked
```

---

*Generated by test-planner agent. Do not edit manually.*
