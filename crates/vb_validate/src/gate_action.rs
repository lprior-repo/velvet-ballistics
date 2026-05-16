//! Gate 12: Action contract completeness
//!
//! Validates that every Do node's action_id has a corresponding entry in the
//! provided action contracts, and that no action contract references a
//! non-existent Do node.

use crate::{ValidationError, ValidationResult};

pub use vb_core::action::ActionContract;
pub use vb_core::workflow::{CompiledNode, CompiledNodeKind, WorkflowParts};

/// Validates that every Do node's action_id has a corresponding entry in the
/// provided action contracts, and that no action contract references a
/// non-existent Do node.
///
/// Gate 12 (contracts): the action contracts table must be in bijection with
/// the set of Do nodes. Every Do node must reference a contracted action, and
/// every contracted action must be used by at least one Do node.
pub fn validate_gate_12_action_contract_completeness(
    parts: &WorkflowParts,
    action_contracts: &[ActionContract],
) -> ValidationResult<()> {
    // Collect all action IDs referenced by Do nodes.
    let mut do_action_ids: Vec<u16> = Vec::new();
    for (node_index, node) in parts.nodes.iter().enumerate() {
        if let CompiledNodeKind::Do { action, .. } = &node.kind {
            let action_val = action.get();
            // Check that this action_id has a corresponding contract.
            let mut found = false;
            for contract in action_contracts {
                if contract.id.get() == action_val {
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(ValidationError::ActionContractMissing {
                    action_id: usize::from(action_val),
                    node_index,
                });
            }
            if !do_action_ids.contains(&action_val) {
                do_action_ids.push(action_val);
            }
        }
    }

    // Check that every contract has at least one Do node referencing it.
    for contract in action_contracts {
        let contract_id = contract.id.get();
        let mut found = false;
        for do_id in &do_action_ids {
            if *do_id == contract_id {
                found = true;
                break;
            }
        }
        if !found {
            return Err(ValidationError::ActionContractOrphan {
                action_id: usize::from(contract_id),
            });
        }
    }

    Ok(())
}
