use crate::shard::Shard;
use crate::{RuntimeError, RuntimeResult};
use vb_core::action::ActionOutputReady;
use vb_core::{ActionTicket, CompiledNodeKind, SlotValue, Taint};
use vb_storage::executor::RegisterReceipt;

pub(crate) struct RegisteredActionContext {
    pub artifact: vb_storage::AcceptedArtifact,
    pub output: ActionOutputReady,
    pub value: i64,
    pub mode: crate::shard::types::RegisteredExecutionMode,
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
        let artifact = self
            .artifact_store
            .load_accepted_artifact(admission.artifact_digest())
            .map_err(|_| RuntimeError::InvalidActionCompletion)?;
        let node = state
            .workflow
            .node(ticket.step)
            .ok_or(RuntimeError::InvalidActionCompletion)?;
        let CompiledNodeKind::Do { action, input } = node.kind else {
            return Err(RuntimeError::InvalidActionCompletion);
        };
        if action != ticket.action
            || state
                .frame
                .read_taint(input)
                .map_err(|_| RuntimeError::InvalidActionCompletion)?
                != Taint::Clean
        {
            return Err(RuntimeError::InvalidActionCompletion);
        }
        let SlotValue::I64(value) = state
            .frame
            .read_slot(input)
            .map_err(|_| RuntimeError::InvalidActionCompletion)?
        else {
            return Err(RuntimeError::InvalidActionCompletion);
        };
        let output_slot = node.output.ok_or(RuntimeError::InvalidActionCompletion)?;
        let encoded = postcard::to_allocvec(&SlotValue::I64(*value))
            .map_err(|_| RuntimeError::EncodeFailed)?;
        let encoded_len = u32::try_from(encoded.len()).map_err(|_| RuntimeError::EncodeFailed)?;
        let output = ActionOutputReady {
            output_slot,
            value: SlotValue::I64(*value),
            taint: Taint::Clean,
            encoded_len,
        };
        super::preflight_action_completion(state, ticket, output)?;
        Ok(RegisteredActionContext {
            artifact,
            output,
            value: *value,
            mode: state.registered_execution,
        })
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
