//! Focused helpers for registered action completion context construction.
//!
//! Extracted from `Shard::registered_action_context` to keep the caller
//! under 25 logical lines while preserving exact artifact loading,
//! policy pin conversion, and context assembly semantics.

use crate::admission::AcceptedArtifactStore;
use crate::shard::types::RunState;
use crate::{RuntimeError, RuntimeResult};
use vb_core::action::ActionTicket;
use vb_core::policy::{PinnedAdmissionPolicy, PolicyDigest};
use vb_storage::AcceptedArtifact;

/// Load the accepted artifact for a registered action admission,
/// mapping storage errors to the action completion error type.
pub(super) fn load_admitted_artifact(
    store: &dyn AcceptedArtifactStore,
    digest: vb_core::WorkflowDigest,
) -> RuntimeResult<AcceptedArtifact> {
    store
        .load_accepted_artifact(digest)
        .map_err(|_| RuntimeError::InvalidActionCompletion)
}

/// Convert a RunPolicyPin to the policy admission policy enum.
/// Pinned digests are converted to actual artifact digests;
/// Unverified is preserved without fabrication.
pub(super) fn convert_policy_pin(pin: &vb_storage::RunPolicyPin) -> PinnedAdmissionPolicy {
    match pin {
        vb_storage::RunPolicyPin::Pinned(digest) => {
            PinnedAdmissionPolicy::Artifact(PolicyDigest::from_bytes(digest.as_bytes()))
        }
        vb_storage::RunPolicyPin::Unverified => PinnedAdmissionPolicy::Unverified,
    }
}

/// Construct the full registered action context from resolved components.
pub(super) fn build_registered_action_context(
    state: &RunState,
    ticket: ActionTicket,
    artifact: AcceptedArtifact,
    policy_pin: PinnedAdmissionPolicy,
) -> RuntimeResult<super::registered_completion::RegisteredActionContext> {
    let (value, input_taint) =
        super::registered_context_helpers::resolve_action_input(state, ticket)?;
    let encoded = super::registered_context_helpers::encode_action_input(value)?;
    let output =
        super::registered_context_helpers::build_action_output(state, ticket, value, &encoded)?;
    super::preflight_action_completion(state, ticket, output)?;
    Ok(super::registered_completion::RegisteredActionContext {
        artifact,
        output,
        value,
        mode: state.registered_execution,
        admission_pin: policy_pin,
        encoded_bytes: encoded,
        input_taint,
    })
}
