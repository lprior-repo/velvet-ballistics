# Architectural Drift Report: `accessors.rs`

**File**: `crates/vb_core/src/engine/expr_eval/accessors.rs`
**Line Count**: 731 lines
**Violation**: 231% over the 300-line hard limit

---

## Executive Summary

| Metric | Value | Status |
|--------|-------|--------|
| Total Lines | 731 | ❌ FATAL |
| Production Code | ~180 lines | ⚠️ 24.6% |
| Test Code | ~540 lines | ⚠️ 73.8% |
| Test Infrastructure | ~57 lines | ⚠️ 7.8% |
| Production/Test Ratio | 1:3 | ❌ INVERTED |

---

## Findings

### 1. FATAL: Hard Line Limit Violation

```
LIMIT:  300 lines
ACTUAL: 731 lines
OVER:   431 lines (231% of budget)
```

**Immediate remediation required.**

---

### 2. FATAL: Test Code Buried Inside Production Module

Lines 190–731 (540 lines) are exclusively test code. This violates the **production/test separation principle**. Tests belonging to `accessors.rs` MUST be moved to `crates/vb_core/src/engine/expr_eval/tests/accessors_tests.rs` or `crates/workspace_tests/`.

**Current structure:**
```
accessors.rs
├── Lines 1-189:  Production code + imports
├── Lines 190-731: Inline test module (540 lines)
```

**Required structure:**
```
accessors.rs        # Max 180 lines - production only
accessors_tests.rs  # ~540 lines - tests in separate file
```

---

### 3. CRITICAL: Duplicate Evaluation Logic

#### `eval_accessor_program_without_store` vs `eval_accessor_program`

| Function | Lines | Purpose |
|----------|-------|---------|
| `eval_accessor_program_without_store` | 10-27 | Accessor eval without ValueStore (field/index traversal NOT supported) |
| `eval_accessor_program` | 29-54 | Accessor eval with ValueStore (full traversal) |

Both functions share:
- Identical empty-path guard (lines 14-17 vs 34-37)
- Identical index-bounds checking pattern (lines 41-45 vs similar)
- Identical checked_add overflow protection (lines 47-51 vs 146-150)

**Refactor**: Extract common traversal loop into a generic helper:
```rust
fn eval_accessor_program_inner<F, T>(
    run: &RunFrame,
    program: &AccessorProgram,
    mut get_current: F,
    mut advance: T,
) -> Result<SlotValue, EngineError>
where
    F: FnMut(SlotValue, PathSegment) -> Result<SlotValue, EngineError>,
    T: FnMut(usize) -> usize;
```

---

### 4. CRITICAL: Duplicate Taint Accumulation Logic

`eval_accessor_with_taint_inner` (lines 118-153) and `eval_accessor_program` (lines 29-54) are structurally identical except for taint tracking.

**Refactor**: Compose the pure evaluation on top of taint-aware evaluation:
```rust
pub(super) fn eval_accessor_program(...) -> Result<SlotValue, EngineError> {
    let (value, _taint) = eval_accessor_program_with_taint(run, store, program)?;
    Ok(value)
}
```

---

### 5. HIGH: Primitive Obsession in Segment Indexing

```rust
// Line 39 - raw usize
let mut index = 0usize;

// Lines 41-45 - manual bounds checking instead of iterator
let segment = program.path.get(index).copied().ok_or({...});

// Lines 47-51 - manual overflow protection
index = index.checked_add(1).ok_or(...);
```

**Scott Wlaschin DDD**: A **Path** is a Value Object with **semantic meaning**, not a raw `Vec<PathSegment>` with index manipulation. The traversal should be expressed as:
```rust
fn eval_accessor_program(...) -> Result<SlotValue, EngineError> {
    program.path.iter().try_fold(current, |acc, segment| {
        traverse_accessor_segment(store, acc, *segment)
    })
}
```

**Primitive obsession**: Raw `usize` for indices, manual bounds checking, manual overflow protection — these are responsibilities of the iterator adapter, not the business logic.

---

### 6. HIGH: Helper Function Duplication in Tests

Test helpers are repeated 3 times with slight variations:

| Helper | Lines | Purpose |
|--------|-------|---------|
| `ensure_equal` | 202-211 | Generic equality assertion |
| `accessor_workflow` | 213-215 | Wrapper for workflow creation |
| `accessor_workflow_with_symbols` | 217-247 | Full workflow creation |
| `test_frame` | 249-251 | RunFrame factory |

**Refactor**: Move to `crates/vb_core/src/engine/expr_eval/tests/helpers.rs`:
```rust
pub mod test_helpers {
    pub fn make_accessor_workflow(path: Vec<PathSegment>) -> CompiledWorkflow { ... }
    pub fn make_test_frame() -> RunFrame { ... }
    pub fn assert_eq<T: Debug + PartialEq>(actual: T, expected: T) -> Result<(), String> { ... }
}
```

---

### 7. MEDIUM: Function Bloat from Error Repetition

Every error case repeats the same pattern:
```rust
EngineError::InternalInvariantViolation {
    reason: "accessor path index checked by loop bound",
}
```

**Refactor**: Add a constructor to `EngineError`:
```rust
impl EngineError {
    fn accessor_bounds_invariant() -> Self {
        Self::InternalInvariantViolation {
            reason: "accessor path index checked by loop bound",
        }
    }
}
```

---

## Required Refactoring Plan

### Phase 1: Extract Tests (Cost: ~30 minutes)
1. Create `crates/vb_core/src/engine/expr_eval/tests/accessors_tests.rs`
2. Move `mod tests { ... }` block (lines 190-731) to new file
3. Update imports to use relative paths to production module
4. Extract test helpers to `tests/helpers.rs`

### Phase 2: Eliminate Duplication (Cost: ~45 minutes)
1. Replace raw-index loop with `Iterator::try_fold` in `eval_accessor_program`
2. Compose `eval_accessor_program` from `eval_accessor_with_taint_inner`
3. Extract `InternalInvariantViolation` factory methods

### Phase 3: Path Value Object Enhancement (Cost: ~30 minutes)
1. Add `AccessorProgram::traverse<'a>() -> impl Iterator<Item = &'a PathSegment>` method
2. Replace manual indexing with `.iter().try_fold()`

### Expected Outcome
```
accessors.rs:           731 lines → ~150 lines (-581)
accessors_tests.rs:     0 lines → ~540 lines
tests/helpers.rs:       0 lines → ~60 lines
```

---

## Verification Checklist

- [ ] `accessors.rs` compiles under `cargo check --lib`
- [ ] New test file compiles and all tests pass
- [ ] `accessors.rs` line count ≤ 300
- [ ] No `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg` in production code
- [ ] Moon v2 lint gates pass (`moon ci` or equivalent)
- [ ] Kani/Fuzz harnesses still valid after refactor

---

## Severity Summary

| Severity | Count | Issues |
|----------|-------|--------|
| FATAL | 2 | Line limit exceeded (731 > 300), Tests inside production module |
| CRITICAL | 2 | Duplicate evaluation logic, Duplicate taint logic |
| HIGH | 2 | Primitive obsession (raw usize indexing), Test helper duplication |
| MEDIUM | 1 | Error factory repetition |
| LOW | 0 | — |

**Total Findings**: 7

---

*Report generated by arch-drift-hammer on 2026-05-29*
*Agent: architectural-drift enforcer*
