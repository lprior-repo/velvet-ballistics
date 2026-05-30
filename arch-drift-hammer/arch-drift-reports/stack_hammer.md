# Arch Drift Report: `stack.rs`

**File:** `crates/vb_core/src/engine/expr_eval/stack.rs`
**Line Count:** 444 (EXCEEDS 300-line limit by 144 lines, 48% over budget)
**Status:** VIOLATION

---

## 1. Structural Violations

### 1.1 File Size (CRITICAL)
- **444 lines** vs. **300-line maximum**
- **Overflow:** 144 lines (48% budget overrun)
- **Verdict:** FILE MUST BE SPLIT

---

## 2. DDD Primitive Obsession Violations

### 2.1 Raw `u8` for Stack Metadata (Lines 10-11)
```rust
pub(super) struct ExprStack {
    values: [SlotValue; MAX_EXPRESSION_STACK_USIZE],
    len: u8,        // PRIMITIVE OBSESSION
    capacity: u8,   // PRIMITIVE OBSESSION
}
```
**Problem:** `u8` is a bare primitive. Should be `StackDepth` and `StackCapacity` types.

**Correct DDD:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct StackDepth(u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackCapacity(u8);
```

---

### 2.2 Untyped Constant `MAX_EXPRESSION_STACK_USIZE` (limits.rs:47)
```rust
pub const MAX_EXPRESSION_STACK_USIZE: usize = 64;
```
**Problem:** Raw `usize` with no type safety. Duplicated with `MAX_EXPRESSION_STACK: u8 = 64` (limits.rs:43).

**Violation:** Two constants representing the same concept with different primitive types.

---

### 2.3 Type Expectors as Primitive Obsession (Lines 91-139)
```rust
pub(super) fn expect_bool(value: SlotValue) -> Result<bool, EngineError> {
    match value {
        SlotValue::Bool(value) => Ok(value),
        other => Err(EngineError::TypeMismatch { expected: "boolean", found: other.type_name() }),
    }
}

pub(super) fn expect_i64(value: SlotValue) -> Result<i64, EngineError> { ... }
pub(super) fn expect_symbol(value: SlotValue) -> Result<SymbolId, EngineError> { ... }
pub(super) fn expect_list(value: SlotValue) -> Result<ListId, EngineError> { ... }
pub(super) fn expect_object(value: SlotValue) -> Result<ObjectId, EngineError> { ... }
```
**Problem:** Five nearly identical functions doing ad-hoc type coercion via `match`. This is textbook primitive obsession - the `SlotValue` enum already carries type information, but instead of using a typed extraction pattern, we're manually matching and unwrapping.

**Correct DDD:** Implement `TryFrom<SlotValue>` for each target type, or use a typed `ValueExtractor` visitor.

---

### 2.4 String Literals for Type Names (Lines 94-96, 104-106, 114-116, etc.)
```rust
expected: "boolean",  // STRING LITERAL
expected: "number",   // STRING LITERAL  
expected: "text",     // STRING LITERAL
expected: "list",     // STRING LITERAL
expected: "object",   // STRING LITERAL
```
**Problem:** These string literals are repeated across multiple error sites. They should be a `TypeName` enum.

**Correct DDD:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeName {
    Boolean,
    Number,
    Text,
    List,
    Object,
}
```

---

## 3. Liskov Subversion / Confusing API Design

### 3.1 `pop_pair` Return Order Confusion (Lines 71-75)
```rust
pub(super) fn pop_pair(stack: &mut ExprStack) -> Result<(SlotValue, SlotValue), EngineError> {
    let right = pop_value(stack)?;  // pops FIRST (top of stack)
    let left = pop_value(stack)?;   // pops SECOND
    Ok((left, right))  // Returns (FIRST-PUSHED, SECOND-PUSHED)
}
```
**Problem:** The function pops in LIFO order but returns `(left, right)` where `left` was pushed first and `right` was pushed last. This is semantically correct but the variable naming (`right` first, `left` second) is backwards and easy to misuse.

**Correct DDD:** Either name it `pop_ordered_pair` with explicit documentation, or use a struct like `BinaryOp { lhs, rhs }` where `lhs` is first-pushed and `rhs` is second-pushed.

---

### 3.2 Redundant Identity Wrappers (Lines 63-69)
```rust
pub(super) fn push_value(stack: &mut ExprStack, value: SlotValue) -> Result<(), EngineError> {
    stack.push(value)  // IDENTITY WRAPPER - adds no value
}

pub(super) fn pop_value(stack: &mut ExprStack) -> Result<SlotValue, EngineError> {
    stack.pop()  // IDENTITY WRAPPER - adds no value
}
```
**Problem:** These are 1:1 pass-through wrappers that increase line count without abstraction benefit.

---

## 4. Refactoring Prescription

### 4.1 Split Into Multiple Files

**Proposed Structure:**
```
engine/expr_eval/
├── mod.rs           (20 lines - unchanged)
├── stack.rs         (~80 lines) - ExprStack struct + push/pop only
├── stack_depth.rs   (~50 lines) - StackDepth, StackCapacity newtypes  
├── value_extractors.rs (~70 lines) - TryFrom<SlotValue> implementations
├── stack_ops.rs     (~60 lines) - pop_pair, pop_triple, pop_i64_pair
└── stack_tests.rs   (inline tests moved from stack.rs)
```

### 4.2 Implement Typed Extraction
```rust
impl TryFrom<SlotValue> for bool {
    type Error = EngineError;
    fn try_from(value: SlotValue) -> Result<Self, Self::Error> {
        match value {
            SlotValue::Bool(v) => Ok(v),
            other => Err(EngineError::TypeMismatch { 
                expected: TypeName::Boolean, 
                found: other.type_name() 
            }),
        }
    }
}
```

### 4.3 Use Struct for Binary Op Results
```rust
#[derive(Debug, Clone, Copy)]
pub struct BinaryOp {
    pub lhs: SlotValue,  // left-hand side (first pushed)
    pub rhs: SlotValue,  // right-hand side (second pushed, top of stack)
}
```

---

## 5. Summary

| Violation Type | Count | Severity |
|----------------|-------|---------|
| File size > 300 lines | 1 | CRITICAL |
| Primitive obsession (u8, usize) | 3 | HIGH |
| String literals for types | 5 | MEDIUM |
| Confusing LIFO naming | 1 | MEDIUM |
| Identity wrapper bloat | 2 | LOW |

**Total Violations:** 12
**Priority:** HIGH - file must be split and primitives reified before further development.

---

*Report generated by architectural-drift agent*
