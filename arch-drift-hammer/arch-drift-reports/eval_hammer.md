# Architectural Drift Report: `eval.rs` (1016 lines)

**File**: `crates/vb_expr/src/eval.rs`
**Status**: MASSIVE ARCHITECTURAL DRIFT - 1016 lines (limit: 300)
**Enforcement**: ZERO TOLERANCE

---

## 1. RESPONSIBILITY MAPPING

| Responsibility | Lines | Violation |
|----------------|-------|-----------|
| Stack management (`push_value`, `pop_value`, `pop_pair`, `pop_triple`) | 935-963 | Low (cohesive) |
| Opcode dispatch (`eval_expr_op_with_store`) | 79-104 | Low - but dispatch table is large |
| Binary arithmetic ops (`eval_add/sub/mul/div_op`) | 184-245 | **PRIMITIVE OBSESSION** |
| Comparison ops (`eval_gt/gte/lt/lte_op`) | 247-285 | **PRIMITIVE OBSESSION** |
| Type narrowing (`expect_bool`, `expect_i64`, `expect_symbol`, `expect_list`, `expect_object`) | 965-1013 | **PRIMITIVE OBSESSION** |
| Helper dispatch (two full dispatch chains) | 338-479 | **DUPLICATION** |
| Helper implementations (store-aware) | 650-933 | Mixed - 283 lines of helper logic |
| Helper error-only stubs | 481-607 | **WASTE** - 126 lines of fallthrough errors |
| Arithmetic helpers (`eval_i64_values_`, `eval_div_values_`, `eval_i64_cmp_values_`) | 287-310 | **PRIMITIVE OBSESSION** |

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### 2.1 Raw `i64` Arithmetic (CRITICAL)

Every arithmetic function takes raw primitives:

```rust
// Lines 287-294 - RAW i64, no type wrapper
fn eval_i64_values_(
    left: i64,
    right: i64,
    op: fn(i64, i64) -> Option<i64>,
) -> ExprResult<SlotValue>
```

**Should be**: A `NumericValue` or `Integer` value object with `checked_add`, `checked_sub`, etc. as methods.

### 2.2 Inline Type Unwrapping (CRITICAL)

```rust
// Lines 184-197 - inline unwrapping in EVERY arithmetic op
fn eval_add_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    match (left, right) {
        (SlotValue::F64(l), SlotValue::F64(r)) => {
            let result = l.get() + r.get();  // raw f64 arithmetic
            let finite = vb_core::value::FiniteF64::new(result)?;
            Ok(SlotValue::F64(finite))
        }
        (SlotValue::I64(l), SlotValue::I64(r)) => eval_i64_values_(l, r, i64::checked_add),
        ...
    }
}
```

**Should be**: `left.add(right)` on a `NumericValue` type.

### 2.3 Comparison Functions Using Raw Primitives

```rust
// Lines 304-310 - fn(&i64, &i64) -> bool
fn eval_i64_cmp_values_(
    left: i64,
    right: i64,
    op: fn(&i64, &i64) -> bool,
) -> ExprResult<SlotValue>
```

### 2.4 `expect_*` Functions Are Primitive Extractors

```rust
// Lines 965-1013 - Five nearly identical type extractors
fn expect_bool(value: SlotValue) -> ExprResult<bool>
fn expect_i64(value: SlotValue) -> ExprResult<i64>
fn expect_symbol(value: SlotValue) -> ExprResult<vb_core::ids::SymbolId>
fn expect_list(value: SlotValue) -> ExprResult<vb_core::ids::ListId>
fn expect_object(value: SlotValue) -> ExprResult<vb_core::ids::ObjectId>
```

**Should be**: `SlotValue::as_bool()`, `SlotValue::as_i64()` etc. methods on the enum.

### 2.5 `usize` Index Without Type Safety

```rust
// Lines 48-56 - raw usize index
let mut index = 0usize;
while index < program.ops.len() {
    let op = *program.ops.as_ref().get(index)...;
    index = next_index(index)?;
}
```

---

## 3. SCOTT WLASCHIN DDD VIOLATIONS

### 3.1 Types Not Squashed Together (Primitive Obsession)

The `SlotValue` enum is discriminated but its contents are raw primitives:
- `SlotValue::F64(vb_core::value::FiniteF64)` - wrapped, but arithmetic is inline
- `SlotValue::I64(i64)` - completely raw
- `SlotValue::Bool(bool)` - completely raw
- `SlotValue::Symbol(vb_core::ids::SymbolId)` - wrapped, but operations are inline

### 3.2 No Value Objects for Domain Concepts

| Domain Concept | Current Type | Should Be |
|---------------|--------------|-----------|
| Length | `i64` | `Length(i64)` with `Positive` refinement |
| Count | `i64` | `Count(i64)` with `NonNegative` refinement |
| Index | `usize` | `Index(usize)` bounded to stack size |
| Numeric result | `SlotValue::I64/F64` | `ArithmeticResult` enum |

### 3.3 "Train Wreck" Pattern

```rust
// Line 324 - chained method call on raw value
let result = -f.get();  // get() on FiniteF64, then negate
let finite = vb_core::value::FiniteF64::new(result)?;
```

**Should be**: `f.negate()` on a `NumericValue`.

### 3.4 Conditional type checking scattered in helper error stubs

Lines 493-605 contain 112 lines of repetitive error stubs that should be replaced with a proper type system.

---

## 4. STRUCTURAL DRIFT

### 4.1 File Size Catastrophe

- **Actual**: 1016 lines
- **Limit**: 300 lines
- **Overflow**: 716 lines (239% over limit)

### 4.2 Dispatch Table Duplication

Both `eval_expr_op_with_store` (lines 79-104) and `eval_helper_with_store` (lines 420-479) contain full match arms for the same operations. This is duplicated dispatch logic.

### 4.3 Two-Stage Helper System

The file contains BOTH `eval_helper` (store-agnostic, errors for complex types) AND `eval_helper_with_store` (store-aware). This is architectural complexity that should be refactored.

---

## 5. PRESCRIPTIVE REMEDIATION

### 5.1 Extract Value Objects (Priority: CRITICAL)

```rust
// NEW: crates/vb_expr/src/value_object.rs
pub struct NumericValue(SlotValue);

impl NumericValue {
    pub fn add(self, other: NumericValue) -> ExprResult<NumericValue>;
    pub fn sub(self, other: NumericValue) -> ExprResult<NumericValue>;
    pub fn mul(self, other: NumericValue) -> ExprResult<NumericValue>;
    pub fn div(self, other: NumericValue) -> ExprResult<NumericValue>;
    pub fn negate(self) -> ExprResult<NumericValue>;
}

pub struct BooleanValue(SlotValue);
pub struct TextValue(SlotValue);
pub struct ListValue(SlotValue);
```

### 5.2 Shrink File to <300 Lines

**Chunk 1** (0-150): Public API entry points
- `eval_expr_program`, `eval_expr_program_with_store`
- `eval_helper_with_store`
- `finish_stack`, `next_index`

**Chunk 2** (150-300): Core stack ops
- `push_value`, `pop_value`, `pop_pair`, `pop_triple`
- `eval_expr_op_with_store`

**Chunk 3** (NEW FILE - arithmetic.rs):
- `eval_binary_op`, `eval_unary_op`
- All arithmetic operations (`eval_add_op`, `eval_sub_op`, etc.)
- `eval_i64_values_`, `eval_div_values_`, `eval_i64_cmp_values_`

**Chunk 4** (NEW FILE - type_narrowing.rs):
- `expect_bool`, `expect_i64`, `expect_symbol`, `expect_list`, `expect_object`

**Chunk 5** (NEW FILE - helpers.rs):
- All `eval_helper_*_with_store` functions

### 5.3 Remove Error-Only `eval_helper` Function

The 126-line stub function (lines 481-607) should be deleted. Only `eval_helper_with_store` should remain.

---

## 6. VERDICT

**ARCHITECTURAL DRIFT: CRITICAL**

This file violates:
1. `<300 line rule` by 716 lines (239% overflow)
2. Scott Wlaschin DDD primitive obsession rules throughout
3. Single Responsibility Principle via multiple unrelated responsibility clusters

**IMMEDIATE ACTION REQUIRED**: Break into 4+ smaller modules before any further feature development.
