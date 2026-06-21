# CE-005: Expression collection operators allocate infallibly on runtime data

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_core/src/engine/expr_eval/ops_text_list.rs:176`
- **Confidence**: confirmed

## Description

Several expression operators allocate through `to_vec`, `collect`, `with_capacity`, or `HashMap::with_capacity` instead of fallible reservation. Under memory pressure or adversarially large stored values, the engine can abort before returning the typed `AllocationFailed` errors used elsewhere.

## Evidence

`Append` clones the full list and pushes without a fallible reserve:

```rust
let mut new_items: Vec<SlotValue> = items.to_vec();
new_items.push(item);
let new_list = store
    .insert_list(new_items.into_boxed_slice())
    .map_err(|_| EngineError::AllocationFailed)?;
```

`AppendIf` repeats the same pattern:

```rust
let mut new_items: Vec<SlotValue> = items.to_vec();
if cond {
    new_items.push(item);
}
```

`Unique` materializes an `IndexSet` and a `Vec` via infallible collection:

```rust
let seen: IndexSet<SlotValue> = items.iter().copied().collect();
let new_list = store
    .insert_list(seen.into_iter().collect::<Vec<_>>().into_boxed_slice())
```

`Merge` also reserves infallibly from runtime object sizes:

```rust
let mut merged: Vec<crate::value_store::ObjectField> =
    Vec::with_capacity(left_fields.len().saturating_add(right_fields.len()));
let mut index: std::collections::HashMap<crate::ids::SymbolId, usize> =
    std::collections::HashMap::with_capacity(
        left_fields.len().saturating_add(right_fields.len()),
    );
```

## Adversarial Check

This is not a generic style complaint. Nearby object/list builders use `try_reserve_exact(...).map_err(|_| EngineError::AllocationFailed)` before pushing runtime-sized data, so the crate already treats allocation failure as a typed engine error. These operators do the same kind of runtime-sized materialization but bypass the fallible boundary.

## Suggested Fix

Precompute required lengths with checked arithmetic, call `try_reserve_exact`, then extend/push. For `Unique` and `Merge`, reserve the set/map/vector fallibly before insertion and return `EngineError::AllocationFailed` on allocation failure.
