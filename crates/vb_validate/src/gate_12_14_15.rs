#![forbid(unsafe_code)]
//! Gates 12, 14, 15: Action contract, slot type consistency, determinism proof.

#![allow(unreachable_pub)]
#![allow(clippy::collapsible_if)]

use crate::{ValidationError, ValidationResult};
use vb_core::action::ActionContract;
use vb_core::workflow::{CompiledNodeKind, WorkflowParts};

pub fn validate_gate_12_action_contract_completeness(
    parts: &WorkflowParts,
    action_contracts: &[ActionContract],
) -> ValidationResult<()> {
    let mut do_action_ids: Vec<u16> = Vec::new();
    for (node_index, node) in parts.nodes.iter().enumerate() {
        if let CompiledNodeKind::Do { action, .. } = &node.kind {
            let action_val = action.get();
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
    for contract in action_contracts {
        let cid = contract.id.get();
        let mut found = false;
        for do_id in &do_action_ids {
            if *do_id == cid {
                found = true;
                break;
            }
        }
        if !found {
            return Err(ValidationError::ActionContractOrphan {
                action_id: usize::from(cid),
            });
        }
    }
    Ok(())
}

pub fn validate_gate_14_slot_type_consistency(parts: &WorkflowParts) -> ValidationResult<()> {
    crate::gates::validate_gate_14_slot_type_consistency(parts)
}

pub fn validate_gate_15_determinism_proof(parts: &WorkflowParts) -> ValidationResult<()> {
    crate::gate_15_resume::validate_resume_targets(parts)
}

#[cfg(test)]
#[path = "gate_12_14_15/tests.rs"]
mod tests;
