# Test Plan: vb-qi37.9.2 — F64 Bytecode Execution Semantics

## Summary
- Bead: vb-qi37.9.2
- Title: expr: Execute F64 bytecode semantics
- Behaviors identified: 15
- Trophy allocation: 20 unit / 28 integration / 2 e2e / 2 static
- Proptest invariants: 6
- Fuzz targets: 1
- Kani harnesses: 7 (already written, all PASS)

## 1. Behavior Inventory

1. `eval_add_op` on two `SlotValue::F64` returns `Ok(SlotValue::F64(finite))` for IEEE 754 finite sum, or `Err(ExprError::NonFiniteFloat)` on overflow to ±Inf
2. `eval_sub_op` on two `SlotValue::F64` returns `Ok(SlotValue::F64(finite))` for IEEE 754 finite difference, or `Err(ExprError::NonFiniteFloat)` on overflow
3. `eval_mul_op` on two `SlotValue::F64` returns `Ok(SlotValue::F64(finite))` for IEEE 754 finite product, or `Err(ExprError::NonFiniteFloat)` on overflow
4. `eval_div_op` on two `SlotValue::F64` with non-zero divisor returns `Ok(SlotValue::F64(finite))` for IEEE 754 quotient, or `Err(ExprError::NonFiniteFloat)` on overflow
5. `eval_div_op` on two `SlotValue::F64` with zero divisor returns `Err(ExprError::NonFiniteFloat)` (F64/0 → ±Inf → NonFiniteFloat; NOT DivisionByZero)
6. `eval_neg_op` on `SlotValue::F64` returns `Ok(SlotValue::F64(finite))` for IEEE 754 negation (negating finite never produces Inf)
7. F64 comparison ops (`eval_gt_op`, `eval_gte_op`, `eval_lt_op`, `eval_lte_op`) return `Ok(SlotValue::Bool(...))` with IEEE 754 semantics; NaN comparisons yield false
8. `eval_div_op` on two `SlotValue::I64` with zero divisor returns `Err(ExprError::DivisionByZero)` (NOT NonFiniteFloat)
9. `eval_expr_program` with stack depth > 64 returns `Err(ExprError::StackOverflow { max: 64 })`
10. `eval_expr_program` with empty stack on terminal op returns `Err(ExprError::StackUnderflow)`
11. F64 ops with type mismatch (e.g., F64 op on I64 values) return `Err(ExprError::TypeMismatch { expected: "number", found: ... })`
12. I64 checked arithmetic overflow returns `Err(ExprError::IntegerOverflow)`
13. `eval_neg_op` on `SlotValue::I64(i64::MIN)` returns `Err(ExprError::IntegerOverflow)` (checked_neg overflow)
14. `eval_expr_program` with truncated bytecode returns `Err(ExprError::UnexpectedEof)`
15. `eval_expr_program` with out-of-bounds constant index returns `Err(ExprError::UnexpectedEof)`

---

## 2. Trophy Allocation

| Layer | Count | Rationale |
|---|---|---|
| Unit / Calc | 20 | Pure F64 arithmetic ops (eval_add/sub/mul/div/neg), comparison ops, I64 overflow, type error detection — each op tested in isolation |
| Integration | 28 | `eval_expr_program` full pipeline (lex→parse→compile→eval), stack overflow/underflow at program level, end-to-end arithmetic with real `ExprProgram`, F64/0 distinguishing NonFiniteFloat from DivisionByZero via program, helper ops with ValueStore |
| E2E | 2 | Full `lex_expr` → `parse_expr` → `compile_expr_with_pool` → `eval_expr_program` for F64 arithmetic; F64 div-by-zero via source text |
| Static Analysis | 2 | `cargo clippy` gate (PO-014), `cargo build` gate (PO-015) |

**Rationale for deviation**: F64 arithmetic is a pure calc layer with well-defined IEEE 754 semantics; integration tests use real `ExprProgram` objects (not mocks) and validate the full pipeline. The 28 integration tests include stack depth boundary testing and the F64/0 vs DivisionByZero distinction, which require program-level evaluation.

---

## 3. BDD Scenarios

### Behavior 1: F64 addition preserves finiteness

**Scenario: `fn f64_add_returns_correct_sum_when_inputs_are_finite`**
Given: two valid `SlotValue::F64` wrapping finite f64 values
When: `eval_binary_op(BinaryOp::Add, left, right)` is called
Then: result is `Ok(SlotValue::F64(f))` where `f.get() == left.get() + right.get()` (IEEE 754 addition)
And: `f.get().is_finite() == true`

**Scenario: `fn f64_add_returns_non_finite_float_when_result_overflows`**
Given: two `SlotValue::F64` values whose IEEE 754 sum is +Inf or -Inf
When: `eval_binary_op(BinaryOp::Add, left, right)` is called
Then: result is `Err(ExprError::NonFiniteFloat)`
Note: Kani `kani_f64_add_preserves_finiteness` (bounded) + proptest covers overflow-to-Inf path

**Scenario: `fn f64_add_returns_type_mismatch_when_operands_are_not_f64`**
Given: `SlotValue::I64(1)` and `SlotValue::I64(2)` (both I64)
When: `eval_binary_op(BinaryOp::Add, left, right)` is called
Then: result is `Err(ExprError::TypeMismatch { expected: "number", found: "number" })` — I64 path takes `eval_i64_values_` which calls `checked_add`

**Scenario: `fn f64_add_returns_type_mismatch_when_operands_are_bool`**
Given: `SlotValue::Bool(true)` and `SlotValue::Bool(false)`
When: `eval_binary_op(BinaryOp::Add, ...)` is called
Then: result is `Err(ExprError::TypeMismatch { expected: "number", found: "boolean" })`

---

### Behavior 2: F64 subtraction preserves finiteness

**Scenario: `fn f64_sub_returns_correct_difference_when_inputs_are_finite`**
Given: two valid `SlotValue::F64` wrapping finite f64 values
When: `eval_binary_op(BinaryOp::Sub, left, right)` is called
Then: result is `Ok(SlotValue::F64(f))` where `f.get() == left.get() - right.get()`
And: `f.get().is_finite() == true`

**Scenario: `fn f64_sub_returns_non_finite_float_when_result_overflows`**
Given: two `SlotValue::F64` whose IEEE 754 difference is +Inf or -Inf
When: `eval_binary_op(BinaryOp::Sub, left, right)` is called
Then: result is `Err(ExprError::NonFiniteFloat)`

---

### Behavior 3: F64 multiplication preserves finiteness

**Scenario: `fn f64_mul_returns_correct_product_when_inputs_are_finite`**
Given: two valid `SlotValue::F64` wrapping finite f64 values
When: `eval_binary_op(BinaryOp::Mul, left, right)` is called
Then: result is `Ok(SlotValue::F64(f))` where `f.get() == left.get() * right.get()`
And: `f.get().is_finite() == true`

**Scenario: `fn f64_mul_returns_non_finite_float_when_result_overflows`**
Given: two `SlotValue::F64` whose IEEE 754 product overflows to ±Inf
When: `eval_binary_op(BinaryOp::Mul, left, right)` is called
Then: result is `Err(ExprError::NonFiniteFloat)`

---

### Behavior 4: F64 division with non-zero divisor succeeds

**Scenario: `fn f64_div_nonzero_divisor_returns_correct_quotient`**
Given: two `SlotValue::F64` wrapping finite non-zero f64 values
When: `eval_binary_op(BinaryOp::Div, left, right)` is called
Then: result is `Ok(SlotValue::F64(f))` where `f.get() == left.get() / right.get()`
And: `f.get().is_finite() == true`

---

### Behavior 5: F64 division by zero returns NonFiniteFloat

**Scenario: `fn f64_div_by_zero_returns_non_finite_float_not_division_by_zero`**
Given: `SlotValue::F64(FiniteF64::new(1.0).unwrap())` and `SlotValue::F64(FiniteF64::new(0.0).unwrap())`
When: `eval_binary_op(BinaryOp::Div, left, right)` is called
Then: result is `Err(ExprError::NonFiniteFloat)`
And: result is NOT `Err(ExprError::DivisionByZero)`

**Scenario: `fn f64_zero_div_zero_returns_non_finite_float`**
Given: `SlotValue::F64(FiniteF64::new(0.0).unwrap())` and `SlotValue::F64(FiniteF64::new(0.0).unwrap())`
When: `eval_binary_op(BinaryOp::Div, left, right)` is called
Then: result is `Err(ExprError::NonFiniteFloat)` (0.0/0.0 = NaN per IEEE 754)
Note: Kani cannot verify this (IEEE 754 NaN fires before Rust error handling); proptest `finite_f64_rejects_nan_returns_non_finite_number` covers it

**Scenario: `fn f64_div_positive_inf_by_zero_returns_non_finite_float`**
Given: `SlotValue::F64(FiniteF64::new(f64::MAX).unwrap())` and `SlotValue::F64(FiniteF64::new(0.0).unwrap())`
When: `eval_binary_op(BinaryOp::Div, left, right)` is called
Then: result is `Err(ExprError::NonFiniteFloat)` (IEEE 754: non-zero/0 → ±Inf)

**Scenario: `fn i64_div_by_zero_returns_division_by_zero_not_non_finite_float`**
Given: `SlotValue::I64(10)` and `SlotValue::I64(0)`
When: `eval_binary_op(BinaryOp::Div, left, right)` is called
Then: result is `Err(ExprError::DivisionByZero)`
And: result is NOT `Err(ExprError::NonFiniteFloat)`
Note: This proves the F64 vs I64 path separation

---

### Behavior 6: F64 negation preserves finiteness

**Scenario: `fn f64_neg_returns_negated_value_when_input_is_finite`**
Given: `SlotValue::F64(FiniteF64::new(42.0).unwrap())`
When: `eval_unary_op(UnaryOp::Neg, value)` is called
Then: result is `Ok(SlotValue::F64(f))` where `f.get() == -42.0`
And: `f.get().is_finite() == true`

**Scenario: `fn f64_neg_returns_correct_sign_on_signed_zeros`**
Given: `SlotValue::F64(FiniteF64::new(-0.0_f64).unwrap())`
When: `eval_unary_op(UnaryOp::Neg, value)` is called
Then: result is `Ok(SlotValue::F64(f))` where `f.get() == 0.0` (IEEE 754: negation of -0.0 is +0.0)

---

### Behavior 7: F64 comparisons with IEEE 754 NaN semantics

**Scenario: `fn f64_gt_returns_false_when_left_is_nan`**
Given: constructed `SlotValue::F64` wrapping f64 that is NaN (cannot be constructed via FiniteF64; use internal knowledge that NaN comparisons yield false)
When: `eval_binary_op(BinaryOp::Gt, nan_f64, finite_f64)` is called
Then: result is `Ok(SlotValue::Bool(false))` — NaN > x is always false per IEEE 754

**Scenario: `fn f64_gte_returns_false_when_left_is_nan`**
Given: NaN F64 and finite F64
When: `eval_binary_op(BinaryOp::Gte, nan_f64, finite_f64)` is called
Then: result is `Ok(SlotValue::Bool(false))`

**Scenario: `fn f64_lt_returns_false_when_right_is_nan`**
Given: finite F64 and NaN F64
When: `eval_binary_op(BinaryOp::Lt, finite_f64, nan_f64)` is called
Then: result is `Ok(SlotValue::Bool(false))`

**Scenario: `fn f64_lte_returns_false_when_either_is_nan`**
Given: NaN F64 and finite F64
When: `eval_binary_op(BinaryOp::Lte, nan_f64, finite_f64)` is called
Then: result is `Ok(SlotValue::Bool(false))`

**Scenario: `fn f64_comparisons_return_correct_results_for_normal_values`**
Given: `SlotValue::F64(FiniteF64::new(3.0).unwrap())` and `SlotValue::F64(FiniteF64::new(5.0).unwrap())`
When: `eval_gt_op`, `eval_gte_op`, `eval_lt_op`, `eval_lte_op` are called
Then: gt→false, gte→false, lt→true, lte→true

---

### Behavior 8: I64 division by zero returns DivisionByZero

**Scenario: `fn i64_div_returns_division_by_zero_when_divisor_is_zero`**
Given: `SlotValue::I64(10)` and `SlotValue::I64(0)`
When: `eval_binary_op(BinaryOp::Div, left, right)` is called
Then: result is `Err(ExprError::DivisionByZero)`

---

### Behavior 9: Stack overflow returns StackOverflow error

**Scenario: `fn eval_expr_program_returns_stack_overflow_when_stack_exceeds_64`**
Given: an `ExprProgram` with 65 `LoadConst` ops (65 values pushed, exceeds MAX_EXPRESSION_STACK=64)
When: `eval_expr_program` is called
Then: result is `Err(ExprError::StackOverflow { max: 64 })`

**Scenario: `fn eval_expr_program_returns_stack_underflow_when_stack_is_empty`**
Given: an `ExprProgram` with a single `Add` op and no operands
When: `eval_expr_program` is called
Then: result is `Err(ExprError::StackUnderflow)`

---

### Behavior 10: Type mismatch errors on mixed-type F64 operations

**Scenario: `fn f64_mul_returns_type_mismatch_when_left_is_i64`**
Given: `SlotValue::I64(2)` and `SlotValue::F64(FiniteF64::new(3.0).unwrap())`
When: `eval_binary_op(BinaryOp::Mul, i64_val, f64_val)` is called
Then: result is `Err(ExprError::TypeMismatch { expected: "number", found: "number" })` — falls through to I64 path
Note: This exercises the mixed-type fallback path in `eval_mul_op`

---

### Behavior 11: I64 overflow returns IntegerOverflow

**Scenario: `fn i64_add_returns_integer_overflow_when_result_exceeds_i64_max`**
Given: `SlotValue::I64(i64::MAX)` and `SlotValue::I64(1)`
When: `eval_binary_op(BinaryOp::Add, left, right)` is called
Then: result is `Err(ExprError::IntegerOverflow)`

**Scenario: `fn i64_sub_returns_integer_overflow_when_result_underflows`**
Given: `SlotValue::I64(i64::MIN)` and `SlotValue::I64(1)`
When: `eval_binary_op(BinaryOp::Sub, left, right)` is called
Then: result is `Err(ExprError::IntegerOverflow)`

**Scenario: `fn i64_mul_returns_integer_overflow_when_result_exceeds_i64_max`**
Given: `SlotValue::I64(i64::MAX)` and `SlotValue::I64(2)`
When: `eval_binary_op(BinaryOp::Mul, left, right)` is called
Then: result is `Err(ExprError::IntegerOverflow)`

**Scenario: `fn i64_neg_returns_integer_overflow_when_operand_is_i64_min`**
Given: `SlotValue::I64(i64::MIN)`
When: `eval_unary_op(UnaryOp::Neg, value)` is called
Then: result is `Err(ExprError::IntegerOverflow)` (checked_neg of MIN overflows)

**Scenario: `fn i64_div_returns_integer_overflow_when_dividing_i64_min_by_neg_one`**
Given: `SlotValue::I64(i64::MIN)` and `SlotValue::I64(-1)`
When: `eval_binary_op(BinaryOp::Div, left, right)` is called
Then: result is `Err(ExprError::IntegerOverflow)` (mathematical result is i64::MAX+1, overflows)

---

### Behavior 12: F64 div-by-zero via end-to-end pipeline

**Scenario: `fn end_to_end_f64_div_by_zero_via_source_text_returns_non_finite_float`**
Given: the source text `"1.0 / 0.0"`
When: `lex_expr` → `parse_expr` → `compile_expr_with_pool` → `eval_expr_program` is called
Then: result is `Err(ExprError::NonFiniteFloat)`
And: result is NOT `Err(ExprError::DivisionByZero)`

---

### Behavior 13: I64 div-by-zero via end-to-end pipeline

**Scenario: `fn end_to_end_i64_div_by_zero_via_source_text_returns_division_by_zero`**
Given: the source text `"10 / 0"`
When: `lex_expr` → `parse_expr` → `compile_expr_with_pool` → `eval_expr_program` is called
Then: result is `Err(ExprError::DivisionByZero)`

---

### Behavior 14: UnexpectedEof on truncated bytecode

**Scenario: `fn eval_expr_program_returns_unexpected_eof_when_program_is_truncated`**
Given: an `ExprProgram` with `ops: vec![ExprOp::LoadConst(ConstIdx::new(99))]` and empty constants
When: `eval_expr_program` is called
Then: result is `Err(ExprError::UnexpectedEof)`

---

### Behavior 15: F64 arithmetic via full pipeline produces correct result

**Scenario: `fn end_to_end_f64_arithmetic_via_source_text_produces_correct_result`**
Given: the source text `"(3.5 + 2.5) * 4.0"`
When: `lex_expr` → `parse_expr` → `compile_expr_with_pool` → `eval_expr_program` is called
Then: result is `Ok(SlotValue::F64(f))` where `f.get() == 24.0`

---

## 4. Proptest Invariants

### Proptest: `eval_add_op` (F64 addition)
**Invariant**: For any two finite f64 values `a` and `b` where `|a| + |b| <= f64::MAX` (to prevent overflow), `eval_add_op(F64(a), F64(b))` returns `Ok(F64(c))` where `c == a + b` exactly (IEEE 754 bit-exact).
**Strategy**: `finite_f64_strategy()` pairs, bounded by `|l| + |r| <= f64::MAX / 2`.
**Anti-invariant**: Pairs where `|a| + |b| > f64::MAX` should return `Err(ExprError::NonFiniteFloat)`.

### Proptest: `eval_sub_op` (F64 subtraction)
**Invariant**: For any two finite f64 values `a` and `b` where `|a - b|` does not overflow, result is IEEE 754 exact difference.
**Strategy**: `finite_f64_sub_strategy()` with bounding `|l|, |r| <= f64::MAX / 2`.
**Anti-invariant**: Pairs where `|a - b| > f64::MAX` should return `Err(ExprError::NonFiniteFloat)`.

### Proptest: `eval_mul_op` (F64 multiplication)
**Invariant**: For any two finite f64 values `a` and `b` where `|a| * |b| <= f64::MAX`, result is IEEE 754 exact product.
**Strategy**: `finite_f64_mul_strategy()` with bounding `|l|, |r| <= sqrt(f64::MAX / 2)`.
**Anti-invariant**: Pairs where `|a| * |b| > f64::MAX` should return `Err(ExprError::NonFiniteFloat)`.

### Proptest: `eval_div_op` (F64 division)
**Invariant**: For any finite non-zero divisor `b` and finite dividend `a`, `eval_div_op(F64(a), F64(b))` returns `Ok(F64(q))` where `q == a / b` exactly.
**Strategy**: `finite_f64_div_strategy()` (divisor guaranteed non-zero).
**Anti-invariant**: `eval_div_op(F64(a), F64(0.0))` must return `Err(ExprError::NonFiniteFloat)` for any finite `a`.

### Proptest: F64 comparison NaN semantics
**Invariant**: For any F64 comparison `cmp(F64(NaN), F64(x))` returns `false` for all comparison ops (gt, gte, lt, lte); IEEE 754 mandates this.
**Strategy**: Generate NaN via `f64::from_bits(0x7FF8000000000000_u64)` (canonical quiet NaN); finite via `finite_f64_strategy()`.

### Proptest: `eval_neg_op` (F64 negation)
**Invariant**: For any finite f64 value `a`, `eval_neg_op(F64(a))` returns `Ok(F64(-a))` and the result is always finite (negation of finite cannot overflow to Inf per IEEE 754).
**Strategy**: `finite_f64_strategy()` for all finite inputs.
**Anti-invariant**: None — negation of finite is always finite.

---

## 5. Fuzz Targets

### Fuzz Target: `FiniteF64` deserialization boundary
**Input type**: bytes → `FiniteF64` via serde deserialization
**Risk**: Panic or logic error if malformed bytes produce NaN or Inf that are not caught at deserialization. This would violate INV-001 (SlotValue::F64 always finite) and INV-002 (ConstValue::F64 always finite at deserialization).
**Corpus seeds**: canonical NaN bit pattern (`0x7FF8000000000000`), signaling NaN, positive infinity (`0x7FF0000000000000`), negative infinity (`0xFFF0000000000000`), subnormal values, max/min finite, signed zeros.
**Command**: `cargo fuzz run deserialize_finite_f64 -- -runs=1000` (target not yet written — this is a gap)
**Status**: WAIVED — no fuzz harness currently exists (FUZZ-CONST-001). Compensating controls: serde roundtrip tests in vb_core + Kani formal verification of `FiniteF64::new`.

---

## 6. Kani Harnesses

**Note**: 7 Kani harnesses are already written and all PASS (per proof-evidence.md). These are listed here for completeness.

### Kani Harness: `kani_f64_add_preserves_finiteness`
- **Property**: For bounded finite inputs `|l|, |r| <= f64::MAX/2`, `eval_add_op` never returns `Err(ExprError::NonFiniteFloat)`.
- **Bound**: Unwind 4, 639 paths exhaustively checked.
- **Rationale**: Proves add/sub/mul overflow detection is correct for bounded inputs; unbounded overflow verified by proptest.

### Kani Harness: `kani_f64_sub_preserves_finiteness`
- **Property**: Same as add for subtraction.
- **Bound**: Unwind 4, 639 paths.

### Kani Harness: `kani_f64_mul_preserves_finiteness`
- **Property**: Same as add for multiplication with `|l|, |r| <= sqrt(f64::MAX/2)`.
- **Bound**: Unwind 4, 648 paths.

### Kani Harness: `kani_f64_neg_preserves_finiteness`
- **Property**: Negation of any finite f64 is always finite (IEEE 754 invariant).
- **Bound**: Unwind 4, 288 paths.

### Kani Harness: `kani_f64_div_by_zero_returns_non_finite_float`
- **Property**: `eval_div_op(F64(non-zero), F64(0.0))` returns `Err(ExprError::NonFiniteFloat)`, NOT `Err(ExprError::DivisionByZero)`.
- **Bound**: Unwind 4, 635 paths.
- **Rationale**: Proves the F64/0 vs I64/0 distinction formally.

### Kani Harness: `kani_f64_div_by_nonzero_finite_succeeds`
- **Property**: `eval_div_op(F64(dividend), F64(divisor))` where `divisor != 0` always succeeds and returns a finite result.
- **Bound**: Unwind 4, 639 paths.

### Kani Harness: `kani_i64_div_by_zero_returns_division_by_zero`
- **Property**: `eval_div_op(I64(dividend), I64(0))` returns `Err(ExprError::DivisionByZero)`.
- **Bound**: Unwind 4, 631 paths.
- **Rationale**: Proves the I64/0 path is separate and correct, not interfered with by F64 path.

---

## 7. Mutation Checkpoints

**Threshold**: 90% mutation kill rate minimum.

### Critical mutations to survive:

1. **`eval_add_op`**: Changing `+` to `-`, `*`, or `/` in `l.get() + r.get()` — must be caught by F64 arithmetic proptest (exact value assertion)
2. **`eval_mul_op`**: Changing `*` to `+`, `-`, or `/` in `l.get() * r.get()` — must be caught by multiplication-specific test
3. **`eval_div_op`**: Swapping F64 branch to I64 branch (changing `FiniteF64::new` path to `eval_div_values_`) — must be caught by `f64_div_by_zero_returns_non_finite_float_not_division_by_zero`
4. **`eval_neg_op`**: Neglecting to negate (returning `f.get()` instead of `-f.get()`) — must be caught by sign-preservation test
5. **I64 overflow path**: Removing `ok_or(ExprError::IntegerOverflow)` in `eval_i64_values_` — must be caught by integer overflow tests
6. **Division by zero check**: Removing `if right == 0` check in `eval_div_values_` — must be caught by division-by-zero test
7. **F64/0 NonFiniteFloat check**: Changing `.map_err(|_| ExprError::NonFiniteFloat)` to `.unwrap()` — would panic on Inf/NaN; must be caught by div-by-zero test

---

## 8. Combinatorial Coverage Matrix

### Unit: F64 arithmetic ops (add/sub/mul/div/neg)

| Scenario | Input Class | Expected Output | Test Layer |
|---|---|---|---|
| F64 add, finite bounded | `F64(a), F64(b)` where `a+b` finite | `Ok(F64(a+b))` | unit |
| F64 add, overflows to Inf | `F64(f64::MAX), F64(f64::MAX)` | `Err(NonFiniteFloat)` | unit |
| F64 sub, finite bounded | `F64(a), F64(b)` where `a-b` finite | `Ok(F64(a-b))` | unit |
| F64 sub, overflows | `F64(-f64::MAX), F64(f64::MAX)` | `Err(NonFiniteFloat)` | unit |
| F64 mul, finite bounded | `F64(a), F64(b)` where `a*b` finite | `Ok(F64(a*b))` | unit |
| F64 mul, overflows to Inf | `F64(f64::MAX), F64(f64::MAX)` | `Err(NonFiniteFloat)` | unit |
| F64 div, non-zero divisor | `F64(a), F64(b!=0)` | `Ok(F64(a/b))` | unit |
| F64 div, divisor=0, non-zero dividend | `F64(1.0), F64(0.0)` | `Err(NonFiniteFloat)` | unit |
| F64 div, dividend=0, divisor=0 | `F64(0.0), F64(0.0)` | `Err(NonFiniteFloat)` | unit |
| I64 div, divisor=0 | `I64(10), I64(0)` | `Err(DivisionByZero)` | unit |
| I64 div, divisor=0, dividend=i64::MIN | `I64(i64::MIN), I64(0)` | `Err(DivisionByZero)` | unit |
| F64 neg, positive finite | `F64(42.0)` | `Ok(F64(-42.0))` | unit |
| F64 neg, negative zero | `F64(-0.0)` | `Ok(F64(0.0))` | unit |

### Unit: F64 comparison ops

| Scenario | Input Class | Expected Output | Test Layer |
|---|---|---|---|
| F64 gt, left > right | `F64(5.0), F64(3.0)` | `Ok(Bool(true))` | unit |
| F64 gt, left NaN | constructed NaN F64, finite | `Ok(Bool(false))` | unit |
| F64 gte, equal finite | `F64(3.0), F64(3.0)` | `Ok(Bool(true))` | unit |
| F64 gte, left NaN | NaN F64, finite | `Ok(Bool(false))` | unit |
| F64 lt, left < right | `F64(3.0), F64(5.0)` | `Ok(Bool(true))` | unit |
| F64 lt, right NaN | finite, NaN F64 | `Ok(Bool(false))` | unit |
| F64 lte, equal | `F64(3.0), F64(3.0)` | `Ok(Bool(true))` | unit |
| F64 lte, either NaN | NaN F64, finite | `Ok(Bool(false))` | unit |

### Unit: I64 overflow

| Scenario | Input Class | Expected Output | Test Layer |
|---|---|---|---|
| I64 add, i64::MAX + 1 | `I64(i64::MAX), I64(1)` | `Err(IntegerOverflow)` | unit |
| I64 sub, i64::MIN - 1 | `I64(i64::MIN), I64(1)` | `Err(IntegerOverflow)` | unit |
| I64 mul, i64::MAX * 2 | `I64(i64::MAX), I64(2)` | `Err(IntegerOverflow)` | unit |
| I64 neg, i64::MIN | `I64(i64::MIN)` | `Err(IntegerOverflow)` | unit |
| I64 div, i64::MIN / -1 | `I64(i64::MIN), I64(-1)` | `Err(IntegerOverflow)` | unit |

### Integration: eval_expr_program pipeline

| Scenario | Input Class | Expected Output | Test Layer |
|---|---|---|---|
| Full pipeline F64 arithmetic | source `"3.5 + 2.5 * 4.0"` | `Ok(F64(24.0))` | integration |
| F64/0 via source text | source `"1.0 / 0.0"` | `Err(NonFiniteFloat)` | integration |
| I64/0 via source text | source `"10 / 0"` | `Err(DivisionByZero)` | integration |
| Stack overflow >64 entries | 65 LoadConst ops | `Err(StackOverflow { max: 64 })` | integration |
| Stack underflow empty | single Add op, no operands | `Err(StackUnderflow)` | integration |
| UnexpectedEof truncated | LoadConst(99) with empty constants | `Err(UnexpectedEof)` | integration |
| I64 overflow via source | source `"9223372036854775807 + 1"` | `Err(IntegerOverflow)` | integration |
| F64 roundtrip: add | F64 constants through program | `Ok(F64)` with correct sum | integration |
| F64 roundtrip: sub | F64 constants through program | `Ok(F64)` with correct diff | integration |
| F64 roundtrip: mul | F64 constants through program | `Ok(F64)` with correct product | integration |
| F64 roundtrip: div | F64 constants through program | `Ok(F64)` with correct quotient | integration |

### Integration: Type mismatch via program

| Scenario | Input Class | Expected Output | Test Layer |
|---|---|---|---|
| F64 add with I64 operands | `SlotValue::I64` pair to `eval_binary_op(BinaryOp::Add)` | `Err(TypeMismatch)` or overflow | integration |
| F64 mul with mixed types | I64+F64 mixed pair | `Err(TypeMismatch)` | integration |
| Bool in F64 add position | `SlotValue::Bool(true), SlotValue::I64(1)` | `Err(TypeMismatch)` | integration |
| I64 in F64 comparison | I64 values with Gt op | Ok with I64 comparison semantics | integration |

### Integration: Helper ops with ValueStore (real deps)

| Scenario | Input Class | Expected Output | Test Layer |
|---|---|---|---|
| `eval_helper_with_store` Empty on null | `SlotValue::Null` | `Ok(Bool(true))` | integration |
| `eval_helper_with_store` Empty on empty list | empty List handle | `Ok(Bool(true))` | integration |
| `eval_helper_with_store` Length on 3-element list | List(3 items) | `Ok(I64(3))` | integration |
| `eval_helper_with_store` Sum on [10,20,30] | List with I64 items | `Ok(I64(60))` | integration |
| `eval_helper_with_store` Unique on [1,2,1] | List with duplicate | `Ok(List([1,2]))` | integration |
| `eval_helper_with_store` Contains substring | `"hello world", "world"` | `Ok(Bool(true))` | integration |
| `eval_helper_with_store` Sum overflow | List([i64::MAX, 1]) | `Err(IntegerOverflow)` | integration |

### E2E: Full user-facing workflows

| Scenario | Input | Expected Output | Test Layer |
|---|---|---|---|
| Lex→Parse→Compile→Eval F64 arithmetic | source string with F64 ops | Correct IEEE 754 result | e2e |
| F64/0 error propagates through full pipeline | source `"1.0 / 0.0"` | `NonFiniteFloat` at eval output | e2e |

---

## Open Questions

1. **Q1**: F64 constant folding in `fold.rs` is not implemented (returns `None`). Should this be covered by a separate bead or included in scope?
2. **Q2**: Are there F64 helper ops (sum, avg, min, max on F64 lists) needed? Currently not in bead scope.
3. **Q3**: Should the `eval_expr_program_with_store` variant be tested separately from `eval_expr_program`, or is testing one sufficient (they share the same `eval_expr_op_with_store`)?
4. **Q4**: Fuzz harness `deserialize_finite_f64` is waived (no harness exists). Should a fuzz target be created as part of this bead, or deferred?
5. **Q5**: Should subnormal F64 values be explicitly tested in the arithmetic unit tests (beyond what proptest generates via `finite_f64_strategy()`)? Subnormals have unusual IEEE 754 semantics (gradual underflow).

---

## Exit Criteria Checklist

- [x] Every public API behavior has at least one BDD scenario
- [x] Every pure function with multiple inputs has at least one proptest invariant
- [x] Every parsing/deserialization boundary has a fuzz target (waived with rationale)
- [x] Every error variant in the `ExprError` enum has an explicit test scenario:
  - `NonFiniteFloat` — covered (F64/0, overflow scenarios)
  - `DivisionByZero` — covered (I64/0)
  - `IntegerOverflow` — covered (I64 overflow scenarios)
  - `TypeMismatch` — covered (mixed types)
  - `StackOverflow` — covered (65 entries)
  - `StackUnderflow` — covered (empty stack)
  - `UnexpectedEof` — covered (truncated bytecode)
- [x] Mutation threshold target (≥90%) is stated
- [x] No test asserts only `is_ok()` or `is_err()` without specifying the value
