#![forbid(unsafe_code)]
//! Journal event to semantic observation normalization.

use vb_core::CapabilitySet;

use crate::JournalEvent;

use super::action::observe_action_event;
use super::helpers::{
    observation_digest, serialized_digest, slot_observation, stable_len, str_digest,
};

use super::types::{
    AskObservation, DigestObservation, DigestSubject, JournalObservation,
    JournalObservationSignature, LifecycleObservation, ObservationSignatureError, StepObservation,
    TerminalObservation, TimerObservation, WaitObservation, SEMANTIC_OBSERVATION_SCHEMA_VERSION,
};

const MAX_OBSERVATIONS_PER_EVENT: usize = 2;

/// Builds a stable semantic signature from ordered journal events.
pub fn semantic_observation_signature(
    events: &[JournalEvent],
) -> Result<JournalObservationSignature, ObservationSignatureError> {
    let observations = semantic_observations(events)?;
    let digest = observation_digest(&observations)?;
    Ok(JournalObservationSignature {
        schema_version: SEMANTIC_OBSERVATION_SCHEMA_VERSION,
        observations,
        digest,
    })
}

/// Normalizes journal events into semantic observations.
pub fn semantic_observations(
    events: &[JournalEvent],
) -> Result<Vec<JournalObservation>, ObservationSignatureError> {
    let capacity = events
        .len()
        .checked_mul(MAX_OBSERVATIONS_PER_EVENT)
        .ok_or(ObservationSignatureError::AllocationFailed)?;
    let mut observations = Vec::new();
    observations
        .try_reserve(capacity)
        .map_err(|_| ObservationSignatureError::AllocationFailed)?;
    for event in events {
        observe_event(event, &mut observations)?;
    }
    Ok(observations)
}

fn observe_event(
    event: &JournalEvent,
    observations: &mut Vec<JournalObservation>,
) -> Result<(), ObservationSignatureError> {
    match event {
        JournalEvent::RunAccepted { .. }
        | JournalEvent::RunAdmission { .. }
        | JournalEvent::RunResumed { .. }
        | JournalEvent::RunRetried { .. } => observe_lifecycle_event(event, observations)?,
        JournalEvent::StepStarted { .. }
        | JournalEvent::StepSucceeded { .. }
        | JournalEvent::StepFailed { .. } => {
            observe_step_event(event, observations);
        }
        JournalEvent::ActionScheduled { .. }
        | JournalEvent::ActionCompletedEvent { .. }
        | JournalEvent::ActionScheduledTicket { .. }
        | JournalEvent::ActionCompletedEnvelope { .. }
        | JournalEvent::ActionFailedEvent { .. }
        | JournalEvent::ActionAbandoned { .. } => observe_action_event(event, observations),
        JournalEvent::SlotWrittenEvent {
            slot,
            value,
            extra,
            attempt,
            ..
        } => observations.push(JournalObservation::Slot(slot_observation(
            *slot,
            *attempt,
            value.as_deref(),
            extra.as_deref(),
        ))),
        JournalEvent::WaitScheduledEvent { .. } | JournalEvent::WaitResolvedEvent { .. } => {
            observe_wait_event(event, observations);
        }
        JournalEvent::AskScheduledEvent { .. }
        | JournalEvent::AskAnsweredEvent { .. }
        | JournalEvent::RunAnswered { .. }
        | JournalEvent::AskTimedOutEvent { .. } => observe_ask_event(event, observations),
        JournalEvent::RetryScheduledEvent { step, attempt, .. } => observations.push(
            JournalObservation::Timer(TimerObservation::RetryScheduled {
                step: *step,
                attempt: *attempt,
            }),
        ),
        JournalEvent::RunCancelled { .. }
        | JournalEvent::RunKilled { .. }
        | JournalEvent::RunFinished { .. }
        | JournalEvent::RunFailedEvent { .. } => observe_terminal_event(event, observations),
    }
    Ok(())
}

fn observe_lifecycle_event(
    event: &JournalEvent,
    observations: &mut Vec<JournalObservation>,
) -> Result<(), ObservationSignatureError> {
    match event {
        JournalEvent::RunAccepted { workflow, .. } => {
            push_digest(observations, DigestSubject::Workflow, workflow.as_bytes());
            observations.push(JournalObservation::Lifecycle(
                LifecycleObservation::Accepted,
            ));
        }
        JournalEvent::RunAdmission {
            artifact_digest,
            granted_capabilities,
            policy,
            ..
        } => observe_admission(
            observations,
            artifact_digest.as_bytes(),
            granted_capabilities,
            *policy,
        )?,
        JournalEvent::RunResumed { .. } => {
            observations.push(JournalObservation::Lifecycle(LifecycleObservation::Resumed));
        }
        JournalEvent::RunRetried { .. } => {
            observations.push(JournalObservation::Lifecycle(LifecycleObservation::Retried));
        }
        _ => {}
    }
    Ok(())
}

fn observe_step_event(event: &JournalEvent, observations: &mut Vec<JournalObservation>) {
    match event {
        JournalEvent::StepStarted { step, attempt, .. } => {
            observations.push(JournalObservation::Step(StepObservation::Started {
                step: *step,
                attempt: *attempt,
            }));
        }
        JournalEvent::StepSucceeded { step, output, .. } => {
            observations.push(JournalObservation::Step(StepObservation::Succeeded {
                step: *step,
                output: *output,
            }));
        }
        _ => {}
    }
}

fn observe_wait_event(event: &JournalEvent, observations: &mut Vec<JournalObservation>) {
    match event {
        JournalEvent::WaitScheduledEvent { step, attempt, .. } => {
            observations.push(JournalObservation::Wait(WaitObservation::Scheduled {
                step: *step,
                attempt: *attempt,
            }))
        }
        JournalEvent::WaitResolvedEvent { step, attempt, .. } => {
            observations.push(JournalObservation::Wait(WaitObservation::Resolved {
                step: *step,
                attempt: *attempt,
            }))
        }
        _ => {}
    }
}

fn observe_ask_event(event: &JournalEvent, observations: &mut Vec<JournalObservation>) {
    match event {
        JournalEvent::AskScheduledEvent { step, attempt, .. } => {
            observations.push(JournalObservation::Ask(AskObservation::Scheduled {
                step: *step,
                attempt: *attempt,
            }))
        }
        JournalEvent::AskAnsweredEvent { step, attempt, .. } => {
            observations.push(JournalObservation::Ask(AskObservation::Answered {
                step: *step,
                attempt: *attempt,
            }))
        }
        JournalEvent::RunAnswered {
            slot_idx, answer, ..
        } => observations.push(JournalObservation::Ask(AskObservation::AnswerRecorded {
            slot: *slot_idx,
            answer: *answer,
        })),
        JournalEvent::AskTimedOutEvent { step, attempt, .. } => {
            observations.push(JournalObservation::Ask(AskObservation::TimedOut {
                step: *step,
                attempt: *attempt,
            }));
            observations.push(JournalObservation::Timer(TimerObservation::AskTimedOut {
                step: *step,
                attempt: *attempt,
            }));
        }
        _ => {}
    }
}

fn observe_terminal_event(event: &JournalEvent, observations: &mut Vec<JournalObservation>) {
    match event {
        JournalEvent::RunCancelled {
            attempt, reason, ..
        } => observations.push(JournalObservation::Terminal(
            TerminalObservation::Cancelled {
                attempt: *attempt,
                reason_digest: reason.as_deref().map(str_digest),
            },
        )),
        JournalEvent::RunKilled { attempt, .. } => {
            observations.push(JournalObservation::Terminal(TerminalObservation::Killed {
                attempt: *attempt,
            }))
        }
        JournalEvent::RunFinished {
            result, attempt, ..
        } => observations.push(JournalObservation::Terminal(
            TerminalObservation::Finished {
                result: *result,
                attempt: *attempt,
            },
        )),
        JournalEvent::RunFailedEvent { attempt, .. } => {
            observations.push(JournalObservation::Terminal(TerminalObservation::Failed {
                attempt: *attempt,
            }))
        }
        _ => {}
    }
}

fn observe_admission(
    observations: &mut Vec<JournalObservation>,
    artifact_digest: [u8; 32],
    capabilities: &CapabilitySet,
    policy: vb_core::RuntimePolicy,
) -> Result<(), ObservationSignatureError> {
    push_digest(observations, DigestSubject::Artifact, artifact_digest);
    let caps_bytes = postcard::to_allocvec(capabilities)
        .map_err(|_| ObservationSignatureError::AllocationFailed)?;
    observations.push(JournalObservation::Lifecycle(
        LifecycleObservation::Admitted {
            policy,
            capabilities_digest: serialized_digest(&caps_bytes)?,
            capability_count: stable_len(capabilities.len())?,
        },
    ));
    Ok(())
}

fn push_digest(
    observations: &mut Vec<JournalObservation>,
    subject: DigestSubject,
    digest: [u8; 32],
) {
    observations.push(JournalObservation::Digest(DigestObservation {
        subject,
        digest,
    }));
}
