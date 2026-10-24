//! Owned root bundle validation.

use crate::errors::CoreError;
use crate::ids::SlotIdx;
use crate::value::{SlotValue, Taint};
use crate::value_store::{ValueStoreCheckpointV1, validate_value_reference};
use crate::workflow::{CompiledWorkflow, InputSlotKind};

use super::errors::{OwnedRootBundleError as Error, ValueKind};

/// Slot tracking state for root validation.
struct Slots {
    seen: Vec<bool>,
    kinds: Vec<Option<InputSlotKind>>,
}

impl Slots {
    fn new(wf: &CompiledWorkflow) -> Result<Self, Error> {
        let count = usize::from(wf.slot_count());
        let mut seen = Vec::new();
        seen.try_reserve_exact(count)
            .map_err(|_| Error::Checkpoint(CoreError::AllocationFailed))?;
        seen.resize(count, false);
        let mut kinds = Vec::new();
        kinds
            .try_reserve_exact(count)
            .map_err(|_| Error::Checkpoint(CoreError::AllocationFailed))?;
        kinds.resize(count, None);
        for input in wf.input_slots() {
            let idx = usize::from(input.slot.get());
            let entry = kinds.get_mut(idx).ok_or(Error::Checkpoint(
                CoreError::InternalInvariantViolation {
                    reason: "owned_root_slots",
                },
            ))?;
            *entry = Some(input.kind);
        }
        Ok(Slots { seen, kinds })
    }
}

fn validate_range(
    roots: &[(SlotIdx, SlotValue, Taint)],
    wf: &CompiledWorkflow,
) -> Result<(), Error> {
    let max = wf.slot_count();
    for (slot, _, _) in roots {
        if slot.get() >= max {
            return Err(Error::RootSlotOutOfRange { slot: *slot, max });
        }
    }
    Ok(())
}

fn validate_duplicates(
    roots: &[(SlotIdx, SlotValue, Taint)],
    slots: &mut Slots,
) -> Result<(), Error> {
    for (slot, _, _) in roots {
        let idx = usize::from(slot.get());
        let entry = slots.seen.get_mut(idx).ok_or(Error::Checkpoint(
            CoreError::InternalInvariantViolation {
                reason: "owned_root_slots",
            },
        ))?;
        if *entry {
            return Err(Error::RootSlotDuplicate { slot: *slot });
        }
        *entry = true;
    }
    Ok(())
}

fn validate_extras(roots: &[(SlotIdx, SlotValue, Taint)], slots: &Slots) -> Result<(), Error> {
    for (slot, _, _) in roots {
        let idx = usize::from(slot.get());
        let entry = slots.kinds.get(idx).ok_or(Error::Checkpoint(
            CoreError::InternalInvariantViolation {
                reason: "owned_root_slots",
            },
        ))?;
        if entry.is_none() {
            return Err(Error::RootSlotExtra { slot: *slot });
        }
    }
    Ok(())
}

fn validate_missing(wf: &CompiledWorkflow, slots: &Slots) -> Result<(), Error> {
    for input in wf.input_slots() {
        let idx = usize::from(input.slot.get());
        let entry = slots.seen.get(idx).ok_or(Error::Checkpoint(
            CoreError::InternalInvariantViolation {
                reason: "owned_root_slots",
            },
        ))?;
        if !*entry {
            return Err(Error::RootSlotMissing { slot: input.slot });
        }
    }
    Ok(())
}

fn kind_matches(expected: InputSlotKind, actual: ValueKind) -> bool {
    match expected {
        InputSlotKind::Any => true,
        InputSlotKind::Null => actual == ValueKind::Null,
        InputSlotKind::Bool => actual == ValueKind::Bool,
        InputSlotKind::I64 => actual == ValueKind::I64,
        InputSlotKind::F64 => actual == ValueKind::F64,
        InputSlotKind::Symbol => actual == ValueKind::Symbol,
        InputSlotKind::List => actual == ValueKind::List,
        InputSlotKind::Object => actual == ValueKind::Object,
        InputSlotKind::Blob => actual == ValueKind::Blob,
    }
}

fn validate_kinds(roots: &[(SlotIdx, SlotValue, Taint)], slots: &Slots) -> Result<(), Error> {
    for (slot, value, _) in roots {
        let idx = usize::from(slot.get());
        let entry = slots.kinds.get(idx).ok_or(Error::Checkpoint(
            CoreError::InternalInvariantViolation {
                reason: "owned_root_slots",
            },
        ))?;
        let expected =
            (*entry).ok_or(Error::Checkpoint(CoreError::InternalInvariantViolation {
                reason: "owned_root_slots",
            }))?;
        let actual = ValueKind::of(*value);
        if !kind_matches(expected, actual) {
            return Err(Error::RootKindMismatch {
                slot: *slot,
                expected,
                actual,
            });
        }
    }
    Ok(())
}

fn validate_references(
    roots: &[(SlotIdx, SlotValue, Taint)],
    cp: &ValueStoreCheckpointV1,
) -> Result<(), Error> {
    for (_, value, _) in roots {
        validate_value_reference(*value, cp).map_err(Error::Checkpoint)?;
    }
    Ok(())
}

pub(super) fn validate(
    roots: &[(SlotIdx, SlotValue, Taint)],
    wf: &CompiledWorkflow,
    cp: &ValueStoreCheckpointV1,
) -> Result<(), Error> {
    validate_range(roots, wf)?;
    let mut slots = Slots::new(wf)?;
    validate_duplicates(roots, &mut slots)?;
    validate_extras(roots, &slots)?;
    validate_missing(wf, &slots)?;
    validate_kinds(roots, &slots)?;
    validate_references(roots, cp)?;
    Ok(())
}
