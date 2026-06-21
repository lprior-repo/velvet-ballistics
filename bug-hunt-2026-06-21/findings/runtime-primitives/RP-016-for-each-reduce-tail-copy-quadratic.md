# RP-016: ForEach And Reduce Copy The Remaining Tail On Every Iteration

- **Severity**: Medium
- **Category**: perf
- **Location**: `crates/vb_runtime/src/primitives/helpers/list.rs:29`
- **Confidence**: confirmed

## Description
ForEach and Reduce represent iteration progress by allocating a fresh list containing every remaining item after each step. That turns an `n` item loop into `O(n^2)` `SlotValue` copies and `O(n)` list allocations on a primitive hot path.

## Evidence
`tail_items` copies every element after index zero into a new `Vec` and boxed slice:

```rust
29: pub(crate) fn tail_items(items: &[SlotValue]) -> Result<Box<[SlotValue]>, EngineError> {
...
39:     let mut tail = Vec::with_capacity(tail_len);
40:     let mut index = 1usize;
41:     while index < items.len() {
42:         let value = *items
43:             .get(index)
...
47:         tail.push(value);
...
54:     Ok(tail.into_boxed_slice())
```

It is called on loop start and every loop advance:

```rust
crates/vb_runtime/src/primitives/for_each.rs:54:     let tail = tail_items(items)?;
crates/vb_runtime/src/primitives/for_each.rs:83:     let tail = tail_items(items)?;
crates/vb_runtime/src/primitives/reduce.rs:50:     let tail = tail_items(items)?;
crates/vb_runtime/src/primitives/reduce.rs:81:     let tail = tail_items(remaining)?;
```

For a list of length `n`, the runtime copies `n-1 + n-2 + ... + 1` items.

## Adversarial Check
The fanout and item limits bound the maximum damage, but they do not make the algorithm linear. This is not a one-time materialization cost; the loop body pays a shrinking full-tail copy each iteration, exactly where primitive execution should be cheapest and most predictable.

## Suggested Fix
Represent iterator state as `(ListId, cursor)` or another bounded cursor state instead of materialized tails. Keep the source list immutable in `ValueStore`, read the current item by checked index, and advance the cursor with checked arithmetic.
