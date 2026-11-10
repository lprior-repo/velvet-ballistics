#![forbid(unsafe_code)]

use crate::{ValidationError, ValidationResult};
use vb_core::workflow::{CompiledNodeKind, WorkflowParts};

// ---------------------------------------------------------------------------
// Gate 14: Slot type consistency
// ---------------------------------------------------------------------------

/// Validates that slots written by multiple nodes have compatible types.
///
/// Gate 14 (types): when multiple `SetConst` nodes write to the same slot, they
/// must write compatible `ConstValue` types. Two `ConstValue` variants are
/// compatible if they share the same discriminant (e.g., both I64, or both
/// Bool). This catches cases where the same slot would receive an I64 from one
/// writer and a Bool from another.
pub fn validate_gate_14_slot_type_consistency(parts: &WorkflowParts) -> ValidationResult<()> {
    let slot_count = usize::from(parts.slot_count);
    if slot_count == 0 {
        return Ok(());
    }

    // For each slot, track the ConstValue discriminant written by SetConst nodes.
    // 0 = unset, 1 = Null, 2 = Bool, 3 = I64, 4 = F64, 5 = Symbol
    let mut slot_const_kind: Vec<u8> = vec![0; slot_count];

    for node in parts.nodes.iter() {
        if let CompiledNodeKind::SetConst { value } = &node.kind {
            let const_idx = value.as_usize();
            if const_idx >= parts.constants.len() {
                // Out-of-range const index; that is caught by gate 10.
                continue;
            }
            if let Some(constant) = parts.constants.get(const_idx) {
                let kind = const_value_discriminant(constant);
                if let Some(slot) = node.output.slot() {
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

/// Returns a discriminant tag for a ConstValue variant.
fn const_value_discriminant(value: &vb_core::value::ConstValue) -> u8 {
    match value {
        vb_core::value::ConstValue::Null => 1,
        vb_core::value::ConstValue::Bool(_) => 2,
        vb_core::value::ConstValue::I64(_) => 3,
        vb_core::value::ConstValue::F64(_) => 4,
        vb_core::value::ConstValue::Symbol(_) => 5,
        // `ConstValue` is `#[non_exhaustive]`; return 0 for unknown
        // variants (does not conflict with known discriminant values).
        _ => 0,
    }
}
