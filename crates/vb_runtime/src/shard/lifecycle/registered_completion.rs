use crate::shard::Shard;
use crate::{RuntimeError, RuntimeResult};
use vb_core::ActionTicket;
use vb_core::Taint;
use vb_core::action::ActionOutputReady;
use vb_core::policy::PinnedAdmissionPolicy;
use vb_storage::executor::RegisterReceipt;

use super::registered_completion_context::{
    build_registered_action_context, convert_policy_pin, load_admitted_artifact,
};

pub(crate) struct RegisteredActionContext {
    pub artifact: vb_storage::AcceptedArtifact,
    pub output: ActionOutputReady,
    pub value: i64,
    pub mode: crate::shard::types::RegisteredExecutionMode,
    /// Admission policy pin converted from RunPolicyPin.
    pub admission_pin: PinnedAdmissionPolicy,
    /// Canonical serialized bytes of the action input value.
    pub encoded_bytes: Vec<u8>,
    /// Actual frame input taint at evaluation time.
    pub input_taint: Taint,
}

impl Shard {
    pub(crate) fn registered_action_context(
        &self,
        ticket: ActionTicket,
    ) -> RuntimeResult<RegisteredActionContext> {
        self.require_pending_action_ownership(ticket)?;
        let state = self
            .run_state_get(ticket.run)
            .ok_or(RuntimeError::RunNotFound)?;
        let admission = state
            .admission
            .as_ref()
            .ok_or(RuntimeError::InvalidActionCompletion)?;
        let artifact =
            load_admitted_artifact(self.artifact_store.as_ref(), admission.artifact_digest())?;
        let admission_pin = convert_policy_pin(&admission.policy_pin());
        build_registered_action_context(state, ticket, artifact, admission_pin)
    }

    pub(crate) fn reject_unregistered_completion(&self, ticket: ActionTicket) -> RuntimeResult<()> {
        if self.artifact_store.executor_catalog().is_none() {
            return Ok(());
        }
        let Some(state) = self.run_state_get(ticket.run) else {
            return Ok(());
        };
        let Some(admission) = &state.admission else {
            return Ok(());
        };
        let artifact = self
            .artifact_store
            .load_accepted_artifact(admission.artifact_digest())
            .map_err(|_| RuntimeError::InvalidActionCompletion)?;
        if let vb_core::artifact::ArtifactEvidence::Packed(packed) = artifact.verification.evidence
            && packed.packs.iter().any(|pack| pack.action == ticket.action)
        {
            return Err(RuntimeError::InvalidActionCompletion);
        }
        Ok(())
    }

    pub(crate) fn handle_registered_action_completion(
        &mut self,
        ticket: ActionTicket,
        receipt: RegisterReceipt,
        output: ActionOutputReady,
    ) -> RuntimeResult<()> {
        let context = self.registered_action_context(ticket)?;
        let catalog = self
            .artifact_store
            .executor_catalog()
            .ok_or(RuntimeError::InvalidActionCompletion)?;
        catalog
            .validate_receipt(&context.artifact, ticket, context.value, &receipt)
            .map_err(|_| RuntimeError::InvalidActionCompletion)?;
        if output != context.output || receipt.value() != context.value {
            return Err(RuntimeError::InvalidActionCompletion);
        }
        self.apply_action_completion(ticket, output)
    }
}
