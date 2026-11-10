//! Owned root bundle validation.

use crate::errors::CoreError;
use crate::ids::SlotIdx;
use crate::value::{SlotValue, Taint};
use crate::value_store::{ValueStoreCheckpointV1, validate_value_reference};
use crate::workflow::{CompiledInputSlot, CompiledWorkflow, InputSlotKind};

use super::errors::{OwnedRootBundleError as Error, ValueKind};

/// Slot tracking state for root validation.
struct Slots {
    seen: Vec<bool>,
    kinds: Vec<Option<CompiledInputSlot>>,
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
            *entry = Some(*input);
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
        if !*entry && input.constraint.requires_value() {
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
        if !(*value == SlotValue::Null && expected.constraint.nullable)
            && !kind_matches(expected.kind, actual)
        {
            return Err(Error::RootKindMismatch {
                slot: *slot,
                expected: expected.kind,
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
    for (slot, value, _) in roots {
        let declaration = slots
            .kinds
            .get(usize::from(slot.get()))
            .and_then(Option::as_ref)
            .ok_or(Error::RootSlotExtra { slot: *slot })?;
        crate::input_validation::validate_input_value(
            declaration,
            *value,
            wf.symbols_count(),
            Some(cp),
        )
        .map_err(|issue| Error::RootConstraintMismatch { slot: *slot, issue })?;
    }
    Ok(())
}

pub(super) fn fill_defaults(
    roots: Box<[(SlotIdx, SlotValue, Taint)]>,
    workflow: &CompiledWorkflow,
) -> Result<Box<[(SlotIdx, SlotValue, Taint)]>, Error> {
    let mut roots = roots.into_vec();
    roots
        .try_reserve_exact(workflow.input_slots().len())
        .map_err(|_| Error::Checkpoint(CoreError::AllocationFailed))?;
    for declaration in workflow.input_slots() {
        if roots.iter().any(|(slot, _, _)| *slot == declaration.slot) {
            continue;
        }
        if let Some(value) = declaration.constraint.default_value {
            let taint = if declaration.constraint.secret {
                Taint::Secret
            } else {
                Taint::Clean
            };
            roots.push((declaration.slot, value, taint));
        }
    }
    Ok(roots.into_boxed_slice())
}
