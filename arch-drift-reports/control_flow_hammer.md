# Architectural Drift Report: `control_flow.rs`

**File**: `crates/vb_validate/src/control_flow.rs`
**Total Lines**: 753 (VIOLATION: <300 rule)
**Report Type**: Architectural Drift Hammer
**Date**: 2026-05-29

---

## EXECUTIVE SUMMARY

| Category | Severity | Count |
|----------|----------|-------|
| Line Count Violation | CRITICAL | 1 (753 > 300) |
| Primitive Obsession | CRITICAL | 5 |
| DDD Cohesion Violation | HIGH | 2 |
| Mixed Responsibilities | MEDIUM | 1 |

---

## 1. LINE COUNT VIOLATION

**Rule**: All source files must be ≤300 lines.
**Status**: ❌ FAIL — 753 lines detected (251% over limit)

The file MUST be split. Suggested decomposition:

| Slice | Est. Lines | Contains |
|-------|------------|----------|
| `control_flow/model.rs` | ~45 | `WorkflowFlow`, `StepFlow` types only |
| `control_flow/validation.rs` | ~100 | Pure validation functions |
| `control_flow/tests.rs` | ~580 | All test modules |

---

## 2. PRIMITIVE OBSESSION VIOLATIONS (CRITICAL)

### 2.1 Raw `usize` for Step Indices
**Location**: Lines 14, 21, 24, 27, 34, 46, 57, 64, 76, 94, 112, 130-145

```rust
// VIOLATION: Raw usize for domain concept "step index"
pub fn validate_control_flow(flow: &WorkflowFlow) -> ValidationResult<()> {
    for (step_index, step) in flow.steps.iter().enumerate() {
        for &target in &step.branch_targets {
            validate_forward_target(step_index, target, flow.steps.len())?;
        }
```

**Domain Concept**: `StepIndex` — a bounded, non-negative step identifier within a workflow.
**Fix**: Create `newtype StepIndex(u16)` or `NonZeroUsize` wrapper with bounded range validation.

### 2.2 Raw `Vec<usize>` for Branch Targets
**Location**: Lines 22, 47, 98, 142

```rust
// VIOLATION: Vec<usize> instead of typed collection
pub branch_targets: Vec<usize>,
```

**Domain Concept**: `BranchTargetSet` — ordered collection of valid forward branch destinations.
**Fix**: `BranchTargets(Vec<StepIndex>)` newtype with validation on construction.

### 2.3 Raw `Option<usize>` for Then Target
**Location**: Lines 26, 53, 103, 144

```rust
// VIOLATION: Option<usize> instead of domain optional
pub then_target: Option<usize>,
```

**Domain Concept**: `ThenTarget` — optional explicit continuation to a forward step.
**Fix**: `ThenTarget(Option<StepIndex>)` newtype.

### 2.4 Raw `bool` Array for Reachability
**Location**: Lines 40, 80-88

```rust
// VIOLATION: Raw bool array instead of domain set
let mut reachable = vec![false; flow.steps.len()];
```

**Domain Concept**: `ReachableSteps` — a set of step indices that are reachable from entry.
**Fix**: `ReachableSteps(BitSet<StepIndex>)` or `ReachableSteps(HashSet<StepIndex>)`.

### 2.5 Raw `Vec<usize>` for DFS Stack
**Location**: Lines 77-78

```rust
// VIOLATION: Exposing raw stack of indices
let mut stack = Vec::with_capacity(flow.steps.len());
stack.push(0_usize);
```

**Domain Concept**: `Worklist<StepIndex>` — internal traversal frontier.
**Fix**: Private to validation module, not exposed in public types.

---

## 3. DDD COHESION VIOLATIONS

### 3.1 Validation Logic Mixed with Domain Model
**Location**: Lines 10-123 (validation) + Lines 129-145 (model)

The `WorkflowFlow` and `StepFlow` types are defined in the same file as validation functions. Per DDD principles, the domain model should be in its own crate/module with no validation dependencies.

**Required Structure**:
```
vb_validate/src/
├── control_flow/
│   ├── mod.rs           (~5 lines - re-exports)
│   ├── model.rs         (~45 lines - WorkflowFlow, StepFlow ONLY)
│   ├── validation.rs    (~100 lines - pure validation functions)
│   └── tests.rs         (~580 lines - all test modules)
```

### 3.2 `ValidationError` Dependency in Domain Model
**Location**: Lines 8, 130

The domain model (`WorkflowFlow`, `StepFlow`) carries `#[derive(Debug, Clone, Default)]` but the module imports `ValidationError` from the parent crate. The model should be validation-framework agnostic.

```rust
use crate::{ValidationError, ValidationResult};  // Line 8 - VIOLATION
```

**Fix**: Move domain types to `vb_core` or a separate `control_flow_types` module with no validation imports.

---

## 4. FUNCTIONAL CORE VIOLATION

### 4.1 Mutations Hidden in "Validation"
**Location**: Lines 76-92 (`mark_reachable`)

```rust
fn mark_reachable(flow: &WorkflowFlow, reachable: &mut [bool]) -> ValidationResult<()> {
    let mut stack = Vec::with_capacity(flow.steps.len());
    stack.push(0_usize);
    while let Some(index) = stack.pop() {
        if *reachable.get(index).ok_or(ValidationError::InvalidThenTarget)? {
            continue;
        }
        *reachable.get_mut(index).ok_or(ValidationError::InvalidThenTarget)? = true;
        push_successors(flow, index, &mut stack);
    }
    Ok(())
}
```

This is imperative algorithm code mutating state. It should be refactored to a pure function returning `ReachableSteps` or a `Result<ReachableSteps, ValidationError>`.

---

## 5. REACHABILITY ALGORITHM DEFECTS

### 5.1 Self-Cycle Detection Gap
**Location**: Lines 45-55 (`validate_forward_targets`)

```rust
fn validate_forward_targets(flow: &WorkflowFlow) -> ValidationResult<()> {
    for (step_index, step) in flow.steps.iter().enumerate() {
        for &target in &step.branch_targets {
            validate_target_index(target, flow.steps.len())?;
            if target <= step_index {  // Only catches backward, NOT self-cycle
                return Err(ValidationError::ControlFlowCycle);
            }
        }
    }
    Ok(())
}
```

**Bug**: `target <= step_index` catches backward branches (target < step_index) and self-cycles (target == step_index). However, the check is only in `validate_forward_targets`, NOT in `validate_forward_only_then` which calls `validate_forward_target` (line 64-74) which DOES check `target <= step_index`. The redundancy is confusing but not incorrect.

### 5.2 `push_successors` Has Unchecked `target < flow.steps.len()` Guard
**Location**: Lines 98-109

```rust
fn push_successors(flow: &WorkflowFlow, index: usize, stack: &mut Vec<usize>) {
    let Some(step) = flow.steps.get(index) else {
        return;
    };
    for &target in &step.branch_targets {
        if target < flow.steps.len() {  // Guard exists but...
            stack.push(target);
        }
    }
    if let Some(then_target) = step.then_target {
        if then_target < flow.steps.len() {  // ...duplicated here
            stack.push(then_target);
        }
    } else if let Some(next) = index.checked_add(1).filter(|&n| n < flow.steps.len()) {
        stack.push(next);
    }
}
```

This redundancy between `push_successors` bounds check and the earlier `validate_forward_targets`/`validate_target_index` checks is a cohesion smell. The validation layer should guarantee these invariants before calling internal algorithms.

---

## 6. TESTING ARCHITECTURE VIOLATION

**Location**: Lines 148-753 (605 lines of tests in same file)

Tests are 80% of this file. Per repository structure rules, tests belong in `crates/workspace_tests/` or behind a `#[cfg(test)]` feature-gated module that gets its own file.

**Recommended Extraction**:
```rust
// control_flow/tests/adversarial.rs
// control_flow/tests/bdd_exact.rs  
// control_flow/tests/flow_factories.rs
```

---

## 7. SUMMARY OF REQUIRED REFACTORS

| # | Action | Priority |
|---|--------|----------|
| 1 | Split file into `model.rs`, `validation.rs`, `tests.rs` | CRITICAL |
| 2 | Create `StepIndex(u16)` newtype, replace raw `usize` | CRITICAL |
| 3 | Create `BranchTargets(Vec<StepIndex>)` wrapper | CRITICAL |
| 4 | Create `ThenTarget(Option<StepIndex>)` wrapper | CRITICAL |
| 5 | Move domain types to validation-framework-agnostic module | HIGH |
| 6 | Refactor `mark_reachable` to pure function | MEDIUM |
| 7 | Extract tests to separate test files | MEDIUM |
| 8 | Remove redundant bounds checks in `push_successors` | LOW |

---

## 8. VERDICT

**ARCHITECTURAL DRIFT STATUS**: ❌ FAIL

This file is a textbook example of **primitive obsession** combined with **god file** anti-pattern. The domain concepts of step indexing, branch targets, and control flow edges are all represented as raw Rust primitives (`usize`, `Vec<usize>`, `Option<usize>`, `bool`). This makes the validation logic fragile and harder to evolve.

**Estimated Refactor Effort**: 2-3 beads

**First Action**: Create `crates/vb_validate/src/control_flow/model.rs` and extract `WorkflowFlow` and `StepFlow` types with newtype wrappers for indices.
