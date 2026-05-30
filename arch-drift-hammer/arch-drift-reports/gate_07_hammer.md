# Architectural Drift Report: `gate_07_stack.rs`

**File**: `/home/lewis/src/velvet-ballistics/crates/vb_validate/src/gate_07_stack.rs`
**Line Count**: 309 lines (**EXCEEDS 300 LINE LIMIT by 9 lines**)
**Status**: `VIOLATION`

---

## 1. Line Count Violation

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 309 | 300 | **+9 over limit** |

---

## 2. Primitive Obsession Violations

### 2.1 Raw `u8` for Stack Depth (No NewType)

| Location | Issue |
|----------|-------|
| Line 11 | `const MAX_EXPR_STACK_DEPTH: u8 = 64;` |
| Lines 45-46 | `let mut depth: u8 = 0; let mut max_depth: u8 = 0;` |
| Line 44 (return type) | `pub fn compute_stack_depth(...) -> ValidationResult<u8>` |
| Line 17 | `let contract_stack = parts.resource_contract.max_expr_stack;` (raw `u8`) |
| Line 25 | `expr.max_stack > contract_stack` (comparing raw `u8` values) |

**Refactor**: Create `StackDepth(u8)` NewType with:
- `impl TryFrom<u8> for StackDepth` bounded to 0..=64
- `impl StackDepth { const MAX: Self; }`
- Replace all `u8` stack depth usages

### 2.2 Scattered `usize::from()` Conversions

| Line | Conversion |
|------|------------|
| 20 | `usize::from(contract_stack)` |
| 21 | `usize::from(MAX_EXPR_STACK_DEPTH)` |
| 27 | `usize::from(expr.max_stack)` |
| 28 | `usize::from(contract_stack)` |
| 35 | `usize::from(expr.max_stack)` |
| 36 | `usize::from(computed)` |
| 52-53 | `usize::from(MAX_EXPR_STACK_DEPTH)` |
| 59 | `usize::from(depth) + usize::from(push_amount)` |

**Violation**: Every error construction requires manual widening. The `StackDepth` NewType should implement `Display`/`fmt::Display` to eliminate these conversions.

### 2.3 Raw Index Type: `expr_index: usize`

**Line 24**: `for (expr_index, expr) in parts.expressions.iter().enumerate()`

**Violation**: `usize` for array index. Should be `ExprIdx` from `vb_core::ids`.

### 2.4 Magic Numbers in Match Arms

**Lines 70-81** (`pop_count` function):

```rust
ExprOp::LoadSlot(_) | ExprOp::LoadConst(_) | ExprOp::LoadAccessor(_) => 0,
ExprOp::Not | ExprOp::Exists | ... => 1,
ExprOp::AppendIf => 3,
_ => 2,
```

**Violations**:
- `0`, `1`, `3`, `2` are magic numbers
- No `PopCount` or `StackEffect` domain type
- Fallback `_ => 2` hidesIntent: what ops actually pop 2?

**Refactor**: Define typed constants:
```rust
const POP_0: u8 = 0;
const POP_1: u8 = 1;
const POP_2: u8 = 2;
const POP_3: u8 = 3;
```

---

## 3. Scott Wlaschin DDD Violations

### 3.1 Loose Functions Instead of Domain Methods

**`pop_count`** (line 69) and **`push_count`** (line 84) are free functions operating on `ExprOp`.

**DDD Principle Violated**: Operations that are intrinsically tied to a type should be methods on that type or on a domain service, not loose functions.

**Refactor Options**:
1. Add `impl ExprOp { fn pop_count(&self) -> u8 { ... } }`
2. Create `StackEffect` domain value object with `pop` and `push` fields

### 3.2 State Machine Not Modeled as Type

**Lines 44-67** (`compute_stack_depth`):

```rust
let mut depth: u8 = 0;
let mut max_depth: u8 = 0;
for op in ops {
    // ... state transitions on raw u8
}
```

**Violation**: This is a **state machine** (StackDepthTracker) tracking:
- `depth`: current stack height
- `max_depth`: peak observed

But it's implemented as raw variable manipulation, making invalid states representable (e.g., `depth` can underflow).

**Refactor**: Create `StackDepthTracker`:
```rust
struct StackDepthTracker {
    current: u8,
    peak: u8,
}
impl StackDepthTracker {
    fn apply_op(&mut self, op: &ExprOp) -> Result<(), StackUnderflow>;
    fn max_depth(self) -> u8;
}
```

### 3.3 `ResourceContract::DEFAULT` Struct Access

**Line 105**: `resource_contract: ResourceContract::DEFAULT`

**DDD Principle Violated**: "Parse, don't validate" - a `DEFAULT` constant on a struct suggests the struct itself knows how to construct valid instances. This should be a proper constructor or builder.

**Refactor**: Use `ResourceContract::default()` or a named constructor like `ResourceContract::conservative()`.

---

## 4. Duplication Architecture Issue

**Critical**: This file (`gate_07_stack.rs`) is a **test-only module** (`#[cfg(test)]` per `lib.rs` line 43) that **duplicates** the production gate 07 logic in `gates.rs` (lines 23-138).

| Location | Content |
|----------|---------|
| `gates.rs` lines 23-138 | Production gate 07 (pop_count, push_count, stack_effect, compute_stack_depth, validate_gate_07_expression_stack_depth) |
| `gate_07_stack.rs` lines 1-309 | Test-only duplicate with identical logic + tests |

**Issue**: Two sources of truth for the same validation logic. If production logic in `gates.rs` changes, tests in `gate_07_stack.rs` may diverge.

**Recommendation**: Delete `gate_07_stack.rs` entirely. Production logic is already in `gates.rs` with tests in `gate_tests.rs`.

---

## 5. Summary of Violations

| Category | Count | Severity |
|----------|-------|----------|
| Line count > 300 | 1 | HIGH |
| Primitive obsession (NewType missing) | 4 | MEDIUM |
| Magic numbers | 1 (4 values) | MEDIUM |
| Loose functions vs domain methods | 2 | MEDIUM |
| State machine not typed | 1 | MEDIUM |
| Struct DEFAULT access | 1 | LOW |
| Test/prod duplication | 1 | HIGH |

---

## 6. Recommended Refactors

### Priority 1: Eliminate Duplication
Delete `gate_07_stack.rs`. Tests already exist in `gate_tests.rs` or should be added there.

### Priority 2: Create `StackDepth` NewType
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct StackDepth(u8);

impl StackDepth {
    pub const MAX: Self = Self(64);
    pub const ZERO: Self = Self(0);
    
    pub fn new(v: u8) -> Option<Self> {
        if v <= 64 { Some(Self(v)) } else { None }
    }
    
    pub fn checked_sub(self, rhs: u8) -> Option<Self> {
        self.0.checked_sub(rhs).map(Self)
    }
    
    pub fn checked_add(self, rhs: u8) -> Option<Self> {
        self.0.checked_add(rhs).map(Self)
    }
}
```

### Priority 3: Type `StackEffect`
```rust
#[derive(Debug, Clone, Copy)]
pub struct StackEffect { pub pop: u8, pub push: u8 }

impl ExprOp {
    pub fn stack_effect(&self) -> StackEffect { ... }
}
```

### Priority 4: Inline Tests into `gate_tests.rs`
Move all `#[cfg(test)]` blocks from `gate_07_stack.rs` into `gate_tests.rs` under a `mod gate_07_tests` section.

---

## 7. Files Affected

| File | Action |
|------|--------|
| `crates/vb_validate/src/gate_07_stack.rs` | **DELETE** (duplication) |
| `crates/vb_validate/src/gates.rs` | Add `StackDepth` NewType, `StackEffect` domain type, refactor `pop_count`/`push_count` to methods |
| `crates/vb_validate/src/gate_tests.rs` | Ensure gate 07 tests cover all paths |

---

**VERDICT**: `gate_07_stack.rs` violates the 300-line rule and exhibits multiple primitive obsession violations. The recommended fix is to **delete this file** and ensure test coverage exists in `gate_tests.rs` for the production code in `gates.rs`.
