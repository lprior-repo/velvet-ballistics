# ARCHITECTURAL DRIFT REPORT: `object_list.rs`

**File**: `crates/vb_core/src/engine/object_list.rs`
**Total Lines**: 442 (VIOLATION: exceeds 300 line limit by 142 lines)
**Status**: 🚨 HAMMER REQUIRED

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 442 | 300 | 🔴 VIOLATION (+142) |
| Production code | ~158 | 300 | ✅ Within limit |
| Test code | ~284 | N/A | Isolated, but total file is 442 |

**Hammer Directive**: Split at the `#[cfg(test)]` boundary. Tests belong in `crates/workspace_tests/` or a sibling `object_list_test.rs` in the same module directory.

---

## 2. RESPONSIBILITY MAP

```
object_list.rs
├── read_object_fields()     → Reads (SymbolId, SlotIdx) pairs into Vec<ObjectField>
├── build_object()            → Public API: constructs ObjectId from field pairs
├── build_object_with_taint() → Internal: constructs ObjectId + accumulated Taint
├── read_list_items()         → Reads SlotIdx slice into Vec<SlotValue>
├── build_list()              → Public API: constructs ListId from slot indices
├── build_list_with_taint()   → Internal: constructs ListId + accumulated Taint
└── tests (17)                → All test coverage
```

---

## 3. PRIMITIVE OBSESSION VIOLATIONS (Scott Wlaschin DDD)

### 3.1 Raw Tuple Slices as Domain Types

| Location | Primitive Type | Missing Domain Type |
|----------|---------------|---------------------|
| L12, L44 | `&[(SymbolId, SlotIdx)]` | `FieldMappingSlice` or `ObjectFields` |
| L89, L116 | `&[SlotIdx]` | `SlotIndexSlice` or `SlotRange` |

**Problem**: The domain concept "ordered collection of field mappings" is represented as a raw `&[(SymbolId, SlotIdx)]`. This allows any `&[(SymbolId, SlotIdx)]` to be passed to any function accepting this type, with no type-level encoding of:
- Whether the slice represents object fields vs. list items
- Whether the mapping is ordered (for objects) vs. unordered
- Whether duplicates are allowed

**Fix**: Introduce `ObjectFields<'a>` and `SlotIndexSlice<'a>` newtypes that wrap the raw slices and provide domain-specific methods.

### 3.2 Manual Index Arithmetic Instead of Iterators

The following pattern appears **4 times** (L19-35, L61-80, L96-107, L137-152):

```rust
let mut index = 0usize;
while index < items.len() {
    let item = items.get(index).ok_or(...)?;
    // process item
    index = index.checked_add(1).ok_or(...)?;
}
```

**Violations**:
- **WET (Write Everything Twice)**: Identical loop structure repeated 4x
- **Primitive obsession**: Using raw `usize` index instead of iterator
- **Bounded arithmetic**: Manual `checked_add` when Rust's iterator handles bounds

**Fix**: Use `.iter().enumerate()` or `.iter().zip()` with `Result` transformation.

### 3.3 Duplicated "Build With Taint" Logic

`build_object_with_taint` (L51-84) and `build_list_with_taint` (L123-157) share:
- Reserve capacity pattern
- Index-bounded loop
- `checked_add` overflow check
- Accumulated taint join

**Problem**: 60+ lines of nearly identical logic for two domain concepts.

**Fix**: Extract common `build_with_taint` generic helper or use iterator combinators.

---

## 4. SPECIFIC CODE SMELLS

### 4.1 Code Duplication: `read_object_fields` vs `read_list_items`

| Aspect | `read_object_fields` (L10-38) | `read_list_items` (L87-110) |
|--------|-------------------------------|------------------------------|
| Reserve | ✅ | ✅ |
| Loop structure | Identical | Identical |
| Bounds check | `.get(index).ok_or(...)` | `.get(index).ok_or(...)` |
| Overflow check | `checked_add(1)` | `checked_add(1)` |

### 4.2 Inconsistent Error Context Strings

- L24: `"build_object field index checked by loop bound"`
- L66: `"build_object field index checked by loop bound"` (duplicated)
- L100: `"build_list item index checked by loop bound"`
- L142: `"build_list item index checked by loop bound"` (duplicated)

**Fix**: These should be enum variants or a single helper, not string duplication.

### 4.3 Unnecessary `ok_or` Chaining

L19-35 could be:

```rust
for &(key, slot) in fields.iter() {
    let value = *run.read_slot(*slot)?;
    entries.push(ObjectField { key: *key, value, taint: Taint::Clean });
}
```

No manual index, no `checked_add`, no `get(index)`.

---

## 5. HAMMER DIRECTIVES

### Directive 1: File Split (MANDATORY)
```
object_list.rs (442 lines)
  → object_list.rs (production: ~158 lines)
  → object_list_test.rs (tests: ~284 lines, #[cfg(test)] module or separate file)
```

### Directive 2: Newtype Wrappers (MANDATORY)
```rust
// Replace &[(SymbolId, SlotIdx)] with:
pub struct ObjectFields<'a>(&'a [(SymbolId, SlotIdx)]);

// Replace &[SlotIdx] with:
pub struct SlotIndexSlice<'a>(&'a [SlotIdx]);
```

### Directive 3: Iterator Refactor (MANDATORY)
Replace all 4 manual while loops with iterator-based alternatives:
```rust
fn read_object_fields(...) -> Result<Vec<ObjectField>, EngineError> {
    fields.iter().map(|&(key, slot)| {
        Ok(ObjectField { key, value: *run.read_slot(slot)?, taint: Taint::Clean })
    }).collect()
}
```

### Directive 4: Extract Taint-Joining Helper (RECOMMENDED)
Factor `build_*_with_taint` into a generic combinator that applies to any iterator.

---

## 6. VERDICT

| Category | Status |
|----------|--------|
| Line count | 🔴 FAIL (442 > 300) |
| Primitive obsession | 🔴 FAIL (raw slices everywhere) |
| DRY violation | 🔴 FAIL (4x identical loop structure) |
| Iterator idioms | 🔴 FAIL (manual index arithmetic) |
| Test isolation | 🟡 PARTIAL (tests in same file, needs split) |

**Recommendation**: Hammer immediately. Create newtypes, refactor to iterators, split tests, then re-evaluate.
