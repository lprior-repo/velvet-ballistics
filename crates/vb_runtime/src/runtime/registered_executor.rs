use super::Runtime;
use super::live_policy_dispatch::AdmissionEvalContext;
use crate::{RuntimeError, shard::ShardCommand};
use vb_core::policy::DecisionRecord;
use vb_core::{ActionTicket, GrantDispatchContext, RunId, ScopedDispatch, ScopedGrant};
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
    PolicyDenied(Box<DecisionRecord>),
}

impl Runtime {
    /// Execute only a canonical pending action with its actual live I64 input.
    /// The scoped grant lifecycle is verified before live dispatch: identity
    /// plus typed time/revocation, with the narrowed scope bound to the
    /// declared pack scope. Rejections happen before any journal write.
    /// A post-effect queue failure preserves the durable receipt for reconciliation.
    pub fn execute_registered_action(
        &mut self,
        adapter: &mut FjallRegister,
        run: RunId,
        wire_step: u64,
        grant: &RegisterGrant,
        presented: ScopedDispatch<'_>,
    ) -> Result<RegisterReceipt, RegisteredExecutionError> {
        let (ticket, context) = self.registered_context(run, wire_step, grant)?;
        self.verify_scoped_execute(presented, &context, ticket, grant)?;
        if context.mode != crate::shard::types::RegisteredExecutionMode::Live {
            return Err(ExecutorError::Unauthorized.into());
        }
        let eval_ctx = AdmissionEvalContext {
            ticket,
            admission_pin: context.admission_pin,
            encoded_bytes: context.encoded_bytes,
            input_taint: context.input_taint,
        };
        self.evaluate_admission_and_persist(eval_ctx)?;
        let receipt = adapter.execute(grant, ticket, context.value)?;
        self.enqueue_registered_completion(ticket, context.output, receipt.clone())?;
        Ok(receipt)
    }

    /// Recovery reads an existing receipt and never invokes a production effect.
    /// The scoped identity bindings are verified, but expiry and revocation
    /// never block history reads. Missing receipts and Blocked recovery fail
    /// before any target write.
    pub fn reconcile_registered_action(
        &mut self,
        adapter: &mut FjallRegister,
        run: RunId,
        wire_step: u64,
        grant: &RegisterGrant,
        scoped: &ScopedGrant,
        dispatch: &GrantDispatchContext,
    ) -> Result<RegisterReceipt, RegisteredExecutionError> {
        let (ticket, context) = self.registered_context(run, wire_step, grant)?;
        self.verify_scoped_reconcile(scoped, dispatch, &context, ticket, grant)?;
        let catalog = self
            .journal
            .executor_catalog()
            .ok_or(ExecutorError::CatalogRequired)?;
        catalog.require_receipt_recovery(&context.artifact, ticket.action)?;
        let receipt = adapter.reconcile(grant, ticket, context.value)?;
        self.enqueue_registered_completion(ticket, context.output, receipt.clone())?;
        Ok(receipt)
    }

    /// Scoped pre-dispatch gate for live execution: full identity plus typed
    /// time/revocation, with both grants narrowed to the declared pack scope.
    fn verify_scoped_execute(
        &self,
        presented: ScopedDispatch<'_>,
        context: &crate::shard::lifecycle::RegisteredActionContext,
        ticket: ActionTicket,
        grant: &RegisterGrant,
    ) -> Result<(), RegisteredExecutionError> {
        presented
            .scoped
            .verify_for_execute(presented.dispatch, presented.revoked_ids)
            .map_err(ExecutorError::Grant)?;
        self.require_narrowed_scope(presented.scoped, &context.artifact, ticket, grant)?;
        Ok(())
    }

    /// Scoped gate for recovery reads: identity and narrowing only.
    fn verify_scoped_reconcile(
        &self,
        scoped: &ScopedGrant,
        dispatch: &GrantDispatchContext,
        context: &crate::shard::lifecycle::RegisteredActionContext,
        ticket: ActionTicket,
        grant: &RegisterGrant,
    ) -> Result<(), RegisteredExecutionError> {
        scoped
            .verify_for_reconcile(dispatch)
            .map_err(ExecutorError::Grant)?;
        self.require_narrowed_scope(scoped, &context.artifact, ticket, grant)?;
        Ok(())
    }

    /// Both grants must carry exactly the declared pack scope.
    fn require_narrowed_scope(
        &self,
        scoped: &ScopedGrant,
        artifact: &vb_storage::AcceptedArtifact,
        ticket: ActionTicket,
        grant: &RegisterGrant,
    ) -> Result<(), RegisteredExecutionError> {
        let catalog = self
            .journal
            .executor_catalog()
            .ok_or(ExecutorError::CatalogRequired)?;
        let declared = catalog.declared_scope(artifact, ticket.action)?;
        if scoped.scope != declared || grant.scope() != declared {
            return Err(ExecutorError::Unauthorized.into());
        }
        Ok(())
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
