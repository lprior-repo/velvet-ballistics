use super::Runtime;
use super::live_policy_dispatch::AdmissionEvalContext;
use crate::{RuntimeError, shard::ShardCommand};
use vb_core::policy::DecisionRecord;
use vb_core::{ActionTicket, RunId};
use vb_storage::executor::{ExecutorError, FjallRegister, RegisterGrant, RegisterReceipt};

/// Runtime authority failures and uncertain adapter outcomes remain distinct.
#[derive(Debug, thiserror::Error)]
pub enum RegisteredExecutionError {
    /// Runtime ticket/state or completion queue failed.
    #[error("registered runtime: {0}")]
    Runtime(#[from] RuntimeError),
    /// Executor rejected authority or requires receipt reconciliation.
    #[error("registered executor: {0}")]
    Executor(#[from] ExecutorError),
    /// Live policy rejected execution before any adapter effect.
    #[error("policy denied execution: {0:?}")]
    PolicyDenied(DecisionRecord),
}

impl Runtime {
    /// Execute only a canonical pending action with its actual live I64 input.
    /// A post-effect queue failure preserves the durable receipt for reconciliation.
    pub fn execute_registered_action(
        &mut self,
        adapter: &mut FjallRegister,
        run: RunId,
        wire_step: u64,
        grant: &RegisterGrant,
    ) -> Result<RegisterReceipt, RegisteredExecutionError> {
        let (ticket, context) = self.registered_context(run, wire_step, grant)?;
        if context.mode != crate::shard::types::RegisteredExecutionMode::Live {
            return Err(ExecutorError::Unauthorized.into());
        }
        let eval_ctx = AdmissionEvalContext {
            ticket: ticket.clone(),
            admission_pin: context.admission_pin,
            encoded_bytes: context.encoded_bytes.clone(),
            input_taint: context.input_taint,
        };
        self.evaluate_admission_and_persist(eval_ctx)?;
        let receipt = adapter.execute(grant, ticket, context.value)?;
        self.enqueue_registered_completion(ticket, context.output, receipt.clone())?;
        Ok(receipt)
    }

    /// Recovery reads an existing receipt and never invokes a production effect.
    /// Missing receipts and Blocked recovery fail before any target write.
    pub fn reconcile_registered_action(
        &mut self,
        adapter: &mut FjallRegister,
        run: RunId,
        wire_step: u64,
        grant: &RegisterGrant,
    ) -> Result<RegisterReceipt, RegisteredExecutionError> {
        let (ticket, context) = self.registered_context(run, wire_step, grant)?;
        let catalog = self
            .journal
            .executor_catalog()
            .ok_or(ExecutorError::CatalogRequired)?;
        catalog.require_receipt_recovery(&context.artifact, ticket.action)?;
        let receipt = adapter.reconcile(grant, ticket, context.value)?;
        self.enqueue_registered_completion(ticket, context.output, receipt.clone())?;
        Ok(receipt)
    }

    pub(super) fn validate_registered_recovery(
        &self,
        artifact: &vb_storage::AcceptedArtifact,
        boundary: crate::recovery::RecoveredRunBoundary,
    ) -> Result<(), RuntimeError> {
        let Some(pending) = boundary.pending_action_ticket() else {
            return Ok(());
        };
        let vb_core::artifact::ArtifactEvidence::Packed(packed) = &artifact.verification.evidence
        else {
            return Ok(());
        };
        if !packed
            .packs
            .iter()
            .any(|pack| pack.action == pending.ticket().action)
        {
            return Ok(());
        }
        let catalog = self
            .journal
            .executor_catalog()
            .ok_or(RuntimeError::InvalidActionCompletion)?;
        catalog
            .require_receipt_recovery(artifact, pending.ticket().action)
            .map_err(|_| RuntimeError::RecoveryCannotResume {
                reason: "registered_recovery_blocked".into(),
            })
    }

    fn registered_context(
        &self,
        run: RunId,
        wire_step: u64,
        grant: &RegisterGrant,
    ) -> Result<
        (
            ActionTicket,
            crate::shard::lifecycle::RegisteredActionContext,
        ),
        RegisteredExecutionError,
    > {
        let ticket = self
            .lookup_pending_action_ticket(run, wire_step)
            .ok_or(RuntimeError::InvalidActionCompletion)?;
        let shard = self.shard_for_run(run)?;
        let context = shard.registered_action_context(ticket)?;
        let catalog = self
            .journal
            .executor_catalog()
            .ok_or(ExecutorError::CatalogRequired)?;
        catalog.verify_artifact(&context.artifact, self.policy)?;
        if grant.artifact_digest() != context.artifact.digest {
            return Err(ExecutorError::Unauthorized.into());
        }
        Ok((ticket, context))
    }

    fn enqueue_registered_completion(
        &mut self,
        ticket: ActionTicket,
        output: vb_core::action::ActionOutputReady,
        receipt: RegisterReceipt,
    ) -> Result<(), RegisteredExecutionError> {
        self.shard_for_mut(ticket.run)?
            .enqueue(ShardCommand::RegisteredActionCompleted {
                ticket,
                receipt,
                output,
            })?;
        Ok(())
    }
}
