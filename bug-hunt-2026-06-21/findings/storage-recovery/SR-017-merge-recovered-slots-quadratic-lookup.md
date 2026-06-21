# SR-017: `merge_recovered_slots` performs O(N·M) slot lookup

- **Severity**: Low
- **Category**: perf
- **Location**: `crates/vb_storage/src/recovery/replay/summary/slots/recovery.rs:79`
- **Confidence**: confirmed

## Description

`merge_recovered_slots` folds event-derived slot overrides into the
workflow-replayed base list using
`base.entries.iter_mut().find(|entry| entry.slot == override_entry.slot)`.
The inner `find` is O(N), making the merge O(N·M) overall in slot count.

## Evidence

```rust
fn merge_recovered_slots(
    mut base: RecoveredSlots,
    overrides: Vec<RecoveredSlotEntry>,
) -> RecoveredSlots {
    for override_entry in overrides {
        match base
            .entries
            .iter_mut()
            .find(|entry| entry.slot == override_entry.slot)
        {
            Some(entry) => *entry = override_entry,
            None => base.entries.push(override_entry),
        }
    }
    base
}
```

This is on the recovery hot path for runs that supply a `CompiledWorkflow`
(`recover_runtime_frame_seed_from_events_with_workflow`). The base list is
produced by replaying the workflow, the overrides are event-derived slot
writes; for large step graphs both vectors can have hundreds of slots,
making the merge noticeably slow.

## Adversarial Check

Slot counts are bounded by the workflow's slot dimension, which is in turn
bounded by `u16`. For typical workflows (single-digit slots per step), the
quadratic term is negligible. But the merge runs every time
`recover_runtime_frame_seed_from_events_with_workflow` is called — which
includes `recover_all_incomplete_runs` walking every run header. The cost
adds up across a fleet scan.

Also note: as written, the function would silently produce duplicate slot
entries if `overrides` contained duplicate slots. The current call path
sources overrides from `accumulator.slot_values` (a `BTreeMap`) so duplicates
cannot arise today, but the helper offers no defense against future callers
that pass unsorted input.

## Suggested Fix

Build a `HashMap<SlotIdx, usize>` of base slot positions once, then look up
each override in O(1):
```rust
fn merge_recovered_slots(mut base: RecoveredSlots, overrides: Vec<RecoveredSlotEntry>) -> RecoveredSlots {
    let mut index: std::collections::HashMap<SlotIdx, usize> =
        base.entries.iter().enumerate().map(|(i, e)| (e.slot, i)).collect();

    for override_entry in overrides {
        match index.get(&override_entry.slot).copied() {
            Some(i) => base.entries[i] = override_entry,
            None => {
                index.insert(override_entry.slot, base.entries.len());
                base.entries.push(override_entry);
            }
        }
    }
    base
}
```
This also makes the duplicate-override behavior explicit (last-write-wins
by position).
