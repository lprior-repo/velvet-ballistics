//! Typed-refusal matrix for replayed suspense/retry evidence.

use vb_core::ids::{RunId, WorkflowDigest};
use vb_storage::recovery::{RecoveryError, recover_runtime_frame_seed_from_events_with_workflow};
use vb_storage::{EventSeq, JournalEvent};

use super::support::{
    RUN, TestResult, action_step, ask_step, contract, live_routed_failure_prefix,
    routed_failure_source,
};

/// Given a valid durable prefix, When duplicate, stale, foreign, missing or
/// out-of-order evidence is replayed, Then recovery refuses it with a typed
/// error instead of merging it into state.
#[test]
fn replay_refuses_duplicate_stale_foreign_missing_and_out_of_order_evidence() -> TestResult {
    let workflow = vb_compile::compile_workflow(routed_failure_source().as_bytes())?;
    let work = action_step(&workflow)?;
    let ask = ask_step(&workflow)?;
    let (prefix, _, _) = live_routed_failure_prefix(&workflow, ask)?;
    let schedule = live_schedule(&workflow)?;
    assert!(
        recover_runtime_frame_seed_from_events_with_workflow(&prefix, &workflow).is_ok(),
        "control prefix must replay"
    );
    let next_seq = || -> TestResult<EventSeq> {
        let last = prefix.last().ok_or("non-empty prefix")?.seq().get();
        Ok(EventSeq::new(last.checked_add(1).ok_or("seq overflow")?))
    };
    let mut duplicate = prefix.clone();
    duplicate.push(prefix.last().ok_or("non-empty prefix")?.clone());
    let mut stale_attempt = prefix.clone();
    if let Some(JournalEvent::ActionFailedEvent { attempt, .. }) = stale_attempt.last_mut() {
        *attempt = attempt.checked_add(1).ok_or("attempt overflow")?;
    }
    let mut foreign = prefix.clone();
    if let Some(JournalEvent::ActionFailedEvent { run, .. }) = foreign.last_mut() {
        *run = RunId::new(7);
    }
    let mut out_of_order = prefix.clone();
    let mut late_schedule = schedule;
    if let JournalEvent::ActionScheduledTicket { seq, .. } = &mut late_schedule {
        *seq = next_seq()?;
    }
    out_of_order.push(late_schedule);
    let missing = prefix.get(1..).ok_or("prefix tail")?.to_vec();
    let mut digest_mismatch = prefix.clone();
    if let Some(JournalEvent::RunAccepted {
        workflow: digest, ..
    }) = digest_mismatch.first_mut()
    {
        *digest = WorkflowDigest::from_bytes([0xa5; 32]);
    }

    let action = match workflow.node(work).ok_or("work step node")?.kind {
        vb_core::workflow::CompiledNodeKind::Do { action, .. } => action,
        _ => return Err("work step must be an action".into()),
    };
    for (label, events, want) in [
        (
            "duplicate action failure",
            duplicate,
            Expect::NonIdempotent { step: work, action },
        ),
        (
            "out-of-order schedule",
            out_of_order,
            Expect::NonIdempotent { step: work, action },
        ),
        (
            "stale attempt",
            stale_attempt,
            Expect::Divergence {
                step: work,
                detail: "action failure attempt does not match schedule ticket",
            },
        ),
        (
            "foreign run",
            foreign,
            Expect::Divergence {
                step: vb_core::ids::StepIdx::ZERO,
                detail: "frame seed recovery received events for multiple runs",
            },
        ),
        (
            "missing RunAccepted",
            missing,
            Expect::Divergence {
                step: vb_core::ids::StepIdx::ZERO,
                detail: "RunAccepted evidence missing",
            },
        ),
        (
            "digest mismatch",
            digest_mismatch,
            Expect::DigestMismatch {
                expected: workflow.digest(),
                found: WorkflowDigest::from_bytes([0xa5; 32]),
            },
        ),
    ] {
        let error = match recover_runtime_frame_seed_from_events_with_workflow(&events, &workflow) {
            Ok(_) => return Err(format!("{label}: accepted invalid evidence").into()),
            Err(error) => error,
        };
        assert!(
            want.matches(&error),
            "{label}: unexpected refusal {error:?}"
        );
    }
    Ok(())
}

/// Reads the live run's scheduled action ticket evidence.
fn live_schedule(workflow: &vb_core::CompiledWorkflow) -> TestResult<JournalEvent> {
    let directory = tempfile::tempdir()?;
    let (journal, mut runtime) = super::support::open(directory.path())?;
    super::support::submit(&journal, &mut runtime, workflow, &[contract()?])?;
    let work = action_step(workflow)?;
    let ticket = runtime
        .lookup_pending_action_ticket(RUN, u64::from(work.get()))
        .ok_or("scheduled action ticket")?;
    runtime.fail_action(
        ticket,
        vb_core::action::ActionFailureCode::InvalidInput.into(),
    )?;
    journal
        .events_for_run_full(RUN)?
        .into_iter()
        .find(|event| matches!(event, JournalEvent::ActionScheduledTicket { .. }))
        .ok_or_else(|| String::from("scheduled ticket evidence").into())
}

/// Exact expected refusal for one mutated journal.
enum Expect {
    NonIdempotent {
        step: vb_core::ids::StepIdx,
        action: vb_core::ids::ActionId,
    },
    Divergence {
        step: vb_core::ids::StepIdx,
        detail: &'static str,
    },
    DigestMismatch {
        expected: WorkflowDigest,
        found: WorkflowDigest,
    },
}

impl Expect {
    fn matches(&self, error: &RecoveryError) -> bool {
        match self {
            Self::NonIdempotent { step, action } => matches!(
                error,
                RecoveryError::NonIdempotentActionBlocked {
                    step: refused_step,
                    action: refused_action,
                } if refused_step == step && refused_action == action
            ),
            Self::Divergence { step, detail } => matches!(
                error,
                RecoveryError::ReplayDivergence {
                    step: refused_step,
                    detail: refused_detail,
                } if refused_step == step && refused_detail == detail
            ),
            Self::DigestMismatch { expected, found } => matches!(
                error,
                RecoveryError::CompiledIrDigestMismatch {
                    expected: refused_expected,
                    found: refused_found,
                } if refused_expected == expected && refused_found == found
            ),
        }
    }
}
