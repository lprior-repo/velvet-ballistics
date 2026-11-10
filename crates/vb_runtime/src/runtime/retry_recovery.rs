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

impl Runtime {
    pub(super) fn retry_timer_recovery_product(
        &self,
        run: RunId,
        prefix: &RecoveryPrefixEvidence,
        recovered: &RecoveryArtifactWorkflow,
    ) -> RuntimeResult<Option<crate::recovery::RuntimeRecoveryProduct>> {
        if crate::recovery::run_is_currently_terminal(&prefix.events) {
            return Ok(None);
        }
        let Some(index) = prefix
            .events
            .iter()
            .rposition(|event| matches!(event, JournalEvent::RetryTimerScheduled { .. }))
        else {
            return Ok(None);
        };
        let event = prefix
            .events
            .get(index)
            .ok_or_else(|| invalid("event index"))?;
        let JournalEvent::RetryTimerScheduled { step, timer, .. } = event else {
            return Err(invalid("event shape"));
        };
        timer.validate()?;
        let mut checkpoint: RunCheckpointV1 =
            postcard::from_bytes(&timer.checkpoint).map_err(|_| invalid("checkpoint decode"))?;
        checkpoint
            .validate_against(
                &recovered.workflow,
                prefix.admission.artifact_digest(),
                &recovered.artifact.action_contracts,
            )
            .map_err(|error| invalid(&error.to_string()))?;
        validate_checkpoint_binding(run, event, &checkpoint, prefix, recovered)?;
        validate_failed_prefix(
            &prefix.events,
            index,
            timer,
            &recovered.workflow,
            &recovered.artifact.action_contracts,
        )?;
        let tail_start = index
            .checked_add(1)
            .ok_or_else(|| invalid("tail index overflow"))?;
        let tail = prefix
            .events
            .get(tail_start..)
            .ok_or_else(|| invalid("tail range"))?;
        let boundary = if tail.is_empty() {
            RecoveredRunBoundary::from_retry_timer(RecoveredRetryTimer {
                step: *step,
                generation: timer.generation,
                deadline_unix_ms: timer.deadline_unix_ms,
            })
        } else {
            let Some((boundary, charge)) = retry_action_boundary(
                tail,
                timer,
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
            boundary
        };
        let frame = vb_core::RunFrame::from_checkpoint_v1(checkpoint.frame.clone())
            .map_err(|_| invalid("checkpoint frame"))?;
        let collect_states = crate::primitives::collect::CollectStates::from_checkpoint_v1(
            checkpoint.collect_states.clone(),
        )
        .map_err(|_| invalid("checkpoint collect state"))?;
        let open_gates = crate::manual_gate::reconstruct_open_gates(&prefix.events)
            .map_err(|source| RuntimeError::GateEvaluationFailed { source })?;
        if !open_gates.is_empty() {
            return Err(invalid("timer conflicts with an open gate"));
        }
        let summary = Self::recover_frame_seed_from_events(&prefix.events)?.summary();
        Ok(Some(crate::recovery::RuntimeRecoveryProduct::resumable(
            crate::recovery::ResumableRecoveryProduct::new(ResumableRecoveryParts {
                run,
                summary,
                frame,
                artifact_digest: prefix.admission.artifact_digest(),
                workflow_digest: prefix.workflow_digest,
                next_seq: prefix.next_seq,
                collect_states,
                boundary,
                retry_checkpoint: Some(checkpoint),
                initial_store: None,
                open_gates,
                admission: prefix.admission.clone(),
            }),
        )))
    }
}
