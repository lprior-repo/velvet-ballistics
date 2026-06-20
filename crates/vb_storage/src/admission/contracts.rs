#![forbid(unsafe_code)]
//! Contract extraction: derive capabilities and idempotency evidence from action contracts.

use crate::error::JournalError;
use vb_core::action::{ActionContract, Idempotency, RetrySafety, SideEffect};

/// Bundles evidence extracted from action contracts for admission.
#[derive(Debug, Clone)]
pub(crate) struct AdmissionInputs {
    pub(crate) required_capabilities: Box<[vb_core::capability::Capability]>,
    pub(crate) idempotency_evidence: IdempotencyEvidence,
}

/// Evidence about idempotency characteristics extracted from action contracts.
#[derive(Debug, Clone)]
pub(crate) struct IdempotencyEvidence {
    pub(crate) keyed: Box<[vb_core::ActionId]>,
    pub(crate) attested: Box<[vb_core::ActionId]>,
}

/// Extracts admission inputs from a slice of action contracts.
pub(crate) fn admission_inputs_from_contracts(
    action_contracts: &[ActionContract],
) -> Result<AdmissionInputs, JournalError> {
    Ok(AdmissionInputs {
        required_capabilities: required_capabilities_from_contracts(action_contracts)?,
        idempotency_evidence: idempotency_evidence_from_contracts(action_contracts)?,
    })
}

/// Collects required capabilities from all action contracts.
pub(crate) fn required_capabilities_from_contracts(
    action_contracts: &[ActionContract],
) -> Result<Box<[vb_core::capability::Capability]>, JournalError> {
    let mut total = 0usize;
    for contract in action_contracts {
        total = total
            .checked_add(contract.required_capabilities.len())
            .ok_or(JournalError::ArtifactMalformed)?;
    }
    let mut required = Vec::new();
    required
        .try_reserve(total)
        .map_err(|_| JournalError::ArtifactMalformed)?;
    for contract in action_contracts {
        for capability in contract.required_capabilities.iter() {
            required.push(capability.clone());
        }
    }
    Ok(required.into_boxed_slice())
}

/// Extracts idempotency evidence (keyed and attested action IDs) from contracts.
fn idempotency_evidence_from_contracts(
    action_contracts: &[ActionContract],
) -> Result<IdempotencyEvidence, JournalError> {
    let keyed = action_contracts
        .iter()
        .filter(|contract| requires_idempotency_key(contract))
        .map(|contract| contract.id)
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let attested = action_contracts
        .iter()
        .map(attested_action_id)
        .collect::<Result<Vec<_>, _>>()?
        .into_boxed_slice();
    Ok(IdempotencyEvidence { keyed, attested })
}

/// Determines whether a contract requires an idempotency key.
fn requires_idempotency_key(contract: &ActionContract) -> bool {
    matches!(
        (contract.retry_safety, contract.idempotency),
        (RetrySafety::RequiresIdempotencyKey, _) | (_, Idempotency::AtLeastOnceExternal)
    )
}

/// Returns the action ID if the contract's idempotency is accepted.
fn attested_action_id(contract: &ActionContract) -> Result<vb_core::ActionId, JournalError> {
    is_contract_idempotency_accepted(contract)
        .then_some(contract.id)
        .ok_or(JournalError::ArtifactMalformed)
}

/// Determines whether a contract's idempotency profile is accepted.
pub(crate) fn is_contract_idempotency_accepted(contract: &ActionContract) -> bool {
    match (
        contract.side_effect,
        contract.retry_safety,
        contract.idempotency,
    ) {
        (SideEffect::Pure, _, _) => true,
        (_, RetrySafety::NotRetrySafe, _) => false,
        (_, _, Idempotency::AtLeastOnceExternal | Idempotency::DeterministicPure) => false,
        (
            _,
            RetrySafety::Idempotent | RetrySafety::RequiresIdempotencyKey,
            Idempotency::IdempotentExternal,
        ) => true,
        // `SideEffect`, `RetrySafety`, and `Idempotency` are all `#[non_exhaustive]`.
        // Unknown combinations are conservatively rejected.
        _ => false,
    }
}
