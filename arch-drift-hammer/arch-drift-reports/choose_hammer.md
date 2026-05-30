# Architectural Drift Report: `choose.rs`

**File**: `crates/vb_core/src/engine/choose.rs`
**Total Lines**: 403 (violates `<300` rule by 103 lines)
**Severity**: CRITICAL

---

## Executive Summary

`choose.rs` is a 403-line file containing branch evaluation logic that violates:
1. **`<300 line` rule** — 103 lines over budget
2. **Scott Wlaschin DDD** — primitive obsession, feature envy, duplicated loop logic
3. **DRY principle** — near-identical `expr` and `slot` choose paths

---

## 1. Line Budget Violation

| Section | Lines | Status |
|---------|-------|--------|
| `choose_*` public functions | 1–73 | OK |
| `jump_to` helper | 114–121 | OK |
| **Tests** | 123–403 | **VIOLATION** |
| **Total** | 403 | **+103 OVER** |

**Root Cause**: 280 lines of tests embedded in production module.

---

## 2. Primitive Obsession Violations

### 2.1 Boolean as Enum Wrapper
```rust
// VIOLATION: SlotValue::Bool(true) / SlotValue::Bool(false)
// Boolean semantics are boxed in an enum
SlotValue::Bool(true) => Ok(Some(branch.target)),
SlotValue::Bool(false) => Ok(None),
```
**Fix**: Extract `AsBool` trait or `BranchCondition` domain type with `bool` interior.

### 2.2 Manual Index Arithmetic
```rust
// VIOLATION: Raw usize index with manual overflow check
let mut index = 0usize;
while index < branches.len() {
    // ...
    index = index.checked_add(1).ok_or({
        EngineError::InternalInvariantViolation { ... }
    })?;
}
```
**Fix**: Use `.iter().enumerate()` or `BranchIter` type that encapsulates traversal.

### 2.3 Raw `Option<StepIdx>` Otherwise
```rust
// VIOLATION: StepIdx is newtype but Option is primitive
otherwise: Option<StepIdx>,
otherwise.ok_or(EngineError::MissingNextStep { step: run.pc() })
```
**Fix**: Create `FallbackTarget(StepIdx)` wrapper or `BranchResult` enum.

### 2.4 Stringly-typed Errors
```rust
// VIOLATION: Raw string reason
EngineError::InternalInvariantViolation {
    reason: "choose expr branch index checked by loop bound",
}
```
**Fix**: Use typed enum variants like `BranchLoopIndexOutOfBounds`.

---

## 3. Duplicated Logic (DRY Violation)

### 3.1 Identical Loop Structure
`choose_expr_target` (lines 21–46) and `choose_slot_target` (lines 75–98) are **85% identical**:

```rust
// BOTH have this exact pattern:
let mut index = 0usize;
while index < branches.len() {
    let branch = branches.get(index).ok_or(EngineError::InternalInvariantViolation {
        reason: "...",
    })?;
    if let Some(target) = choose_*_branch_target(...)?
        return Ok(target);
    index = index.checked_add(1).ok_or({...})?;
}
otherwise.ok_or(EngineError::MissingNextStep { step: run.pc() })
```

### 3.2 Near-Identical Branch Target Resolution
`choose_expr_branch_target` (48–64) and `choose_slot_branch_target` (100–112):
```rust
// BOTH:
match value {
    SlotValue::Bool(true) => Ok(Some(branch.target)),
    SlotValue::Bool(false) => Ok(None),
    other => Err(EngineError::TypeMismatch { expected: "boolean", ... }),
}
```

---

## 4. Feature Envy

The `choose_*` functions operate on primitives rather than domain objects:

| Function | Accesses | Envies |
|----------|----------|--------|
| `choose_expr_target` | `plan`, `run`, `store` | Expression evaluator |
| `choose_slot_target` | `run` only | `RunFrame` |

**Fix**: Move logic into `BranchEvaluator` or `ChoiceContext` domain object.

---

## 5. Refactoring Prescription

### 5.1 Extract Domain Types

```rust
// NEW: domain/branch.rs
pub struct BranchEvaluator {
    run: RunFrame,
    store: ValueStore,
    plan: CompiledWorkflow,
}

impl BranchEvaluator {
    pub fn evaluate_slot(&mut self, branches: &[SlotBranch], fallback: FallbackTarget)
        -> Result<Target, BranchError> { ... }
    
    pub fn evaluate_expr(&mut self, branches: &[ExprBranch], fallback: FallbackTarget)
        -> Result<Target, BranchError> { ... }
}
```

### 5.2 Extract BranchIter

```rust
// NEW: Iterator over branch slice with overflow protection
pub struct BranchIter<'a, B> {
    branches: &'a [B],
    index: usize,
}
```

### 5.3 Extract Boolean Semantic

```rust
// NEW: trait on SlotValue
pub trait AsBool {
    fn as_bool(&self) -> Result<bool, BranchError>;
}
```

### 5.4 Move Tests

```
crates/vb_core/src/engine/
├── choose.rs          # ~120 lines (production only)
└── choose_test.rs     # ~280 lines (tests only)
```

OR move to `crates/workspace_tests/`.

---

## 6. Risk Assessment

| Risk | Level | Rationale |
|------|-------|-----------|
| Line count | CRITICAL | 403 > 300 |
| Duplication | HIGH | 3 functions nearly identical |
| Primitive obsession | HIGH | 4 distinct violations |
| Test isolation | MEDIUM | Tests in production module |

---

## 7. Verdict

**ARCHITECTURAL DRIFT: CONFIRMED**

This file requires mandatory refactoring before landing. All 103 excess lines must be removed via extraction to domain types and test file relocation.

**Priority**:
1. Extract `BranchEvaluator` domain object
2. Extract `BranchIter` iterator wrapper
3. Move 280 test lines to separate test file
4. Replace stringly-typed errors with enum variants
