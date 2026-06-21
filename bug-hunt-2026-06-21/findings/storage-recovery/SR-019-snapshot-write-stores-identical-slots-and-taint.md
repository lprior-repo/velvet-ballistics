# SR-019: `write_recovered_snapshot` stores identical payloads in `slots` and `taint`

- **Severity**: Info
- **Category**: simplification
- **Location**: `crates/vb_storage/src/recovery/snapshot_write.rs:23`
- **Confidence**: confirmed

## Description

`write_recovered_snapshot` derives both the `slots` and `taint` byte buffers
from the *same* `slot_entries: Vec<(SlotIdx, SlotValue, Taint)>`. The two
stored payloads are therefore postcard-encodings of identical data, doubling
on-disk snapshot size without adding any information. The reader
(`decode_snapshot_slots`) then goes to some trouble to merge the two
vectors, always finding exact matches.

## Evidence

```rust
let slot_entries = snapshot_slot_entries(seed);
let slots = encode_snapshot_slots(seed, &slot_entries)?;
let taint = encode_snapshot_taint(seed, &slot_entries)?;
journal.put_snapshot(&RunSnapshot {
    run: seed.summary.run,
    seq: seed.summary.last_seq,
    workflow,
    slots,
    taint,
})?;
```

Both `encode_snapshot_slots` and `encode_snapshot_taint` are
`postcard::to_allocvec(slot_entries)` — they do not project different fields
out of the entries. The decoder then has an O(N·M) merge step (see SR-012)
that exists purely to find the matching entry in the duplicate vector.

## Adversarial Check

The argument for the current shape is "the two fields exist on the wire
format for forward compatibility — future snapshots may diverge slots and
taint into separate encodings." That is plausible, but until they actually
diverge the writer is paying double the postcard encode cost, double the
storage cost, and forcing the decoder to do an O(N·M) lookup that always
succeeds. If divergence is planned, the writer should at least project
`(slot, value)` into `slots` and `(slot, taint)` into `taint` so the two
fields carry distinct information; otherwise the redundancy is pure waste.

## Suggested Fix

If the two fields are intended to carry different projections, change the
two encoders to project their respective fields:
```rust
fn encode_snapshot_slots(seed, entries) -> ... {
    let projected: Vec<(SlotIdx, SlotValue)> = entries.iter().map(|(s, v, _)| (*s, *v)).collect();
    postcard::to_allocvec(&projected)
}
fn encode_snapshot_taint(seed, entries) -> ... {
    let projected: Vec<(SlotIdx, Taint)> = entries.iter().map(|(s, _, t)| (*s, *t)).collect();
    postcard::to_allocvec(&projected)
}
```
This shrinks both payloads and gives the decoder a real reason to merge
them. If the redundancy is intentional, document it in a comment so future
readers do not "fix" it back.
