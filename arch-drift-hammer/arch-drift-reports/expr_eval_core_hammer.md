# Architectural Drift Report: `expr_eval/core.rs`

**File:** `crates/vb_core/src/engine/expr_eval/core.rs`  
**Line Count:** 419 (exceeds 300-line maximum)  
**Status:** DRIFT DETECTED — REFACTOR REQUIRED

---

## 1. Line Count Violation

| Region | Lines | Problem |
|--------|-------|---------|
| Production code | 1–123 | Clean (~103 lines) |
| Inline test module | 124–419 | **~295 lines** — bloats file to 419 |

**Root Cause:** Tests are embedded in the same file as production code. The `#[cfg(test)] mod tests { ... }` block contains 18 test functions that exercise every code path in the production region. This violates the architectural rule that test code should be in sibling test modules or a dedicated `tests/` crate.

**Evidence:**
- Lines 124–419 are exclusively `mod tests` 
- The `expr_eval/mod.rs` already has `#[cfg(test)] mod tests;` pointing to `tests.rs` (1745 lines)
- The inline tests here duplicate what `tests.rs` already covers

---

## 2. Responsibility Map

| Function | Responsibility | Lines | Assessment |
|----------|----------------|-------|------------|
| `expression_op` | Safe indexed lookup of `ExprOp` | 14–20 | Clean |
| `next_expr_index` | Checked increment with overflow guard | 22–28 | Clean |
| `finish_expr_stack` | Validates single-value result | 30–38 | Clean |
| `eval_expr_inner` | Core evaluation loop (dispatcher) | 40–59 | **Mixed concern** — loop + taint accumulation |
| `eval_load_slot` | Reads slot + taint propagation | 61–71 | **Taint leak** |
| `eval_load_const` | Loads constant from plan | 73–84 | Clean |
| `eval_expr_op` | Top-level op dispatch | 86–102 | Clean |
| `eval_expr_with_store` | Public API with store | 106–113 | Thin wrapper |
| `eval_expr` | Public API without store | 115–122 | Thin wrapper |

---

## 3. Primitive Obsession Violations

### 3.1 `usize` Arithmetic Throughout

**Violation:** Loop index `index: usize` is used for `expression_op` and `next_expr_index`. This is raw primitive obsession — arithmetic on raw `usize` rather than a domain-typed instruction pointer.

```rust
// Lines 51–56 — raw usize arithmetic in hot loop
let mut index = 0usize;
while index < program.ops.len() {
    let op = expression_op(program.ops.as_ref(), index)?;
    eval_expr_op(plan, run, store, op, &mut stack, &mut taint_accum)?;
    index = next_expr_index(index)?;
}
```

**Scott Wlaschin Diagnosis:** No `newtype` for instruction pointer. The domain concept "position in expression program" is represented as a bare `usize` rather than `InstructionIndex` or `ExprProgramCounter`.

**Refactor:** Create `struct InstructionPointer(usize)` with `fn next(&self) -> Result<InstructionPointer, EngineError>` that encapsulates the overflow check.

### 3.2 `Taint` Accumulation in Evaluation Loop

**Violation:** `eval_expr_inner` mixes execution loop with taint lattice computation via `join_taint`. Taint propagation is a **cross-cutting concern** that should be a decorator/wrapper, not embedded in the core evaluator.

```rust
// Line 50 — taint state threaded through pure evaluation
let mut taint_accum = Taint::Clean;
// ... inside loop:
eval_expr_op(plan, run, store, op, &mut stack, &mut taint_accum)?;
```

**Scott Wlaschin Diagnosis:** The `eval_expr_op` function returns `Result<(), EngineError>` — a pure unit type on success. Taint accumulation requires `&mut Taint` to be passed in, which is a **隐式副作用** (implicit side effect). This violates the principle of making side effects explicit in the type signature.

**Refactor:** `eval_expr_op` should return `Result<Taint, EngineError>` or evaluation should be wrapped in a taint-tracking decorator.

### 3.3 `SlotValue` Primitive Overload

**Violation:** `SlotValue` is a large enum with 16+ variants (`I64`, `U64`, `F64`, `String`, `Bool`, `Timestamp`, etc.). Operations like `Add`, `Mul` etc. live in `ops.rs` and pattern-match on two `SlotValue`s to produce a third. This is not primitive obsession per se, but **feature envy** — the operations are defined externally to the value type.

---

## 4. DDD Structural Issues

### 4.1 Cross-Cutting Taint Concern

`Taint` is a security lattice (`Clean`, `Secret`, `DerivedFromSecret`) that propagates through every slot read and must flow out of every expression evaluation. The current design sprinkles taint logic into:
- `eval_load_slot` (line 69)
- `eval_expr_inner` (line 50, 54)
- `eval_expr_op` (line 92)

**Scott Wlaschin:** Taint propagation is a **decorator pattern** or **monad** that should wrap the pure expression evaluator. Currently it's an **aspect** cut across the evaluation logic.

### 4.2 Evaluation Stack vs. Expression Stack

`ExprStack` is a domain stack for expression evaluation. The stack is created in `eval_expr_inner` (line 49) with a size derived from the `ExprProgram`. The separation between:
- `ExprStack` (evaluation stack)
- `ValueStore` (heap-allocated value storage)

Is unclear from this file alone. The stack exists only for the duration of one expression evaluation.

### 4.3 No Value-Object Wrappers for Index Types

`ExprIdx`, `SlotIdx`, `ConstIdx`, `AccessorIdx` are already newtyped (good), but `usize` is still used for:
- `program.ops.len()` comparison (line 52)
- `program.ops.as_ref()` indexing (line 53)
- Stack depth (line 49, `program.max_stack`)

---

## 5. Recommendations

### Priority 1 — Move Inline Tests (Immediate Fix)

Extract `mod tests { ... }` (lines 124–419) to a new file `tests/core.rs` in the same directory and update `mod.rs` to `mod tests;` pointing to it. This reduces the file to **~123 lines**.

```rust
// In expr_eval/mod.rs, change:
// #[cfg(test)]
// mod tests;  ← already present, pointing to tests.rs
```

Actually, since `expr_eval/mod.rs` already has `#[cfg(test)] mod tests;` pointing to `tests.rs` (line 19), the inline tests at lines 124–419 are **duplicative**. They should be deleted entirely and the tests should live in `tests.rs`.

### Priority 2 — InstructionPointer Newtype

Replace raw `usize` in the evaluation loop with a domain-typed instruction pointer:

```rust
struct InstructionPointer(usize);

impl InstructionPointer {
    fn next(self) -> Result<Self, EngineError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "expression op index overflow",
            })
    }
    
    fn get(self, ops: &[ExprOp]) -> Result<ExprOp, EngineError> {
        ops.get(self.0)
            .copied()
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "expression op index checked by loop bound",
            })
    }
}
```

### Priority 3 — Explicit Taint in Return Types

Change `eval_expr_op` to return taint:

```rust
fn eval_expr_op(...) -> Result<Taint, EngineError> { ... }
```

Then `eval_expr_inner` becomes a fold over ops that accumulates taint explicitly via `join_taint`.

### Priority 4 — Taint Decorator (Future)

Extract taint-tracking into a wrapper:

```rust
fn eval_expr_with_taintTracking(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    store: &mut ValueStore,
    expr: ExprIdx,
) -> Result<(SlotValue, Taint), EngineError> {
    let (value, _) = eval_expr_inner(plan, run, store, expr)?; // pure
    let taint = compute_taint_from_dependencies(run, expr)?;
    Ok((value, taint))
}
```

---

## 6. Summary

| Issue | Severity | Fix |
|-------|----------|-----|
| 419 lines (>300) | **CRITICAL** | Delete inline `mod tests` (lines 124–419), tests belong in `tests.rs` |
| Raw `usize` for instruction pointer | HIGH | Newtype `InstructionPointer` |
| Implicit taint in `&mut Taint` | HIGH | Return `Taint` from `eval_expr_op` |
| Taint cross-cutting evaluation | MEDIUM | Decorator/wrapper pattern |

**Immediate Action:** Delete lines 124–419 (the entire `#[cfg(test)] mod tests` block). The file will drop to 123 lines and be compliant.

---

## 7. Files Referenced

- `crates/vb_core/src/engine/expr_eval/mod.rs` — module boundary
- `crates/vb_core/src/engine/expr_eval/tests.rs` — 1745-line test suite (should cover these cases)
- `crates/vb_core/src/engine/expr_eval/stack.rs` — `ExprStack`
- `crates/vb_core/src/engine/expr_eval/ops.rs` — `eval_expr_operator`
- `crates/vb_core/src/engine/expr_eval/accessors.rs` — `eval_load_accessor`
- `crates/vb_core/src/value.rs` — `SlotValue`, `Taint`
- `crates/vb_core/src/ids.rs` — `ExprIdx`, `SlotIdx`, `ConstIdx`
