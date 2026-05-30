# ARCHITECTURAL DRIFT REPORT: `ops_text_list.rs`

**File**: `crates/vb_core/src/engine/expr_eval/ops_text_list.rs`
**Total Lines**: 1045
**Severity**: CRITICAL — 3× VIOLATION of <300 line rule

---

## EXECUTIVE SUMMARY

| Category | Finding | Severity |
|----------|---------|----------|
| File Size | 1045 lines (limit: 300) | CRITICAL |
| Primitive Obsession | Raw `Vec<SlotValue>` scattered throughout | HIGH |
| DDD Cohesion | No value objects for text/list operations | HIGH |
| Inline Tests | 821 lines of tests in production crate | MEDIUM |
| Type Dispatch | Massive match statements for polymorphism | HIGH |

---

## 1. LINE COUNT VIOLATION

**Requirement**: All files must be ≤300 lines.
**Actual**: 1045 lines.
**Ratio**: 3.48× over limit.

### Breakdown

| Section | Lines | % of Limit |
|---------|-------|------------|
| Production ops (1-222) | 222 | 74% ✓ |
| Inline tests (224-1045) | 822 | 274% ✗ |
| **Total** | **1045** | **348% ✗** |

---

## 2. TEXT OPERATIONS DOMAIN MAP

```
TextOps
├── contains(haystack: Symbol, needle: Symbol) → Bool
├── starts_with(text: Symbol, prefix: Symbol) → Bool
└── ends_with(text: Symbol, suffix: Symbol) → Bool
```

### Primitive Obsession Violations

| Operation | Problem | Should Be |
|-----------|---------|-----------|
| `eval_contains` | Manual `SymbolId` → `&str` fetch + `contains(&str)` | `TextValue::contains(&self, other: &TextValue)` |
| `eval_starts_with` | Manual `SymbolId` → `&str` fetch + `starts_with(&str)` | `TextValue::starts_with(&self, prefix: &TextValue)` |
| `eval_ends_with` | Manual `SymbolId` → `&str` fetch + `ends_with(&str)` | `TextValue::ends_with(&self, suffix: &TextValue)` |

Each text op follows this anti-pattern:
```rust
// ANTI-PATTERN (current)
let haystack_id = expect_symbol(haystack)?;
let haystack_str = store.symbol(haystack_id).map_err(...)?;
push_value(stack, SlotValue::Bool(haystack_str.contains(needle_str)))

// SHOULD BE (DDD)
let haystack = TextValue::from_symbol(haystack_id, store)?;
let needle = TextValue::from_symbol(needle_id, store)?;
push_value(stack, SlotValue::Bool(haystack.contains(&needle)?))
```

---

## 3. LIST OPERATIONS DOMAIN MAP

```
ListOps
├── has(list: ListId, item: SlotValue) → Bool
├── length(value: SlotValue) → i64         ← polymorphic
├── empty(value: SlotValue) → Bool         ← polymorphic
├── sum(list: ListId) → i64
├── count(list: ListId) → i64
├── append(list: ListId, item: SlotValue) → ListId
├── append_if(list: ListId, item: SlotValue, cond: Bool) → ListId
└── unique(list: ListId) → ListId
```

### Primitive Obsession Violations

| Operation | Problem | Should Be |
|-----------|---------|-----------|
| `eval_has` | Raw `&[SlotValue]` slice, manual contains | `ListValue::has(&self, item: &SlotValue)` |
| `eval_length` | 28-line match dispatching on `SlotValue` variant | `SlotValue::length(&self, store: &ValueStore)` |
| `eval_empty` | 27-line match dispatching on `SlotValue` variant | `SlotValue::is_empty(&self, store: &ValueStore)` |
| `eval_sum` | Raw `for &item in items` loop | `ListValue::sum(&self)?` |
| `eval_count` | Raw `items.len()` | `ListValue::count(&self)` |
| `eval_append` | `Vec::to_vec()` + push, manual box conversion | `ListValue::append(&self, item: SlotValue)?` |
| `eval_append_if` | Same as append + conditional | `ListValue::append_if(&self, item, cond)?` |
| `eval_unique` | O(n²) `Vec::contains()` instead of `HashSet` | `ListValue::unique(&self)?` |

---

## 4. POLYMORPHIC TYPE DISPATCH: `eval_length`

**Lines 70-102**: 32 lines of match-based polymorphism.

```rust
pub(super) fn eval_length(stack: &mut ExprStack, store: &ValueStore) -> Result<(), EngineError> {
    let value = super::stack::pop_value(stack)?;
    let len = match value {
        SlotValue::Symbol(id) => { /* 4 lines */ }
        SlotValue::List(id) => { /* 4 lines */ }
        SlotValue::Object(id) => { /* 4 lines */ }
        other => { /* 4 lines */ }
    };
    // ... overflow check ...
}
```

**Problem**: This is a type switch anti-pattern. Scott Wlaschin calls this "pattern matching on data" instead of "tell, don't ask."

**DDD Fix**: Implement `Lengthable` trait:
```rust
trait Lengthable {
    fn length(&self, store: &ValueStore) -> Result<usize, EngineError>;
}

impl Lengthable for SlotValue {
    fn length(&self, store: &ValueStore) -> Result<usize, EngineError> {
        match self {
            SlotValue::Symbol(id) => Ok(store.symbol(*id)?.len()),
            SlotValue::List(id) => Ok(store.list(*id)?.len()),
            SlotValue::Object(id) => Ok(store.object(*id)?.len()),
            _ => Err(EngineError::TypeMismatch { ... }),
        }
    }
}
```

---

## 5. O(n²) ALGORITHM: `eval_unique`

**Lines 203-222**:

```rust
let mut seen: Vec<SlotValue> = Vec::new();
for &item in items {
    if !seen.contains(&item) {  // ← O(n) per item = O(n²) total
        seen.push(item);
    }
}
```

**Problem**: Uses `Vec::contains()` which is O(n) for each element, making `eval_unique` O(n²).

**Fix**: Use `Vec::contains()` on a first-pass unique scan OR introduce a HashSet for tracking seen items:
```rust
let mut seen: Vec<SlotValue> = Vec::with_capacity(items.len());
let mut dupes: Vec<SlotValue> = Vec::new();
for &item in items {
    if seen.contains(&item) {
        dupes.push(item);
    } else {
        seen.push(item);
    }
}
// dedupe seen
for dupe in dupes { seen.retain(|x| x != &dupe); }
```

Or better: return `seen` directly since we only want unique in order.

---

## 6. STACK MECHANICS SPILLAGE

Every function manually:
1. Pops values from stack
2. Unwraps IDs via `expect_*` helpers
3. Fetches from store
4. Computes
5. Pushes result

**DDD Violation**: Stack operations are interleaved with domain logic. These should be separated:

```
┌─────────────────────────────────────┐
│  PRESENTATION LAYER                 │
│  eval_contains() {                  │
│    let (a, b) = pop_pair(stack);     │  ← Stack mechanics
│    let result = TextOps::contains(a, b, store)?;  ← Domain
│    push_value(stack, result);        │  ← Stack mechanics
│  }                                  │
└─────────────────────────────────────┘

SHOULD BE:

┌─────────────────────────────────────┐
│  DOMAIN LAYER                       │
│  TextOps::contains(hay, needle, store) → Result<Bool, EngineError>
└─────────────────────────────────────┘
┌─────────────────────────────────────┐
│  PRESENTATION LAYER (thin)          │
│  eval_contains(stack, store) {      │
│    let (a, b) = pop_pair(stack);     │
│    let result = TextOps::contains(a, b, store)?;  // ← Domain call
│    push_value(stack, result);        │
│  }                                  │
└─────────────────────────────────────┘
```

---

## 7. INLINE TEST MODULE: 821 LINES

**Lines 224-1045**: `#[cfg(test)] mod tests`

**Violations**:
1. **Location**: Tests belong in `crates/workspace_tests/`, not production crate
2. **Size**: 821 lines of inline tests is a separate code smell
3. **Test infrastructure duplication**: `eval_ops`, `eval_ops_with_slots`, `ensure_equal` helpers are repeated test boilerplate that should be in a shared test harness

### Test Infrastructure (should be extracted)

```rust
// Lines 238-305: 67 lines of test helpers
fn ensure_equal<T>(actual: T, expected: T) -> Result<(), String>
fn eval_ops(ops: Vec<ExprOp>, ...) -> Result<SlotValue, String>
fn eval_ops_with_slots(ops: Vec<ExprOp>, ...) -> Result<SlotValue, String>
```

**Fix**: Extract to `crates/vb_core/tests/text_list_ops.rs` or better, `crates/workspace_tests/engine_text_list_ops/`.

---

## 8. SPECIFIC CODE QUALITY ISSUES

### Issue 1: `eval_append` clones entire list (O(n) memory)

```rust
let mut new_items: Vec<SlotValue> = items.to_vec();  // Full copy
new_items.push(item);
let new_list = store.insert_list(new_items.into_boxed_slice())?;
```

**Problem**: Always copies the entire list, even when appending.
**Better**: If ValueStore supported in-place append (which it can't safely), or use a more efficient immutable data structure.

### Issue 2: Error messages use stringly-typed reasons

```rust
EngineError::InvalidCompiledWorkflow {
    reason: "length exceeds i64 range",  // Stringly-typed
}
```

**Should be**: `EngineError::LengthExceedsI64Range` variant or similar.

### Issue 3: `expect_i64` used in sum loop

```rust
for &item in items {
    let n = expect_i64(item)?;  // Fails on non-i64 element
    sum = sum.checked_add(n)?;
}
```

**Problem**: `sum` on a list assumes all elements are i64. If list contains heterogeneous types, it fails with a cryptic index-based error instead of a domain error like "sum requires numeric list".

---

## 9. RECOMMENDED REFACTORING

### Phase 1: Split the file

```
ops_text_list.rs (300 lines)
├── TextOps value object impl
├── ListOps value object impl  
├── eval_* presentation functions (thin wrappers)
└── NO TESTS (move to workspace_tests)
```

```
ops_text_list_text.rs (100 lines)
├── TextValue domain type
├── impl TextValue { contains, starts_with, ends_with }

ops_text_list_list.rs (150 lines)
├── ListValue domain type
├── impl ListValue { has, length, empty, sum, count, append, append_if, unique }

ops_text_list_eval.rs (100 lines)
├── eval_contains, eval_starts_with, eval_ends_with
├── eval_has, eval_length, eval_empty, eval_sum, eval_count, eval_append, eval_append_if, eval_unique
```

### Phase 2: Extract inline tests

```
crates/workspace_tests/
└── expr_eval_text_list_ops.rs  (821 lines)
    └── Move all #[test] modules here
```

### Phase 3: Implement value objects

```rust
// TextValue domain type
pub struct TextValue<'a>(&'a str);

impl<'a> TextValue<'a> {
    pub fn from_symbol(id: SymbolId, store: &'a ValueStore) -> Result<Self, EngineError> {
        Ok(TextValue(store.symbol(id)?))
    }
    pub fn contains(&self, needle: &TextValue) -> bool {
        self.0.contains(needle.0)
    }
    pub fn starts_with(&self, prefix: &TextValue) -> bool {
        self.0.starts_with(prefix.0)
    }
    pub fn ends_with(&self, suffix: &TextValue) -> bool {
        self.0.ends_with(suffix.0)
    }
}
```

### Phase 4: Fix O(n²) unique

```rust
impl ListValue {
    pub fn unique(&self) -> Result<ListId, EngineError> {
        use std::collections::HashSet;
        let mut seen: HashSet<&SlotValue> = HashSet::new();
        let mut result: Vec<SlotValue> = Vec::new();
        for item in self.items() {
            if seen.insert(item) {
                result.push(*item);
            }
        }
        // ... store.insert_list
    }
}
```

---

## 10. SUMMARY SCORECARD

| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| Lines | 1045 | ≤300 | 🔴 FAIL |
| Value objects | 0 | 2+ | 🔴 FAIL |
| Inline tests | 821 lines | 0 lines | 🔴 FAIL |
| O(n²) algos | 1 | 0 | 🔴 FAIL |
| Type dispatch match arms | 27-28 | 0 | 🔴 FAIL |
| Primitive obsession instances | 10+ | 0 | 🔴 FAIL |

---

## 11. MANDATORY ACTIONS

1. **SPLIT FILE** into ≤300 line chunks
2. **MOVE TESTS** to `crates/workspace_tests/`
3. **CREATE** `TextValue` and `ListValue` domain types
4. **REPLACE** O(n²) unique with HashSet-based dedup
5. **EXTRACT** polymorphic dispatch from `eval_length`/`eval_empty` into trait

---

*Report generated by architectural-drift agent. File must be <300 lines before re-review.*
