# CE-003: Replay accessor evaluation drops field/list-item taint

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_core/src/replay/ops.rs:95`
- **Confidence**: confirmed

## Description

Replay joins only the root slot taint when loading an accessor. Engine evaluation joins taint from every traversed object field or list item, so replay can reconstruct the same value with weaker taint.

## Evidence

Replay reads root taint, resolves the accessor value through taintless store APIs, and pushes the value:

```rust
let root_taint = run
    .read_taint(accessor_program.root)
    .map_err(|_| ReplayError::Internal {
        reason: "read_taint failed for accessor root",
    })?;
let value = eval_accessor_for_replay(run, store, accessor_program)?;
*taint_accum = join_taint(*taint_accum, root_taint);
stack.push(value)
```

The replay traversal uses `object_field` and `list_item`, which return only values:

```rust
current = match (current, segment) {
    (SlotValue::Object(object), crate::workflow::PathSegment::Field(field)) => store
        .object_field(object, field)
        ...,
    (SlotValue::List(list), crate::workflow::PathSegment::Index(idx)) => store
        .list_item(list, idx)
        ...,
```

Engine evaluation uses the taint-aware traversal and joins each segment taint:

```rust
let (value, segment_taint) = traverse_accessor_segment_with_taint(store, current, segment)?;
accumulated_taint = crate::value::join_taint(accumulated_taint, segment_taint);
current = value;
```

## Adversarial Check

This is not hypothetical if aggregate object/list slot taint happens to mirror every child. The `ValueStore` has taint-aware accessor APIs and the engine uses them deliberately, which means field/list item taint is part of the semantic contract. Replay bypasses that contract and can downgrade taint for accessor-derived outputs.

## Suggested Fix

Change replay accessor traversal to use `object_field_with_taint` and `list_item_with_taint`, return `(SlotValue, Taint)`, and join segment taint exactly as `engine::expr_eval::accessors` does.
