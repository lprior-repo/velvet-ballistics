#![forbid(unsafe_code)]

//! Verifier-side idempotency contract checks for typed workflow IR.

use thiserror::Error;
use vb_core::action::{ActionContract, Idempotency, RetrySafety, SideEffect};
use vb_core::ids::ActionId;
use vb_core::workflow::{CompiledNodeKind, WorkflowParts};

/// Result type for verifier idempotency contract checks.
pub type IdempotencyContractResult<T> = Result<T, IdempotencyContractError>;

/// Accumulated idempotency contract violations in deterministic traversal order.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("IDEMPOTENCY_CONTRACT_VIOLATIONS")]
pub struct IdempotencyContractErrors(pub Box<[IdempotencyContractViolation]>);

/// Workflow-level idempotency contract failure.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum IdempotencyContractError {
    /// A Do node references an action absent from the workflow registry.
    #[error("ACTION_CONTRACT_MISSING")]
    ActionContractMissing {
        /// Missing action identifier.
        action_id: ActionId,
        /// Referencing node index.
        node_index: usize,
    },
    /// A workflow-specific registry entry is unused by the workflow.
    #[error("ACTION_CONTRACT_ORPHAN")]
    ActionContractOrphan {
        /// Orphan action identifier.
        action_id: ActionId,
    },
    /// One or more side-effecting idempotency declarations are invalid.
    #[error(transparent)]
    IdempotencyViolations(IdempotencyContractErrors),
}

/// Single statically detectable idempotency contract violation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum IdempotencyContractViolation {
    /// Side-effecting actions may not declare retry-unsafe behavior.
    #[error("IDEMPOTENCY_RETRY_UNSAFE")]
    SideEffectingRetryUnsafe {
        /// Violating action identifier.
        action: ActionId,
        /// Declared side-effect class.
        side_effect: SideEffect,
        /// Declared idempotency class.
        idempotency: Idempotency,
        /// Declared retry-safety class.
        retry_safety: RetrySafety,
    },
    /// Side-effecting actions may not declare at-least-once external behavior.
    #[error("IDEMPOTENCY_AT_LEAST_ONCE_EXTERNAL")]
    SideEffectingAtLeastOnceExternal {
        /// Violating action identifier.
        action: ActionId,
        /// Declared side-effect class.
        side_effect: SideEffect,
        /// Declared idempotency class.
        idempotency: Idempotency,
        /// Declared retry-safety class.
        retry_safety: RetrySafety,
    },
    /// Side-effecting actions may not declare deterministic-pure semantics.
    #[error("IDEMPOTENCY_SIDE_EFFECTING_DETERMINISTIC_PURE")]
    SideEffectingDeterministicPure {
        /// Violating action identifier.
        action: ActionId,
        /// Declared side-effect class.
        side_effect: SideEffect,
        /// Declared idempotency class.
        idempotency: Idempotency,
        /// Declared retry-safety class.
        retry_safety: RetrySafety,
    },
}

impl IdempotencyContractViolation {
    /// Stable machine-readable diagnostic category.
    #[must_use]
    pub const fn reason_category(&self) -> &'static str {
        match self {
            Self::SideEffectingRetryUnsafe { .. } => "IDEMPOTENCY_RETRY_UNSAFE",
            Self::SideEffectingAtLeastOnceExternal { .. } => "IDEMPOTENCY_AT_LEAST_ONCE_EXTERNAL",
            Self::SideEffectingDeterministicPure { .. } => {
                "IDEMPOTENCY_SIDE_EFFECTING_DETERMINISTIC_PURE"
            }
        }
    }
}

/// Validates workflow-specific contract completeness, then idempotency legality.
pub fn validate_workflow_idempotency_contracts(
    parts: &WorkflowParts,
    action_contracts: &[ActionContract],
) -> IdempotencyContractResult<()> {
    ensure_contract_completeness(parts, action_contracts)?;
    collect_workflow_idempotency_violations(parts, action_contracts)
        .map_err(IdempotencyContractError::IdempotencyViolations)
}

/// Validates one action contract against the static idempotency decision table.
pub fn validate_action_idempotency_contract(
    contract: &ActionContract,
) -> Result<(), IdempotencyContractViolation> {
    is_statically_idempotent_contract(contract)
}

/// Collects all idempotency violations in input contract traversal order.
pub fn collect_idempotency_contract_violations(
    action_contracts: &[ActionContract],
) -> Result<(), IdempotencyContractErrors> {
    errors_from_violations(
        action_contracts
            .iter()
            .filter_map(|contract| is_statically_idempotent_contract(contract).err()),
    )
}

/// Returns whether a single action contract is statically idempotent.
pub fn is_statically_idempotent_contract(
    contract: &ActionContract,
) -> Result<(), IdempotencyContractViolation> {
    match (
        contract.side_effect,
        contract.retry_safety,
        contract.idempotency,
    ) {
        (SideEffect::None, _, _) => Ok(()),
        (side_effect, RetrySafety::Unsafe, idempotency) => {
            Err(IdempotencyContractViolation::SideEffectingRetryUnsafe {
                action: contract.id,
                side_effect,
                idempotency,
                retry_safety: RetrySafety::Unsafe,
            })
        }
        (side_effect, retry_safety, Idempotency::AtLeastOnceExternal) => Err(
            IdempotencyContractViolation::SideEffectingAtLeastOnceExternal {
                action: contract.id,
                side_effect,
                idempotency: Idempotency::AtLeastOnceExternal,
                retry_safety,
            },
        ),
        (side_effect, retry_safety, Idempotency::DeterministicPure) => Err(
            IdempotencyContractViolation::SideEffectingDeterministicPure {
                action: contract.id,
                side_effect,
                idempotency: Idempotency::DeterministicPure,
                retry_safety,
            },
        ),
        (_, RetrySafety::Safe | RetrySafety::KeyRequired, Idempotency::IdempotentExternal) => {
            Ok(())
        }
    }
}

fn ensure_contract_completeness(
    parts: &WorkflowParts,
    action_contracts: &[ActionContract],
) -> IdempotencyContractResult<()> {
    match first_missing_contract(parts, action_contracts) {
        Some(error) => Err(error),
        None => match first_orphan_contract(parts, action_contracts) {
            Some(error) => Err(error),
            None => Ok(()),
        },
    }
}

fn first_missing_contract(
    parts: &WorkflowParts,
    action_contracts: &[ActionContract],
) -> Option<IdempotencyContractError> {
    parts
        .nodes
        .iter()
        .enumerate()
        .find_map(|(node_index, node)| {
            do_action(&node.kind).and_then(|action_id| {
                has_contract(action_contracts, action_id)
                    .then_some(())
                    .map_or_else(
                        || {
                            Some(IdempotencyContractError::ActionContractMissing {
                                action_id,
                                node_index,
                            })
                        },
                        |_| None,
                    )
            })
        })
}

fn first_orphan_contract(
    parts: &WorkflowParts,
    action_contracts: &[ActionContract],
) -> Option<IdempotencyContractError> {
    action_contracts.iter().find_map(|contract| {
        has_do_action(parts, contract.id).then_some(()).map_or_else(
            || {
                Some(IdempotencyContractError::ActionContractOrphan {
                    action_id: contract.id,
                })
            },
            |_| None,
        )
    })
}

fn collect_workflow_idempotency_violations(
    parts: &WorkflowParts,
    action_contracts: &[ActionContract],
) -> Result<(), IdempotencyContractErrors> {
    errors_from_violations(parts.nodes.iter().filter_map(|node| {
        do_action(&node.kind)
            .and_then(|action_id| find_contract(action_contracts, action_id))
            .and_then(|contract| is_statically_idempotent_contract(contract).err())
    }))
}

fn errors_from_violations(
    violations: impl Iterator<Item = IdempotencyContractViolation>,
) -> Result<(), IdempotencyContractErrors> {
    let collected: Box<[_]> = violations.collect();
    if collected.is_empty() {
        Ok(())
    } else {
        Err(IdempotencyContractErrors(collected))
    }
}

fn do_action(kind: &CompiledNodeKind) -> Option<ActionId> {
    match kind {
        CompiledNodeKind::Do { action, .. } => Some(*action),
        _ => None,
    }
}

fn has_contract(action_contracts: &[ActionContract], action_id: ActionId) -> bool {
    action_contracts
        .iter()
        .any(|contract| contract.id == action_id)
}

fn find_contract(
    action_contracts: &[ActionContract],
    action_id: ActionId,
) -> Option<&ActionContract> {
    action_contracts
        .iter()
        .find(|contract| contract.id == action_id)
}

fn has_do_action(parts: &WorkflowParts, action_id: ActionId) -> bool {
    parts
        .nodes
        .iter()
        .filter_map(|node| do_action(&node.kind))
        .any(|do_action_id| do_action_id == action_id)
}
