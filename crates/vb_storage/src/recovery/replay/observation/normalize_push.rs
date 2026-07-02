#![forbid(unsafe_code)]
#![allow(dead_code)]
//! Per-variant push helpers for [`super::normalize`].
//!
//! Each helper projects exactly one [`crate::JournalEvent`] variant
//! into its [`super::types::JournalObservation`] and stays within the
//! Farley 25-line limit. The helpers are split across focused
//! sub-modules so this dispatcher stays under the source-length cap.

use vb_core::{CapabilitySet, RuntimePolicy, SlotIdx, StepIdx, WorkflowDigest};

use super::digest::str_digest;
use super::helpers::{
    capability_set_digest, policy_tag, slot_observation, workflow_digest_observation,
};
use super::types::{
    AskObservation, ConstAnswerObservation, DigestObservation, DigestSubject, JournalObservation,
    LifecycleObservation, StepObservation, TerminalObservation, TimerObservation, WaitObservation,
};

/// Project `RunAccepted` into a workflow digest + Accepted lifecycle.
pub(super) fn push_run_accepted(out: &mut Vec<JournalObservation>, workflow: WorkflowDigest) {
    let wf = workflow_digest_observation(DigestSubject::Workflow, workflow);
    out.push(JournalObservation::Digest(wf));
    out.push(JournalObservation::Lifecycle(
        LifecycleObservation::Accepted { workflow: wf },
    ));
}

/// Project `RunAdmission` into an artifact digest + Admitted lifecycle.
pub(super) fn push_run_admission(
    out: &mut Vec<JournalObservation>,
    artifact_digest: WorkflowDigest,
    granted_capabilities: &CapabilitySet,
    policy: RuntimePolicy,
) {
    let artifact = workflow_digest_observation(DigestSubject::Artifact, artifact_digest);
    let capabilities = capability_set_digest(granted_capabilities);
    let capability_count = u32::try_from(granted_capabilities.len()).unwrap_or(u32::MAX);
    out.push(JournalObservation::Digest(DigestObservation {
        subject: DigestSubject::Artifact,
        bytes: artifact.bytes,
    }));
    out.push(JournalObservation::Lifecycle(
        LifecycleObservation::Admitted {
            artifact,
            capabilities,
            capability_count,
            policy_tag: policy_tag(policy),
        },
    ));
}

/// Project `StepStarted` into a `Step::Started` observation.
pub(super) fn push_step_started(out: &mut Vec<JournalObservation>, step: StepIdx, attempt: u16) {
    out.push(JournalObservation::Step(StepObservation::Started {
        step,
        attempt,
    }));
}

/// Project `StepSucceeded` into a `Step::Succeeded` observation.
pub(super) fn push_step_succeeded(
    out: &mut Vec<JournalObservation>,
    step: StepIdx,
    output: SlotIdx,
) {
    out.push(JournalObservation::Step(StepObservation::Succeeded {
        step,
        output,
    }));
}

/// Project `SlotWrittenEvent` into a `Slot` observation with optional
/// value / extra digests.
pub(super) fn push_slot_written(
    out: &mut Vec<JournalObservation>,
    slot: SlotIdx,
    value: Option<&[u8]>,
    extra: Option<&[u8]>,
    attempt: u16,
) {
    out.push(JournalObservation::Slot(slot_observation(
        slot, attempt, value, extra,
    )));
}

/// Project `WaitScheduledEvent` into a `Wait::Scheduled` observation.
pub(super) fn push_wait_scheduled(out: &mut Vec<JournalObservation>, step: StepIdx, attempt: u16) {
    out.push(JournalObservation::Wait(WaitObservation::Scheduled {
        step,
        attempt,
    }));
}

/// Project `WaitResolvedEvent` into a `Wait::Resolved` observation.
pub(super) fn push_wait_resolved(out: &mut Vec<JournalObservation>, step: StepIdx, attempt: u16) {
    out.push(JournalObservation::Wait(WaitObservation::Resolved {
        step,
        attempt,
    }));
}

/// Project `AskScheduledEvent` into an `Ask::Scheduled` observation.
pub(super) fn push_ask_scheduled(out: &mut Vec<JournalObservation>, step: StepIdx, attempt: u16) {
    out.push(JournalObservation::Ask(AskObservation::Scheduled {
        step,
        attempt,
    }));
}

/// Project `AskAnsweredEvent` into an `Ask::Answered` observation.
pub(super) fn push_ask_answered(out: &mut Vec<JournalObservation>, step: StepIdx, attempt: u16) {
    out.push(JournalObservation::Ask(AskObservation::Answered {
        step,
        attempt,
    }));
}

/// Project `RunAnswered` into an `Ask::AnswerRecorded` observation.
pub(super) fn push_run_answered(
    out: &mut Vec<JournalObservation>,
    slot_idx: SlotIdx,
    answer: vb_core::ConstValue,
) {
    out.push(JournalObservation::Ask(AskObservation::AnswerRecorded {
        slot: slot_idx,
        answer: ConstAnswerObservation::from_const(answer),
    }));
}

/// Project `AskTimedOutEvent` into an `Ask::TimedOut` + `Timer::AskTimedOut` pair.
pub(super) fn push_ask_timed_out(out: &mut Vec<JournalObservation>, step: StepIdx, attempt: u16) {
    out.push(JournalObservation::Ask(AskObservation::TimedOut {
        step,
        attempt,
    }));
    out.push(JournalObservation::Timer(TimerObservation::AskTimedOut {
        step,
        attempt,
    }));
}

/// Project `RetryScheduledEvent` into a `Timer::RetryScheduled` observation.
pub(super) fn push_retry_scheduled(out: &mut Vec<JournalObservation>, step: StepIdx, attempt: u16) {
    out.push(JournalObservation::Timer(
        TimerObservation::RetryScheduled { step, attempt },
    ));
}

/// Project `RunCancelled` into a `Terminal::Cancelled` observation.
pub(super) fn push_run_cancelled(
    out: &mut Vec<JournalObservation>,
    attempt: u16,
    reason: Option<&str>,
) {
    let reason_observation = reason.map(str_digest);
    out.push(JournalObservation::Terminal(
        TerminalObservation::Cancelled {
            attempt,
            reason: reason_observation,
        },
    ));
}

/// Project `RunKilled` into a `Terminal::Killed` observation.
pub(super) fn push_run_killed(out: &mut Vec<JournalObservation>, attempt: u16) {
    out.push(JournalObservation::Terminal(TerminalObservation::Killed {
        attempt,
    }));
}

/// Project `RunFinished` into a `Terminal::Finished` observation.
pub(super) fn push_run_finished(out: &mut Vec<JournalObservation>, result: SlotIdx, attempt: u16) {
    out.push(JournalObservation::Terminal(
        TerminalObservation::Finished { result, attempt },
    ));
}

/// Project `RunFailedEvent` into a `Terminal::Failed` observation.
pub(super) fn push_run_failed(out: &mut Vec<JournalObservation>, attempt: u16) {
    out.push(JournalObservation::Terminal(TerminalObservation::Failed {
        attempt,
    }));
}

/// Project `RunResumed` into a `Lifecycle::Resumed` observation.
pub(super) fn push_run_resumed(out: &mut Vec<JournalObservation>) {
    out.push(JournalObservation::Lifecycle(LifecycleObservation::Resumed));
}

/// Project `RunRetried` into a `Lifecycle::Retried` observation.
pub(super) fn push_run_retried(out: &mut Vec<JournalObservation>) {
    out.push(JournalObservation::Lifecycle(LifecycleObservation::Retried));
}
