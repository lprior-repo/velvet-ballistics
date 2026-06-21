# CF-009: `object_field_with_taint` reports wrong error when value exists but taint is missing

- **Severity**: Low
- **Category**: correctness
- **Location**: `crates/vb_core/src/value_store.rs:264`
- **Confidence**: confirmed

## Description

`object_field_with_taint` does two independent lookups: first for the
value in `object_field_index`, then for the taint in
`object_taint_index`. If the value is found but the taint is not, the
function returns `CoreError::ObjectFieldNotFound`, which is misleading —
the field *was* found, the taint index is just inconsistent. The
function should surface this as an `InternalInvariantViolation` so the
desync is visible.

## Evidence

```rust
let value = index
    .get(&key)
    .copied()
    .ok_or(CoreError::ObjectFieldNotFound { field: key })?;
let taint_index = self
    .object_taint_index
    .get(idx)
    .ok_or(CoreError::ObjectOutOfBounds { object: id })?;
let taint = taint_index
    .get(&key)
    .copied()
    .ok_or(CoreError::ObjectFieldNotFound { field: key })?;   // <-- misleading
Ok((value, taint))
```

(`crates/vb_core/src/value_store.rs:255-273`)

## Adversarial Check

One might argue "the two indices are populated together in `insert_object`,
so the desync is impossible." But the entire purpose of error variants like
`InternalInvariantViolation` is to make "impossible" states observable when
they happen anyway (e.g. after a bug, a serialization round-trip, or a
partial recovery). Reporting `ObjectFieldNotFound` causes the caller to
treat an invariant violation as a normal "key absent" outcome, hiding the
real bug.

## Suggested Fix

```rust
let taint = taint_index.get(&key).copied().ok_or(
    CoreError::InternalInvariantViolation {
        reason: "object_taint_index_missing_field",
    },
)?;
```
