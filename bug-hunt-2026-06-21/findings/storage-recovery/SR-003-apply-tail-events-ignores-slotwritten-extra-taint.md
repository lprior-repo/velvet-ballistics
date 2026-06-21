# SR-003: `apply_tail_events` ignores `SlotWrittenEvent.extra`, uses stale frame taint

- **Severity**: High
- **Category**: bug
- **Location**: `crates/vb_storage/src/recovery/event_replay/tail.rs:172`
- **Confidence**: confirmed

## Description

When `apply_tail_events` processes a `SlotWrittenEvent`, it ignores the
event's `extra` field (which carries the slot's intended taint envelope) and
instead derives the taint from `frame.read_taint(*slot)` — i.e. whatever
value happens to already be in the frame. For a freshly hydrated frame (no
prior slot writes), this means every `SlotWrittenEvent` is written with
`Taint::Clean`, regardless of the taint encoded in the event. This silently
over-classifies Secret-derived slot writes as Clean and is inconsistent with
the accumulator path in `frame_seed/accumulator.rs`, which does decode
`extra`.

## Evidence

```rust
JournalEvent::SlotWrittenEvent { slot, value, .. } => {      // <-- `extra` ignored
    if let Some(bytes) = value {
        let slot_value = postcard::from_bytes(bytes).map_err(|_| {
            RecoveryError::ReplayDivergence { ... }
        })?;
        let taint = match resolve_slot_taint_read(observe_slot_taint_read(
            frame.read_taint(*slot),                            // <-- existing taint, not event's
        )) {
            SlotTaintResolution::Use(taint) => taint,
            SlotTaintResolution::FailClosed => {
                return Err(RecoveryError::SlotTaintReadFailed { slot: *slot });
            }
        };
        frame
            .write_slot_with_taint(*slot, slot_value, taint)
            ...
    }
    executed = executed.saturating_add(1);
}
```

Compare to the accumulator's correct handling
(`recovery/replay/summary/frame_seed/accumulator.rs:208`):
```rust
fn record_slot_write(
    mut self,
    slot: SlotIdx,
    value: &Option<Vec<u8>>,
    extra: &Option<Vec<u8>>,
) -> RecoveryResult<Self> {
    ...
    let recovered_taint =
        crate::recovery::replay::summary::slots::taint::recovered_slot_taint(
            slot, slot_value, extra,
        )?;
    self.slot_values.insert(slot, slot_value);
    self.slot_taint.insert(slot, recovered_taint.taint);
    ...
}
```

Concrete corruption scenario for events-only hydration
(`hydrate_run_frame_from_events` is built on `apply_tail_events`):

1. Frame is created with default taint (Clean) on every slot.
2. Tail event `SlotWrittenEvent { slot: 5, value: V_secret, extra: Some(<Secret taint envelope>) }`.
3. `tail.rs` reads `frame.read_taint(5)` → `Uninitialized` → resolves to
   `Taint::Clean` (see `taint.rs:33`).
4. Frame writes `(5, V_secret, Clean)`.

A slot that should have been marked `Secret` is now `Clean`. Downstream
consumers that gate outputs on taint (e.g. log scrubbers, slot redaction)
will leak the secret.

For snapshot+tail hydration, the same path "accidentally" preserves the
snapshot's prior taint via `frame.read_taint`, but only for slots already
present in the snapshot. Slots new to the tail default to Clean even when the
event explicitly says otherwise.

## Adversarial Check

A counter-argument: "this is intentional — tail application intentionally
inherits taint from the snapshot." That cannot be right because
`hydrate_run_frame_from_events` (events-only path) calls `apply_tail_events`
on a freshly-created frame, so there is no prior taint to inherit and every
slot ends up Clean. Even for snapshot+tail, the design breaks the first time
a slot is newly introduced in the tail. And the inconsistency with the
accumulator path proves the two implementations disagree about what the
correct taint is for the same event — they cannot both be right.

The function `recovered_slot_taint` exists specifically to decode the
envelope and is used by the accumulator, so there is no question about
intent: the tail path is missing the call.

## Suggested Fix

Bind `extra` in the match arm and use the same decoder the accumulator uses:
```rust
JournalEvent::SlotWrittenEvent { slot, value, extra, .. } => {
    if let Some(bytes) = value {
        let slot_value = postcard::from_bytes::<SlotValue>(bytes).map_err(|_| {
            RecoveryError::ReplayDivergence { ... }
        })?;
        let recovered = crate::recovery::replay::summary::slots::taint::recovered_slot_taint(
            *slot, slot_value, extra,
        )?;
        frame.write_slot_with_taint(*slot, slot_value, recovered.taint).map_err(...)?;
    }
    executed = executed.saturating_add(1);
}
```
This restores parity with the accumulator path and removes the dependence on
whatever happens to be in the frame's taint array.
