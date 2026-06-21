# SR-012: `decode_snapshot_slots` performs O(N·M) taint lookup

- **Severity**: Medium
- **Category**: perf
- **Location**: `crates/vb_storage/src/recovery/snapshot_decode.rs:42`
- **Confidence**: confirmed

## Description

`decode_snapshot_slots` decodes two `Vec<(SlotIdx, SlotValue, Taint)>`
buffers (slots and taint), then for each entry in `slots` performs a linear
`iter().find_map(...)` over the entire `taint` vector to look up an explicit
taint override. The result is O(N·M) in the slot count, which is O(N²) when
the two vectors are the same size (which they are — `snapshot_write.rs`
writes identical `slot_entries` into both fields).

## Evidence

```rust
let mut entries = Vec::new();
for (slot, value, default_taint) in slots {
    let explicit_taint = taint
        .iter()
        .find_map(|(t_slot, _, t_taint)| {
            if *t_slot == slot {
                Some(*t_taint)
            } else {
                None
            }
        })
        .unwrap_or(default_taint);
    entries.push(RecoveredSlotEntry {
        slot,
        value,
        taint: explicit_taint,
    });
}
```

The hot path here is snapshot hydration, which is on the recovery critical
path. For a run with thousands of slots, the quadratic lookup dominates the
decode cost and adds allocator pressure (the inner closure captures state
that prevents iterator chaining).

## Adversarial Check

Snapshot size is bounded by `MAX_SNAPSHOT_BYTES` and typical runs may have
only a few dozen slots, so the quadratic term may not matter in steady
state. But snapshot decode runs on every recovery invocation and on every
`hydrate_run_frame` call (which happens during `recover_all_incomplete_runs`
for every run header). The cost compounds linearly with the number of runs.
A `HashMap<SlotIdx, Taint>` is the standard fix and adds no API surface.

Also note (separate concern, related file): `snapshot_write.rs` writes the
*same* `slot_entries` payload into both the `slots` and `taint` fields, so
the lookup always finds a match. The override logic exists to support a
hypothetical future divergence that the writer does not yet produce.

## Suggested Fix

Build a HashMap once before the loop:
```rust
let taint_map: std::collections::HashMap<SlotIdx, Taint> = taint
    .into_iter()
    .map(|(slot, _, taint)| (slot, taint))
    .collect();

let entries = slots
    .into_iter()
    .map(|(slot, value, default_taint)| RecoveredSlotEntry {
        slot,
        value,
        taint: taint_map.get(&slot).copied().unwrap_or(default_taint),
    })
    .collect();
```
This makes the merge O(N + M) and the iteration is now a clean `map`/`collect`.
