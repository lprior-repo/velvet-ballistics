//! Gate 14: Slot type consistency
//! Gate 15: Determinism proof
//!
//! Gate 14 validates that slots written by multiple nodes have compatible types.
//! Gate 15 validates that every path between two non-deterministic nodes
//! consists only of deterministic nodes.

use crate::{ValidationError, ValidationResult};

pub use vb_core::workflow::{CompiledNode, CompiledNodeKind, WorkflowParts};
pub use vb_core::ids::SlotIdx;

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
                if let Some(slot) = node.output {
                    let slot_usize = slot.as_usize();
                    if slot_usize < slot_count {
                        let existing = slot_const_kind.get(slot_usize).copied().ok_or(
                            ValidationError::SlotTypeInconsistency {
                                slot: slot_usize,
                            },
                        )?;
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
    }
}

/// Validates that every path between two non-deterministic nodes consists only
/// of deterministic nodes, ensuring that the workflow can be faithfully
/// replayed from journal evidence.
///
/// Gate 15 (determinism): non-deterministic nodes (Do/Action, Ask) are
/// suspension points. All deterministic nodes (SetConst, Copy, EvalExpr,
/// BuildObject, BuildList, Finish, Nop) can be replayed from journal evidence.
/// This gate checks that between any two non-deterministic nodes on a control
/// flow path, there are only deterministic nodes. A consecutive pair of
/// non-deterministic nodes without an intervening deterministic-only region is
/// flagged as an error because the second node's effects cannot be separated
/// from the first's non-determinism in the journal.
///
/// Simplified: for each node, if it is non-deterministic, check that its `next`
/// target (if any) is either deterministic or a valid suspension join. Two
/// non-deterministic nodes may not be directly chained.
pub fn validate_gate_15_determinism_proof(parts: &WorkflowParts) -> ValidationResult<()> {
    let node_count = parts.nodes.len();

    for (node_index, node) in parts.nodes.iter().enumerate() {
        if !is_non_deterministic(&node.kind) {
            continue;
        }

        // Walk the `next` chain from this node. If we encounter another
        // non-deterministic node without any intervening deterministic-only
        // nodes, that is a violation.
        if let Some(next_step) = node.next {
            let next_usize = next_step.as_usize();
            if next_usize < node_count {
                if let Some(next_node) = parts.nodes.get(next_usize) {
                    if is_non_deterministic(&next_node.kind) {
                        return Err(ValidationError::NonDeterministicPath {
                            from_node: node_index,
                            to_node: next_usize,
                        });
                    }
                }
            }
        }
    }

    Ok(())
}

/// Returns true if the node kind is non-deterministic (requires external input
/// that cannot be replayed from journal evidence alone).
fn is_non_deterministic(kind: &CompiledNodeKind) -> bool {
    matches!(kind, CompiledNodeKind::Do { .. } | CompiledNodeKind::Ask { .. })
}
