# Martin Fowler Test Plan: vb-qi37.2.2 — ValueStore Arena Cap Enforcement

## Test Taxonomy

Per Martin Fowler's testing philosophy, this plan covers:
1. **Happy path tests** — normal insertion and access
2. **Error path tests** — cap exceeded, bounds violations, malformed input
3. **Edge case tests** — empty values, exact limits, duplicate keys
4. **Contract verification tests** — verify specific contract clauses
5. **Violation tests** — adversarial inputs designed to break invariants

---

## Happy Path Tests

### HP-1: Symbol round-trip

**Given** an empty `ValueStore`
**When** I insert a symbol `"hello"`
**Then** the returned `SymbolId` resolves back to `"hello"`

```
cargo test -p vb_core -- value_store_insert_symbol_empty_string_is_valid
cargo test -p vb_core -- symbol_and_blob_accessors_return_payloads
```

---

### HP-2: List round-trip with mixed types

**Given** an empty `ValueStore`
**When** I insert a list `[Null, Bool(true), I64(-42), List(ListId(99))]`
**Then** each index resolves to the correct value

```
cargo test -p vb_core -- value_store_list_with_mixed_slot_value_types
```

---

### HP-3: Object field lookup

**Given** a `ValueStore` with an object `{key1: Bool(true), key2: I64(42)}`
**When** I query `object_field(obj_id, key1)` and `object_field(obj_id, key2)`
**Then** I receive `Bool(true)` and `I64(42)` respectively

```
cargo test -p vb_core -- list_item_and_object_field_accessors_are_checked
```

---

### HP-4: Blob round-trip

**Given** an empty `ValueStore`
**When** I insert blob `b"deadbeef"`
**Then** the returned `BlobId` resolves to `b"deadbeef"`

```
cargo test -p vb_core -- symbol_and_blob_accessors_return_payloads
```

---

### HP-5: Uncapped store allows unlimited inserts

**Given** `ValueStore::new()` (uncapped)
**When** I insert 100 symbols
**Then** all 100 insert operations succeed and `total_arena_count() == 100`

```
cargo test -p vb_core -- value_store_new_has_no_cap_and_allows_unlimited_inserts
```

---

### HP-6: Capped store allows inserts up to cap

**Given** `ValueStore::with_max_slots(3)`
**When** I insert exactly 3 entries (1 symbol + 1 list + 1 blob)
**Then** all 3 succeed and `total_arena_count() == 3`

```
cargo test -p vb_core -- value_store_with_max_slots_allows_inserts_up_to_cap
```

---

### HP-7: Monotonic IDs across all arenas

**Given** an empty `ValueStore`
**When** I insert symbols, lists, objects, and blobs sequentially
**Then** IDs within each arena are monotonically increasing (0, 1, 2, ...)

```
cargo test -p vb_core -- value_store_sequential_ids_are_monotonic
```

---

## Error Path Tests

### EP-1: Arena cap exceeded rejects insert

**Given** `ValueStore::with_max_slots(1)` with 1 symbol already inserted
**When** I attempt to insert a second symbol
**Then** I receive `Err(CoreError::BudgetExceeded { budget: "max_slots", limit: 1 })`
**And** the store state is unchanged

```
cargo test -p vb_core -- value_store_with_max_slots_one_rejects_second_insert
```

---

### EP-2: Oversized symbol rejected

**Given** an empty `ValueStore`
**When** I insert a symbol with `MAX_SYMBOL_BYTES_PER_VALUE + 1` bytes
**Then** I receive `Err(CoreError::ResourceLimitExceeded { resource: "symbol_bytes" })`
**And** `symbol_count() == 0`

```
cargo test -p vb_core -- insert_symbol_rejects_payload_over_hard_bound
```

---

### EP-3: Oversized list rejected

**Given** an empty `ValueStore`
**When** I insert a list with `MAX_LIST_ITEMS_PER_VALUE + 1` items
**Then** I receive `Err(CoreError::ResourceLimitExceeded { resource: "list_items" })`
**And** `list_count() == 0`

```
cargo test -p vb_core -- insert_list_rejects_payload_over_hard_bound
```

---

### EP-4: Oversized object rejected

**Given** an empty `ValueStore`
**When** I insert an object with `MAX_OBJECT_FIELDS_PER_VALUE + 1` fields
**Then** I receive `Err(CoreError::ResourceLimitExceeded { resource: "object_fields" })`
**And** `object_count() == 0`

```
cargo test -p vb_core -- insert_object_rejects_payload_over_hard_bound
```

---

### EP-5: Oversized blob rejected

**Given** an empty `ValueStore`
**When** I insert a blob with `MAX_BLOB_BYTES_PER_VALUE + 1` bytes
**Then** I receive `Err(CoreError::ResourceLimitExceeded { resource: "blob_bytes" })`
**And** `blob_count() == 0`

```
cargo test -p vb_core -- insert_blob_rejects_payload_over_hard_bound
```

---

### EP-6: Invalid symbol ID returns out-of-bounds

**Given** an empty `ValueStore`
**When** I call `store.symbol(SymbolId::new(0))`
**Then** I receive `Err(CoreError::SymbolOutOfBounds { symbol: SymbolId::new(0) })`

```
cargo test -p vb_core -- value_store_empty_store_rejects_symbol_id_zero
```

---

### EP-7: Invalid list ID returns out-of-bounds

**Given** an empty `ValueStore`
**When** I call `store.list(ListId::new(0))`
**Then** I receive `Err(CoreError::ListOutOfBounds { list: ListId::new(0) })`

```
cargo test -p vb_core -- value_store_empty_store_rejects_list_id_zero
```

---

### EP-8: Invalid object ID returns out-of-bounds

**Given** an empty `ValueStore`
**When** I call `store.object(ObjectId::new(0))`
**Then** I receive `Err(CoreError::ObjectOutOfBounds { object: ObjectId::new(0) })`

```
cargo test -p vb_core -- value_store_empty_store_rejects_object_id_zero
```

---

### EP-9: Invalid blob ID returns out-of-bounds

**Given** an empty `ValueStore`
**When** I call `store.blob(BlobId::new(0))`
**Then** I receive `Err(CoreError::BlobOutOfBounds { blob: BlobId::new(0) })`

```
cargo test -p vb_core -- value_store_empty_store_rejects_blob_id_zero
```

---

### EP-10: List index out-of-bounds

**Given** a `ValueStore` with a list of length 3
**When** I call `store.list_item(list_id, 3)`
**Then** I receive `Err(CoreError::ListIndexOutOfBounds { index: 3 })`

```
cargo test -p vb_core -- value_store_list_index_at_exact_length_is_rejected
```

---

### EP-11: u32::MAX list index rejected

**Given** a `ValueStore` with any list
**When** I call `store.list_item(list_id, u32::MAX)`
**Then** I receive `Err(CoreError::ListIndexOutOfBounds { index: u32::MAX })`

```
cargo test -p vb_core -- value_store_list_item_max_u32_index_rejected
```

---

### EP-12: Object field not found

**Given** a `ValueStore` with an object that has field `key1`
**When** I call `store.object_field(obj_id, key2)` where `key2` is absent
**Then** I receive `Err(CoreError::ObjectFieldNotFound { field: key2 })`

```
cargo test -p vb_core -- value_store_object_field_missing_key_returns_not_found
```

---

## Edge Case Tests

### EC-1: Empty symbol is valid

**Given** an empty `ValueStore`
**When** I insert symbol `""`
**Then** I receive a valid `SymbolId`
**And** `store.symbol(id)` returns `""`

```
cargo test -p vb_core -- value_store_insert_symbol_empty_string_is_valid
```

---

### EC-2: Empty list is valid

**Given** an empty `ValueStore`
**When** I insert list `[]`
**Then** I receive a valid `ListId`
**And** `store.list(id)` returns `&[]`

```
cargo test -p vb_core -- value_store_insert_list_empty_is_valid
```

---

### EC-3: Empty object is valid

**Given** an empty `ValueStore`
**When** I insert object `[]`
**Then** I receive a valid `ObjectId`
**And** `store.object(id)` returns `&[]`

```
cargo test -p vb_core -- value_store_insert_object_empty_is_valid
```

---

### EC-4: Empty blob is valid

**Given** an empty `ValueStore`
**When** I insert blob `b""`
**Then** I receive a valid `BlobId`
**And** `store.blob(id)` returns `&[]`

```
cargo test -p vb_core -- value_store_insert_blob_empty_is_valid
```

---

### EC-5: Symbol at exact MAX length accepted

**Given** an empty `ValueStore`
**When** I insert a symbol of exactly `MAX_SYMBOL_BYTES_PER_VALUE` bytes
**Then** the insert succeeds

```
cargo test -p vb_core -- value_store_symbol_at_exact_max_length_is_accepted
```

---

### EC-6: List at exact MAX length accepted

**Given** an empty `ValueStore`
**When** I insert a list of exactly `MAX_LIST_ITEMS_PER_VALUE` items
**Then** the insert succeeds

```
cargo test -p vb_core -- value_store_list_at_exact_max_length_is_accepted
```

---

### EC-7: Object at exact MAX fields accepted

**Given** an empty `ValueStore`
**When** I insert an object of exactly `MAX_OBJECT_FIELDS_PER_VALUE` fields
**Then** the insert succeeds

```
cargo test -p vb_core -- value_store_object_at_exact_max_fields_is_accepted
```

---

### EC-8: Blob at exact MAX bytes accepted

**Given** an empty `ValueStore`
**When** I insert a blob of exactly `MAX_BLOB_BYTES_PER_VALUE` bytes
**Then** the insert succeeds

```
cargo test -p vb_core -- value_store_blob_at_exact_max_bytes_is_accepted
```

---

### EC-9: Duplicate object key returns first occurrence

**Given** an object with duplicate key `{k: V1, k: V2}`
**When** I query `store.object_field(obj_id, k)`
**Then** I receive `V1` (first occurrence wins)

```
cargo test -p vb_core -- value_store_object_field_returns_first_duplicate_key
```

---

### EC-10: Object field on wrong object returns not found

**Given** two objects where only obj1 has key `k`
**When** I query `store.object_field(obj2, k)`
**Then** I receive `Err(CoreError::ObjectFieldNotFound { field: k })`

```
cargo test -p vb_core -- value_store_object_field_on_wrong_object_returns_not_found
```

---

### EC-11: Default equals new

**Given** `ValueStore::default()` and `ValueStore::new()`
**When** I compare them
**Then** they are equal

```
cargo test -p vb_core -- value_store_default_is_same_as_new
```

---

## Contract Verification Tests

### CV-1: Cap check is pre-mutation (INV1)

**Given** `ValueStore::with_max_slots(1)` with 1 symbol inserted
**When** I attempt to insert a second symbol (which fails with cap exceeded)
**Then** `symbol_count() == 1` (no partial mutation)

```
cargo test -p vb_core -- value_store_rejected_symbol_over_max_does_not_mutate_arena
```

**Variants:** Same for list, object, blob

---

### CV-2: Total arena count is sum of all arenas (INV1/C3)

**Given** `ValueStore::with_max_slots(10)` with 2 symbols, 1 list, 1 object inserted
**When** I call `total_arena_count()`
**Then** I receive `4`

```
cargo test -p vb_core -- value_store_counts_track_insertions
```

---

### CV-3: Clone is equal (INV3)

**Given** a `ValueStore` with 1 symbol
**When** I clone the store
**Then** `store == cloned`

```
cargo test -p vb_core -- value_store_clone_is_equal
```

---

## Violation / Adversarial Tests

### VA-1: High ID rejected on populated store

**Given** a `ValueStore` with 1 symbol inserted (id=0)
**When** I call `store.symbol(SymbolId::new(1))`
**Then** I receive `Err(SymbolOutOfBounds)`

```
cargo test -p vb_core -- value_store_symbol_handle_high_id_rejected
```

**Variants:** Same for list, object, blob

---

### VA-2: Never-inserted ID returns out-of-bounds

**Given** a `ValueStore` that has never had a blob inserted
**When** I call `store.blob(BlobId::new(0))`
**Then** I receive `Err(BlobOutOfBounds)`

```
cargo test -p vb_core -- value_store_blob_id_that_was_never_inserted_returns_out_of_bounds
```

---

### VA-3: One byte over blob limit rejected

**Given** an empty `ValueStore`
**When** I insert a blob of `MAX_BLOB_BYTES_PER_VALUE + 1` bytes
**Then** I receive `Err(ResourceLimitExceeded { resource: "blob_bytes" })`

```
cargo test -p vb_core -- value_store_blob_one_byte_over_limit_is_rejected
```

---

### VA-4: List item index zero on empty list fails

**Given** a `ValueStore` with an empty list `[]`
**When** I call `store.list_item(list_id, 0)`
**Then** I receive `Err(CoreError::ListIndexOutOfBounds { index: 0 })`

```
cargo test -p vb_core -- value_store_list_item_index_zero_on_empty_list_fails
```

---

### VA-5: Max list accesses edges without unchecked indexing

**Given** a list at exactly `MAX_LIST_ITEMS_PER_VALUE` items
**When** I access index 0 and index `MAX_LIST_ITEMS_PER_VALUE - 1`
**Then** both succeed
**And** accessing index `MAX_LIST_ITEMS_PER_VALUE` fails

```
cargo test -p vb_core -- value_store_exact_max_list_accesses_edges_without_unchecked_indexing
```

---

### VA-6: Max object duplicate key first-wins index

**Given** an object at exactly `MAX_OBJECT_FIELDS_PER_VALUE` fields with a duplicate key
**When** I query the duplicate key
**Then** I receive the first occurrence's value

```
cargo test -p vb_core -- value_store_exact_max_object_preserves_duplicate_first_wins_index
```

---

## Test Execution Order

1. Run all unit tests: `cargo test -p vb_core -- value_store`
2. Run integration tests: `cargo test -p vb_runtime`
3. Run Kani: `cargo kani --tests`
4. Run Miri: `cargo miri test -p vb_core -- value_store`

---

*End of Martin Fowler test plan.*