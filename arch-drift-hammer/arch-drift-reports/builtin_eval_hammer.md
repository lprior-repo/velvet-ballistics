# Architectural Drift Report: `builtin_eval.rs`

**File**: `crates/vb_expr/src/builtin_eval.rs`
**Total Lines**: 349
**Line Limit**: 300
**Violation**: YES — 49 lines over limit

---

## 1. LINE COUNT VIOLATION

| Metric | Value |
|--------|-------|
| Actual Lines | 349 |
| Limit | 300 |
| Overage | 49 (16.3%) |

**Required Action**: File MUST be split. The 250-line `#[cfg(test)]` module is the primary offender — it is 3.5x larger than the implementation code (~70 lines).

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### 2.1 Raw Function Pointer Types

```rust
// Line 74: raw `fn(i64, i64) -> Option<i64>` function pointer
fn eval_i64_values(
    left: SlotValue,
    right: SlotValue,
    op: fn(i64, i64) -> Option<i64>,
) -> ExprResult<SlotValue>

// Line 92: raw `fn(&i64, &i64) -> bool` function pointer
fn eval_i64_cmp_values(
    left: SlotValue,
    right: SlotValue,
    op: fn(&i64, &i64) -> bool,
) -> ExprResult<SlotValue>
```

**Violation**: These raw function pointers encode no semantics. They should be newtyped into specific operation markers:
- `AddOp`, `SubOp`, `MulOp` → wrap the checked arithmetic
- `GtOp`, `LtOp`, etc. → wrap the comparison

### 2.2 Direct `i64` Arithmetic in Helper Functions

Functions `eval_i64_values`, `eval_div_values`, and `eval_i64_cmp_values` all operate directly on raw `i64` without domain wrapping.

**Violation**: `Parse, don't validate` is not applied. Every call site must validate type before calling these helpers. There should be a `NumericValue` or `IntegerValue` newtype that performs validation on construction.

### 2.3 Direct `SlotValue` Pattern Matching

```rust
// Lines 44-56: exhaustive match on raw enum variants
match op {
    BinaryOp::And => Ok(SlotValue::Bool(expect_bool(left)? && expect_bool(right)?)),
    BinaryOp::Or => Ok(SlotValue::Bool(expect_bool(left)? || expect_bool(right)?)),
    // ...
}
```

**Violation**: `SlotValue` is a primitive enum. Operations like `expect_bool`, `expect_i64` are validation-as-parse scattered across call sites. Should use a domain type with a single `try_from` implementation.

---

## 3. DDD PRINCIPLE VIOLATIONS

### 3.1 No Value Objects for Numeric Operations

The arithmetic operations (`Add`, `Sub`, `Mul`, `Div`) are implemented as direct `i64::checked_*` calls. Scott Wlaschin DDD requires:
- **Value Objects**: Immutable, self-validating types
- **Operations as functions**: No ad-hoc arithmetic in expression evaluation

Current state: Arithmetic is sprinkled across helper functions without a coherent numeric value object.

### 3.2 Stack-Based Primitive Manipulation

The module exposes and uses:
- `pop_pair`, `pop_value`, `push_value` — raw stack manipulation
- `expect_bool`, `expect_i64` — type validation at every call site

**Violation**: These are procedural stack operations, not domain actions. DDD expects workflows to be explicit state transitions on well-typed domain objects.

### 3.3 Missing Domain: `BinaryOperator` and `UnaryOperator` Types

The lexer types `BinaryOp` and `UnaryOp` leak into the evaluation layer. A proper DDD architecture would have:
- `ArithmeticOp { Add, Sub, Mul, Div }` for numeric binary ops
- `LogicalOp { And, Or }` for boolean binary ops
- `ComparisonOp { Gt, Gte, Lt, Lte, Eq, NotEq }` for comparison ops
- `UnaryArithmeticOp { Neg }`, `UnaryLogicalOp { Not }`

---

## 4. SECURITY BUG: BH-BE-001

### Finding: `eval_div_values` Misdiagnoses `i64::MIN / -1`

```rust
// Lines 80-87
fn eval_div_values(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    let left_i64 = expect_i64(left)?;
    let right_i64 = expect_i64(right)?;
    let value = left_i64
        .checked_div(right_i64)
        .ok_or(ExprError::DivisionByZero)?;  // BUG: reports DivisionByZero for overflow case
    Ok(SlotValue::I64(value))
}
```

**Root Cause**: `checked_div` returns `None` for TWO distinct cases:
1. Division by zero (`x / 0`) → correct error: `DivisionByZero`
2. Overflow case (`i64::MIN / -1`) → wrong error: `DivisionByZero` (should be `IntegerOverflow`)

**Impact**: Callers that distinguish between `DivisionByZero` and `IntegerOverflow` will misroute control flow. This is a **misdiagnosis vulnerability** (HIGH severity).

**Note**: The test `blackhat_be_001_div_values_misreports_min_div_neg_one` (lines 119-130) **confirms this bug is currently present** in `builtin_eval::eval_div_values`. The parallel function in `eval.rs` handles this correctly by checking zero explicitly.

---

## 5. TEST SPIRAL

The `blackhat_tests` module (lines 99-349) is 250 lines covering:
- BH-BE-001 to BH-BE-018: 18 security/overflow tests

**Problem**: 250 lines of tests in the same file as 70 lines of implementation creates an imbalance. Tests should be:
- In a separate `builtin_eval_tests.rs` or `tests/builtin_eval_blackhat.rs`
- Referenced by integration test crates, not inline

**This is a structural cohesion violation**: Tests and implementation must be separately compiled, separately reviewable artifacts.

---

## 6. REFACTORING PRESCRIPTION

### Step 1: Split the File

```
builtin_eval.rs (70 lines implementation)
  └── keeps: pub fn eval_eq, eval_binary_stack, eval_unary_stack, eval_binary_op, eval_unary_op
  └── keeps: fn eval_i64_values, eval_div_values, eval_i64_cmp_values (temporarily)

builtin_eval_tests.rs (250 lines)
  └── moves: mod blackhat_tests
  └── NOTE: These tests are BLACKHAT adversarial tests, not standard unit tests
```

### Step 2: Fix BH-BE-001

```rust
fn eval_div_values(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    let left_i64 = expect_i64(left)?;
    let right_i64 = expect_i64(right)?;
    if right_i64 == 0 {
        return Err(ExprError::DivisionByZero);
    }
    // i64::MIN / -1 is the only overflow case for checked_div
    if left_i64 == i64::MIN && right_i64 == -1 {
        return Err(ExprError::IntegerOverflow);
    }
    Ok(SlotValue::I64(left_i64 / right_i64))
}
```

### Step 3: Introduce NumericValue Newtype

```rust
pub struct NumericValue(i64);

impl NumericValue {
    pub fn add(self, other: NumericValue) -> Result<NumericValue, ExprError> {
        self.0
            .checked_add(other.0)
            .map(NumericValue)
            .ok_or(ExprError::IntegerOverflow)
    }
    // similar for sub, mul, div
}
```

---

## 7. SUMMARY

| Category | Status | Count |
|----------|--------|-------|
| Line Count | **VIOLATION** | 349 > 300 |
| Primitive Obsession | **VIOLATION** | 3 major patterns |
| DDD Cohesion | **VIOLATION** | Stack-based primitives, no value objects |
| Security Bug | **CONFIRMED** | BH-BE-001 misdiagnosis |
| Test Organization | **VIOLATION** | 250-line test module inline |

**OVERALL**: `builtin_eval.rs` requires immediate refactoring:
1. Split out `blackhat_tests` to separate file
2. Fix `eval_div_values` overflow misdiagnosis
3. Introduce `NumericValue` newtype wrapper
4. Create domain-specific operation types

---

*Report generated by architectural-drift agent*
*Workspace: arch-drift-hammer (JJ)*
