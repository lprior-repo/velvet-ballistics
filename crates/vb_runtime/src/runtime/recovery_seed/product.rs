use super::super::*;
use crate::runtime::{RecoveredSeedProductContext, Runtime};

pub(super) struct RecoveredResumableComponents {
    frame: vb_core::RunFrame,
    collect_states: crate::primitives::collect::CollectStates,
    open_gates: Vec<crate::shard::types::OpenGate>,
    bound_open_gates: Vec<crate::shard::types::BoundOpenGate>,
}

struct RecoveredAccounting {
    action_attempts: Box<[u16]>,
    journal_bytes_written: u64,
    tenant: Option<vb_core::ids::TenantId>,
}

struct ResumableAuthority {
    summary: vb_storage::recovery::RecoveryRuntimeSummary,
    artifact_digest: vb_core::WorkflowDigest,
    workflow_digest: vb_core::WorkflowDigest,
    next_seq: vb_storage::EventSeq,
    admission: crate::admission::RunAdmission,
    run_started_millis: Option<u64>,
}

struct ResumablePayload {
    accounting: RecoveredAccounting,
    restored_store: Option<vb_core::value_store::ValueStoreCheckpointV1>,
    authority: ResumableAuthority,
}

pub(super) fn replay_store(
    context: &RecoveredSeedProductContext,
) -> RuntimeResult<Option<vb_core::value_store::ValueStore>> {
    context
        .recovered_store
        .clone()
        .map(vb_core::value_store::ValueStore::from_checkpoint_v1)
        .transpose()
        .map_err(|error| RuntimeError::Recovery {
            error: format!("recovered value store invalid: {error}"),
        })
}

pub(super) fn nonresumable_product(
    context: &RecoveredSeedProductContext,
    retained_inputs: &crate::recovery::RecoveredRetainedInputs,
    store: Option<&vb_core::value_store::ValueStore>,
) -> Option<crate::recovery::RuntimeRecoveryProduct> {
    let cannot_resume = crate::recovery::classify_full_recovery_resume(
        context.seed.seed(),
        context.boundary,
        retained_inputs.initial_inputs(),
        store,
    );
    if !cannot_resume.is_resumable() {
        return Some(crate::recovery::RuntimeRecoveryProduct::cannot_resume(
            context.seed.summary(),
            cannot_resume,
        ));
    }
    crate::recovery::run_is_currently_terminal(&context.prefix.events).then(|| {
        crate::recovery::RuntimeRecoveryProduct::cannot_resume_terminal(context.seed.summary())
    })
}

pub(super) fn resumable_components(
    context: &mut RecoveredSeedProductContext,
    retained_inputs: &crate::recovery::RecoveredRetainedInputs,
    store: Option<&vb_core::value_store::ValueStore>,
) -> RuntimeResult<RecoveredResumableComponents> {
    let frame = match context.complete_frame.take() {
        Some(frame) => frame,
        None => crate::recovery::hydrate_frame_for_full_recovery_with_inputs(
            context.seed.seed(),
            context.boundary,
            retained_inputs.initial_inputs().to_vec(),
            store,
        )?,
    };
    let collect_states = Runtime::hydrate_recovered_collect_states(&context.prefix.events)?;
    let open_gates = crate::manual_gate::reconstruct_open_gates(&context.prefix.events)
        .map_err(|source| RuntimeError::GateEvaluationFailed { source })?;
    let bound_open_gates = crate::manual_gate::reconstruct_bound_open_gates(&context.prefix.events)
        .map_err(|source| RuntimeError::GateEvaluationFailed { source })?;
    Ok(RecoveredResumableComponents {
        frame,
        collect_states,
        open_gates,
        bound_open_gates,
    })
}

fn recovered_accounting(
    context: &RecoveredSeedProductContext,
    frame_step_count: u16,
) -> RuntimeResult<RecoveredAccounting> {
    Ok(RecoveredAccounting {
        action_attempts: crate::recovery::recover_action_attempts(
            &context.prefix.events,
            frame_step_count,
        ),
        journal_bytes_written: crate::recovery::recover_journal_bytes_written(
            &context.prefix.events,
        )?,
        tenant: crate::recovery::hydrate_run_tenant(&context.prefix.events),
    })
}

pub(super) fn resumable_parts(
    context: RecoveredSeedProductContext,
    retained_inputs: &crate::recovery::RecoveredRetainedInputs,
    components: RecoveredResumableComponents,
) -> RuntimeResult<crate::recovery::ResumableRecoveryParts> {
    let payload = resumable_payload(&context, retained_inputs, components.frame.step_count())?;
    Ok(crate::recovery::ResumableRecoveryParts {
        run: context.run,
        summary: payload.authority.summary,
        frame: components.frame,
        artifact_digest: payload.authority.artifact_digest,
        workflow_digest: payload.authority.workflow_digest,
        next_seq: payload.authority.next_seq,
        collect_states: components.collect_states,
        boundary: context.boundary,
        retry_checkpoint: None,
        open_gates: components.open_gates,
        bound_open_gates: components.bound_open_gates,
        admission: payload.authority.admission,
        run_started_millis: payload.authority.run_started_millis,
        recovered_action_attempts: Some(payload.accounting.action_attempts),
        recovered_journal_bytes_written: Some(payload.accounting.journal_bytes_written),
        recovered_tenant: payload.accounting.tenant,
        restored_store: payload.restored_store,
    })
}

fn resumable_payload(
    context: &RecoveredSeedProductContext,
    retained_inputs: &crate::recovery::RecoveredRetainedInputs,
    frame_step_count: u16,
) -> RuntimeResult<ResumablePayload> {
    Ok(ResumablePayload {
        accounting: recovered_accounting(context, frame_step_count)?,
        restored_store: restored_store(context, retained_inputs),
        authority: resumable_authority(context),
    })
}

fn resumable_authority(context: &RecoveredSeedProductContext) -> ResumableAuthority {
    ResumableAuthority {
        summary: context.seed.summary(),
        artifact_digest: context.prefix.admission.artifact_digest(),
        workflow_digest: context.prefix.workflow_digest,
        next_seq: context.prefix.next_seq,
        admission: context.prefix.admission.clone(),
        run_started_millis: context.prefix.run_started_millis,
    }
}

fn restored_store(
    context: &RecoveredSeedProductContext,
    retained_inputs: &crate::recovery::RecoveredRetainedInputs,
) -> Option<vb_core::value_store::ValueStoreCheckpointV1> {
    context
        .recovered_store
        .clone()
        .or_else(|| retained_inputs.store_checkpoint())
}
