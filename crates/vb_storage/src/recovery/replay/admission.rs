#![forbid(unsafe_code)]
//! Durable run-admission evidence checks for full-journal recovery.

use crate::recovery::{RecoveryError, RecoveryResult};
use crate::{EventSeq, JournalEvent};
use vb_core::{RunId, RuntimePolicy, StepIdx, WorkflowDigest};

/// Verifies that full-journal replay is backed by meaningful admission evidence.
pub fn verify_run_admission_evidence(
    events: &[JournalEvent],
    run: RunId,
    expected_policy_digests: &[(StepIdx, WorkflowDigest)],
) -> RecoveryResult<()> {
    let (admission_seq, artifact_digest, policy) =
        single_run_admission(events, run, expected_policy_digests)?;

    let (accepted_seq, workflow_digest) = single_run_accepted(events, run, admission_seq)?;

    verify_admission_sequence(run, accepted_seq, admission_seq)?;
    verify_admission_digest(run, workflow_digest, artifact_digest)?;
    verify_policy_expectations(policy, expected_policy_digests)
}

fn single_run_admission(
    events: &[JournalEvent],
    run: RunId,
    expected_policy_digests: &[(StepIdx, WorkflowDigest)],
) -> RecoveryResult<(EventSeq, WorkflowDigest, RuntimePolicy)> {
    let selected = events.iter().try_fold(None, |selected, event| {
        let Some(found) = run_admission_digest(event, run) else {
            return Ok(selected);
        };

        if selected.is_some() {
            Err(replay_divergence(
                "duplicate RunAdmission evidence",
                run,
                found.0,
            ))
        } else {
            Ok(Some(found))
        }
    })?;

    if let Some(found) = selected {
        Ok(found)
    } else {
        missing_admission_error(run, expected_policy_digests)
    }
}

fn single_run_accepted(
    events: &[JournalEvent],
    run: RunId,
    admission_seq: EventSeq,
) -> RecoveryResult<(EventSeq, WorkflowDigest)> {
    let selected = events.iter().try_fold(None, |selected, event| {
        let Some(found) = run_accepted_digest(event, run) else {
            return Ok(selected);
        };

        if selected.is_some() {
            Err(replay_divergence(
                "duplicate RunAccepted evidence",
                run,
                found.0,
            ))
        } else {
            Ok(Some(found))
        }
    })?;

    selected.ok_or_else(|| {
        replay_divergence(
            "run admission has no RunAccepted evidence",
            run,
            admission_seq,
        )
    })
}

fn missing_admission_error<T>(
    run: RunId,
    expected_policy_digests: &[(StepIdx, WorkflowDigest)],
) -> RecoveryResult<T> {
    let Some((step, expected)) = expected_policy_digests.first() else {
        return Err(RecoveryError::PolicyDigestExpectationMissing { run });
    };

    Err(RecoveryError::PolicyDigestUnavailable {
        run,
        step: *step,
        expected: *expected,
    })
}

fn run_admission_digest(
    event: &JournalEvent,
    expected_run: RunId,
) -> Option<(EventSeq, WorkflowDigest, RuntimePolicy)> {
    match event {
        JournalEvent::RunAdmission {
            run,
            seq,
            artifact_digest,
            policy,
            ..
        } if *run == expected_run => Some((*seq, *artifact_digest, *policy)),
        _ => None,
    }
}

fn run_accepted_digest(
    event: &JournalEvent,
    expected_run: RunId,
) -> Option<(EventSeq, WorkflowDigest)> {
    match event {
        JournalEvent::RunAccepted { run, seq, workflow } if *run == expected_run => {
            Some((*seq, *workflow))
        }
        _ => None,
    }
}

fn verify_admission_sequence(
    run: RunId,
    accepted_seq: EventSeq,
    admission_seq: EventSeq,
) -> RecoveryResult<()> {
    let Some(next_seq) = accepted_seq.get().checked_add(1).map(EventSeq::new) else {
        return Err(replay_divergence(
            "run admission cannot follow max RunAccepted sequence",
            run,
            admission_seq,
        ));
    };

    if admission_seq == next_seq {
        Ok(())
    } else {
        Err(RecoveryError::ReplayDivergence {
            step: StepIdx::ZERO,
            detail: format!(
                "run {run:?} admission sequence invalid: expected {next_seq:?}, found {admission_seq:?}"
            ),
        })
    }
}

fn verify_admission_digest(
    run: RunId,
    expected: WorkflowDigest,
    found: WorkflowDigest,
) -> RecoveryResult<()> {
    if expected == found {
        Ok(())
    } else {
        Err(RecoveryError::RunAdmissionArtifactDigestMismatch {
            run,
            expected,
            found,
        })
    }
}

fn verify_policy_expectations(
    policy: RuntimePolicy,
    expected_policy_digests: &[(StepIdx, WorkflowDigest)],
) -> RecoveryResult<()> {
    let found = runtime_policy_digest(policy)?;
    let Some((step, expected)) = expected_policy_digests
        .iter()
        .find(|(_, expected)| *expected != found)
    else {
        return Ok(());
    };

    Err(RecoveryError::PolicyDigestMismatch {
        step: *step,
        expected: *expected,
        found,
    })
}

fn runtime_policy_digest(policy: RuntimePolicy) -> RecoveryResult<WorkflowDigest> {
    let bytes = postcard::to_allocvec(&policy).map_err(|_| RecoveryError::ReplayDivergence {
        step: StepIdx::ZERO,
        detail: "runtime policy digest encoding failed".to_owned(),
    })?;
    let hash = blake3::hash(&bytes);
    Ok(WorkflowDigest::from_bytes(*hash.as_bytes()))
}

fn replay_divergence(detail: &'static str, run: RunId, seq: EventSeq) -> RecoveryError {
    RecoveryError::ReplayDivergence {
        step: StepIdx::ZERO,
        detail: format!("run {run:?} {detail} at {seq:?}"),
    }
}
