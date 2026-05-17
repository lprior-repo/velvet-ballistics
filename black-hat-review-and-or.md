# BLACK-HAT REVIEW: LETHAL-2 AND/OR Short-Circuit Implementation

**Review Scope:**
- `crates/vb_expr/src/eval.rs:161-162`
- `crates/vb_expr/src/eval/evaluate.rs:146-147`
- `crates/vb_expr/src/eval/builtin_eval.rs:44-45`
- `crates/vb_expr/src/eval/tests/and_or_short_circuit_tests.rs`
- `velvet-ballistics-MASTER.md Section 46 / "Short-Circuit Policy" (line 2528-2530)`

**Verdict: REJECTED — CONTRACT PARITY FAILURE**

---

## PHASE 1: Contract & Bead Parity — FAIL

### The Contract (Non-Negotiable)

From `velvet-ballistics-MASTER.md:2528-2530`:

> **`and` and `or` do NOT short-circuit.**
> Both operands are popped from the expression stack and **evaluated before** the boolean operator applies.
> **A type error in the second operand fires even when the first operand determines the result.**

This is the LETHAL-2 contract. It is unambiguous.

### The Implementation (Lines Under Review)

**eval.rs:161-162:**
```rust
BinaryOp::And => Ok(SlotValue::Bool(expect_bool(left)? && expect_bool(right)?)),
BinaryOp::Or => Ok(SlotValue::Bool(expect_bool(left)? || expect_bool(right)?)),
```

**evaluate.rs:146-147:** IDENTICAL
**builtin_eval.rs:44-45:** IDENTICAL

### CONTRACT VIOLATION: Rust `&&`/`||` Short-Circuit

The Rust `&&` and `||` operators **short-circuit**:
- For `&&`: if left is `false`, right is never evaluated
- For `||`: if left is `true`, right is never evaluated

The `?` operator propagates errors, which also short-circuits.

**Example:** `eval_binary_op(And, Bool(false), I64(0))`
1. `expect_bool(Bool(false))?` → `Ok(false)` — evaluation proceeds
2. `false && ...` short-circuits → `expect_bool(I64(0))?` is **NEVER called**
3. Result: `Ok(Bool(false))`

**Contract requirement:** Both operands MUST be evaluated. `I64(0)` MUST be evaluated, producing `TypeMismatch`. The contract explicitly states: *"A type error in the second operand fires even when the first operand determines the result."*

**The implementation does NOT evaluate the second operand when the first determines the result.**

### The Tests Are Testing the WRONG Thing

**Test file header (lines 2-7):**
```rust
//! Tests for AND/OR short-circuit behavior (LETHAL-2)
//! These tests prove that AND and OR evaluate BOTH operands before combining,
//! even when the first operand produces an error.
```

The header claims these tests prove NO short-circuit. They do not.

**B2 (lines 63-82):**
```rust
/// B2: AND returns false when first is false (optimization allowed — right NOT evaluated)
#[test]
fn and_returns_false_when_first_is_false_and_does_not_evaluate_right() -> ExprResult<()> {
    let left = SlotValue::Bool(false);
    let right = SlotValue::I64(0);  // INVALID BOOL
    let result = eval_binary_op(BinaryOp::And, left, right)?;
    assert_eq!(result, SlotValue::Bool(false));  // EXPECTS NO ERROR
}
```

This test explicitly verifies that when `left=false`, `right` is **NOT evaluated** and no error surfaces. This is the OPPOSITE of what the contract requires.

**B5 (lines 120-140):** Same defect for OR.

**P3 (lines 756-780):**
```rust
/// Invariant P3: AND with false left is always false regardless of right validity.
/// When left = SlotValue::Bool(false) and right is ANY SlotValue:
///   eval_binary_op(And, Bool(false), right) == Ok(Bool(false))
/// This is the short-circuit optimization case.
```

Explicitly labels short-circuit as an "optimization case." The contract calls it a VIOLATION.

**P4 (lines 782-806):** Same defect for OR.

**Integration tests (lines 573-594):**
```rust
#[test]
fn integration_and_false_any() -> ExprResult<()> {
    // "false and 1" should return false WITHOUT evaluating 1
    ...
}
```

Comment explicitly states "should return false WITHOUT evaluating 1." Contract requires evaluating `1` and producing `TypeMismatch`.

**Error accumulation tests (lines 172-282):** These tests DO NOT verify both operands were evaluated. They only verify that SOME error occurred. They cannot detect short-circuit because if left errors, right is never evaluated.

---

## PHASE 2: Farley Engineering Rigor — FAIL

### Function Complexity
`eval_binary_op` at 15 lines (lines 159-174) is UNDER the 25-line threshold. PASS on complexity.

### Separation of Concerns
The use of `&&`/`||` mixes short-circuit logic with error propagation. A proper implementation would:
1. Evaluate left
2. Evaluate right
3. Combine results

The current form blends step 1 and 2 via short-circuit, making error accumulation impossible.

---

## PHASE 3: Holzman Rust — PARTIAL FAIL

### Make Illegal States Unrepresentable
`BinaryOp` enum correctly models operators. PASS.

### Parse, Don't Validate
`expect_bool` validates at the boundary. PASS.

### Types as Documentation
Boolean parameters are not present here. PASS.

### Workflows as Explicit State Transitions
The contract mandates a specific evaluation order: both operands BEFORE operator. The implementation violates this by short-circuiting. FAIL.

---

## PHASE 4: Ruthless Simplicity — PASS

The implementation is simple and readable. The defect is semantic, not structural. No `unwrap`, `expect`, `panic`, `todo`, or `unimplemented` present.

---

## PHASE 5: Bitter Truth — FAIL

**The "Sniff Test":** The author wrote tests that document short-circuit as a feature and an "optimization." They knew the contract. They chose to implement and test the opposite.

**YAGNI Violation:** Short-circuit optimization was implemented without a contract requirement. The contract explicitly forbids it.

---

## MANDATED FIXES

### 1. Fix the Implementation

Replace short-circuit operators with explicit evaluation:

```rust
BinaryOp::And => {
    let left_bool = expect_bool(left)?;
    let right_bool = expect_bool(right)?;
    Ok(SlotValue::Bool(left_bool && right_bool))
}
BinaryOp::Or => {
    let left_bool = expect_bool(left)?;
    let right_bool = expect_bool(right)?;
    Ok(SlotValue::Bool(left_bool || right_bool))
}
```

This ensures BOTH operands are evaluated before the boolean operator applies.

### 2. Delete or Rewrite These Tests

| Test | Issue | Action |
|------|-------|--------|
| `and_returns_false_when_first_is_false_and_does_not_evaluate_right` | Documents contract violation | DELETE |
| `or_returns_true_when_first_is_true_and_does_not_evaluate_right` | Documents contract violation | DELETE |
| `proptest_and_false_left_always_false` | Tests short-circuit "optimization" | REWRITE to verify TypeMismatch |
| `proptest_or_true_left_always_true` | Tests short-circuit "optimization" | REWRITE to verify TypeMismatch |
| `integration_and_false_any` | Expects short-circuit result | REWRITE to expect TypeMismatch |
| `integration_or_true_any` | Expects short-circuit result | REWRITE to expect TypeMismatch |

### 3. Add Correct Error Accumulation Tests

```rust
#[test]
fn and_false_with_invalid_right_produces_type_mismatch() -> ExprResult<()> {
    // Contract: both operands evaluated even when left determines result
    let left = SlotValue::Bool(false);
    let right = SlotValue::I64(0);
    let result = eval_binary_op(BinaryOp::And, left, right);
    assert!(matches!(result, Err(ExprError::TypeMismatch { .. })));
    Ok(())
}
```

---

## SUMMARY

| Phase | Verdict |
|-------|---------|
| Contract & Bead Parity | **FAIL** — Short-circuit violates LETHAL-2 |
| Farley Engineering Rigor | PASS |
| Holzman Rust | PARTIAL FAIL — Workflow semantics violated |
| Ruthless Simplicity | PASS |
| Bitter Truth | FAIL — Tests document wrong behavior |

**REJECTED.** Rewrite implementation to evaluate both operands. Rewrite tests to verify error accumulation per contract.
