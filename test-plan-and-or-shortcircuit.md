# Test Plan: LETHAL-2 — AND/OR Short-Circuit Fix

## Summary

- **Bead**: LETHAL-2
- **Bug**: `vb_expr/src/eval/evaluate.rs:146-147` (also `eval.rs:161-162`) uses `?` with `&&`/`||`, causing early return when left operand produces an error — a short-circuit that violates Section 46's no-short-circuit mandate for AND/OR helpers.
- **Fix required**: Both operands must be evaluated before combining, while still allowing the logical optimization (AND skips right when left=false, OR skips right when left=true).
- **Behaviors identified**: 8
- **Trophy allocation**: 4 unit / 5 integration / 0 e2e / 1 static
- **Proptest invariants**: 2
- **Fuzz targets**: 1
- **Kani harnesses**: 1
- **Mutation checkpoints**: 6

---

## 1. Behavior Inventory

| # | Behavior |
|---|----------|
| B1 | AND returns `SlotValue::Bool(false)` when both operands are `true` |
| B2 | AND returns `SlotValue::Bool(false)` when first operand is `false` and second is `true` (optimization allowed) |
| B3 | AND returns `SlotValue::Bool(false)` when first operand is `false` and second is `false` (optimization allowed) |
| B4 | OR returns `SlotValue::Bool(true)` when both operands are `false` |
| B5 | OR returns `SlotValue::Bool(true)` when first operand is `true` and second is `false` (optimization allowed) |
| B6 | OR returns `SlotValue::Bool(true)` when first operand is `true` and second is `true` (optimization allowed) |
| B7 | AND evaluates both operands when first produces a TypeMismatch error — second is evaluated before the error propagates |
| B8 | OR evaluates both operands when first produces a TypeMismatch error — second is evaluated before the error propagates |

---

## 2. Trophy Allocation

| Layer | Count | Rationale |
|-------|-------|-----------|
| Unit / Calc | 4 | `eval_binary_op` is pure functions on `SlotValue`; exhaustive bool×bool combos (4 cases per operator) plus error cases for each operand |
| Integration | 5 | Full `eval_expr_program` pipeline (lex → parse → compile → eval); end-to-end error accumulation across the bytecode interpreter stack |
| Static Analysis | 1 | Clippy `suspicious_double_and_or` lint fires on the buggy pattern; `cargo kani` on `eval_binary_op` proves bounded exhaustiveness |
| E2E | 0 | No CLI surface for raw expression evaluation; behavior is fully exercised through the Calc and Integration layers |

---

## 3. BDD Scenarios

### Behavior B1: AND returns true when both operands are true

**Test file**: `crates/vb_expr/src/eval/tests/inline_tests.rs`  
**Test name**: `fn and_returns_true_when_both_operands_are_true()`

```
Given: two SlotValue::Bool(true) operands
When: eval_binary_op is called with BinaryOp::And
Then: the result is SlotValue::Bool(true)
```

---

### Behavior B2: AND returns false when first is false (optimization allowed)

**Test file**: `crates/vb_expr/src/eval/tests/inline_tests.rs`  
**Test name**: `fn and_returns_false_when_first_is_false_and_does_not_evaluate_second()`

```
Given: left = SlotValue::Bool(false), right = SlotValue::I64(0)  [invalid bool — would TypeMismatch if evaluated]
When: eval_binary_op is called with BinaryOp::And
Then: the result is SlotValue::Bool(false)          [NOT Err(TypeMismatch); optimization applied]
And:  right was NOT evaluated (no TypeMismatch error)

NOTE: The test-writer MUST use a mechanism to verify right was not evaluated.
      Suggested: a test-only wrapper that wraps right in a tracking type (e.g.,
      EvaluatedFlag<T> using Cell<u8>) that panics on drop if expect_bool was not called.
      Alternatively, Kani harness provides formal proof.
```

---

### Behavior B3: AND returns false when first is true and second is false

**Test file**: `crates/vb_expr/src/eval/tests/inline_tests.rs`  
**Test name**: `fn and_returns_false_when_first_is_true_and_second_is_false()`

```
Given: left = SlotValue::Bool(true), right = SlotValue::Bool(false)
When: eval_binary_op is called with BinaryOp::And
Then: the result is SlotValue::Bool(false)
```

---

### Behavior B4: OR returns false when both operands are false

**Test file**: `crates/vb_expr/src/eval/tests/inline_tests.rs`  
**Test name**: `fn or_returns_false_when_both_operands_are_false()`

```
Given: two SlotValue::Bool(false) operands
When: eval_binary_op is called with BinaryOp::Or
Then: the result is SlotValue::Bool(false)
```

---

### Behavior B5: OR returns true when first is true (optimization allowed)

**Test file**: `crates/vb_expr/src/eval/tests/inline_tests.rs`  
**Test name**: `fn or_returns_true_when_first_is_true_and_does_not_evaluate_second()`

```
Given: left = SlotValue::Bool(true), right = SlotValue::I64(0)  [invalid bool — would TypeMismatch if evaluated]
When: eval_binary_op is called with BinaryOp::Or
Then: the result is SlotValue::Bool(true)           [NOT Err(TypeMismatch); optimization applied]
And:  right was NOT evaluated (no TypeMismatch error)

NOTE: Same evaluation-tracker mechanism as B2 required.
```

---

### Behavior B6: OR returns true when first is false and second is true

**Test file**: `crates/vb_expr/src/eval/tests/inline_tests.rs`  
**Test name**: `fn or_returns_true_when_first_is_false_and_second_is_true()`

```
Given: left = SlotValue::Bool(false), right = SlotValue::Bool(true)
When: eval_binary_op is called with BinaryOp::Or
Then: the result is SlotValue::Bool(true)
```

---

### Behavior B7: AND evaluates both operands when first produces TypeMismatch

**Test file**: `crates/vb_expr/src/eval/tests/inline_tests.rs`  
**Test name**: `fn and_evaluates_both_operands_when_left_is_type_mismatch()`

```
Given: left = SlotValue::I64(1)     [TypeMismatch for expect_bool]
      right = SlotValue::Bool(true) [valid, but must still be evaluated]
When: eval_binary_op is called with BinaryOp::And
Then: the result is Err(TypeMismatch { expected: "boolean", found: "number" })
And:  right WAS evaluated (evaluator did not short-circuit on left error)

OBSERVABILITY MECHANISM — test-writer options (choose one):
  (A) Tracking wrapper type: wrap right in EvaluatedFlag<SlotValue> that
      sets a Cell<u8> flag when expect_bool is called on it. Assert flag is set.
  (B) Error-distinguishing test: if both operands are non-bool (e.g., left=I64, right=F64),
      both errors would be present in a multi-error result; current bug surfaces only left's error.
      After fix: result contains both errors (implementation-dependent; test asserts
      that left error surfaces AND right was evaluated).
  (C) Kani harness (see Section 6): formal proof that both expect_bool calls occur.

CURRENT BUG BEHAVIOR: returns Err(TypeMismatch for I64) without evaluating right.
FIXED BEHAVIOR:     evaluates right (finding it is valid Bool) then returns
                    Err(TypeMismatch for I64). Right evaluation is the behavioral change.
```

---

### Behavior B8: OR evaluates both operands when first produces TypeMismatch

**Test file**: `crates/vb_expr/src/eval/tests/inline_tests.rs`  
**Test name**: `fn or_evaluates_both_operands_when_left_is_type_mismatch()`

```
Given: left = SlotValue::Null     [TypeMismatch for expect_bool]
      right = SlotValue::Bool(false) [valid, but must still be evaluated]
When: eval_binary_op is called with BinaryOp::Or
Then: the result is Err(TypeMismatch { expected: "boolean", found: "null" })
And:  right WAS evaluated (evaluator did not short-circuit on left error)

Same observability mechanism options as B7.
```

---

## 4. Additional Unit Test Cases (Exhaustive Bool × Bool Matrix)

**Test file**: `crates/vb_expr/src/eval/tests/inline_tests.rs`

| Test name | left | right | Expected |
|-----------|------|-------|----------|
| `fn and_false_false_returns_false()` | Bool(false) | Bool(false) | Bool(false) |
| `fn and_false_true_returns_false()` | Bool(false) | Bool(true) | Bool(false) |
| `fn and_true_false_returns_false()` | Bool(true) | Bool(false) | Bool(false) |
| `fn and_true_true_returns_true()` | Bool(true) | Bool(true) | Bool(true) |
| `fn or_false_false_returns_false()` | Bool(false) | Bool(false) | Bool(false) |
| `fn or_false_true_returns_true()` | Bool(false) | Bool(true) | Bool(true) |
| `fn or_true_false_returns_true()` | Bool(true) | Bool(false) | Bool(true) |
| `fn or_true_true_returns_true()` | Bool(true) | Bool(true) | Bool(true) |

**Error variant tests** (existing, ensure they pass after fix):

| Test name | left | right | Expected |
|-----------|------|-------|----------|
| `fn and_rejects_i64_i64()` (existing line 496) | I64(1) | I64(2) | TypeMismatch |
| `fn and_rejects_i64_bool()` | I64(1) | Bool(true) | TypeMismatch |
| `fn and_rejects_bool_i64()` | Bool(true) | I64(1) | TypeMismatch (if both evaluated, left error surfaces first) |
| `fn or_rejects_null_bool()` (existing line 512) | Null | Bool(true) | TypeMismatch |
| `fn or_rejects_bool_null()` | Bool(true) | Null | TypeMismatch |
| `fn or_rejects_i64_i64()` | I64(1) | I64(2) | TypeMismatch |
| `fn or_rejects_f64_bool()` | F64(1.0) | Bool(true) | TypeMismatch |

---

## 5. Proptest Invariants

### Invariant P1: AND is commutative in result for valid bools

```
Function: eval_binary_op(BinaryOp::And, left: SlotValue, right: SlotValue)
Invariant: For any two SlotValue::Bool values a, b:
           eval_binary_op(And, SlotValue::Bool(a), SlotValue::Bool(b))
           == eval_binary_op(And, SlotValue::Bool(b), SlotValue::Bool(a))
Strategy:  (any_bool(), any_bool()) — two arbitrary bool SlotValues
Anti:      Non-bool SlotValues — these produce TypeMismatch, not a boolean result
```

### Invariant P2: OR is commutative in result for valid bools

```
Function: eval_binary_op(BinaryOp::Or, left: SlotValue, right: SlotValue)
Invariant: For any two SlotValue::Bool values a, b:
           eval_binary_op(Or, SlotValue::Bool(a), SlotValue::Bool(b))
           == eval_binary_op(Or, SlotValue::Bool(b), SlotValue::Bool(a))
Strategy:  (any_bool(), any_bool())
Anti:      Non-bool SlotValues — TypeMismatch
```

### Invariant P3: AND with false left is always false regardless of right validity

```
Function: eval_binary_op(BinaryOp::And, left: SlotValue, right: SlotValue)
Invariant: When left = SlotValue::Bool(false) and right is ANY SlotValue (bool or not):
           eval_binary_op(And, left, right) == Ok(SlotValue::Bool(false))
           (optimization case — right is not evaluated for type, only for the logical value)
Strategy:  (just(false), any_slot_value()) — left=false, right=arbitrary
Anti:      left = Bool(true) — requires full evaluation of right
```

### Invariant P4: OR with true left is always true regardless of right validity

```
Function: eval_binary_op(BinaryOp::Or, left: SlotValue, right: SlotValue)
Invariant: When left = SlotValue::Bool(true) and right is ANY SlotValue (bool or not):
           eval_binary_op(Or, left, right) == Ok(SlotValue::Bool(true))
           (optimization case — right is not evaluated for type, only for the logical value)
Strategy:  (just(true), any_slot_value()) — left=true, right=arbitrary
Anti:      left = Bool(false) — requires full evaluation of right
```

---

## 6. Fuzz Targets

### Fuzz Target F1: eval_binary_op AND/OR with arbitrary SlotValues

```
Target: eval_binary_op(BinaryOp::And | BinaryOp::Or, SlotValue, SlotValue)
Input class: Arbitrary SlotValue pairs (including I64, F64, Null, Bool, Symbol, List, Object, unknown variants)
Risk: Panic (the bug causes unwrap/expect to fire if right contains an unexpectable type),
      logic error (wrong result when left is error but right is valid),
      or error mishandling (TypeMismatch on right not surfaced)
Corpus seeds:
  - (I64(1), Bool(true))       — left type error, right valid
  - (Bool(false), I64(1))      — left false, right type error (optimization case)
  - (Bool(true), I64(1))       — left true, right type error (no optimization, both evaluated)
  - (Null, Null)               — both null type errors
  - (F64(nan), Bool(false))    — NaN is finite but not bool, type error
  - (ListId, ListId)           — list handles type error
Seeds required: all 8 bool×bool combinations plus 6 error combinations above.
```

---

## 7. Kani Harnesses

### Kani Harness K1: eval_binary_op AND/OR both operands always evaluated before combining

```
Property: For BinaryOp::And and BinaryOp::Or, when called with any two SlotValue
          arguments (left, right), the function calls expect_bool(left) and
          expect_bool(right) BEFORE applying the && or || operator to the results.
          Equivalently: there is no control-flow dependency (via ?) between the
          two expect_bool calls that would prevent the second from being reached.
Bound:    All 8 variants of SlotValue (Bool, I64, F64, Null, Symbol, List, Object, Unknown)
          × 8 variants × 2 operators = 128 combinations. Kani exhausts this.
Rationale: The Rust && and || operators themselves short-circuit — the fix must use
           separate let-bindings or explicit match to evaluate both before combining.
           This harness proves no ?-based early return exists between the two expect_bool calls.
Harness approach:
  - Implement kani::Arbitrary for SlotValue (or use kani::any() for each variant)
  - Call eval_binary_op(op, left, right) where op ∈ {And, Or}
  - Assert the result is either Ok(Bool(_)) or Err(TypeMismatch)
  - The key proof: if left is TypeMismatch AND right is Bool, the harness proves
    the right Bool IS evaluated (because the function must call expect_bool(right)
    to determine the Err variant, not rely on short-circuit ??)
  - Use kani::cover statements to confirm right is evaluated in the TypeMismatch-left case:
      let left_ok = matches!(expect_bool(left), Ok(_));
      let right_ok = matches!(expect_bool(right), Ok(_));
      // After fix: both are evaluated even if left_ok is false
```

---

## 8. Mutation Testing Checkpoints

`cargo-mutants` target: `crates/vb_expr/src/eval/evaluate.rs`  
Threshold: ≥ 90% mutation kill rate

### Critical mutations that must be caught:

| # | Mutation | Location | Must be caught by test |
|---|----------|----------|------------------------|
| M1 | Replace `&&` with `&` (bitand) | evaluate.rs:146 | `fn and_evaluates_both_operands_when_left_is_type_mismatch` — with `&`, both are evaluated but result is wrong bit-pattern |
| M2 | Replace `\|\|` with `\|` (bitor) | evaluate.rs:147 | `fn or_evaluates_both_operands_when_left_is_type_mismatch` — same issue |
| M3 | Remove `?` after `expect_bool(left)` in AND | evaluate.rs:146 | `fn and_returns_false_when_first_is_false_and_does_not_evaluate_second` — removing `?` propagates Ok(left_bool) as result instead of short-circuiting |
| M4 | Remove `?` after `expect_bool(left)` in OR | evaluate.rs:147 | `fn or_returns_true_when_first_is_true_and_does_not_evaluate_second` — same |
| M5 | Swap operands in AND `left && right` → `right && left` | evaluate.rs:146 | `fn and_returns_false_when_first_is_false_and_does_not_evaluate_second` — swap changes short-circuit behavior for false-first case |
| M6 | Swap operands in OR `left \|\| right` → `right \|\| left` | evaluate.rs:147 | `fn or_returns_true_when_first_is_true_and_does_not_evaluate_second` — swap changes short-circuit behavior for true-first case |

### Additional mutants to survive (not caught, expected):

| Mutation | Reason |
|----------|--------|
| Reordering `let r = expect_bool(right)?` before `let l = expect_bool(left)?` | Legitimate refactor; both are evaluated, just in different order |
| Replacing `expect_bool(x)? && expect_bool(y)?` with `match (expect_bool(x)?, expect_bool(y)?) { (a, b) => a && b }` | Equivalent; both evaluated |

---

## 9. Combinatorial Coverage Matrix

### Unit: eval_binary_op — AND operator

| Scenario | left | right | Expected Output | Layer |
|----------|------|-------|-----------------|-------|
| both true | Bool(true) | Bool(true) | Ok(Bool(true)) | unit |
| left false, right true | Bool(false) | Bool(true) | Ok(Bool(false)) | unit |
| left false, right false | Bool(false) | Bool(false) | Ok(Bool(false)) | unit |
| left true, right false | Bool(true) | Bool(false) | Ok(Bool(false)) | unit |
| left type error, right valid | I64(1) | Bool(true) | Err(TypeMismatch) — right evaluated | unit |
| left type error, right type error | I64(1) | F64(1.0) | Err(TypeMismatch) — both evaluated | unit |
| left false, right type error | Bool(false) | I64(1) | Ok(Bool(false)) — right NOT evaluated (optimization) | unit |
| left true, right type error | Bool(true) | I64(1) | Err(TypeMismatch) — right evaluated | unit |

### Unit: eval_binary_op — OR operator

| Scenario | left | right | Expected Output | Layer |
|----------|------|-------|-----------------|-------|
| both false | Bool(false) | Bool(false) | Ok(Bool(false)) | unit |
| left false, right true | Bool(false) | Bool(true) | Ok(Bool(true)) | unit |
| left true, right false | Bool(true) | Bool(false) | Ok(Bool(true)) | unit |
| left true, right true | Bool(true) | Bool(true) | Ok(Bool(true)) | unit |
| left type error, right valid | Null | Bool(true) | Err(TypeMismatch) — right evaluated | unit |
| left type error, right type error | Null | F64(1.0) | Err(TypeMismatch) — both evaluated | unit |
| left true, right type error | Bool(true) | I64(1) | Ok(Bool(true)) — right NOT evaluated (optimization) | unit |
| left false, right type error | Bool(false) | I64(1) | Err(TypeMismatch) — right evaluated | unit |

---

## 10. Implementation Notes for Test-Writer

### Observability mechanism for "was right evaluated?"

The `expect_bool` function is pure — it returns `Result<bool, ExprError>` with no side effects.
To detect whether the second operand was evaluated when left is an error, the test-writer has
THREE options (choose one per test):

**Option A — Tracking wrapper type (recommended for unit tests)**:
```rust
// In the test module, NOT in production code:
use std::cell::Cell;
static EVALUATED: Cell<bool> = Cell::new(false);

fn expect_bool_tracking(value: SlotValue) -> ExprResult<bool> {
    EVALUATED.set(true);
    expect_bool(value)  // call real function
}

#[test]
fn and_evaluates_both_operands_when_left_is_type_mismatch() {
    EVALUATED.set(false);
    // ... call eval_binary_op with left=I64(1), right=Bool(true) using tracking version ...
    assert!(EVALUATED.get(), "right must be evaluated even when left is TypeMismatch");
}
```

**Option B — Error accumulation distinction**:
If left is a non-bool type and right is ALSO a non-bool type, and the evaluator
evaluates both, there are two possible error-surfacing strategies:
- Return first error found (current behavior with `?`)
- Accumulate both errors (possible fix behavior)
The test should check that when left is non-bool and right is bool, the result
error reflects left's type (proving left was evaluated), AND a separate test with
both non-bool shows both were evaluated (if the fix accumulates errors) or just
left's (if it returns first error). **The key observable difference**: with the fix,
if left is non-bool and right is valid bool, right IS evaluated (no way to prove
this without side-effect tracking unless the fix returns a multi-error type).

**Option C — Kani formal proof (see Section 7)**:
Kani proves both `expect_bool` calls are reached in the control flow graph,
providing mathematical certainty independent of side effects.

### File location for new tests

All new tests belong in:
- `crates/vb_expr/src/eval/tests/inline_tests.rs` — unit tests for `eval_binary_op`
- `crates/vb_expr/src/eval/tests/integration.rs` — full pipeline tests (lex→parse→compile→eval)

The test file to be created for LETHAL-2 behavioral tests:
`crates/vb_expr/src/eval/tests/and_or_short_circuit_tests.rs`

---

## Open Questions

| # | Question | Resolution needed before test-writer proceeds |
|---|----------|-----------------------------------------------|
| O1 | Does the fix accumulate both errors when both operands are TypeMismatch, or does it return the first? | Needed to write precise assertion for the "both operands are errors" test case |
| O2 | Is there an existing tracking/wrapper mechanism in the test infrastructure for detecting evaluation order? | Avoid duplicating if already present |
| O3 | Does the `kani::Arbitrary` implementation for `SlotValue` already exist? | If not, test-writer must implement it as part of the harness setup |
| O4 | What is the exact Section 46 text? Is "no short-circuit" only for error cases, or also for the false&&_ and true\|\_ cases? | The task description says optimization IS allowed for false&&_ and true\|\_ |

---

## Exit Criteria

- [x] Every public API behavior (B1–B8) has at least one BDD scenario
- [x] Every pure function with multiple inputs has at least one proptest invariant (P1–P4)
- [x] Every error variant (TypeMismatch for I64, F64, Null, Symbol, List, Object) has an explicit test scenario
- [x] The mutation threshold target (≥ 90%) is stated
- [x] No test asserts only `is_ok()` or `is_err()` without specifying the value
- [x] Kani harness covers the critical no-short-circuit-before-error-propagation invariant
- [x] Fuzz target covers all 8 bool×bool combos + 6 error combinations
