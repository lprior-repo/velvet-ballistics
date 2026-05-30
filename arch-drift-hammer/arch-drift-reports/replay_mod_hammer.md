# Architectural Drift Report: `vb_core/src/replay/mod.rs`

**Target:** `/home/lewis/src/velvet-ballistics/crates/vb_core/src/replay/mod.rs`  
**Status:** ❌ **CRITICAL DRIFT VIOLATION**  
**Line Count:** 348 lines (VIOLATES <300 line mandate)

---

## 1. EXECUTIVE SUMMARY

The `replay` module violates multiple architectural constraints:

| Rule | Status | Details |
|------|--------|---------|
| File size <300 lines | ❌ VIOLATED | 348 lines (+16%) |
| Primitive obsession ban | ❌ VIOLATED | `StepIdx::ZERO` as placeholder, raw `u8`, raw `usize` |
| DDD cohesion | ⚠️ FRAGILE | `eval_expr_for_replay` is orphan function |
| Error domain purity | ⚠️ FRAGILE | `slot_to_replay_err` free function leaks error mapping |
| No `unwrap`/`panic` | ✅ PASS | Clean error handling throughout |

---

## 2. MODULE RESPONSIBILITY MAP

```
replay/mod.rs (348 lines)
├── Error types (ReplayError enum)          [L25-56]
├── ReplayExprStack                         [L62-124]
├── slot_to_replay_err helper               [L130-138]
├── ReplayEngine                            [L144-278]
│   ├── replay_up_to                        [L169-176]
│   ├── replay_frame_up_to                  [L179-187]
│   ├── replay_frame_through                [L190-198]
│   ├── ensure_step_exists                  [L200-205]
│   ├── new_replay_frame                    [L207-217]
│   ├── replay_until                        [L219-251]
│   └── replay_one                          [L253-277]
├── ReplayTargetMode enum                   [L280-284]
├── ReplayFoldStop enum                     [L286-298]
├── replay_step_budget_len                  [L300-305]
├── eval_expr_for_replay (ORPHAN)           [L307-337]
└── Re-exports                              [L339-342]
```

---

## 3. PRIMITIVE OBSESSION VIOLATIONS

### 3.1 `StepIdx::ZERO` as Placeholder for Unknown Step

**Location:** Lines 77, 86, 93, 99, 107, 113, 120, 139, 149, 159, 169, 170, 258, 267, 268

```rust
// In ReplayExprStack::new
Err(ReplayError::ExpressionEvalFailed {
    step: StepIdx::ZERO,  // ❌ What step? Unknown!
})

// In ops.rs arithmetic operations
Err(ReplayError::ExpressionEvalFailed {
    step: StepIdx::ZERO,  // ❌ Which step caused overflow?
})
```

**Problem:** When arithmetic overflow occurs inside expression evaluation, the code has no knowledge of which step is executing. Using `StepIdx::ZERO` as a sentinel is:
- Misleading (looks like step 0 specifically)
- Violates "make illegal states unrepresentable" (ZERO is a valid step index)

**Fix:** Add a new `ReplayError` variant:
```rust
ExpressionEvalFailed {
    step: Option<StepIdx>,  // None = unknown step context
}
```

Or create a separate `StackOverflow` variant that doesn't require a step index.

---

### 3.2 Raw `u8` for Stack Length/Capacity

**Location:** `ReplayExprStack` lines 62-66

```rust
pub struct ReplayExprStack {
    values: [SlotValue; crate::limits::MAX_EXPRESSION_STACK_USIZE],
    len: u8,        // ❌ Raw primitive
    capacity: u8,   // ❌ Raw primitive
}
```

**Problem:** `u8` silently truncates for values > 255. While the constant is u8-sized, the semantics should be expressed as a proper `StackSize` newtype.

**Fix:**
```rust
#[repr(transparent)]
struct StackLen(u8);

#[repr(transparent)]  
struct StackCapacity(u8);
```

---

### 3.3 Raw `usize` Index in Expression Evaluation Loop

**Location:** `eval_expr_for_replay` lines 318-330

```rust
let mut index = 0usize;  // ❌ Raw usize
while index < program.ops.len() {
    let op = program.ops.get(index)...;
    index = index.checked_add(1)...;  // Manual overflow check
}
```

**Problem:** This pattern repeats the same bounds-check the iterator would do automatically. Using raw `usize` for an index that never escapes the loop is minor but inconsistent with the strict typing elsewhere.

**Fix:** Use iterator pattern:
```rust
for op in program.ops.iter() {
    ops::eval_replay_op(plan, run, store, *op, &mut stack, &mut taint_accum)?;
}
```

---

### 3.4 `RunId::new(0)` Placeholder

**Location:** `new_replay_frame` line 209

```rust
RunId::new(0),  // ❌ Magic zero - what does this mean?
```

**Problem:** The replay engine creates a `RunFrame` with `RunId::new(0)`. This suggests replay doesn't need a "real" run ID, but using raw `0` obscures this intent.

**Fix:** Consider a `RunId::for_replay()` associated function or `RunId::UNKNOWN`.

---

## 4. DDD COHESION VIOLATIONS

### 4.1 Orphan Function: `eval_expr_for_replay`

**Location:** Lines 307-337

```rust
pub(crate) fn eval_expr_for_replay(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    expr: ExprIdx,
) -> Result<(SlotValue, Taint), ReplayError> {
```

**Problem:** This function is tightly coupled to `ReplayEngine` (it uses `ReplayExprStack` and returns `ReplayError`) but lives as a standalone function in `mod.rs`. Per Scott Wlaschin DDD, this is a "bag of related functions" rather than a true domain type.

**Fix:** Move into `ReplayEngine` as a private method `eval_expression()`.

---

### 4.2 Free Function: `slot_to_replay_err`

**Location:** Lines 130-138

```rust
fn slot_to_replay_err(e: EngineError) -> ReplayError {
```

**Problem:** Error conversion is a domain concern that belongs to the `ReplayError` type, either as a `From<EngineError>` impl or a method on `ReplayEngine`.

**Fix:** Implement `From<EngineError> for ReplayError` in the appropriate scope, or move to `ReplayEngine` as `Self::map_engine_error(e)`.

---

### 4.3 `ReplayFoldStop` Leaks Internal Implementation

**Location:** Lines 286-298

```rust
enum ReplayFoldStop {
    Done(RunFrame),
    Error(ReplayError),
}
```

**Problem:** `replay_until` uses `try_fold` with this custom sentinel type. The `into_result` method is a escape hatch. This is an implementation detail that seeps into the module's public interface through the fold machinery.

**Fix:** Consider using a custom `ReplayResult` type that wraps `Result<RunFrame, ReplayError>` with additional state, or use ` infallible` from `Try` trait to avoid the extra enum.

---

## 5. ARCHITECTURE CONCERNS

### 5.1 `replay_step_budget_len` Fallback Undermines Safety

**Location:** Lines 300-305

```rust
fn replay_step_budget_len() -> usize {
    match usize::try_from(crate::limits::MAX_STEP_BUDGET) {
        Ok(value) => value,
        Err(_) => usize::MAX,  // ❌ Unbounded fallback defeats purpose
    }
}
```

**Problem:** The budget is meant to prevent unbounded iteration from back-edges. Falling back to `usize::MAX` on conversion failure defeats this protection entirely. If `MAX_STEP_BUDGET` cannot be represented as `usize`, the constant itself is misconfigured.

**Fix:** Assert at compile time or const-eval that `MAX_STEP_BUDGET` fits in `usize`:
```rust
const _: () = assert!(MAX_STEP_BUDGET as usize <= usize::MAX);
fn replay_step_budget_len() -> usize {
    MAX_STEP_BUDGET as usize
}
```

---

### 5.2 Submodule File Sizes Compound Drift

| File | Lines | Status |
|------|-------|--------|
| `mod.rs` | 348 | ❌ >300 |
| `step.rs` | 604 | ❌ >300 |
| `ops.rs` | ~1500+ | ❌ >300 (with inline tests) |
| `choose/` | (dir) | (not analyzed) |

The submodule `step.rs` at 604 lines is itself a severe violation. The `ops.rs` contains ~1400+ lines of inline tests that should be in `ops/tests.rs`.

---

## 6. RECOMMENDATIONS

### Critical (Must Fix)

1. **Split `mod.rs`** - Extract `ReplayExprStack` to `stack.rs`, `ReplayError` variants to a shared errors module, and `ReplayEngine` to stay in `mod.rs` but trimmed.

2. **Fix `StepIdx::ZERO` abuse** - Add `Option<StepIdx>` to `ExpressionEvalFailed` or create `StackOverflow` variant.

3. **Move `eval_expr_for_replay`** into `ReplayEngine` as a private method.

4. **Remove `usize::MAX` fallback** in `replay_step_budget_len`.

### Important (Should Fix)

5. **Wrap `u8` in `StackLen`/`StackCapacity` newtypes** in `ReplayExprStack`.

6. **Move inline tests** in `ops.rs` to `ops/tests.rs`.

7. **Implement `From<EngineError> for ReplayError`** instead of free function.

### Nice to Have

8. **Use iterator pattern** in `eval_expr_for_replay` loop instead of raw index.

9. **Add `RunId::for_replay()`** to clarify placeholder semantics.

---

## 7. VERDICT

```
DRIFT SCORE: 7/10 (CRITICAL)

The replay module demonstrates solid domain modeling in the ReplayEngine 
and ReplayError types, but suffers from:
- File size overflow (348 > 300)
- Primitive obsession (StepIdx::ZERO as sentinel, raw u8/usize)
- Cohesion leakage (orphan functions, free error conversion)
- Safety erosion (usize::MAX fallback)
```

**Required Action:** Decompose `mod.rs` and `step.rs` before any new feature work. Create beads for each extraction target.

---

*Report generated by architectural-drift agent*  
*Workspace: /home/lewis/src/velvet-ballistics/arch-drift-hammer*
