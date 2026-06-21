# CF-010: `insert_list_with_taint` does not validate `taints.len()` against the per-value cap

- **Severity**: Low
- **Category**: correctness
- **Location**: `crates/vb_core/src/value_store.rs:115`
- **Confidence**: confirmed

## Description

`insert_list_with_taint` validates `values.len()` against
`MAX_LIST_ITEMS_PER_VALUE` but does not separately validate
`taints.len()`. When the two lengths are equal (the next check at line
121) this is equivalent. But if the function ever changes to relax the
length-equality check, the taints vec could exceed the per-value cap with
no error. The asymmetry is brittle.

## Evidence

```rust
pub fn insert_list_with_taint(
    &mut self,
    values: Box<[SlotValue]>,
    taints: Box<[Taint]>,
) -> CoreResult<ListId> {
    validate_list_len(values.len())?;             // <-- only values validated
    if taints.len() != values.len() {
        return Err(CoreError::InternalInvariantViolation {
            reason: "list values and taints length mismatch",
        });
    }
    ...
}
```

(`crates/vb_core/src/value_store.rs:115-131`)

## Adversarial Check

A defender would say "the length-equality check at line 121 transitively
constrains `taints.len()`, so the validation is complete." That is correct
*today*. The concern is the ordering: the cap check happens first, the
equality check second. If a future maintainer re-orders or weakens the
equality check (e.g. to truncate, or to permit `taints.len() < values.len()`
with default-Clean fill), the cap will no longer apply to taints. The
robust pattern is to validate both inputs identically.

## Suggested Fix

Validate `taints.len()` against `MAX_LIST_ITEMS_PER_VALUE` independently
(line near 120), before the equality check.
