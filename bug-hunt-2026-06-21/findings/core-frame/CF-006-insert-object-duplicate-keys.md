# CF-006: `ValueStore::insert_object` allows duplicate keys — slice and index disagree

- **Severity**: High
- **Category**: correctness
- **Location**: `crates/vb_core/src/value_store.rs:134`
- **Confidence**: confirmed

## Description

`insert_object` builds a secondary `IndexMap` from `field.key` to
`field.value`, using `entry(field.key).or_insert(...)` so the *first*
occurrence of each key wins. But the original `fields` slice is also
stored verbatim (`self.objects.push(fields)`). If the caller passes
`[{A, 1}, {A, 2}]`, `object(id)?` returns a slice containing both
entries, while `object_field(id, A)` returns `1` — the two read paths
disagree on the same object.

## Evidence

```rust
let mut index = IndexMap::new();
let mut taint_index = IndexMap::new();
let mut field_pos = 0usize;
while field_pos < fields.len() {
    let field = fields.get(field_pos).ok_or(...)?;
    index.entry(field.key).or_insert(field.value);
    taint_index.entry(field.key).or_insert(field.taint);
    field_pos = field_pos.checked_add(1).ok_or(...)?;
}
self.object_field_index.push(index);
self.object_taint_index.push(taint_index);
self.objects.push(fields);                  // <-- still contains duplicates
```

(`crates/vb_core/src/value_store.rs:139-158`)

Read paths:

```rust
pub fn object(&self, id: ObjectId) -> CoreResult<&[ObjectField]> { ... }   // returns raw slice
pub fn object_field(&self, id: ObjectId, key: SymbolId) -> CoreResult<SlotValue> {
    ... index.get(&key).copied().ok_or(CoreError::ObjectFieldNotFound { field: key })
}
```

(`value_store.rs:189-194` and `:237-247`)

## Adversarial Check

A defender might argue "the caller is responsible for not passing
duplicates." Then the function should *reject* duplicates with an error,
not silently build a secondary index that disagrees with the primary
slice. Worse, an attacker (or a buggy IR generator) that constructs an
object with a duplicate key can produce a state where
`object_field(id, A) != object(id)?.iter().find(|f| f.key == A).value`,
which is a correctness footgun for any code that iterates the slice
instead of using the index (e.g. serialization, snapshotting).

## Suggested Fix

On encountering a duplicate key, return
`Err(CoreError::InvalidCompiledWorkflow { reason: "duplicate_object_key" })`.
Alternatively, document the first-wins behavior and have `object(id)?`
return a deduplicated view (one entry per key matching the index).
