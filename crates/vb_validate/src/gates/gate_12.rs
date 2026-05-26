#![forbid(unsafe_code)]
//! Gate 12: Action contract completeness

use crate::{ValidationError, ValidationResult};
use vb_core::action::ActionContract;
use vb_core::capability::Capability;
use vb_core::ids::{AccessorIdx, ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId};
use vb_core::workflow::{
use vb_core::span::Span;
    AccessorProgram, CompiledNode, CompiledNodeKind, ExprOp, ExprProgram, PathSegment,
    WorkflowParts,
};

pub const MAX_CAPABILITY_NAME_BYTES: usize = 128;

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
                 span: Span::ZERO});
            }
            if !do_action_ids.contains(&action_val) {
                do_action_ids.push(action_val);
            }
        }
    }

    for contract in action_contracts {
        validate_action_contract_capability_schema(contract)?;
    }

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
             span: Span::ZERO});
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
         span: Span::ZERO});
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
         span: Span::ZERO});
    }
    if len > MAX_CAPABILITY_NAME_BYTES {
        return Err(ValidationError::CapabilityNameTooLong {
            action_id: usize::from(action_id.get()),
            capability_index,
            len,
            max: MAX_CAPABILITY_NAME_BYTES,
         span: Span::ZERO});
    }
    if !is_capability_name_grammar_valid(name) {
        return Err(ValidationError::CapabilityNameInvalid {
            action_id: usize::from(action_id.get()),
            capability_index,
            name: name.to_owned(),
         span: Span::ZERO});
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
                 span: Span::ZERO})
        })
        .map_or(Ok(()), Err)
}
