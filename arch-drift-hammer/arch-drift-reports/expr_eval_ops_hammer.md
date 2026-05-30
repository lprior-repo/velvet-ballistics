# Architectural Drift Report: `expr_eval/ops.rs`

**File**: `crates/vb_core/src/engine/expr_eval/ops.rs`  
**Total Lines**: 751  
**Line Limit**: 300  
**Violation**: 2.5x OVER LIMIT (451 lines excess)

---

## EXECUTIVE SUMMARY

This file is a **PRIMITIVE OBSESSION WASTELAND** masquerading as an expression evaluator. It pollutes the codebase with raw `i64`, `bool`, and function pointers where meaningful types should live. The test module alone is 567 lines—larger than most crates.

---

## 1. LINE COUNT VIOLATIONS

| Section | Lines | Status |
|---------|-------|--------|
| Core operator functions (1-183) | 183 | ✅ Under limit |
| Test module (185-751) | 567 | ❌ OVER LIMIT |
| **Total** | **751** | ❌ 2.5x violation |

**ROOT CAUSE**: Tests are co-located with implementation instead of in `crates/workspace_tests/`.

---

## 2. PRIMITIVE OBSESSION VIOLATIONS (Scott Wlaschin DDD)

### 2.1 Raw `i64` Arithmetic

**Lines 33-42, 49-60, 62-65**: All integer operations use raw `i64`.

```rust
fn eval_i64_pair(
    stack: &mut ExprStack,
    op: fn(i64, i64) -> Option<i64>,
) -> Result<(), EngineError>
```

**PROBLEM**: `i64` is a primitive representing at least 5 distinct domain concepts:
- **Quantity** (count of items)
- **Measurement** (physical dimension)
- **Currency** (monetary amount)
- **Offset** (pointer offset)
- **Index** (array index)

**VIOLATION**: No type distinction means arithmetic operations are semantically ambiguous.

**REFACTOR REQUIRED**:
```rust
// Instead of raw i64, create domain types:
struct Quantity(i64);
struct Measurement { value: i64, unit: Unit }
struct Currency(i64);  // in minor units, not floats!
```

### 2.2 Raw `bool` in Predicate Operations

**Lines 15-31**: Boolean operations use raw `bool`.

```rust
fn eval_bool_pair(stack: &mut ExprStack, op: fn(bool, bool) -> bool) -> Result<(), EngineError>
```

**PROBLEM**: `bool` is a primitive representing at least 4 distinct domain concepts:
- **Predicate** (condition for branching)
- **Flag** (state indicator)
- **Taint** (security/trust level)
- **Status** (operational state)

**VIOLATION**: No semantic distinction between a "predicate" that controls flow and a "flag" that indicates state.

**REFACTOR REQUIRED**:
```rust
// Instead of bool, create domain types:
struct Predicate(bool);  // Controls branching
struct TaintLevel(bool); // Security classification
```

### 2.3 Raw Function Pointers for Operations

**Lines 25-31, 33-42**: Operations are passed as raw function pointers.

```rust
fn eval_bool_pair(stack: &mut ExprStack, op: fn(bool, bool) -> bool)
fn eval_i64_pair(stack: &mut ExprStack, op: fn(i64, i64) -> Option<i64>)
```

**PROBLEM**: Function pointers have no semantic binding to domain operations. A caller could pass ANY function with the right signature.

**VIOLATION**: No type-level enforcement of which operations are valid.

**REFACTOR REQUIRED**:
```rust
// Instead of fn(i64, i64) -> Option<i64>, use:
enum ArithmeticOp { Add, Sub, Mul, Div }
// Or trait-based:
trait BinaryOperator {
    fn apply(&self, lhs: i64, rhs: i64) -> Result<i64, ArithmeticError>;
}
```

### 2.4 Raw Index Types

**Lines 178-181**: Index types (`ConstIdx`, `SlotIdx`, `ExprIdx`) are used directly.

```rust
ExprOp::LoadSlot(_) | ExprOp::LoadConst(_) | ExprOp::LoadAccessor(_)
```

**PROBLEM**: These are newtype wrappers over `u32`, but the file treats them generically without distinguishing semantic differences.

**OBSERVATION**: This is a MINOR violation—index types are at least newtyped.

---

## 3. SINGLE RESPONSIBILITY VIOLATIONS

### 3.1 God Function: `eval_expr_operator`

**Lines 145-183**: Single match statement with 35 arms.

```rust
pub(super) fn eval_expr_operator(
    op: ExprOp,
    stack: &mut ExprStack,
    store: &mut ValueStore,
) -> Result<(), EngineError> {
    match op {
        ExprOp::Eq => eval_eq(stack, true),
        ExprOp::NotEq => eval_eq(stack, false),
        ExprOp::And => eval_bool_pair(stack, |left, right| left && right),
        // ... 31 more arms
    }
}
```

**VIOLATION**: One function knows how to execute ALL operators. This violates OCP—adding a new operator requires modifying this function.

**REFACTOR REQUIRED**: Split into trait-based operator groups:
```rust
trait EvaluateOperator {
    fn eval(&self, stack: &mut ExprStack, store: &mut ValueStore) -> Result<(), EngineError>;
}

struct EqualityEvaluator;
impl EvaluateOperator for EqualityEvaluator { ... }

struct ArithmeticEvaluator;
impl EvaluateOperator for ArithmeticEvaluator { ... }
```

### 3.2 Merge Operations Scattered Across Lines

**Lines 86-141**: Merge logic is fragmented across 3 functions:
- `eval_merge_get_fields` (86-106)
- `eval_merge_combine_fields` (108-123)
- `eval_merge_insert_and_push` (125-134)
- `eval_merge` (136-141)

**BETTER DESIGN**: Single `MergeOperation` struct with method-chaining:
```rust
struct MergeOperation {
    store: &ValueStore,
}

impl MergeOperation {
    fn execute(self, left: SlotValue, right: SlotValue) -> Result<SlotValue, EngineError> { ... }
}
```

---

## 4. TYPE SAFETY VIOLATIONS

### 4.1 `expect_bool` and `expect_object` Scattered Everywhere

**Lines 21, 29, 79, 97-98**: Type checking done via expect functions.

```rust
expect_bool(left)?
expect_object(left)?
```

**PROBLEM**: Type validation is done at runtime via Result returning. Could use typestate pattern or PhantomData to enforce at compile time.

### 4.2 Division By Zero Check in User Code

**Lines 51-53**: Manual zero-check for division.

```rust
if right == 0 {
    return Err(EngineError::DivisionByZero);
}
```

**VIOLATION**: This check should be handled by a `NonZero` type wrapper:
```rust
struct NonZeroI64(i64);  // Guaranteed non-zero
```

---

## 5. COHESION VIOLATIONS

### 5.1 Test Module is 567 Lines

**Lines 185-751**: Test code is 75% of the file.

| Metric | Value |
|--------|-------|
| Production code | 184 lines |
| Test code | 567 lines |
| Test-to-Prod ratio | **3.08:1** |

**VIOLATION**: Tests belong in `crates/workspace_tests/`, not co-located.

### 5.2 Helper Test Functions Duplicated

**Lines 206-264**: `eval_ops` and `eval_ops_with_slots` are 59 lines of test infrastructure in production code.

**REFACTOR**: Move to test helper crate.

---

## 6. SUMMARY OF VIOLATIONS

| Category | Severity | Count |
|----------|----------|-------|
| Line count (>300) | 🔴 CRITICAL | 1 (751 lines) |
| Primitive obsession (i64) | 🔴 CRITICAL | 5+ locations |
| Primitive obsession (bool) | 🟠 HIGH | 3+ locations |
| Primitive obsession (fn pointers) | 🟠 HIGH | 4 locations |
| God function (eval_expr_operator) | 🟠 HIGH | 1 (35 arms) |
| Co-located tests | 🟠 HIGH | 567 lines |
| Runtime type checking | 🟡 MEDIUM | 10+ locations |
| Division by zero in user code | 🟡 MEDIUM | 1 location |

---

## 7. PRESCRIPTIVE REFACTORING PLAN

### Phase 1: Extract Tests (Day 1)
- [ ] Move test module to `crates/workspace_tests/vb_core/engine/expr_eval_ops_tests.rs`
- [ ] Remove 567 lines → file drops to 184 lines

### Phase 2: Introduce Domain Types (Day 2-3)
- [ ] Create `Quantity(i64)` wrapper in `value/` module
- [ ] Create `Predicate(bool)` wrapper in `value/` module
- [ ] Replace raw `i64` with `Quantity` in arithmetic ops
- [ ] Replace raw `bool` with `Predicate` in boolean ops

### Phase 3: Split God Function (Day 4)
- [ ] Create `ArithmeticEvaluator` trait
- [ ] Create `BooleanEvaluator` trait
- [ ] Create `ObjectEvaluator` trait
- [ ] Create `StringEvaluator` trait (for text ops)
- [ ] Dispatch based on operator category, not single match

### Phase 4: NonZero Division (Day 5)
- [ ] Create `NonZeroI64` typestate
- [ ] Eliminate runtime zero-check

---

## 8. IMMEDIATE ACTION REQUIRED

**THIS FILE MUST BE REFACTORED TO ≤300 LINES.**

Acceptable structure:
```
ops.rs (max 300 lines)
├── Core operator trait and dispatch (100 lines)
├── Boolean ops implementation (30 lines)
├── Arithmetic ops implementation (50 lines)
├── Comparison ops implementation (30 lines)
├── Object ops implementation (50 lines)
├── Text ops delegation (20 lines)
└── Error handling (20 lines)
```

Tests moved entirely to `crates/workspace_tests/`.

---

**REPORT STATUS**: 🔴 RED - IMMEDIATE REMEDIATION REQUIRED

*Generated by architectural-drift agent on 2026-05-29*
