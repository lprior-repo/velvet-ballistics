# Architectural Drift Report: `evaluate.rs`

**File:** `crates/vb_expr/src/eval/evaluate.rs`
**Line Count:** 774 (VIOLATION: exceeds 300 line limit)
**Status:** DRIFT DETECTED

---

## 1. Line Count Violation

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 774 | 300 | ❌ VIOLATION |

**Required splits:** Minimum 3 files.

---

## 2. Responsibility Map

The file conflates **5 distinct responsibilities** that must be separated:

| Responsibility | Lines | Description |
|----------------|-------|-------------|
| **VM Core** | 18–63 | Program entry, stack management, index tracking |
| **Opcode Dispatch** | 65–116 | LoadSlot, LoadConst dispatch and execution |
| **Arithmetic Ops** | 118–312 | Binary/unary arithmetic and comparison |
| **Helper Dispatch** | 314–491 | ExprHelper API and arity validation |
| **Helper Impls** | 493–773 | Exists, Length, Empty, Count, Unique, Contains, StartsWith, EndsWith, Has, Append, AppendIf, Merge, Sum |

---

## 3. Primitive Obsession Violations

### 3.1 Raw `usize` Stack Indexing

**Location:** Lines 34, 98, 109

```rust
let mut index = 0usize;                    // line 34
slots.get(idx.as_usize())                  // line 98
constants.get(idx.as_usize())             // line 109
```

**Problem:** `SlotIdx` and `ConstIdx` newtypes exist but are converted to raw `usize` immediately for indexing.

**Fix:** All indexing must go through bounds-checked accessors on the newtype, not raw `.as_usize()`.

---

### 3.2 Raw `i64` Arithmetic Function Pointers

**Location:** Lines 264–287

```rust
fn eval_i64_values_(
    left: i64,
    right: i64,
    op: fn(i64, i64) -> Option<i64>,   // RAW FUNCTION POINTER
) -> ExprResult<SlotValue> {
    let value = op(left, right).ok_or(ExprError::IntegerOverflow)?;
    Ok(SlotValue::I64(value))
}

fn eval_i64_cmp_values_(
    left: i64,
    right: i64,
    op: fn(&i64, &i64) -> bool,        // RAW FUNCTION POINTER
) -> ExprResult<SlotValue> {
    Ok(SlotValue::Bool(op(&left, &right)))
}
```

**Problem:** Using `fn` pointers instead of an enum variant dispatch or a sealed trait. This is unchecked at compile time — you could pass `i64::checked_add` where `i64::checked_sub` is expected.

**Fix:** Use `enum class ArithmeticOp { Add, Sub, Mul, Div }` with a `checked_eval(self, l: i64, r: i64) -> Option<i64>` method. Similarly for comparison ops.

---

### 3.3 Stringly-Typed Reference Errors

**Locations:** Lines 507, 513, 519, 542, 548, 554, 576, 590, 620, 625, 639, 645, 659, 665, 680, 695, 716, 738, 743, 766

```rust
reference: format!("symbol:{id:?}"),   // line 507
reference: format!("list:{list_id:?}"), // line 576
```

**Problem:** Error messages constructed via `format!` with ad-hoc string templates. No centralized reference type.

**Fix:** Introduce `enum class RefKind { Symbol, List, Object }` and a `RefPath(RefKind, DebugValue)` newtype. Every helper impl uses the same construction.

---

### 3.4 Stringly-Typed Type Expectations

**Locations:** Lines 308, 525, 560, 611

```rust
expected: "number".into(),              // line 308
expected: "text, list, or object".into(), // line 525
expected: "text, list, object, or null".into(), // line 560
expected: "list, text, or object".into(),  // line 611
```

**Problem:** Ad-hoc string literals for type expectations. No `TypeName` enum.

**Fix:** `enum class TypeName { Number, Text, List, Object, Null }` with `.label()` returning `&'static str`.

---

## 4. Duplication Violations (Bitter Truth)

### 4.1 Triplicate `expect_symbol` + `store.symbol()` Pattern

Every helper that dereferences a symbol does this exact sequence:

```rust
let id = expect_symbol(*value)?;
let s = store.symbol(id).map_err(|_| ExprError::InvalidReference {
    reference: format!("symbol:{id:?}"),
})?;
```

This appears **10 times** (lines 505–509, 540–544, 615–626, 635–646, 655–666, 674–681, 692–696, 711–716, 733–743, 762–767).

**Fix:** `fn resolve_symbol(store: &ValueStore, slot: SlotValue) -> ExprResult<Symbol>`

### 4.2 Triplicate `expect_list` + `store.list()` Pattern

Same structure for list dereferencing, appears **6 times**.

**Fix:** `fn resolve_list(store: &ValueStore, slot: SlotValue) -> ExprResult<&[SlotValue]>`

### 4.3 Triplicate `expect_object` + `store.object()` Pattern

Same structure for object dereferencing, appears **3 times**.

**Fix:** `fn resolve_object(store: &ValueStore, slot: SlotValue) -> ExprResult<&[ObjectField]>`

### 4.4 Near-Identical Arithmetic Twins

`eval_add_op`, `eval_sub_op`, `eval_mul_op` are structurally identical — they differ only in the binary operation used and the finite-check variant. Lines 161–206 are copy-paste with different `op` arguments.

**Fix:** Single `eval_arithmetic_binop(op: ArithmeticOp, left, right) -> ExprResult<SlotValue>`

### 4.5 Near-Identical Comparison Twins

`eval_gt_op`, `eval_gte_op`, `eval_lt_op`, `eval_lte_op` are structurally identical — lines 224–261.

**Fix:** Single `eval_cmp_op(cmp: CmpOp, left, right) -> ExprResult<SlotValue>`

---

## 5. Required File Split

| New File | Contents | Est. Lines |
|----------|----------|------------|
| `eval_vm.rs` | `eval_expr_program*`, `eval_expr_op_with_store`, `finish_stack`, `next_index`, slot/const loading | ~150 |
| `eval_arithmetic.rs` | Binary/unary ops, `eval_i64_values_`, `eval_div_values_`, `eval_i64_cmp_values_` | ~150 |
| `eval_helpers.rs` | `eval_helper_with_store`, all `eval_helper_*` implementations | ~400 |
| `mod.rs` | Re-exports, keeps public API surface | ~25 |

---

## 6. Summary

| Violation | Severity |
|-----------|----------|
| 774 lines (>300 limit) | **CRITICAL** |
| Raw `fn` pointers for arithmetic dispatch | **HIGH** |
| 10× repeated symbol-dereference pattern | **HIGH** |
| 6× repeated list-dereference pattern | **HIGH** |
| 3× repeated object-dereference pattern | **HIGH** |
| 5× copy-paste arithmetic ops | **MEDIUM** |
| 4× copy-paste comparison ops | **MEDIUM** |
| Stringly-typed reference errors | **MEDIUM** |
| Stringly-typed type expectations | **MEDIUM** |

**Bottom line:** This file is a textbook case of primitive obsession + copy-paste sprawl. The arithmetic operations need an enum dispatch, all store lookups need helper functions, and the file must be split into 3–4 pieces.
