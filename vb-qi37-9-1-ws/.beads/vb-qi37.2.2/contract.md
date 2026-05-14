# Contract: vb-qi37.2.2 — ValueStore Arena Cap Enforcement

## Module Scope

- **File:** `crates/vb_core/src/value_store.rs`
- **Type:** `ValueStore`
- **Bead:** `runtime: Enforce per-run value arena caps`

## Purpose

`ValueStore` provides cold value arenas backing handle-only runtime slot values (symbols, lists, objects, blobs). This contract governs **per-run aggregate arena cap enforcement**: a hard limit on the total number of arena entries (across all four arenas) that can be inserted during a single run.

---

## Constructor Invariants

### C1: `ValueStore::new()` creates an uncapped store

```rust
pub const fn new() -> Self
```

**Preconditions:** None.

**Postconditions:**
- `self.max_arena_entries == 0`
- All arena vectors (`symbols`, `lists`, `objects`, `blobs`) are empty
- All parallel taint vectors are empty

**Errors:** None.

---

### C2: `ValueStore::with_max_slots(max_slots: u16)` creates a capped store

```rust
pub fn with_max_slots(max_slots: u16) -> Self
```

**Preconditions:** `max_slots > 0`

**Postconditions:**
- `self.max_arena_entries == u64::from(max_slots)`
- All arena vectors are empty
- `total_arena_count() == 0`

**Errors:** None.

**Note:** `max_slots` is a **per-run** cap, not a per-arena cap. The cap applies to the sum of all four arena lengths.

---

## Insertion Operations

### I1: Symbol Insertion

```rust
pub fn insert_symbol(&mut self, value: impl Into<Box<str>>) -> CoreResult<SymbolId>
```

**Preconditions:**
- `validate_symbol_len(value.len())` must pass (`MAX_SYMBOL_BYTES_PER_VALUE`)

**Postconditions (success):**
- `result == SymbolId::new(symbols.len() - 1)` — monotonic ID assignment
- `self.symbols[symbol_index(result)?] == value`
- `total_arena_count()` increased by 1

**Postconditions (failure — cap exceeded):**
- `self.symbols` unchanged (atomic rejection)
- Returns `Err(CoreError::BudgetExceeded { budget: "max_slots", limit })`

**Errors:**
- `CoreError::ResourceLimitExceeded { resource: "symbol_bytes" }` if symbol too large
- `CoreError::BudgetExceeded { budget: "max_slots", limit }` if arena cap exceeded
- `CoreError::ResourceLimitExceeded { resource: "symbols" }` if u32 ID overflow

---

### I2: List Insertion

```rust
pub fn insert_list(&mut self, values: impl Into<Box<[SlotValue]>>) -> CoreResult<ListId>
```

**Preconditions:**
- `validate_list_len(values.len())` must pass

**Postconditions (success):**
- `result == ListId::new(lists.len() - 1)`
- `self.lists[list_index(result)?] == values`
- `self.list_taints[list_index(result)?]` is all `Taint::Clean`
- `total_arena_count()` increased by 1

**Postconditions (failure):**
- All vectors unchanged
- Returns `Err(CoreError::BudgetExceeded { budget: "max_slots", limit })`

**Errors:**
- `CoreError::ResourceLimitExceeded { resource: "list_items" }` if too many items
- `CoreError::BudgetExceeded` if cap exceeded
- `CoreError::ResourceLimitExceeded { resource: "lists" }` if u32 ID overflow

---

### I3: List Insertion with Taint

```rust
pub fn insert_list_with_taint(
    &mut self,
    values: Box<[SlotValue]>,
    taints: Box<[Taint]>,
) -> CoreResult<ListId>
```

**Preconditions:**
- `validate_list_len(values.len())` must pass
- `taints.len() == values.len()`

**Postconditions (success):** Same as I2, with per-item taints.

**Postconditions (failure):**
- All vectors unchanged
- Returns `Err(CoreError::BudgetExceeded { budget: "max_slots", limit })`

**Errors:**
- `CoreError::InternalInvariantViolation` if taint/value length mismatch
- `CoreError::ResourceLimitExceeded { resource: "list_items" }`
- `CoreError::BudgetExceeded`
- `CoreError::ResourceLimitExceeded { resource: "lists" }`

---

### I4: Object Insertion

```rust
pub fn insert_object(&mut self, fields: impl Into<Box<[ObjectField]>>) -> CoreResult<ObjectId>
```

**Preconditions:**
- `validate_object_len(fields.len())` must pass

**Postconditions (success):**
- `result == ObjectId::new(objects.len() - 1)`
- `self.objects[object_index(result)?] == fields`
- `self.object_field_index[object_index(result)?]` contains all field keys mapped to values
- `self.object_taint_index[object_index(result)?]` contains all field keys mapped to taints
- `total_arena_count()` increased by 1

**Postconditions (failure):**
- All vectors unchanged
- Returns `Err(CoreError::BudgetExceeded { budget: "max_slots", limit })`

**Errors:**
- `CoreError::ResourceLimitExceeded { resource: "object_fields" }`
- `CoreError::BudgetExceeded`
- `CoreError::InternalInvariantViolation` if field index overflow during building
- `CoreError::ResourceLimitExceeded { resource: "objects" }` if u32 ID overflow

---

### I5: Blob Insertion

```rust
pub fn insert_blob(&mut self, bytes: impl Into<Bytes>) -> CoreResult<BlobId>
```

**Preconditions:**
- `validate_blob_len(bytes.len())` must pass

**Postconditions (success):**
- `result == BlobId::new(blobs.len() - 1)`
- `self.blobs[blob_index(result)?] == bytes`
- `total_arena_count()` increased by 1

**Postconditions (failure):**
- `self.blobs` unchanged
- Returns `Err(CoreError::BudgetExceeded { budget: "max_slots", limit })`

**Errors:**
- `CoreError::ResourceLimitExceeded { resource: "blob_bytes" }`
- `CoreError::BudgetExceeded`
- `CoreError::ResourceLimitExceeded { resource: "blobs" }` if u64 ID overflow

---

## Accessor Operations

### A1: Symbol Resolution

```rust
pub fn symbol(&self, id: SymbolId) -> CoreResult<&str>
```

**Preconditions:** None.

**Postconditions:**
- Returns `Err(CoreError::SymbolOutOfBounds { symbol: id })` if `symbol_index(id) >= self.symbols.len()`
- Returns `&self.symbols[symbol_index(id)]` otherwise

---

### A2: List Resolution

```rust
pub fn list(&self, id: ListId) -> CoreResult<&[SlotValue]>
```

**Postconditions:**
- Returns `Err(CoreError::ListOutOfBounds { list: id })` if `list_index(id) >= self.lists.len()`
- Returns the list slice otherwise

---

### A3: Object Resolution

```rust
pub fn object(&self, id: ObjectId) -> CoreResult<&[ObjectField]>
```

**Postconditions:**
- Returns `Err(CoreError::ObjectOutOfBounds { object: id })` if out of bounds
- Returns the object fields slice otherwise

---

### A4: Blob Resolution

```rust
pub fn blob(&self, id: BlobId) -> CoreResult<&[u8]>
```

**Postconditions:**
- Returns `Err(CoreError::BlobOutOfBounds { blob: id })` if out of bounds
- Returns the blob bytes otherwise

---

### A5: List Item Access

```rust
pub fn list_item(&self, id: ListId, index: u32) -> CoreResult<SlotValue>
```

**Postconditions:**
- Returns `Err(CoreError::ListIndexOutOfBounds { index })` if `index >= list.len()`
- Returns the item otherwise

---

### A6: Object Field Access

```rust
pub fn object_field(&self, id: ObjectId, key: SymbolId) -> CoreResult<SlotValue>
```

**Postconditions:**
- Returns `Err(CoreError::ObjectFieldNotFound { field: key })` if key not in index
- Returns the value otherwise

---

## Count & Capacity Operations

### C3: `total_arena_count()`

```rust
pub fn total_arena_count(&self) -> u64
```

**Postconditions:**
- Returns `symbols.len() + lists.len() + objects.len() + blobs.len()` as u64
- Uses `saturating_add` to avoid overflow
- Returns 0 for empty store

---

### C4: `max_arena_entries()`

```rust
pub const fn max_arena_entries(&self) -> u64
```

**Postconditions:**
- Returns the configured cap (0 means uncapped)

---

## Internal Invariants

### INV1: Arena Cap Check is Pre-mutation

`check_arena_cap()` is called **before** any push to an arena vector. If it returns `Err`, no vector is mutated.

**Proof:** All five insert operations follow this pattern:
```
1. validate_<type>_len(...)
2. check_arena_cap()?   ← gate
3. next_<type>_id(...)
4. push(...)            ← only if step 2 succeeds
```

---

### INV2: Monotonic ID Assignment

For each arena type T with `insert_T` and `T_id` handle:
- If `store.insert_T(...)` returns `Ok(id)` and `store.insert_T(...)` returns `Ok(id')` in the same store instance,
  then `id.get() < id'.get()`.

**Proof:** ID is `T::new(current_len)` where `current_len` is the arena length **before** the push. Each successful push increments length by 1.

---

### INV3: Rejection Atomicity

When any insert operation returns `Err`:
- No arena vector was mutated
- No parallel taint index was mutated
- No ID was issued

---

### INV4: Handle Validity Post-Drop

If `id` was never returned from an insert operation on a store instance, accessing that handle on the same store instance returns an out-of-bounds error.

---

## Error Taxonomy

| Error Kind | Condition | Affected Operations |
|---|---|---|
| `CoreError::BudgetExceeded { budget: "max_slots", limit }` | `total_arena_count() >= max_arena_entries` | All insert ops |
| `CoreError::ResourceLimitExceeded { resource: "symbol_bytes" }` | Symbol bytes > `MAX_SYMBOL_BYTES_PER_VALUE` | `insert_symbol` |
| `CoreError::ResourceLimitExceeded { resource: "list_items" }` | List items > `MAX_LIST_ITEMS_PER_VALUE` | `insert_list`, `insert_list_with_taint` |
| `CoreError::ResourceLimitExceeded { resource: "object_fields" }` | Object fields > `MAX_OBJECT_FIELDS_PER_VALUE` | `insert_object` |
| `CoreError::ResourceLimitExceeded { resource: "blob_bytes" }` | Blob bytes > `MAX_BLOB_BYTES_PER_VALUE` | `insert_blob` |
| `CoreError::ResourceLimitExceeded { resource: "symbols" }` | u32 ID overflow | `insert_symbol` |
| `CoreError::ResourceLimitExceeded { resource: "lists" }` | u32 ID overflow | `insert_list`, `insert_list_with_taint` |
| `CoreError::ResourceLimitExceeded { resource: "objects" }` | u32 ID overflow | `insert_object` |
| `CoreError::ResourceLimitExceeded { resource: "blobs" }` | u64 ID overflow | `insert_blob` |
| `CoreError::InternalInvariantViolation` | Taint/value length mismatch, field index overflow | `insert_list_with_taint`, `insert_object` |
| `CoreError::SymbolOutOfBounds` | Invalid symbol handle | `symbol`, `symbol_index` |
| `CoreError::ListOutOfBounds` | Invalid list handle | `list`, `list_item`, etc. |
| `CoreError::ObjectOutOfBounds` | Invalid object handle | `object`, `object_field`, etc. |
| `CoreError::BlobOutOfBounds` | Invalid blob handle | `blob`, etc. |
| `CoreError::ListIndexOutOfBounds` | Index >= list length | `list_item` |
| `CoreError::ObjectFieldNotFound` | Key not in object | `object_field` |

---

## Edge Cases

| Case | Expected Behavior |
|---|---|
| Insert when `total_arena_count() == max_arena_entries - 1` | Success, count becomes equal to cap |
| Insert when `total_arena_count() == max_arena_entries` | `Err(CoreError::BudgetExceeded)` |
| Insert into uncapped store (`max_arena_entries == 0`) | Always succeeds (unless other limits) |
| Empty symbol/list/object/blob | Accepted; ID issued, count incremented |
| `index >= list.len()` on existing list | `Err(CoreError::ListIndexOutOfBounds)` |
| `index == u32::MAX` | Always `Err(CoreError::ListIndexOutOfBounds)` |
| Duplicate object field key | First occurrence wins (linear scan) |
| Handle from a different store instance | Returns out-of-bounds error |
| `ValueStore::new() == ValueStore::default()` | True (both have cap 0, empty arenas) |

---

## Relationship to AggregateResourceBudget

`ValueStore` arena caps are **per-run limits** enforced by the runtime. They complement the compile-time `AggregateResourceBudget` which tracks workflow-level resource consumption:

- `AggregateResourceBudget` limits are derived at compile time from the workflow IR
- `ValueStore` caps are runtime enforcement of arena entry counts
- Both are enforced at run admission and during execution

The runtime caps (`MAX_VALUES_PER_RUN` from `limits.rs`) establish an absolute upper bound; `ValueStore::with_max_slots(n)` allows a stricter subset limit for a specific run.

---

## Waivers

| Waiver | Reason | Compensating Evidence |
|---|---|---|
| WAIVER-001 | ValueStore mutable Rust data structures — not Lean-owned | 20+ unit tests, integration tests |

---

*End of contract. All artifacts must be verified before implementation proceeds.*