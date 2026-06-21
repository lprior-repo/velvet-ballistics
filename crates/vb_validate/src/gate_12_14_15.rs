#![forbid(unsafe_code)]
//! Gates 12, 14, 15: Action contract, slot type consistency, determinism proof.

#![allow(unreachable_pub)]
#![allow(clippy::collapsible_if)]

use std::collections::HashSet;

use crate::{ValidationError, ValidationResult};
use vb_core::action::ActionContract;
use vb_core::workflow::{CompiledNodeKind, WorkflowParts};

pub fn validate_gate_12_action_contract_completeness(
    parts: &WorkflowParts,
    action_contracts: &[ActionContract],
) -> ValidationResult<()> {
    let mut do_action_ids: HashSet<u16> = HashSet::new();
    for (node_index, node) in parts.nodes.iter().enumerate() {
        if let CompiledNodeKind::Do { action, .. } = &node.kind {
            let action_val = action.get();
            let has_contract = action_contracts
                .iter()
                .any(|contract| contract.id.get() == action_val);
            if !has_contract {
                return Err(ValidationError::ActionContractMissing {
                    action_id: usize::from(action_val),
                    node_index,
                });
            }
            do_action_ids.insert(action_val);
        }
    }
    for contract in action_contracts {
        let cid = contract.id.get();
        if !do_action_ids.contains(&cid) {
            return Err(ValidationError::ActionContractOrphan {
                action_id: usize::from(cid),
            });
        }
    }
    Ok(())
}

pub fn validate_gate_14_slot_type_consistency(parts: &WorkflowParts) -> ValidationResult<()> {
    let slot_count = usize::from(parts.slot_count);
    if slot_count == 0 {
        return Ok(());
    }
    let mut slot_const_kind: Vec<u8> = vec![0; slot_count];
    for node in &parts.nodes {
        let CompiledNodeKind::SetConst { value } = &node.kind else {
            continue;
        };
        let cidx = value.as_usize();
        let Some(constant) = parts.constants.get(cidx) else {
            continue;
        };
        let kind = const_value_discriminant(constant);
        let Some(slot) = node.output else {
            continue;
        };
        let su = slot.as_usize();
        if su >= slot_count {
            continue;
        }
        let existing = slot_const_kind
            .get(su)
            .copied()
            .ok_or(ValidationError::SlotTypeInconsistency { slot: su })?;
        if existing == 0 {
            if let Some(e) = slot_const_kind.get_mut(su) {
                *e = kind;
            }
        } else if existing != kind {
            return Err(ValidationError::SlotTypeInconsistency { slot: su });
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

pub fn validate_gate_15_determinism_proof(parts: &WorkflowParts) -> ValidationResult<()> {
    let node_count = parts.nodes.len();
    for (node_index, node) in parts.nodes.iter().enumerate() {
        if !is_non_deterministic(&node.kind) {
            continue;
        }
        let Some(next_step) = node.next else {
            continue;
        };
        let nu = next_step.as_usize();
        if nu >= node_count {
            continue;
        }
        let Some(next_node) = parts.nodes.get(nu) else {
            continue;
        };
        if is_non_deterministic(&next_node.kind) {
            return Err(ValidationError::NonDeterministicPath {
                from_node: node_index,
                to_node: nu,
            });
        }
    }
    Ok(())
}

fn is_non_deterministic(kind: &CompiledNodeKind) -> bool {
    matches!(
        kind,
        CompiledNodeKind::Do { .. } | CompiledNodeKind::Ask { .. }
    )
}

#[cfg(test)]
#[path = "gate_12_14_15/tests.rs"]
mod tests;
