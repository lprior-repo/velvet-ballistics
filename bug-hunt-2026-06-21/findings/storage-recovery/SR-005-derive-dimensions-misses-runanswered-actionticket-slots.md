# SR-005: `derive_dimensions_from_snapshot_and_tail` misses `RunAnswered` and `ActionScheduledTicket` slot indices

- **Severity**: Medium
- **Category**: bug
- **Location**: `crates/vb_storage/src/recovery/snapshot_decode.rs:109`
- **Confidence**: confirmed

## Description

`derive_dimensions_from_snapshot_and_tail` is responsible for computing
`slot_count` from the union of snapshot slot entries and tail events. Its
`match` only updates `max_slot` for `SlotWrittenEvent`, `RunFinished`, and
the `output` field of `ActionCompletedEnvelope`. It misses two event kinds
that `checkpoint_slot` (in the same crate, `incident/model/checkpoint.rs`)
explicitly treats as slot-bearing: `RunAnswered { slot_idx, .. }` and
`ActionScheduledTicket { output, .. }`. When either of those events carries
the highest slot index in the run, the derived `slot_count` is too small and
later slot writes (or hydration) will index out of bounds.

## Evidence

`snapshot_decode.rs:104-115`:
```rust
JournalEvent::ActionCompletedEnvelope { ticket, output, .. } => {
    max_step = Some(max_step.map_or(ticket.step, |s| s.max(ticket.step)));
    min_step = Some(min_step.map_or(ticket.step, |s| s.min(ticket.step)));
    max_slot = Some(max_slot.map_or(*output, |s| s.max(*output)));     // envelope output OK
}
JournalEvent::SlotWrittenEvent { slot, .. }
| JournalEvent::RunFinished { result: slot, .. } => {
    max_slot = Some(max_slot.map_or(*slot, |s| s.max(*slot)));          // only these two
}
_ => {}
```

But `ActionScheduledTicket` (lines 100-103) only updates `max_step`, not
`max_slot`, even though the event carries an `output: SlotIdx`. And
`RunAnswered { slot_idx, .. }` falls through to the wildcard `_ => {}`,
even though the run writes its answer into `slot_idx`.

Compare with `incident/model/checkpoint.rs:45-54`:
```rust
fn checkpoint_slot(event: &JournalEvent) -> Option<u16> {
    match event {
        JournalEvent::StepSucceeded { output, .. }
        | JournalEvent::ActionCompletedEnvelope { output, .. } => Some(output.get()),
        JournalEvent::SlotWrittenEvent { slot, .. } => Some(slot.get()),
        JournalEvent::RunFinished { result, .. } => Some(result.get()),
        JournalEvent::RunAnswered { slot_idx, .. } => Some(slot_idx.get()),       // <-- here
        JournalEvent::ActionScheduledTicket { output, .. } => Some(output.get()),  // <-- here
        _ => None,
    }
}
```

So the two functions disagree about which events bear slots.

Failure mode: a run that ends with `RunAnswered { slot_idx: SlotIdx(7) }`
and no other slot reference will derive `slot_count = 0`, then
`hydrate_run_frame` calls `RunFrame::new(run, first_step, step_count, 0)`.
Subsequent slot operations on slot 7 will return out-of-bounds errors that
surface as `RecoveryError::ReplayDivergence` with a misleading "slot write
out of bounds" detail, hiding the real cause (under-sized slot array).

## Adversarial Check

A counter-argument is that `ActionScheduledTicket.output` is the slot the
action *will* write to, so the slot might not exist yet — but the frame
still needs capacity for it. And `RunAnswered.slot_idx` is the slot the
answer was written to, so it absolutely must exist in the frame. Either
way, the slot index is a durable upper bound on the slot dimension. The
asymmetry with `checkpoint_slot` proves the omission is an oversight, not
an intentional scoping decision.

## Suggested Fix

Either reuse `checkpoint_slot` (it already returns `Option<u16>` for the
right set of events) or add the two missing arms:
```rust
JournalEvent::ActionScheduledTicket { ticket, output, .. } => {
    max_step = Some(max_step.map_or(ticket.step, |s| s.max(ticket.step)));
    min_step = Some(min_step.map_or(ticket.step, |s| s.min(ticket.step)));
    max_slot = Some(max_slot.map_or(*output, |s| s.max(*output)));
}
JournalEvent::RunAnswered { slot_idx, .. } => {
    max_slot = Some(max_slot.map_or(*slot_idx, |s| s.max(*slot_idx)));
}
```
Better still, extract one shared `slot_index_for_event` predicate and use it
from both `checkpoint_slot` and `derive_dimensions_from_snapshot_and_tail`
so the two cannot drift again.
