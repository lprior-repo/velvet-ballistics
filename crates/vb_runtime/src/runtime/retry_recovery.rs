use super::*;
use crate::recovery::{RecoveredRetryTimer, RecoveredRunBoundary, ResumableRecoveryParts};
use crate::run_checkpoint::RunCheckpointV1;
use vb_storage::{JournalEvent, retry_timer::RetryTimerV1};
#[path = "retry_recovery/validation.rs"]
mod validation;
use validation::{retry_action_boundary, validate_checkpoint_binding, validate_failed_prefix};

fn invalid(reason: &str) -> RuntimeError {
    RuntimeError::Recovery {
        error: format!("retry timer: {reason}"),
    }
}

struct RetryTimerRestoredState {
    checkpoint: RunCheckpointV1,
    frame: vb_core::RunFrame,
    collect_states: crate::primitives::collect::CollectStates,
    open_gates: Vec<crate::shard::types::OpenGate>,
    bound_open_gates: Vec<crate::shard::types::BoundOpenGate>,
}

struct RetryTimerSchedule<'a> {
    step: vb_core::StepIdx,
    timer: &'a RetryTimerV1,
}

struct RetryTimerRecoveryRecord<'a> {
    schedule: RetryTimerSchedule<'a>,
    checkpoint: RunCheckpointV1,
}

struct RetryTimerMetadata {
    summary: vb_storage::recovery::RecoveryRuntimeSummary,
    journal_bytes: u64,
    tenant: Option<vb_core::ids::TenantId>,
}

struct RetryTimerProductContext<'a> {
    run: RunId,
    prefix: &'a RecoveryPrefixEvidence,
    boundary: RecoveredRunBoundary,
    restored: RetryTimerRestoredState,
    metadata: RetryTimerMetadata,
}

impl Runtime {
    pub(super) fn retry_timer_recovery_product(
        &self,
        run: RunId,
        prefix: &RecoveryPrefixEvidence,
        recovered: &RecoveryArtifactWorkflow,
    ) -> RuntimeResult<Option<crate::recovery::RuntimeRecoveryProduct>> {
        let Some(index) = latest_retry_timer_index(prefix) else {
            return Ok(None);
        };
        let mut record = retry_timer_record(run, index, prefix, recovered)?;
        let tail = retry_timer_tail(prefix, index)?;
        let Some(boundary) =
            self.retry_timer_boundary(tail, record.schedule, recovered, &mut record.checkpoint)?
        else {
            return Ok(None);
        };
        let restored = retry_timer_state(prefix, record.checkpoint)?;
        Ok(Some(resumable_retry_timer_product(
            run, prefix, boundary, restored,
        )?))
    }

    fn retry_timer_boundary(
        &self,
        tail: &[JournalEvent],
        schedule: RetryTimerSchedule<'_>,
        recovered: &RecoveryArtifactWorkflow,
        checkpoint: &mut RunCheckpointV1,
    ) -> RuntimeResult<Option<RecoveredRunBoundary>> {
        if tail.is_empty() {
            return Ok(Some(retry_timer_boundary(schedule)));
        }
        self.retry_action_timer_boundary(tail, schedule, recovered, checkpoint)
    }

    fn retry_action_timer_boundary(
        &self,
        tail: &[JournalEvent],
        schedule: RetryTimerSchedule<'_>,
        recovered: &RecoveryArtifactWorkflow,
        checkpoint: &mut RunCheckpointV1,
    ) -> RuntimeResult<Option<RecoveredRunBoundary>> {
        let Some((boundary, charge)) = retry_action_boundary(
            tail,
            schedule.timer,
            &recovered.workflow,
            &recovered.artifact.action_contracts,
        )?
        else {
            return Ok(None);
        };
        checkpoint.journal_bytes_written = checkpoint
            .journal_bytes_written
            .checked_add(charge)
            .ok_or_else(|| invalid("journal charge overflow"))?;
        self.validate_registered_recovery(&recovered.artifact, boundary)?;
        Ok(Some(boundary))
    }
}

fn retry_timer_record<'a>(
    run: RunId,
    index: usize,
    prefix: &'a RecoveryPrefixEvidence,
    recovered: &RecoveryArtifactWorkflow,
) -> RuntimeResult<RetryTimerRecoveryRecord<'a>> {
    let event = prefix
        .events
        .get(index)
        .ok_or_else(|| invalid("event index"))?;
    let (step, timer) = retry_timer_event(event)?;
    timer.validate()?;
    let checkpoint = decode_retry_timer_checkpoint(timer)?;
    validate_retry_timer_checkpoint((run, index), event, &checkpoint, prefix, recovered)?;
    Ok(RetryTimerRecoveryRecord {
        schedule: RetryTimerSchedule { step, timer },
        checkpoint,
    })
}

fn retry_timer_tail(
    prefix: &RecoveryPrefixEvidence,
    index: usize,
) -> RuntimeResult<&[JournalEvent]> {
    let tail_start = index
        .checked_add(1)
        .ok_or_else(|| invalid("tail index overflow"))?;
    prefix
        .events
        .get(tail_start..)
        .ok_or_else(|| invalid("tail range"))
}

fn retry_timer_boundary(schedule: RetryTimerSchedule<'_>) -> RecoveredRunBoundary {
    RecoveredRunBoundary::from_retry_timer(RecoveredRetryTimer {
        step: schedule.step,
        generation: schedule.timer.generation,
        deadline_unix_ms: schedule.timer.deadline_unix_ms,
    })
}

fn retry_timer_event(event: &JournalEvent) -> RuntimeResult<(vb_core::StepIdx, &RetryTimerV1)> {
    match event {
        JournalEvent::RetryTimerScheduled { step, timer, .. } => Ok((*step, timer)),
        _ => Err(invalid("event shape")),
    }
}

fn decode_retry_timer_checkpoint(timer: &RetryTimerV1) -> RuntimeResult<RunCheckpointV1> {
    postcard::from_bytes(&timer.checkpoint).map_err(|_| invalid("checkpoint decode"))
}

fn validate_retry_timer_checkpoint(
    run_and_index: (RunId, usize),
    event: &JournalEvent,
    checkpoint: &RunCheckpointV1,
    prefix: &RecoveryPrefixEvidence,
    recovered: &RecoveryArtifactWorkflow,
) -> RuntimeResult<()> {
    let (run, index) = run_and_index;
    checkpoint
        .validate_against(
            &recovered.workflow,
            prefix.admission.artifact_digest(),
            &recovered.artifact.action_contracts,
        )
        .map_err(|error| invalid(&error.to_string()))?;
    validate_checkpoint_binding(run, event, checkpoint, prefix, recovered)?;
    let (_, timer) = retry_timer_event(event)?;
    validate_failed_prefix(
        &prefix.events,
        index,
        timer,
        &recovered.workflow,
        &recovered.artifact.action_contracts,
    )
}

fn retry_timer_state(
    prefix: &RecoveryPrefixEvidence,
    checkpoint: RunCheckpointV1,
) -> RuntimeResult<RetryTimerRestoredState> {
    let frame = vb_core::RunFrame::from_checkpoint_v1(checkpoint.frame.clone())
        .map_err(|_| invalid("checkpoint frame"))?;
    let collect_states = crate::primitives::collect::CollectStates::from_checkpoint_v1(
        checkpoint.collect_states.clone(),
    )
    .map_err(|_| invalid("checkpoint collect state"))?;
    let open_gates = crate::manual_gate::reconstruct_open_gates(&prefix.events)
        .map_err(|source| RuntimeError::GateEvaluationFailed { source })?;
    let bound_open_gates = crate::manual_gate::reconstruct_bound_open_gates(&prefix.events)
        .map_err(|source| RuntimeError::GateEvaluationFailed { source })?;
    if !open_gates.is_empty() || !bound_open_gates.is_empty() {
        return Err(invalid("timer conflicts with an open gate"));
    }
    Ok(RetryTimerRestoredState {
        checkpoint,
        frame,
        collect_states,
        open_gates,
        bound_open_gates,
    })
}

fn resumable_retry_timer_product(
    run: RunId,
    prefix: &RecoveryPrefixEvidence,
    boundary: RecoveredRunBoundary,
    restored: RetryTimerRestoredState,
) -> RuntimeResult<crate::recovery::RuntimeRecoveryProduct> {
    Ok(crate::recovery::RuntimeRecoveryProduct::resumable(
        crate::recovery::ResumableRecoveryProduct::new(retry_timer_parts(
            run, prefix, boundary, restored,
        )?),
    ))
}

fn retry_timer_parts(
    run: RunId,
    prefix: &RecoveryPrefixEvidence,
    boundary: RecoveredRunBoundary,
    restored: RetryTimerRestoredState,
) -> RuntimeResult<ResumableRecoveryParts> {
    build_retry_timer_parts(RetryTimerProductContext {
        run,
        prefix,
        boundary,
        restored,
        metadata: retry_timer_metadata(prefix)?,
    })
}

fn build_retry_timer_parts(
    context: RetryTimerProductContext<'_>,
) -> RuntimeResult<ResumableRecoveryParts> {
    Ok(ResumableRecoveryParts {
        run: context.run,
        summary: context.metadata.summary,
        frame: context.restored.frame,
        artifact_digest: context.prefix.admission.artifact_digest(),
        workflow_digest: context.prefix.workflow_digest,
        next_seq: context.prefix.next_seq,
        collect_states: context.restored.collect_states,
        boundary: context.boundary,
        retry_checkpoint: Some(context.restored.checkpoint),
        restored_store: None,
        open_gates: context.restored.open_gates,
        bound_open_gates: context.restored.bound_open_gates,
        admission: context.prefix.admission.clone(),
        run_started_millis: context.prefix.run_started_millis,
        recovered_action_attempts: None,
        recovered_journal_bytes_written: Some(context.metadata.journal_bytes),
        recovered_tenant: context.metadata.tenant,
    })
}

fn retry_timer_metadata(prefix: &RecoveryPrefixEvidence) -> RuntimeResult<RetryTimerMetadata> {
    Ok(RetryTimerMetadata {
        summary: Runtime::recover_frame_seed_from_events(&prefix.events)?.summary(),
        journal_bytes: crate::recovery::recover_journal_bytes_written(&prefix.events)?,
        tenant: crate::recovery::hydrate_run_tenant(&prefix.events),
    })
}

fn latest_retry_timer_index(prefix: &RecoveryPrefixEvidence) -> Option<usize> {
    if crate::recovery::run_is_currently_terminal(&prefix.events) {
        return None;
    }
    prefix
        .events
        .iter()
        .rposition(|event| matches!(event, JournalEvent::RetryTimerScheduled { .. }))
}
