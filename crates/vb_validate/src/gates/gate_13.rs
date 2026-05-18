#![forbid(unsafe_code)]
//! Gate 14: Slot type consistency

use crate::{ValidationError, ValidationResult};
use vb_core::action::ActionContract;
use vb_core::capability::Capability;
use vb_core::ids::{AccessorIdx, ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId};
use vb_core::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, ExprOp, ExprProgram, PathSegment,
    WorkflowParts,
};

pub fn validate_gate_14_slot_type_consistency(parts: &WorkflowParts) -> ValidationResult<()> {
    let slot_count = usize::from(parts.slot_count);
    if slot_count == 0 {
        return Ok(());
    }

    let mut slot_const_kind: Vec<u8> = vec![0; slot_count];

    for node in parts.nodes.iter() {
        if let CompiledNodeKind::SetConst { value } = &node.kind {
            let const_idx = value.as_usize();
            if const_idx >= parts.constants.len() {
                continue;
            }
            if let Some(constant) = parts.constants.get(const_idx) {
                let kind = const_value_discriminant(constant);
                if let Some(slot) = node.output {
                    let slot_usize = slot.as_usize();
                    if slot_usize < slot_count {
                        let existing = slot_const_kind
                            .get(slot_usize)
                            .copied()
                            .ok_or(ValidationError::SlotTypeInconsistency { slot: slot_usize })?;
                        if existing == 0 {
                            if let Some(entry) = slot_const_kind.get_mut(slot_usize) {
                                *entry = kind;
                            }
                        } else if existing != kind {
                            return Err(ValidationError::SlotTypeInconsistency {
                                slot: slot_usize,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn const_value_discriminant(value: &vb_core::value::ConstValue) -> u8 {
    match value {
        vb_core::value::ConstValue::Null => 1,
        vb_core::value::ConstValue::Bool(_) => 2,
        vb_core::value::ConstValue::I64(_) => 3,
        vb_core::value::ConstValue::F64(_) => 4,
        vb_core::value::ConstValue::Symbol(_) => 5,
        _ => 0,
    }
}
