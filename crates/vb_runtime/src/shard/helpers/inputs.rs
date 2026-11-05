#![forbid(unsafe_code)]
//! Input seeding helpers for shard operations.

use vb_core::frame::RunFrame;
use vb_core::ids::SlotIdx;
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::CompiledWorkflow;

use crate::{RuntimeError, RuntimeResult};

/// Seeds input slots on a frame before deterministic execution.
pub fn seed_input_slots(
    frame: &mut RunFrame,
    inputs: &[(SlotIdx, SlotValue)],
    workflow: &CompiledWorkflow,
) -> RuntimeResult<()> {
    for (slot, value) in inputs {
        let declared = workflow.input_slots().iter().find(|cs| cs.slot == *slot);
        if let Some(kind) = declared.map(|cs| cs.kind)
            && !slot_value_matches_kind(value, kind)
        {
            return Err(RuntimeError::RuntimeInputSlotKindMismatch {
                slot: *slot,
                expected: kind,
                actual: slot_value_kind(value),
            });
        }
        let taint = if declared.is_some_and(|cs| cs.constraint.secret) {
            Taint::Secret
        } else {
            Taint::Clean
        };
        frame
            .write_slot_with_taint(*slot, *value, taint)
            .map_err(|_| RuntimeError::InvalidRecoveryHydration)?;
    }
    Ok(())
}

fn slot_value_matches_kind(value: &SlotValue, kind: vb_core::InputSlotKind) -> bool {
    if kind == vb_core::InputSlotKind::Any {
        return matches!(
            value,
            SlotValue::Null
                | SlotValue::Bool(_)
                | SlotValue::I64(_)
                | SlotValue::F64(_)
                | SlotValue::Symbol(_)
                | SlotValue::List(_)
                | SlotValue::Object(_)
                | SlotValue::Blob(_)
        );
    }
    matches!(
        (value, kind),
        (SlotValue::Null, vb_core::InputSlotKind::Null)
            | (SlotValue::Bool(_), vb_core::InputSlotKind::Bool)
            | (SlotValue::I64(_), vb_core::InputSlotKind::I64)
            | (SlotValue::F64(_), vb_core::InputSlotKind::F64)
            | (SlotValue::Symbol(_), vb_core::InputSlotKind::Symbol)
            | (SlotValue::List(_), vb_core::InputSlotKind::List)
            | (SlotValue::Object(_), vb_core::InputSlotKind::Object)
            | (SlotValue::Blob(_), vb_core::InputSlotKind::Blob)
    )
}

fn slot_value_kind(value: &SlotValue) -> vb_core::InputSlotKind {
    match value {
        SlotValue::Null => vb_core::InputSlotKind::Null,
        SlotValue::Bool(_) => vb_core::InputSlotKind::Bool,
        SlotValue::I64(_) => vb_core::InputSlotKind::I64,
        SlotValue::F64(_) => vb_core::InputSlotKind::F64,
        SlotValue::Symbol(_) => vb_core::InputSlotKind::Symbol,
        SlotValue::List(_) => vb_core::InputSlotKind::List,
        SlotValue::Object(_) => vb_core::InputSlotKind::Object,
        SlotValue::Blob(_) => vb_core::InputSlotKind::Blob,
        &_ => vb_core::InputSlotKind::Blob,
    }
}
