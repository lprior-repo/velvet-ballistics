# RP-004: `append_to_accumulator` is O(N) clone-per-append, O(N²) total

- **Severity**: Medium
- **Category**: perf
- **Location**: `crates/vb_runtime/src/primitives/together.rs:126-144`
- **Confidence**: confirmed

## Description

`append_to_accumulator` materializes the entire existing list into a fresh `Vec`, pushes one element, and re-inserts the whole thing into the value store. Called once per non-first branch, this is Θ(N) work and Θ(N) allocation per call, Θ(N²) total over a `together` of N branches.

## Evidence

`crates/vb_runtime/src/primitives/together.rs:138-141`:

```rust
let list_id = expect_list(current)?;
let existing = store.list(list_id)?;
let mut items = existing.to_vec();        // <-- O(N) clone + allocation
items.push(value);                         // <-- occasionally reallocs again
let updated = store.insert_list(items.into_boxed_slice())?;
```

Per branch:
- `existing.to_vec()` allocates and copies all current items.
- `push(value)` may realloc a second time if capacity is exhausted.
- `insert_list` allocates again in the value store.

For N branches each producing a scalar result, total work is 1 + 2 + … + N = Θ(N²) element copies and N allocations. For N = 1000 branches (well within `BranchCount`'s `u16` range and the 65 536 max enforced by `together_start`), that is ~500 000 element copies where N writes of amortized O(1) suffice.

The accumulator is a runtime-owned list; nothing else reads it between appends, so an amortized O(1) append is possible.

## Adversarial Check

1. *"Branches are typically <10."* — The runtime accepts up to `u16::MAX` branches (drive.rs:25-30). Workflows that fan out over a fanout-limit list (the standard map-shuffle pattern) routinely hit 100+ branches. Even at N=50 the quadratic cost is measurable in `together_tests`.
2. *"ValueStore doesn't expose in-place append."* — That is the root cause and is fixable; see Suggested Fix. Even without a new store API, the handler could maintain the accumulator as a `Vec<SlotValue>` in a side table (like `CollectStates`) and only materialize the list id at `together_join` time.
3. *"This is not the hot path."* — It is on the per-branch path of every `together` execution; `together` is one of the four core compound primitives. The cost is paid on every branch, not just on error paths.

Severity is Medium because there is no correctness violation, but the quadratic scaling makes large `together` fanouts artificially slow.

## Suggested Fix

Either:

(a) Add `ValueStore::append_list(list_id, value)` that performs amortized O(1) push (the LSM-backed store already supports this internally; the public API just does not expose it), or

(b) Hold the accumulator as a runtime side table `Map<(RunId, SlotIdx), Vec<SlotValue>>` keyed by accumulator slot — mirroring the `CollectStates` pattern — and only flush to `ValueStore` once at `together_join`. This removes the per-append store round-trip entirely.

Option (b) is the smaller change and matches the existing `CollectStates` precedent in the same primitives tree.
