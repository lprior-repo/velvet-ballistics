# ARCHITECTURAL DRIFT HAMMER REPORT

**File**: `crates/vb_runtime/src/primitives/for_each/tests.rs`
**Status**: 🔴 VIOLATION — 309 lines (exceeds 300-line limit)
**Workspace**: `/home/lewis/src/velvet-ballistics/arch-drift-hammer`

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 309 | 300 | 🔴 OVER |

**Verdict**: File MUST be split. No exceptions.

---

## 2. TEST RESPONSIBILITY MAP

The 23 tests in this file cover three distinct primitives:

### `for_each_start` Tests (10 tests)
| Test | Category |
|------|----------|
| `for_each_start_returns_continue_when_list_has_items` | Happy path |
| `for_each_start_returns_done_when_list_is_empty` | Happy path |
| `for_each_start_returns_error_when_input_is_not_list` | Error path |
| `for_each_start_returns_error_when_limit_exceeded` | Error path |
| `for_each_start_returns_error_when_output_missing` | Error path |
| `for_each_start_increments_executed_counter` | Side effect |
| `for_each_start_limit_zero_allows_empty_list` | Boundary |
| `for_each_start_limit_zero_rejects_single_item` | Boundary |
| `for_each_start_null_input_returns_type_mismatch` | Error path |
| `for_each_next_output_slot_same_as_iterator_overwrite` | Edge case |

### `for_each_next` Tests (6 tests)
| Test | Category |
|------|----------|
| `for_each_next_returns_continue_while_items_remain` | Happy path |
| `for_each_next_returns_done_when_tail_empty` | Happy path |
| `for_each_next_returns_error_when_output_missing` | Error path |
| `for_each_next_returns_error_when_iterator_is_not_list` | Error path |
| `for_each_next_increments_executed_counter` | Side effect |
| `for_each_next_output_slot_same_as_iterator_overwrite` | Edge case |

### `for_each_join` Tests (3 tests)
| Test | Category |
|------|----------|
| `for_each_join_returns_done_signal` | Happy path |
| `for_each_join_materializes_ordered_results` | Happy path |
| `for_each_join_returns_error_when_output_missing` | Error path |
| `for_each_join_returns_error_when_next_missing` | Error path |

### Cross-cutting Concerns (4 tests that implicitly test iteration workflow)
| Test | Category |
|------|----------|
| `for_each_start_returns_error_when_input_is_not_list` | Error path |
| `for_each_start_returns_error_when_limit_exceeded` | Error path |
| `for_each_start_limit_zero_allows_empty_list` | Boundary |
| `for_each_start_limit_zero_rejects_single_item` | Boundary |

---

## 3. PRIMITIVE OBSESSION VIOLATIONS

### 3.1 Magic Numbers — Unnamed Constants

| Location | Raw Value | Semantic Meaning |
|----------|-----------|------------------|
| Line 28 | `100` | Iteration limit — used as `limit: 100` but never named |
| Line 133 | `2` | Limit for "limit exceeded" test — not a named constant |
| Line 257 | `0` | Zero limit boundary case |

**Problem**: `100` appears as the de facto "normal" limit throughout tests. This is `DEFAULT_FOR_EACH_LIMIT` or similar but it has no name.

**DDD Violation**: Value objects should be named. `FanoutLimit` exists in the type system (line 6 of `for_each.rs` imports `FanoutLimit`), but tests use raw integers.

### 3.2 Stringly-Typed Error Expectations

| Location | Raw String | Semantic Meaning |
|----------|------------|------------------|
| Line 118 | `"list"` | Expected type in TypeMismatch |
| Line 119 | `"number"` | Found type in TypeMismatch |
| Line 135 | `"for_each_limit"` | Resource name in IterationLimitExceeded |
| Line 189-191 | `"list"`, `"boolean"` | Type mismatch expectations |
| Line 285-287 | `"list"`, `"null"` | Type mismatch expectations |

**Problem**: These string literals are duplicated across multiple tests. They represent type names and resource identifiers that should be constants from `vb_core::errors` or the type system.

**DDD Violation**: Primitive obsession — strings used where a typed enum/domain type would be safer.

### 3.3 Raw Slot/Step Index Construction

Every test creates slot and step indices via `SlotIdx::new(N)` and `StepIdx::new(N)`:

```rust
let input = SlotIdx::new(0);
let item_slot = SlotIdx::new(1);
let output_slot = SlotIdx::new(2);
let body = StepIdx::new(1);
let done = StepIdx::new(2);
```

**Problem**: While `SlotIdx` and `StepIdx` ARE newtypes (good!), the pattern of always constructing with `.new(N)` on every test suggests the domain roles (input slot, item slot, output slot) are not codified as types.

**Suggested refactor**:
```rust
// Instead of:
let input = SlotIdx::new(0);
let item_slot = SlotIdx::new(1);

// Have a test fixture type:
struct ForEachSlots {
    input: SlotIdx,
    item: SlotIdx,
    output: SlotIdx,
    body: StepIdx,
    done: StepIdx,
}
```

### 3.4 Unnamed Iteration Workflow

The tests implicitly document a 3-phase workflow:
1. `for_each_start` — initializes iterator, binds first item
2. `for_each_next` — advances (called repeatedly in a loop body)
3. `for_each_join` — materializes results after loop completes

**Problem**: This workflow is not named or modeled as a domain type. There is no `ForEachWorkflow` or `IterationContext` type that captures the state machine.

---

## 4. TEST ORGANIZATION VIOLATIONS

### 4.1 No Test Categorization

Tests are alphabetically ordered, not semantically grouped:

```
for_each_start_* (10 tests)
for_each_next_* (6 tests)  
for_each_join_* (4 tests, 1 is misfiled)
```

**Problem**: The misfiled test `for_each_next_output_slot_same_as_iterator_overwrite` at line 294 actually tests `for_each_next` but appears after `for_each_join` tests. This is a navigation hazard.

### 4.2 No Test Tags/Groups

There are no `#[test]` subgroups or module-level categorization for:
- Happy path tests
- Error path tests
- Boundary/edge case tests
- Side-effect tests

---

## 5. RECOMMENDED REFACTORING PLAN

### Phase 1: Split the File (Resolve 300-line violation)

```
for_each/tests.rs (309 lines)
        │
        ├── for_each/start_tests.rs    (~110 lines, 10 tests)
        ├── for_each/next_tests.rs     (~75 lines, 6 tests)
        ├── for_each/join_tests.rs     (~65 lines, 4 tests)
        └── for_each/common/mod.rs     (~50 lines, shared helpers)
```

**Total after split**: ~300 lines across 4 files (within limit)

### Phase 2: Name the Magic Numbers

Add to `vb_core` or `vb_runtime`:
```rust
/// Default iteration limit for for_each primitives.
pub const DEFAULT_FOR_EACH_LIMIT: u32 = 100;
```

Or better, use `FanoutLimit::default()` if that impl exists.

### Phase 3: Extract Error String Constants

```rust
// In vb_core::errors or appropriate module
pub const EXPECTED_LIST_TYPE: &str = "list";
pub const FOUND_NUMBER_TYPE: &str = "number";
pub const FOUND_BOOL_TYPE: &str = "boolean";
pub const FOUND_NULL_TYPE: &str = "null";
pub const FOR_EACH_LIMIT_RESOURCE: &str = "for_each_limit";
```

### Phase 4: Create Test Fixture Types

```rust
/// Slot and step indices for for_each iteration tests.
struct ForEachTestFrame {
    run: RunFrame,
    store: ValueStore,
    input: SlotIdx,
    item: SlotIdx,
    output: SlotIdx,
    body: StepIdx,
    done: StepIdx,
}

impl ForEachTestFrame {
    fn new() -> Self { ... }
    fn with_list(self, items: Vec<SlotValue>) -> Self { ... }
}
```

---

## 6. SUMMARY SCORECARD

| Criterion | Status | Notes |
|-----------|--------|-------|
| Line count ≤ 300 | 🔴 FAIL | 309 lines, 3% over |
| Named constants for magic numbers | 🔴 FAIL | `100`, `2`, `0` are raw |
| Error strings are domain constants | 🔴 FAIL | `"list"`, `"for_each_limit"` repeated |
| Test organization by category | 🔴 FAIL | Alphabetical only |
| Primitive obsession in assertions | 🟡 WARN | String comparisons in match arms |
| Workflow modeled as type | 🟡 WARN | No `ForEachWorkflow` or `IterationContext` |

---

## 7. VERDICT

**ARCHITECTURAL DRIFT CONFIRMED**

The file exceeds the 300-line limit and exhibits multiple primitive obsession violations in its test assertions. The tests themselves are well-structured (good use of `fresh_frame`, `list_in_slot` helpers, proper error assertion patterns), but the organizational layer is lacking.

**Required Actions**:
1. Split into `start_tests.rs`, `next_tests.rs`, `join_tests.rs`, and `common/mod.rs`
2. Name the iteration limit constant(s)
3. Extract error string constants to a shared module
4. Group tests by category (happy/error/boundary) within each file
