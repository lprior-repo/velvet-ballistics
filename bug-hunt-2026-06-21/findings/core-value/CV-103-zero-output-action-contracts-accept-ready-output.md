# CV-103: Zero-output action contracts can accept a ready output

- **Severity**: High
- **Category**: correctness
- **Location**: `crates/vb_core/src/action/validate.rs:226`
- **Confidence**: confirmed

## Description

The action outcome validator treats `output_slot_count == 0` as if every output slot is valid. A contract that declares zero produced outputs can still accept `ActionOutcome::Ready` with an output slot and value.

## Evidence

Ready outcomes validate the output slot against `contract.output_slot_count`:

```rust
check_output_slot_in_bounds(output_ready.output_slot, contract.output_slot_count)?;
```

But the bounds check only rejects out-of-range slots when `max_slots > 0`:

```rust
fn check_output_slot_in_bounds(slot: SlotIdx, max_slots: u16) -> Result<(), ActionError> {
    let slot_raw = slot.get();
    if u32::from(slot_raw) >= u32::from(max_slots) && max_slots > 0 {
        return Err(ActionError::OutputSlotOutOfBounds {
            slot: slot_raw,
            max_slots,
        });
    }
    Ok(())
}
```

Dispatch validation also ignores the contract (`_contract`) and only checks that the output slot exists in the frame:

```rust
pub fn validate_action_dispatch(
    _contract: &ActionContract,
    frame: &RunFrame,
    input_slot: SlotIdx,
    output_slot: SlotIdx,
) -> Result<(), ActionError> {
    ...
    if output_slot.as_usize() >= usize::from(frame.slot_count()) {
        return Err(ActionError::DispatchFailed);
    }
```

## Adversarial Check

`output_slot_count` is documented on `ActionContract` as the number of output slots produced. Zero therefore means no output is permitted, not an unbounded output count. The special `&& max_slots > 0` branch makes the most restrictive contract the least restrictive one.

## Suggested Fix

Reject any ready output when `max_slots == 0`. Also make `validate_action_dispatch` enforce the contract's output count, or document why dispatch output slots are absolute frame slots while outcome output slots are contract-relative.
