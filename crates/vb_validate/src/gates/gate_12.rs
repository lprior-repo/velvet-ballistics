#![forbid(unsafe_code)]
//! Gate 12: Action contract completeness

use std::collections::HashSet;

use crate::{ValidationError, ValidationResult};
use vb_core::action::ActionContract;
use vb_core::capability::Capability;
use vb_core::ids::ActionId;
use vb_core::workflow::{CompiledNodeKind, WorkflowParts};

pub const MAX_CAPABILITY_NAME_BYTES: usize = 128;

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
        validate_action_contract_capability_schema(contract)?;
    }

    for contract in action_contracts {
        let contract_id = contract.id.get();
        if !do_action_ids.contains(&contract_id) {
            return Err(ValidationError::ActionContractOrphan {
                action_id: usize::from(contract_id),
            });
        }
    }

    Ok(())
}

fn validate_action_contract_capability_schema(contract: &ActionContract) -> ValidationResult<()> {
    for (capability_index, capability) in contract.required_capabilities.iter().enumerate() {
        validate_required_capability(contract.id, capability_index, capability)?;
    }
    validate_no_duplicate_capability_requirements(contract)
}

fn validate_required_capability(
    contract_action: ActionId,
    capability_index: usize,
    capability: &Capability,
) -> ValidationResult<()> {
    validate_capability_name(contract_action, capability_index, capability.name())?;
    if capability.action_id() != contract_action {
        return Err(ValidationError::CapabilityActionMismatch {
            contract_action_id: usize::from(contract_action.get()),
            capability_action_id: usize::from(capability.action_id().get()),
            capability_index,
        });
    }
    Ok(())
}

fn validate_capability_name(
    action_id: ActionId,
    capability_index: usize,
    name: &str,
) -> ValidationResult<()> {
    let len = name.len();
    if len == 0 {
        return Err(ValidationError::CapabilityNameEmpty {
            action_id: usize::from(action_id.get()),
            capability_index,
        });
    }
    if len > MAX_CAPABILITY_NAME_BYTES {
        return Err(ValidationError::CapabilityNameTooLong {
            action_id: usize::from(action_id.get()),
            capability_index,
            len,
            max: MAX_CAPABILITY_NAME_BYTES,
        });
    }
    if !is_capability_name_grammar_valid(name) {
        return Err(ValidationError::CapabilityNameInvalid {
            action_id: usize::from(action_id.get()),
            capability_index,
            name: name.to_owned(),
        });
    }
    Ok(())
}

fn is_capability_name_grammar_valid(name: &str) -> bool {
    name.bytes()
        .try_fold(true, |segment_start, byte| match byte {
            b'.' => (!segment_start).then_some(true),
            b'a'..=b'z' => Some(false),
            b'0'..=b'9' | b'_' => (!segment_start).then_some(false),
            _ => None,
        })
        == Some(false)
}

fn validate_no_duplicate_capability_requirements(
    contract: &ActionContract,
) -> ValidationResult<()> {
    contract
        .required_capabilities
        .iter()
        .enumerate()
        .find_map(|(duplicate_index, duplicate)| {
            contract
                .required_capabilities
                .iter()
                .take(duplicate_index)
                .enumerate()
                .find(|(_, first)| {
                    first.action_id() == duplicate.action_id() && first.name() == duplicate.name()
                })
                .map(|(first_index, _)| ValidationError::CapabilityDuplicate {
                    action_id: usize::from(contract.id.get()),
                    first_index,
                    duplicate_index,
                    name: duplicate.name().to_owned(),
                })
        })
        .map_or(Ok(()), Err)
}
