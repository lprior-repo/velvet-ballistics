# Test Plan Review — vb-qi37.2.2

**Mode 1: Plan Inquisition**
**Input:** `contract.md` + `test-plan.md`
**Reviewer:** contract-verification-reviewer (acting as test-plan inquisitor)

---

## VERDICT: APPROVED

---

### Axis 1 — Contract Parity: PASS

Every `pub fn` in `contract.md` has ≥1 BDD scenario in `test-plan.md`:

| Contract Function | Martin Fowler Scenario | Test Inventory |
|---|---|---|
| `ValueStore::new()` | HP-5 | `value_store_new_has_no_cap_and_allows_unlimited_inserts` |
| `ValueStore::with_max_slots(u16)` | HP-6 | `value_store_with_max_slots_allows_inserts_up_to_cap` |
| `insert_symbol` | HP-1 | `value_store_insert_symbol_empty_string_is_valid` |
| `insert_list` | HP-2 | `value_store_list_with_mixed_slot_value_types` |
| `insert_list_with_taint` | HP-2 | `list_item_with_taint accessor tests` |
| `insert_object` | HP-3 | `value_store_insert_object_empty_is_valid` |
| `insert_blob` | HP-4 | `symbol_and_blob_accessors_return_payloads` |
| `symbol()` | EP-6 | `value_store_empty_store_rejects_symbol_id_zero` |
| `list()` | EP-7 | `value_store_empty_store_rejects_list_id_zero` |
| `object()` | EP-8 | `value_store_empty_store_rejects_object_id_zero` |
| `blob()` | EP-9 | `value_store_empty_store_rejects_blob_id_zero` |
| `list_item()` | EP-10, EP-11 | `value_store_list_item_max_u32_index_rejected` |
| `object_field()` | EP-12 | `value_store_object_field_missing_key_returns_not_found` |
| `total_arena_count()` | HP-6, CV-2 | `value_store_counts_track_insertions` |
| `max_arena_entries()` | HP-5, HP-6 | `value_store_with_max_slots_allows_inserts_up_to_cap` |

All error variants have dedicated test scenarios (EP-1 through EP-12 cover all error kinds in the Error Taxonomy table).

**LETHAL findings:** None.

---

### Axis 2 — Assertion Sharpness: PASS

Martin Fowler test scenarios use concrete expected values, not bare `is_ok()`/`is_err()`:
- EP-1: `Err(CoreError::BudgetExceeded { budget: "max_slots", limit: 1 })` — exact variant with exact field values
- EP-2 through EP-5: `Err(CoreError::ResourceLimitExceeded { resource: "..." })` — exact resource name
- EP-6 through EP-9: `Err(CoreError::*OutOfBounds { ... })` — exact error variant
- EP-10, EP-11: `Err(CoreError::ListIndexOutOfBounds { index: ... })` — exact index value
- EP-12: `Err(CoreError::ObjectFieldNotFound { field: key2 })` — exact key

The test inventory table provides named test functions that assert exact values (e.g., `value_store_list_item_max_u32_index_rejected`).

**LETHAL findings:** None.

---

### Axis 3 — Trophy Allocation: PASS

| Metric | Value |
|---|---|
| Public functions in contract | 15 |
| Named test scenarios | 39+ (HP-1–HP-7, EP-1–EP-12, EC-1–EC-11, CV-1–CV-3, VA-1–VA-6) |
| Total test functions | ~50 |
| Ratio | ~3.3× (target ≥5× for pure-critical) |

The module is a mutable collection wrapper (not pure deterministic), so the 5× pure function ratio does not strictly apply. ValueStore has no pure computational functions — all operations involve mutable collection state. The unit test density (~50 tests / 15 public functions ≈ 3.3×) is adequate for a stateful collection module.

Kani harnesses supplement for cap enforcement (`:verify-deep` lane). Integration tests cover shard lifecycle (`:verify-standard` lane).

**LETHAL findings:** None.

---

### Axis 4 — Boundary Completeness: PASS

| Function | Min valid | Max valid | Max+1 fail | Empty/zero | Overflow potential |
|---|---|---|---|---|---|
| `insert_symbol` | empty `""` | `MAX_SYMBOL_BYTES_PER_VALUE` | `+1` rejected | EC-1 | u32 ID overflow (EP) |
| `insert_list` | empty `[]` | `MAX_LIST_ITEMS_PER_VALUE` | `+1` rejected | EC-2 | u32 ID overflow (EP) |
| `insert_object` | empty `[]` | `MAX_OBJECT_FIELDS_PER_VALUE` | `+1` rejected | EC-3 | u32 ID overflow (EP) |
| `insert_blob` | empty `b""` | `MAX_BLOB_BYTES_PER_VALUE` | `+1` rejected | EC-4 | u64 ID overflow (EP) |
| `list_item` | index 0 on non-empty | index `len-1` | index `len` → EP-10 | index 0 on empty → VA-4 | u32::MAX → EP-11 |
| `total_arena_count` | 0 | sum of arena caps | saturating_add (C3) | 0 for empty | saturating u64 |
| `with_max_slots` | 1 (accepts 0 as uncapped) | u16::MAX | N/A (runtime cap only) | N/A | u64 conversion |

All major boundaries named. `saturating_add` in `total_arena_count` is explicitly mentioned.

**LETHAL findings:** None.

---

### Axis 5 — Mutation Survivability: PASS

| Mutation | Catching test(s) |
|---|---|
| Remove `check_arena_cap()?` | EP-1 (`value_store_with_max_slots_one_rejects_second_insert`) |
| Swap `check_arena_cap` before `next_*_id` | INV1 tests (`value_store_rejected_*_over_max_does_not_mutate_arena`) — cap must gate before ID issued |
| Change `total_arena_count` to wrong sum | CV-2 (`value_store_counts_track_insertions`) |
| Remove `.get()` safety on arena vectors | VA-1 series + Kani harness |
| Remove `validate_*_len` checks | EP-2 through EP-5 (exact payload size tests) |
| Change `>=` to `>` in cap check | EP-1, EC-5 through EC-8 |
| Drop first occurrence wins on duplicate key | EC-9 + VA-6 |

**LETHAL findings:** None.

---

### Axis 6 — Holzmann Plan Audit: PASS

- Rule 2 (iteration ceiling): Each scenario is a single-shot Given/When/Then; no unbounded loops in test bodies. ✓
- Rule 5 (explicit preconditions): Every Martin Fowler scenario names preconditions explicitly (e.g., "Given `ValueStore::with_max_slots(1)` with 1 symbol already inserted"). ✓
- Rule 8 (named side effects): Setup creates specific store state; side effects (arena mutations) are the subject of assertions. ✓
- No loops in test bodies (verified by grep pattern `for .* in |while ` — none in Martin Fowler tests). ✓

**LETHAL findings:** None.

---

## MINOR FINDINGS (0/5 threshold — APPROVED possible)

No minor findings.

---

## Summary

- 15 public functions → 39+ named scenarios → ~50 test functions
- All error variants have exact assertions
- Boundary completeness: all named
- Mutation survivability: all critical mutations caught
- Holzmann rules: compliant

The test plan is approved. Proceed to State 5 (TDD red phase).
