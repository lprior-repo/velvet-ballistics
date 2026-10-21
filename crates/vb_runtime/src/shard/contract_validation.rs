#![forbid(unsafe_code)]
//! Contract validation helpers for admission with action contracts.
//! Validates that submitted action contracts form a well-formed
//! dense, sorted registry compatible with the production
//! resolve_contract dispatch path.
//!
//! Validation order (first error wins):
//! 1. build_admission (run/digest/capability set construction)
//! 2. Duplicate action IDs (global pass)
//! 3. Empty action names (global pass)
//! 4. Dense registry index == ID (global pass)
//! 5. Capability coverage (global pass)
//! 6. Workflow Do binding (referenced Do actions must have contracts)

use std::collections::HashSet;

use vb_core::action::ActionContract;
use vb_core::ids::ActionId;
use vb_core::workflow::CompiledWorkflow;

use crate::{RuntimeError, RuntimeResult};

/// GLOBAL pass: check for duplicate action IDs across all contracts.
fn check_duplicate_ids(contracts: &[ActionContract]) -> RuntimeResult<()> {
    let mut seen_ids = HashSet::new();
    for contract in contracts.iter() {
        if !seen_ids.insert(contract.id) {
            return Err(RuntimeError::ContractValidationError {
                action: contract.id,
                error: "duplicate action ID".to_string(),
            });
        }
    }
    Ok(())
}

/// GLOBAL pass: check that all action contract names are non-empty.
fn check_empty_names(contracts: &[ActionContract]) -> RuntimeResult<()> {
    for contract in contracts.iter() {
        if contract.name.as_str().is_empty() {
            return Err(RuntimeError::ContractValidationError {
                action: contract.id,
                error: "action name is empty".to_string(),
            });
        }
    }
    Ok(())
}

/// GLOBAL pass: validate dense registry — index must equal action ID.
fn check_dense_index(contracts: &[ActionContract]) -> RuntimeResult<()> {
    for (index, contract) in contracts.iter().enumerate() {
        let expected_id = ActionId::new(u16::try_from(index).map_err(|_| {
            RuntimeError::ContractValidationError {
                action: contract.id,
                error: "contract index exceeds ActionId range".to_string(),
            }
        })?);
        if contract.id != expected_id {
            return Err(RuntimeError::ContractValidationError {
                action: contract.id,
                error: format!(
                    "contract at index {} has ID {} (expected {})",
                    index,
                    contract.id.get(),
                    expected_id.get()
                ),
            });
        }
    }
    Ok(())
}

/// Validates that action contract IDs match their array indices and are
/// well-formed. Runs duplicate, empty-name, and index checks as separate
/// global passes so duplicate detection takes precedence over index errors.
pub fn validate_contract_ids(contracts: &[ActionContract]) -> RuntimeResult<()> {
    check_duplicate_ids(contracts)?;
    check_empty_names(contracts)?;
    check_dense_index(contracts)?;
    Ok(())
}

/// Validates that granted capabilities cover all contract requirements.
pub fn validate_contract_capabilities(
    contracts: &[ActionContract],
    caps: &vb_core::capability::CapabilitySet,
) -> RuntimeResult<()> {
    for contract in contracts.iter() {
        for required in contract.required_capabilities.iter() {
            if !caps.grants(required) {
                return Err(RuntimeError::AdmissionCapabilityDenied {
                    action: contract.id,
                    required: required.clone(),
                    granted: caps.clone(),
                });
            }
        }
    }
    Ok(())
}

/// Validates that every referenced Do action in the workflow has a
/// contract at its exact registry index with matching ID.
pub fn validate_workflow_action_binding(
    workflow: &CompiledWorkflow,
    contracts: &[ActionContract],
) -> RuntimeResult<()> {
    for step in 0u16..workflow.node_count() {
        let step_idx = vb_core::ids::StepIdx::new(step);
        let node = match workflow.node(step_idx) {
            Some(node) => node,
            None => continue,
        };
        if let vb_core::workflow::CompiledNodeKind::Do { action, .. } = node.kind {
            let action_index = usize::from(action.get());
            let contract = match contracts.get(action_index) {
                Some(contract) => contract,
                None => {
                    return Err(RuntimeError::ContractValidationError {
                        action,
                        error: "no contract for workflow Do action".to_string(),
                    });
                }
            };
            if contract.id != action {
                return Err(RuntimeError::ContractValidationError {
                    action,
                    error: "contract ID mismatch for workflow Do action".to_string(),
                });
            }
        }
    }
    Ok(())
}
