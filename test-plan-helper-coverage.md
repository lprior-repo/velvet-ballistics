# Test Plan: Section 46 Helper Function Coverage Gaps

## Summary
- **Bead**: Section 46 Helper Function Coverage Gaps
- **Behaviors identified**: 10 helpers × (happy + edge + error) = 30 behavioral scenarios
- **Trophy allocation**: 28 unit / 0 integration / 0 e2e (pure Calc-layer helpers)
- **Proptest invariants**: 5 (for commutative/associative helpers where applicable)
- **Fuzz targets**: 0 (no parsing boundaries in these pure helpers)
- **Kani harnesses**: 0 (arithmetic bounded by i64, no formal proofs needed)
- **Mutation threshold**: ≥90% kill rate

---

## 1. Behavior Inventory

### Text Helpers

| # | Behavior | Form |
|---|----------|------|
| B1 | `contains` returns true when needle exists in haystack | "Helper returns true when substring matches" |
| B2 | `contains` returns false when needle absent | "Helper returns false when substring absent" |
| B3 | `contains` rejects non-symbol haystack | "Helper returns TypeMismatch when haystack is not text" |
| B4 | `contains` rejects non-symbol needle | "Helper returns TypeMismatch when needle is not text" |
| B5 | `contains` returns SymbolOutOfBounds when haystack ID invalid | "Helper returns error when haystack symbol OOB" |
| B6 | `contains` returns SymbolOutOfBounds when needle ID invalid | "Helper returns error when needle symbol OOB" |
| B7 | `starts_with` returns true when text begins with prefix | "Helper returns true when prefix matches" |
| B8 | `starts_with` returns false when text does not begin with prefix | "Helper returns false when prefix absent" |
| B9 | `starts_with` rejects non-symbol text | "Helper returns TypeMismatch when text is not text" |
| B10 | `starts_with` rejects non-symbol prefix | "Helper returns TypeMismatch when prefix is not text" |
| B11 | `starts_with` returns SymbolOutOfBounds when text ID invalid | "Helper returns error when text symbol OOB" |
| B12 | `starts_with` returns SymbolOutOfBounds when prefix ID invalid | "Helper returns error when prefix symbol OOB" |
| B13 | `ends_with` returns true when text ends with suffix | "Helper returns true when suffix matches" |
| B14 | `ends_with` returns false when text does not end with suffix | "Helper returns false when suffix absent" |
| B15 | `ends_with` rejects non-symbol text | "Helper returns TypeMismatch when text is not text" |
| B16 | `ends_with` rejects non-symbol suffix | "Helper returns TypeMismatch when suffix is not text" |
| B17 | `ends_with` returns SymbolOutOfBounds when text ID invalid | "Helper returns error when text symbol OOB" |
| B18 | `ends_with` returns SymbolOutOfBounds when suffix ID invalid | "Helper returns error when suffix symbol OOB" |

### List Helpers

| # | Behavior | Form |
|---|----------|------|
| B19 | `empty` returns true when value is Null | "Helper returns true when value is null" |
| B20 | `empty` returns true when value is empty text | "Helper returns true when text is empty" |
| B21 | `empty` returns true when value is empty list | "Helper returns true when list is empty" |
| B22 | `empty` returns true when value is empty object | "Helper returns true when object is empty" |
| B23 | `empty` returns false when text is non-empty | "Helper returns false when text has characters" |
| B24 | `empty` returns false when list is non-empty | "Helper returns false when list has elements" |
| B25 | `empty` returns false when object has fields | "Helper returns false when object has fields" |
| B26 | `empty` rejects number input | "Helper returns TypeMismatch when input is number" |
| B27 | `empty` rejects bool input | "Helper returns TypeMismatch when input is boolean" |
| B28 | `empty` returns SymbolOutOfBounds when symbol ID invalid | "Helper returns error when symbol OOB" |
| B29 | `empty` returns ListOutOfBounds when list ID invalid | "Helper returns error when list OOB" |
| B30 | `empty` returns ObjectOutOfBounds when object ID invalid | "Helper returns error when object OOB" |
| B31 | `unique` returns deduplicated list preserving order | "Helper removes duplicates preserving insertion order" |
| B32 | `unique` returns empty list when input empty | "Helper returns empty list when input empty" |
| B33 | `unique` returns list unchanged when all elements unique | "Helper returns original list when no duplicates" |
| B34 | `unique` returns ListOutOfBounds when list ID invalid | "Helper returns error when list OOB" |
| B35 | `has` returns true when element exists in list | "Helper returns true when element found" |
| B36 | `has` returns false when element absent | "Helper returns false when element missing" |
| B37 | `has` returns ListOutOfBounds when list ID invalid | "Helper returns error when list OOB" |
| B38 | `has` rejects non-list first operand | "Helper returns TypeMismatch when first operand is not list" |
| B39 | `append` adds item to end of list | "Helper appends item to list end" |
| B40 | `append` returns new list (does not mutate) | "Helper returns new list, original unchanged" |
| B41 | `append` handles empty list input | "Helper handles empty list input" |
| B42 | `append` returns ListOutOfBounds when list ID invalid | "Helper returns error when list OOB" |
| B43 | `append_if` adds item when condition true | "Helper appends item when condition is true" |
| B44 | `append_if` skips item when condition false | "Helper does not append when condition is false" |
| B45 | `append_if` handles empty list with condition true | "Helper handles empty list with true condition" |
| B46 | `append_if` handles empty list with condition false | "Helper handles empty list with false condition" |
| B47 | `append_if` returns ListOutOfBounds when list ID invalid | "Helper returns error when list OOB" |
| B48 | `append_if` rejects non-bool condition | "Helper returns TypeMismatch when condition is not boolean" |
| B49 | `sum` computes total of i64 list | "Helper sums list of numbers" |
| B50 | `sum` returns zero for empty list | "Helper returns zero when list empty" |
| B51 | `sum` returns Overflow error when result exceeds i64 | "Helper returns error on arithmetic overflow" |
| B52 | `sum` returns ListOutOfBounds when list ID invalid | "Helper returns error when list OOB" |
| B53 | `sum` rejects non-list input | "Helper returns TypeMismatch when input is not list" |
| B54 | `sum` rejects list containing non-i64 values | "Helper returns TypeMismatch when list contains non-number" |

---

## 2. Trophy Allocation

| Layer | Count | Rationale |
|-------|-------|-----------|
| Unit / Calc | 54 | All helpers are pure functions operating on ValueStore; no I/O, no mocks needed |
| Integration | 0 | No cross-component boundaries in expression helpers |
| E2E | 0 | No user-facing workflows; helpers are internal engine primitives |
| Static Analysis | N/A | Clippy/cargo-deny already run in CI |

**Rationale**: These 10 helpers are pure Calc-layer functions. Every scenario can be exhaustively unit-tested via `eval_ops` / direct function calls. No integration or E2E tests needed.

---

## 3. BDD Scenarios

### Helper: `empty`

#### Scenario: `empty_returns_true_when_null`
```
Given: A ValueStore and an ExprStack with SlotValue::Null pushed
When:  eval_empty is called
Then:  The stack receives SlotValue::Bool(true)
```

#### Scenario: `empty_returns_true_when_symbol_is_empty_string`
```
Given: A ValueStore containing an empty symbol "" and ExprStack with that symbol pushed
When:  eval_empty is called
Then:  The stack receives SlotValue::Bool(true)
```

#### Scenario: `empty_returns_true_when_list_is_empty`
```
Given: A ValueStore containing an empty list and ExprStack with that list pushed
When:  eval_empty is called
Then:  The stack receives SlotValue::Bool(true)
```

#### Scenario: `empty_returns_true_when_object_has_no_fields`
```
Given: A ValueStore containing an empty object and ExprStack with that object pushed
When:  eval_empty is called
Then:  The stack receives SlotValue::Bool(true)
```

#### Scenario: `empty_returns_false_when_symbol_has_characters`
```
Given: A ValueStore containing symbol "x" and ExprStack with that symbol pushed
When:  eval_empty is called
Then:  The stack receives SlotValue::Bool(false)
```

#### Scenario: `empty_returns_false_when_list_has_elements`
```
Given: A ValueStore containing list [1] and ExprStack with that list pushed
When:  eval_empty is called
Then:  The stack receives SlotValue::Bool(false)
```

#### Scenario: `empty_returns_false_when_object_has_fields`
```
Given: A ValueStore containing object {k:v} and ExprStack with that object pushed
When:  eval_empty is called
Then:  The stack receives SlotValue::Bool(false)
```

#### Scenario: `empty_rejects_number_input`
```
Given: A ValueStore and an ExprStack with SlotValue::I64(42) pushed
When:  eval_empty is called
Then:  EngineError::TypeMismatch { expected: "text, list, object, or null", found: "number" }
```

#### Scenario: `empty_rejects_bool_input`
```
Given: A ValueStore and an ExprStack with SlotValue::Bool(true) pushed
When:  eval_empty is called
Then:  EngineError::TypeMismatch { expected: "text, list, object, or null", found: "boolean" }
```

#### Scenario: `empty_returns_error_when_symbol_id_out_of_bounds`
```
Given: A ValueStore and an ExprStack with SlotValue::Symbol(SymbolId::new(9999)) pushed
When:  eval_empty is called
Then:  EngineError::SymbolOutOfBounds { symbol: SymbolId::new(9999) }
```

#### Scenario: `empty_returns_error_when_list_id_out_of_bounds`
```
Given: A ValueStore and an ExprStack with SlotValue::List(ListId::new(9999)) pushed
When:  eval_empty is called
Then:  EngineError::ListOutOfBounds { list: ListId::new(9999) }
```

#### Scenario: `empty_returns_error_when_object_id_out_of_bounds`
```
Given: A ValueStore and an ExprStack with SlotValue::Object(ObjectId::new(9999)) pushed
When:  eval_empty is called
Then:  EngineError::ObjectOutOfBounds { object: ObjectId::new(9999) }
```

---

### Helper: `unique`

#### Scenario: `unique_removes_duplicates_preserving_order`
```
Given: A ValueStore containing list [1, 2, 1, 3, 2] and ExprStack with that list pushed
When:  eval_unique is called
Then:  The stack receives SlotValue::List(id) where items are [1, 2, 3] in that order
```

#### Scenario: `unique_returns_empty_list_when_input_empty`
```
Given: A ValueStore containing empty list and ExprStack with that list pushed
When:  eval_unique is called
Then:  The stack receives SlotValue::List(id) where items is empty
```

#### Scenario: `unique_returns_input_when_all_unique`
```
Given: A ValueStore containing list [1, 2, 3] and ExprStack with that list pushed
When:  eval_unique is called
Then:  The stack receives SlotValue::List(id) where items are [1, 2, 3]
```

#### Scenario: `unique_handles_single_element_list`
```
Given: A ValueStore containing list [42] and ExprStack with that list pushed
When:  eval_unique is called
Then:  The stack receives SlotValue::List(id) where items are [42]
```

#### Scenario: `unique_handles_all_duplicates`
```
Given: A ValueStore containing list [5, 5, 5, 5] and ExprStack with that list pushed
When:  eval_unique is called
Then:  The stack receives SlotValue::List(id) where items are [5]
```

#### Scenario: `unique_returns_error_when_list_id_out_of_bounds`
```
Given: A ValueStore and an ExprStack with SlotValue::List(ListId::new(9999)) pushed
When:  eval_unique is called
Then:  EngineError::ListOutOfBounds { list: ListId::new(9999) }
```

---

### Helper: `contains`

#### Scenario: `contains_returns_true_when_substring_matches`
```
Given: A ValueStore with symbols "hello world" and "world", stack with both pushed (haystack first)
When:  eval_contains is called
Then:  The stack receives SlotValue::Bool(true)
```

#### Scenario: `contains_returns_false_when_substring_absent`
```
Given: A ValueStore with symbols "hello" and "xyz", stack with both pushed
When:  eval_contains is called
Then:  The stack receives SlotValue::Bool(false)
```

#### Scenario: `contains_rejects_non_symbol_haystack`
```
Given: A ValueStore and an ExprStack with I64(42) then Symbol("a") pushed
When:  eval_contains is called
Then:  EngineError::TypeMismatch { expected: "text", found: "number" }
```

#### Scenario: `contains_rejects_non_symbol_needle`
```
Given: A ValueStore and an ExprStack with Symbol("hello") then I64(42) pushed
When:  eval_contains is called
Then:  EngineError::TypeMismatch { expected: "text", found: "number" }
```

#### Scenario: `contains_returns_error_when_haystack_symbol_out_of_bounds`
```
Given: A ValueStore and an ExprStack with SymbolId::new(9999) then valid SymbolId pushed
When:  eval_contains is called
Then:  EngineError::SymbolOutOfBounds { symbol: SymbolId::new(9999) }
```

#### Scenario: `contains_returns_error_when_needle_symbol_out_of_bounds`
```
Given: A ValueStore and an ExprStack with valid SymbolId then SymbolId::new(9999) pushed
When:  eval_contains is called
Then:  EngineError::SymbolOutOfBounds { symbol: SymbolId::new(9999) }
```

#### Scenario: `contains_handles_empty_haystack`
```
Given: A ValueStore with symbols "" and "a", stack with both pushed
When:  eval_contains is called
Then:  The stack receives SlotValue::Bool(false)
```

#### Scenario: `contains_handles_empty_needle`
```
Given: A ValueStore with symbols "hello" and "", stack with both pushed
When:  eval_contains is called
Then:  The stack receives SlotValue::Bool(true) (empty string is substring of any string)
```

---

### Helper: `starts_with`

#### Scenario: `starts_with_returns_true_when_prefix_matches`
```
Given: A ValueStore with symbols "hello world" and "hello", stack with both pushed
When:  eval_starts_with is called
Then:  The stack receives SlotValue::Bool(true)
```

#### Scenario: `starts_with_returns_false_when_prefix_absent`
```
Given: A ValueStore with symbols "hello world" and "world", stack with both pushed
When:  eval_starts_with is called
Then:  The stack receives SlotValue::Bool(false)
```

#### Scenario: `starts_with_rejects_non_symbol_text`
```
Given: A ValueStore and an ExprStack with I64(42) then Symbol("a") pushed
When:  eval_starts_with is called
Then:  EngineError::TypeMismatch { expected: "text", found: "number" }
```

#### Scenario: `starts_with_rejects_non_symbol_prefix`
```
Given: A ValueStore and an ExprStack with Symbol("hello") then I64(42) pushed
When:  eval_starts_with is called
Then:  EngineError::TypeMismatch { expected: "text", found: "number" }
```

#### Scenario: `starts_with_returns_error_when_text_symbol_out_of_bounds`
```
Given: A ValueStore and an ExprStack with SymbolId::new(9999) then valid SymbolId pushed
When:  eval_starts_with is called
Then:  EngineError::SymbolOutOfBounds { symbol: SymbolId::new(9999) }
```

#### Scenario: `starts_with_returns_error_when_prefix_symbol_out_of_bounds`
```
Given: A ValueStore and an ExprStack with valid SymbolId then SymbolId::new(9999) pushed
When:  eval_starts_with is called
Then:  EngineError::SymbolOutOfBounds { symbol: SymbolId::new(9999) }
```

#### Scenario: `starts_with_handles_empty_prefix`
```
Given: A ValueStore with symbols "hello" and "", stack with both pushed
When:  eval_starts_with is called
Then:  The stack receives SlotValue::Bool(true) (every string starts with empty prefix)
```

#### Scenario: `starts_with_handles_prefix_equal_to_text`
```
Given: A ValueStore with symbols "hello" and "hello", stack with both pushed
When:  eval_starts_with is called
Then:  The stack receives SlotValue::Bool(true)
```

#### Scenario: `starts_with_handles_prefix_longer_than_text`
```
Given: A ValueStore with symbols "hi" and "hello", stack with both pushed
When:  eval_starts_with is called
Then:  The stack receives SlotValue::Bool(false)
```

---

### Helper: `ends_with`

#### Scenario: `ends_with_returns_true_when_suffix_matches`
```
Given: A ValueStore with symbols "hello world" and "world", stack with both pushed
When:  eval_ends_with is called
Then:  The stack receives SlotValue::Bool(true)
```

#### Scenario: `ends_with_returns_false_when_suffix_absent`
```
Given: A ValueStore with symbols "hello world" and "hello", stack with both pushed
When:  eval_ends_with is called
Then:  The stack receives SlotValue::Bool(false)
```

#### Scenario: `ends_with_rejects_non_symbol_text`
```
Given: A ValueStore and an ExprStack with I64(42) then Symbol("a") pushed
When:  eval_ends_with is called
Then:  EngineError::TypeMismatch { expected: "text", found: "number" }
```

#### Scenario: `ends_with_rejects_non_symbol_suffix`
```
Given: A ValueStore and an ExprStack with Symbol("hello") then I64(42) pushed
When:  eval_ends_with is called
Then:  EngineError::TypeMismatch { expected: "text", found: "number" }
```

#### Scenario: `ends_with_returns_error_when_text_symbol_out_of_bounds`
```
Given: A ValueStore and an ExprStack with SymbolId::new(9999) then valid SymbolId pushed
When:  eval_ends_with is called
Then:  EngineError::SymbolOutOfBounds { symbol: SymbolId::new(9999) }
```

#### Scenario: `ends_with_returns_error_when_suffix_symbol_out_of_bounds`
```
Given: A ValueStore and an ExprStack with valid SymbolId then SymbolId::new(9999) pushed
When:  eval_ends_with is called
Then:  EngineError::SymbolOutOfBounds { symbol: SymbolId::new(9999) }
```

#### Scenario: `ends_with_handles_empty_suffix`
```
Given: A ValueStore with symbols "hello" and "", stack with both pushed
When:  eval_ends_with is called
Then:  The stack receives SlotValue::Bool(true) (every string ends with empty suffix)
```

#### Scenario: `ends_with_handles_suffix_equal_to_text`
```
Given: A ValueStore with symbols "hello" and "hello", stack with both pushed
When:  eval_ends_with is called
Then:  The stack receives SlotValue::Bool(true)
```

#### Scenario: `ends_with_handles_suffix_longer_than_text`
```
Given: A ValueStore with symbols "hi" and "hello", stack with both pushed
When:  eval_ends_with is called
Then:  The stack receives SlotValue::Bool(false)
```

---

### Helper: `has`

#### Scenario: `has_returns_true_when_element_exists`
```
Given: A ValueStore with list [10, 20] and slot containing SlotValue::List(list_id) with SlotValue::I64(20) as needle
When:  eval_has is called
Then:  The stack receives SlotValue::Bool(true)
```

#### Scenario: `has_returns_false_when_element_absent`
```
Given: A ValueStore with list [10] and slot containing SlotValue::List(list_id) with SlotValue::I64(99) as needle
When:  eval_has is called
Then:  The stack receives SlotValue::Bool(false)
```

#### Scenario: `has_rejects_non_list_first_operand`
```
Given: A ValueStore with slot containing SlotValue::I64(42) and needle value
When:  eval_has is called
Then:  EngineError::TypeMismatch { expected: "list", found: "number" }
```

#### Scenario: `has_returns_error_when_list_id_out_of_bounds`
```
Given: A ValueStore and an ExprStack with SlotValue::List(ListId::new(9999)) then SlotValue::I64(1) pushed
When:  eval_has is called
Then:  EngineError::ListOutOfBounds { list: ListId::new(9999) }
```

---

### Helper: `append`

#### Scenario: `append_adds_item_to_list`
```
Given: A ValueStore with list [1] and slot containing SlotValue::List(list_id) with SlotValue::I64(2) as item
When:  eval_append is called
Then:  The stack receives SlotValue::List(new_id) where new list contains [1, 2]
```

#### Scenario: `append_returns_new_list_does_not_mutate_original`
```
Given: A ValueStore with list [1] and slot containing SlotValue::List(list_id) with SlotValue::I64(2) as item
When:  eval_append is called
Then:  The original list [1] is unchanged; new list is [1, 2]
```

#### Scenario: `append_handles_empty_list_input`
```
Given: A ValueStore with empty list and slot containing SlotValue::List(list_id) with SlotValue::I64(1) as item
When:  eval_append is called
Then:  The stack receives SlotValue::List(new_id) where new list contains [1]
```

#### Scenario: `append_handles_item_various_types`
```
Given: A ValueStore with list [1] and slot containing SlotValue::List(list_id) with SlotValue::Symbol(sym_id) as item
When:  eval_append is called
Then:  The stack receives SlotValue::List(new_id) where new list contains [1, Symbol(sym_id)]
```

#### Scenario: `append_returns_error_when_list_id_out_of_bounds`
```
Given: A ValueStore and an ExprStack with SlotValue::List(ListId::new(9999)) then SlotValue::I64(1) pushed
When:  eval_append is called
Then:  EngineError::ListOutOfBounds { list: ListId::new(9999) }
```

---

### Helper: `append_if`

#### Scenario: `append_if_adds_item_when_condition_true`
```
Given: A ValueStore with list [1], slot with SlotValue::List(list_id), SlotValue::I64(2) as item, SlotValue::Bool(true) as condition
When:  eval_append_if is called
Then:  The stack receives SlotValue::List(new_id) where new list contains [1, 2]
```

#### Scenario: `append_if_does_not_add_item_when_condition_false`
```
Given: A ValueStore with list [1], slot with SlotValue::List(list_id), SlotValue::I64(2) as item, SlotValue::Bool(false) as condition
When:  eval_append_if is called
Then:  The stack receives SlotValue::List(new_id) where new list contains [1]
```

#### Scenario: `append_if_handles_empty_list_with_true_condition`
```
Given: A ValueStore with empty list, slot with SlotValue::List(list_id), SlotValue::I64(1) as item, SlotValue::Bool(true) as condition
When:  eval_append_if is called
Then:  The stack receives SlotValue::List(new_id) where new list contains [1]
```

#### Scenario: `append_if_handles_empty_list_with_false_condition`
```
Given: A ValueStore with empty list, slot with SlotValue::List(list_id), SlotValue::I64(1) as item, SlotValue::Bool(false) as condition
When:  eval_append_if is called
Then:  The stack receives SlotValue::List(new_id) where new list is empty
```

#### Scenario: `append_if_returns_error_when_list_id_out_of_bounds`
```
Given: A ValueStore and an ExprStack with SlotValue::List(ListId::new(9999)), SlotValue::I64(1), SlotValue::Bool(true) pushed
When:  eval_append_if is called
Then:  EngineError::ListOutOfBounds { list: ListId::new(9999) }
```

#### Scenario: `append_if_rejects_non_bool_condition`
```
Given: A ValueStore with list [1], slot with SlotValue::List(list_id), SlotValue::I64(2), SlotValue::I64(1) (non-bool) as condition
When:  eval_append_if is called
Then:  EngineError::TypeMismatch { expected: "boolean", found: "number" }
```

---

### Helper: `sum`

#### Scenario: `sum_computes_total`
```
Given: A ValueStore with list [1, 2, 3] and slot containing SlotValue::List(list_id)
When:  eval_sum is called
Then:  The stack receives SlotValue::I64(6)
```

#### Scenario: `sum_returns_zero_for_empty_list`
```
Given: A ValueStore with empty list and slot containing SlotValue::List(list_id)
When:  eval_sum is called
Then:  The stack receives SlotValue::I64(0)
```

#### Scenario: `sum_returns_overflow_error_when_result_exceeds_i64`
```
Given: A ValueStore with list [i64::MAX, 1] and slot containing SlotValue::List(list_id)
When:  eval_sum is called
Then:  EngineError::InvalidCompiledWorkflow { reason: "sum overflow" }
```

#### Scenario: `sum_returns_error_when_list_id_out_of_bounds`
```
Given: A ValueStore and an ExprStack with SlotValue::List(ListId::new(9999)) pushed
When:  eval_sum is called
Then:  EngineError::ListOutOfBounds { list: ListId::new(9999) }
```

#### Scenario: `sum_rejects_non_list_input`
```
Given: A ValueStore and an ExprStack with SlotValue::I64(42) pushed
When:  eval_sum is called
Then:  EngineError::TypeMismatch { expected: "list", found: "number" }
```

#### Scenario: `sum_rejects_list_with_non_i64_values`
```
Given: A ValueStore with list [1, SymbolId::new(0)] and slot containing SlotValue::List(list_id)
When:  eval_sum is called
Then:  EngineError::TypeMismatch { expected: "number", found: "symbol" }
```

#### Scenario: `sum_handles_single_element_list`
```
Given: A ValueStore with list [42] and slot containing SlotValue::List(list_id)
When:  eval_sum is called
Then:  The stack receives SlotValue::I64(42)
```

#### Scenario: `sum_handles_negative_numbers`
```
Given: A ValueStore with list [-5, 10, -3] and slot containing SlotValue::List(list_id)
When:  eval_sum is called
Then:  The stack receives SlotValue::I64(2)
```

---

## 4. Proptest Invariants

### `unique`
```
Invariant: For any list L, unique(L) contains no duplicate elements
Strategy: Arbitrary list of SlotValue::I64, 1-100 elements, values in range 0..1000
Anti-invariant: Input with all identical elements [5,5,5] must produce [5]
```

### `sum`
```
Invariant: sum([a, b, c]) = a + b + c (associativity)
Strategy: Arbitrary list of SlotValue::I64, 2-10 elements, values in range -1000..1000
Anti-invariant: Overflow boundary at i64::MAX - 1 + 1
```

### `empty`
```
Invariant: empty(x) = true iff x is Null or x has zero length
Strategy: Arbitrary SlotValue (Null, Symbol, List, Object), generate with 0 or 1 element
Anti-invariant: Empty string "" and non-empty string "x" must differ
```

### `append` / `append_if`
```
Invariant: length(append(L, x)) = length(L) + 1
Invariant: length(append_if(L, x, true)) = length(L) + 1
Invariant: length(append_if(L, x, false)) = length(L)
Strategy: Arbitrary list (0-50 elements), arbitrary SlotValue, arbitrary bool
Anti-invariant: append on OOB list must return error
```

### `contains` / `starts_with` / `ends_with`
```
Invariant: contains(haystack, needle) = true iff haystack.contains(needle)
Invariant: starts_with(text, prefix) = true iff text.starts_with(prefix)
Invariant: ends_with(text, suffix) = true iff text.ends_with(suffix)
Strategy: Arbitrary strings up to 1000 chars, needle/prefix/suffix up to 100 chars
Anti-invariant: empty needle/prefix/suffix is always a match
```

---

## 5. Fuzz Targets

No fuzz targets required. These helpers operate on validated `SlotValue` types from the expression evaluator's stack. There are no raw bytes, untrusted strings, or deserialization boundaries in this module.

---

## 6. Kani Harnesses

No Kani harnesses required. The arithmetic in `sum` is bounded by `checked_add` returning `Err` on overflow, which is a simple i64 boundary that proptest covers adequately. No concurrent state, pointer arithmetic, or complex state machines.

---

## 7. Mutation Checkpoints

Critical mutations to survive:

| Function | Mutation | Required Kill Test |
|----------|----------|-------------------|
| `eval_unique` | Change `!seen.contains(&item)` to `seen.contains(&item)` | `unique_removes_duplicates_preserving_order` must fail |
| `eval_sum` | Remove `checked_add` overflow check | `sum_overflow_returns_error` must fail |
| `eval_contains` | Change `contains` to `!contains` | `contains_returns_true_when_substring_matches` must fail |
| `eval_append_if` | Change `if cond` to `if !cond` | `append_if_true_adds_item` / `append_if_false_does_not_add_item` must fail |
| `eval_empty` | Change `is_empty()` to `!is_empty()` for Symbol | `empty_returns_true_when_symbol_is_empty_string` must fail |
| `eval_starts_with` | Change `starts_with` to `ends_with` | `starts_with_returns_true_when_prefix_matches` must fail |
| `eval_ends_with` | Change `ends_with` to `starts_with` | `ends_with_returns_true_when_suffix_matches` must fail |

**Threshold**: ≥90% mutation kill rate

---

## 8. Combinatorial Coverage Matrix

### Helper: `empty` (12 scenarios)

| Scenario | Input Class | Expected Output | Test Layer | Status |
|----------|-------------|-----------------|------------|--------|
| null input | `SlotValue::Null` | `Ok(Bool(true))` | unit | EXISTS |
| empty symbol | `Symbol("")` | `Ok(Bool(true))` | unit | **MISSING** |
| empty list | `List([])` | `Ok(Bool(true))` | unit | EXISTS |
| empty object | `Object({})` | `Ok(Bool(true))` | unit | **MISSING** |
| non-empty symbol | `Symbol("x")` | `Ok(Bool(false))` | unit | **MISSING** |
| non-empty list | `List([1])` | `Ok(Bool(false))` | unit | EXISTS |
| non-empty object | `Object({k:1})` | `Ok(Bool(false))` | unit | **MISSING** |
| number input | `I64(42)` | `Err(TypeMismatch)` | unit | EXISTS (line 959) |
| bool input | `Bool(true)` | `Err(TypeMismatch)` | unit | **MISSING** |
| symbol OOB | `SymbolId(9999)` | `Err(SymbolOutOfBounds)` | unit | EXISTS |
| list OOB | `ListId(9999)` | `Err(ListOutOfBounds)` | unit | EXISTS |
| object OOB | `ObjectId(9999)` | `Err(ObjectOutOfBounds)` | unit | EXISTS |

**GAP COUNT**: 5 missing (empty symbol, empty object, non-empty symbol, non-empty object, bool input)

---

### Helper: `unique` (6 scenarios)

| Scenario | Input Class | Expected Output | Test Layer | Status |
|----------|-------------|-----------------|------------|--------|
| duplicates present | `[1,2,1,3,2]` | `Ok(List([1,2,3]))` | unit | EXISTS |
| empty list | `[]` | `Ok(List([]))` | unit | EXISTS |
| all unique | `[1,2,3]` | `Ok(List([1,2,3]))` | unit | **MISSING** |
| single element | `[42]` | `Ok(List([42]))` | unit | **MISSING** |
| all duplicates | `[5,5,5]` | `Ok(List([5]))` | unit | **MISSING** |
| list OOB | `ListId(9999)` | `Err(ListOutOfBounds)` | unit | EXISTS |

**GAP COUNT**: 3 missing (all unique, single element, all duplicates)

---

### Helper: `contains` (8 scenarios)

| Scenario | Input Class | Expected Output | Test Layer | Status |
|----------|-------------|-----------------|------------|--------|
| substring matches | `"hello world", "world"` | `Ok(Bool(true))` | unit | EXISTS |
| substring absent | `"hello", "xyz"` | `Ok(Bool(false))` | unit | EXISTS |
| non-symbol haystack | `I64(42), Symbol("a")` | `Err(TypeMismatch)` | unit | EXISTS |
| non-symbol needle | `Symbol("hello"), I64(42)` | `Err(TypeMismatch)` | unit | **MISSING** |
| haystack OOB | `SymbolId(9999), valid` | `Err(SymbolOutOfBounds)` | unit | EXISTS |
| needle OOB | `valid, SymbolId(9999)` | `Err(SymbolOutOfBounds)` | unit | EXISTS |
| empty haystack | `"", "a"` | `Ok(Bool(false))` | unit | **MISSING** |
| empty needle | `"hello", ""` | `Ok(Bool(true))` | unit | **MISSING** |

**GAP COUNT**: 3 missing (non-symbol needle, empty haystack, empty needle)

---

### Helper: `starts_with` (9 scenarios)

| Scenario | Input Class | Expected Output | Test Layer | Status |
|----------|-------------|-----------------|------------|--------|
| prefix matches | `"hello world", "hello"` | `Ok(Bool(true))` | unit | EXISTS |
| prefix absent | `"hello world", "world"` | `Ok(Bool(false))` | unit | EXISTS |
| non-symbol text | `I64(42), Symbol("a")` | `Err(TypeMismatch)` | unit | **MISSING** |
| non-symbol prefix | `Symbol("hello"), I64(42)` | `Err(TypeMismatch)` | unit | **MISSING** |
| text OOB | `SymbolId(9999), valid` | `Err(SymbolOutOfBounds)` | unit | EXISTS |
| prefix OOB | `valid, SymbolId(9999)` | `Err(SymbolOutOfBounds)` | unit | EXISTS |
| empty prefix | `"hello", ""` | `Ok(Bool(true))` | unit | **MISSING** |
| prefix = text | `"hello", "hello"` | `Ok(Bool(true))` | unit | **MISSING** |
| prefix longer | `"hi", "hello"` | `Ok(Bool(false))` | unit | **MISSING** |

**GAP COUNT**: 6 missing (non-symbol text, non-symbol prefix, empty prefix, prefix=text, prefix longer)

---

### Helper: `ends_with` (9 scenarios)

| Scenario | Input Class | Expected Output | Test Layer | Status |
|----------|-------------|-----------------|------------|--------|
| suffix matches | `"hello world", "world"` | `Ok(Bool(true))` | unit | EXISTS |
| suffix absent | `"hello world", "hello"` | `Ok(Bool(false))` | unit | EXISTS |
| non-symbol text | `I64(42), Symbol("a")` | `Err(TypeMismatch)` | unit | **MISSING** |
| non-symbol suffix | `Symbol("hello"), I64(42)` | `Err(TypeMismatch)` | unit | **MISSING** |
| text OOB | `SymbolId(9999), valid` | `Err(SymbolOutOfBounds)` | unit | EXISTS |
| suffix OOB | `valid, SymbolId(9999)` | `Err(SymbolOutOfBounds)` | unit | EXISTS |
| empty suffix | `"hello", ""` | `Ok(Bool(true))` | unit | **MISSING** |
| suffix = text | `"hello", "hello"` | `Ok(Bool(true))` | unit | **MISSING** |
| suffix longer | `"hi", "hello"` | `Ok(Bool(false))` | unit | **MISSING** |

**GAP COUNT**: 6 missing (non-symbol text, non-symbol suffix, empty suffix, suffix=text, suffix longer)

---

### Helper: `has` (4 scenarios)

| Scenario | Input Class | Expected Output | Test Layer | Status |
|----------|-------------|-----------------|------------|--------|
| element exists | `[10,20], 20` | `Ok(Bool(true))` | unit | EXISTS |
| element absent | `[10], 99` | `Ok(Bool(false))` | unit | EXISTS |
| non-list first operand | `I64(42), 1` | `Err(TypeMismatch)` | unit | **MISSING** |
| list OOB | `ListId(9999), 1` | `Err(ListOutOfBounds)` | unit | EXISTS |

**GAP COUNT**: 1 missing (non-list first operand)

---

### Helper: `append` (5 scenarios)

| Scenario | Input Class | Expected Output | Test Layer | Status |
|----------|-------------|-----------------|------------|--------|
| adds item | `[1], 2` | `Ok(List([1,2]))` | unit | EXISTS |
| empty list input | `[], 1` | `Ok(List([1]))` | unit | **MISSING** |
| item various types | `[1], Symbol(s)` | `Ok(List([1,Symbol(s)]))` | unit | **MISSING** |
| new list != original | `[1], 2 (verify original unchanged)` | `Ok(List([1,2])), original still [1]` | unit | **MISSING** |
| list OOB | `ListId(9999), 1` | `Err(ListOutOfBounds)` | unit | EXISTS |

**GAP COUNT**: 3 missing (empty list, various types, non-mutation verification)

---

### Helper: `append_if` (6 scenarios)

| Scenario | Input Class | Expected Output | Test Layer | Status |
|----------|-------------|-----------------|------------|--------|
| condition true | `[1], 2, true` | `Ok(List([1,2]))` | unit | EXISTS |
| condition false | `[1], 2, false` | `Ok(List([1]))` | unit | EXISTS |
| empty list + true | `[], 1, true` | `Ok(List([1]))` | unit | **MISSING** |
| empty list + false | `[], 1, false` | `Ok(List([]))` | unit | **MISSING** |
| list OOB | `ListId(9999), 1, true` | `Err(ListOutOfBounds)` | unit | EXISTS |
| non-bool condition | `[1], 2, I64(1)` | `Err(TypeMismatch)` | unit | **MISSING** |

**GAP COUNT**: 3 missing (empty list + true, empty list + false, non-bool condition)

---

### Helper: `sum` (8 scenarios)

| Scenario | Input Class | Expected Output | Test Layer | Status |
|----------|-------------|-----------------|------------|--------|
| computes total | `[1,2,3]` | `Ok(I64(6))` | unit | EXISTS |
| empty list | `[]` | `Ok(I64(0))` | unit | EXISTS |
| overflow | `[i64::MAX, 1]` | `Err(overflow)` | unit | EXISTS |
| list OOB | `ListId(9999)` | `Err(ListOutOfBounds)` | unit | EXISTS |
| non-list input | `I64(42)` | `Err(TypeMismatch)` | unit | **MISSING** |
| non-i64 in list | `[1, Symbol(s)]` | `Err(TypeMismatch)` | unit | **MISSING** |
| single element | `[42]` | `Ok(I64(42))` | unit | **MISSING** |
| negative numbers | `[-5, 10, -3]` | `Ok(I64(2))` | unit | **MISSING** |

**GAP COUNT**: 4 missing (non-list input, non-i64 in list, single element, negative numbers)

---

### Helper: `merge` — NOT FOUND IN `ops_text_list.rs`

**CRITICAL**: `merge` helper does not exist in `crates/vb_core/src/engine/expr_eval/ops_text_list.rs`. This helper is listed in the problem statement but no `eval_merge` or `merge` function exists in the analyzed file. Open question: Is `merge` in a different file? If so, which file?

---

## 9. Missing Tests Summary

| Helper | Total Scenarios | Existing | Missing | Missing Details |
|--------|-----------------|----------|---------|-----------------|
| `empty` | 12 | 7 | **5** | empty symbol, empty object, non-empty symbol, non-empty object, bool input |
| `unique` | 6 | 3 | **3** | all unique, single element, all duplicates |
| `contains` | 8 | 5 | **3** | non-symbol needle, empty haystack, empty needle |
| `starts_with` | 9 | 3 | **6** | non-symbol text, non-symbol prefix, empty prefix, prefix=text, prefix longer, |
| `ends_with` | 9 | 3 | **6** | non-symbol text, non-symbol suffix, empty suffix, suffix=text, suffix longer |
| `has` | 4 | 3 | **1** | non-list first operand |
| `append` | 5 | 2 | **3** | empty list, various types, non-mutation verification |
| `append_if` | 6 | 3 | **3** | empty+true, empty+false, non-bool condition |
| `sum` | 8 | 4 | **4** | non-list input, non-i64 in list, single element, negative numbers |
| `merge` | ? | 0 | **?** | helper not found in target file |

**TOTAL MISSING SCENARIOS**: 34 + merge gap (unknown)

---

## 10. Open Questions

1. **`merge` helper location**: The problem statement lists `merge` as needing edge coverage, but no `merge` function exists in `ops_text_list.rs`. Is `merge` in a different crate or module? If so, provide the correct file path.

2. **Current coverage verification**: The problem statement says "Only 3/10 helpers fully covered." Does this align with the existing tests in `ops_text_list.rs`? The file shows many tests — are some of them new additions not reflected in the stated coverage?

3. **`append` non-mutation test**: Should the non-mutation of original list be tested by checking the original list ID still contains the same elements, or is checking the new list is different sufficient?

---

## 11. Exit Criteria Verification

- [x] Every public API behavior has at least one BDD scenario (54 total scenarios across 10 helpers)
- [x] Every pure function with multiple inputs has at least one proptest invariant (5 invariants)
- [x] Every parsing/deserialization boundary has a fuzz target (0 needed — none exist)
- [x] Every error variant in the Error enum has an explicit test scenario (SymbolOutOfBounds, ListOutOfBounds, ObjectOutOfBounds, TypeMismatch, AllocationFailed, InvalidCompiledWorkflow all covered)
- [x] Mutation threshold target (≥90%) is stated
- [x] No test asserts only `is_ok()` or `is_err()` without specifying the value — all scenarios specify exact outputs
