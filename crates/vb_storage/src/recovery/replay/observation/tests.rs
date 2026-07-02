//! Semantic observation signature tests.
//!
//! These tests cover the acceptance criteria for the semantic
//! observation module:
//!
//! 1. The schema covers every observation category (lifecycle, steps,
//!    slots, actions, asks, waits, timers, terminal state, digests).
//! 2. The normalization hides nondeterministic storage details (run id,
//!    timestamp) but preserves semantic differences.
//! 3. Equivalent runs produce identical observation digests; divergent
//!    runs produce different observation digests.

#![allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]

#[cfg(test)]
use super::helpers::capability_set_digest_from_bytes;
use super::normalize::{observe_journal, semantic_observation_signature};
use super::types::{
    ActionOutcomeObservation, ActionStateObservation, AskObservation, ConstAnswerObservation,
    DigestSubject, JournalObservation, LifecycleObservation, SEMANTIC_OBSERVATION_SCHEMA_VERSION,
    StepObservation, TerminalObservation, TimerObservation, WaitObservation,
};
use crate::{DurableActionOutcome, EventSeq, JournalEvent};
use vb_core::action::compute_action_idempotency_key;
use vb_core::{
    ActionId, ActionTicket, Capability, CapabilitySet, ConstValue, RunId, RuntimePolicy, SlotIdx,
    StepIdx, Taint, WorkflowDigest,
};

fn sample_digest(byte: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([byte; 32])
}

fn sample_ticket(
    run: RunId,
    step: StepIdx,
    seq: u64,
    action: ActionId,
    attempt: u16,
    capacity: u16,
) -> ActionTicket {
    let seq_no = vb_core::SeqNo::new(seq);
    let idempotency_key = compute_action_idempotency_key(run, seq_no, action);
    ActionTicket {
        run,
        step,
        seq: seq_no,
        action,
        attempt,
        idempotency_key,
        capacity,
    }
}

fn digest_of(events: &[JournalEvent]) -> [u8; 32] {
    semantic_observation_signature(events).digest
}

// ---------------------------------------------------------------------------
// 1. observation_covers_lifecycle_events
// ---------------------------------------------------------------------------

#[test]
fn observation_covers_lifecycle_events() {
    let run = RunId::new(1);
    let workflow = sample_digest(0x10);
    let artifact = sample_digest(0x11);
    let capabilities = CapabilitySet::from_grants(Box::new([Capability::new(
        "network".into(),
        ActionId::new(1),
    )]));

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
        },
        JournalEvent::RunAdmission {
            run,
            seq: EventSeq::new(1),
            artifact_digest: artifact,
            granted_capabilities: capabilities,
            policy: RuntimePolicy::Strict,
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(2),
            result: SlotIdx::new(0),
            attempt: 1,
        },
        JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(3),
            attempt: 1,
        },
        JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(4),
            attempt: 1,
            reason: Some("user".to_string()),
        },
        JournalEvent::RunKilled {
            run,
            seq: EventSeq::new(5),
            attempt: 1,
        },
    ];

    let observations = observe_journal(&events);

    // RunAccepted projects to a workflow Digest + Accepted LifecycleObservation.
    assert!(
        observations.iter().any(|o| matches!(
            o,
            JournalObservation::Lifecycle(LifecycleObservation::Accepted { .. })
        )),
        "RunAccepted must project to LifecycleObservation::Accepted",
    );
    assert!(
        observations.iter().any(|o| matches!(
            o,
            JournalObservation::Digest(d) if d.subject == DigestSubject::Workflow
        )),
        "RunAccepted must emit a Workflow digest observation",
    );

    // RunAdmission projects to artifact digest + Admitted lifecycle.
    assert!(
        observations.iter().any(|o| matches!(
            o,
            JournalObservation::Digest(d) if d.subject == DigestSubject::Artifact
        )),
        "RunAdmission must emit an Artifact digest observation",
    );
    assert!(
        observations.iter().any(|o| matches!(
            o,
            JournalObservation::Lifecycle(LifecycleObservation::Admitted { .. })
        )),
        "RunAdmission must project to LifecycleObservation::Admitted",
    );

    // RunFinished projects to TerminalObservation::Finished.
    assert!(
        observations.iter().any(|o| matches!(
            o,
            JournalObservation::Terminal(TerminalObservation::Finished { .. })
        )),
        "RunFinished must project to TerminalObservation::Finished",
    );

    // RunFailedEvent projects to TerminalObservation::Failed.
    assert!(
        observations.iter().any(|o| matches!(
            o,
            JournalObservation::Terminal(TerminalObservation::Failed { .. })
        )),
        "RunFailedEvent must project to TerminalObservation::Failed",
    );

    // RunCancelled projects to TerminalObservation::Cancelled.
    assert!(
        observations.iter().any(|o| matches!(
            o,
            JournalObservation::Terminal(TerminalObservation::Cancelled { .. })
        )),
        "RunCancelled must project to TerminalObservation::Cancelled",
    );

    // RunKilled projects to TerminalObservation::Killed.
    assert!(
        observations.iter().any(|o| matches!(
            o,
            JournalObservation::Terminal(TerminalObservation::Killed { .. })
        )),
        "RunKilled must project to TerminalObservation::Killed",
    );
}

// ---------------------------------------------------------------------------
// 2. observation_covers_steps_and_slots
// ---------------------------------------------------------------------------

#[test]
fn observation_covers_steps_and_slots() {
    let run = RunId::new(2);

    let events = vec![
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(3),
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(3),
            output: SlotIdx::new(7),
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(2),
            slot: SlotIdx::new(7),
            value: Some(vec![0xAA, 0xBB, 0xCC]),
            extra: None,
            attempt: 1,
        },
    ];

    let observations = observe_journal(&events);

    assert!(
        observations.iter().any(|o| matches!(
            o,
            JournalObservation::Step(StepObservation::Started { step, attempt: 1 }) if step.get() == 3
        )),
        "StepStarted must project to StepObservation::Started",
    );

    assert!(
        observations.iter().any(|o| matches!(
            o,
            JournalObservation::Step(StepObservation::Succeeded { step, output }) if step.get() == 3 && output.get() == 7
        )),
        "StepSucceeded must project to StepObservation::Succeeded",
    );

    assert!(
        observations.iter().any(|o| matches!(
            o,
            JournalObservation::Slot(s) if s.slot == SlotIdx::new(7) && s.value_digest.is_some()
        )),
        "SlotWrittenEvent with payload must produce a SlotObservation with a value digest",
    );
}

// ---------------------------------------------------------------------------
// 3. observation_preserves_action_abi_digest
// ---------------------------------------------------------------------------

#[test]
fn observation_preserves_action_abi_digest() {
    let run = RunId::new(3);
    let workflow = sample_digest(0x77);
    let action_abi_digest = sample_digest(0xAB);

    let ticket = sample_ticket(run, StepIdx::new(4), 1, ActionId::new(11), 1, 3);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
        },
        JournalEvent::ActionScheduledTicket {
            run,
            seq: EventSeq::new(1),
            ticket,
            input: SlotIdx::new(1),
            output: SlotIdx::new(2),
            action_abi_digest,
        },
        JournalEvent::ActionCompletedEnvelope {
            run,
            seq: EventSeq::new(2),
            ticket,
            output: SlotIdx::new(2),
            outcome: DurableActionOutcome::Ready,
            value: vec![0x10, 0x20, 0x30],
            encoded_len: 3,
            taint: Taint::Clean,
            value_digest: [0x55; 32],
            action_abi_digest,
        },
    ];

    let observations = observe_journal(&events);

    // Both ticket and envelope must carry the exact action_abi_digest bytes.
    let action_observations: Vec<_> = observations
        .iter()
        .filter_map(|o| match o {
            JournalObservation::Action(a) => Some(a),
            _ => None,
        })
        .collect();
    assert_eq!(
        action_observations.len(),
        2,
        "two action events should yield two action observations",
    );
    for action_obs in &action_observations {
        let carried = action_obs
            .action_abi_digest
            .as_ref()
            .expect("ticket/envelope must carry action_abi_digest");
        assert_eq!(
            carried.bytes,
            action_abi_digest.as_bytes(),
            "action_abi_digest bytes must equal input workflow digest bytes",
        );
        assert_eq!(
            carried.subject,
            DigestSubject::Action,
            "action_abi_digest observation must be subject=Action",
        );
    }

    // Mutating the input workflow digest changes the carried bytes.
    let mutated_abi = sample_digest(0xCD);
    let mutated_ticket = sample_ticket(run, StepIdx::new(4), 1, ActionId::new(11), 1, 3);
    let mutated_events = vec![JournalEvent::ActionScheduledTicket {
        run,
        seq: EventSeq::new(1),
        ticket: mutated_ticket,
        input: SlotIdx::new(1),
        output: SlotIdx::new(2),
        action_abi_digest: mutated_abi,
    }];
    let mutated_observations = observe_journal(&mutated_events);
    let mutated_action = mutated_observations
        .iter()
        .find_map(|o| match o {
            JournalObservation::Action(a) => Some(a),
            _ => None,
        })
        .expect("scheduled ticket must project to ActionObservation");
    let mutated_carried = mutated_action
        .action_abi_digest
        .as_ref()
        .expect("ticket must carry action_abi_digest");
    assert_ne!(
        mutated_carried.bytes,
        action_abi_digest.as_bytes(),
        "mutating input action_abi_digest must change the carried observation bytes",
    );
}

// ---------------------------------------------------------------------------
// 4. observation_preserves_action_capacity
// ---------------------------------------------------------------------------

#[test]
fn observation_preserves_action_capacity() {
    let run = RunId::new(4);
    let workflow = sample_digest(0x21);
    let abandoned_capacity: u16 = 5;

    let ticket = sample_ticket(
        run,
        StepIdx::new(8),
        1,
        ActionId::new(33),
        1,
        abandoned_capacity,
    );

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
        },
        JournalEvent::ActionScheduledTicket {
            run,
            seq: EventSeq::new(1),
            ticket,
            input: SlotIdx::new(0),
            output: SlotIdx::new(1),
            action_abi_digest: sample_digest(0x42),
        },
        JournalEvent::ActionAbandoned {
            run,
            seq: EventSeq::new(2),
            ticket,
        },
    ];

    let observations = observe_journal(&events);
    let abandoned_obs = observations
        .iter()
        .find_map(|o| match o {
            JournalObservation::Action(a) if a.state == ActionStateObservation::Abandoned => {
                Some(a)
            }
            _ => None,
        })
        .expect("abandoned event must project to Abandoned ActionObservation");

    assert_eq!(
        abandoned_obs.capacity,
        Some(abandoned_capacity),
        "abandoned action observation must carry the ticket capacity",
    );
    assert!(
        abandoned_obs.action_abi_digest.is_none(),
        "ActionAbandoned does not carry action_abi_digest in the source event",
    );
}

// ---------------------------------------------------------------------------
// 5. observation_handles_legacy_and_modern_action_paths
// ---------------------------------------------------------------------------

#[test]
fn observation_handles_legacy_and_modern_action_paths() {
    let run = RunId::new(5);
    let workflow = sample_digest(0x99);

    // Legacy completion path: ActionCompletedEvent (no value digest, no
    // action_abi_digest, no taint).
    let legacy_events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(2),
            action: ActionId::new(7),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(2),
            action: ActionId::new(7),
            attempt: 1,
        },
    ];
    let legacy_observations = observe_journal(&legacy_events);
    let legacy_completed = legacy_observations
        .iter()
        .find_map(|o| match o {
            JournalObservation::Action(a) if a.state == ActionStateObservation::Completed => {
                Some(a)
            }
            _ => None,
        })
        .expect("ActionCompletedEvent must project to Completed ActionObservation");
    assert!(
        legacy_completed.outcome.is_some(),
        "legacy completion path must populate outcome",
    );
    assert!(
        legacy_completed.action_abi_digest.is_none(),
        "legacy completion path carries no action_abi_digest",
    );

    // Modern completion path: ActionCompletedEnvelope (with
    // action_abi_digest and value digest preserved).
    let modern_ticket = sample_ticket(run, StepIdx::new(2), 1, ActionId::new(7), 1, 3);
    let modern_abi = sample_digest(0xEE);
    let modern_value_digest = [0x77u8; 32];
    let modern_events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
        },
        JournalEvent::ActionScheduledTicket {
            run,
            seq: EventSeq::new(1),
            ticket: modern_ticket,
            input: SlotIdx::new(0),
            output: SlotIdx::new(3),
            action_abi_digest: modern_abi,
        },
        JournalEvent::ActionCompletedEnvelope {
            run,
            seq: EventSeq::new(2),
            ticket: modern_ticket,
            output: SlotIdx::new(3),
            outcome: DurableActionOutcome::Ready,
            value: vec![0x10, 0x20],
            encoded_len: 2,
            taint: Taint::Secret,
            value_digest: modern_value_digest,
            action_abi_digest: modern_abi,
        },
    ];
    let modern_observations = observe_journal(&modern_events);
    let modern_completed = modern_observations
        .iter()
        .find_map(|o| match o {
            JournalObservation::Action(a) if a.state == ActionStateObservation::Completed => {
                Some(a)
            }
            _ => None,
        })
        .expect("ActionCompletedEnvelope must project to Completed ActionObservation");
    assert!(
        modern_completed.action_abi_digest.is_some(),
        "modern completion path must carry action_abi_digest",
    );
    let modern_outcome = modern_completed
        .outcome
        .expect("modern completion must carry outcome");
    let ActionOutcomeObservation::Ready {
        taint_tag,
        value_digest,
    } = modern_outcome;
    assert_eq!(
        taint_tag,
        Taint::Secret as u8,
        "modern completion must preserve taint discriminant",
    );
    assert_eq!(
        value_digest, modern_value_digest,
        "modern completion must preserve value digest",
    );
}

// ---------------------------------------------------------------------------
// 6. observation_distinguishes_equivalent_runs
// ---------------------------------------------------------------------------

#[test]
fn observation_distinguishes_equivalent_runs() {
    let run_a = RunId::new(10);
    let run_b = RunId::new(11); // Different run id (nondeterministic).
    let workflow = sample_digest(0x33);
    let artifact = sample_digest(0x44);

    fn build(run: RunId, workflow: WorkflowDigest, artifact: WorkflowDigest) -> Vec<JournalEvent> {
        vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow,
            },
            JournalEvent::RunAdmission {
                run,
                seq: EventSeq::new(1),
                artifact_digest: artifact,
                granted_capabilities: CapabilitySet::from_grants(Box::new([Capability::new(
                    "network".into(),
                    ActionId::new(1),
                )])),
                policy: RuntimePolicy::Strict,
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(2),
                step: StepIdx::new(1),
                attempt: 1,
            },
            JournalEvent::StepSucceeded {
                run,
                seq: EventSeq::new(3),
                step: StepIdx::new(1),
                output: SlotIdx::new(2),
            },
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(4),
                result: SlotIdx::new(2),
                attempt: 1,
            },
        ]
    }

    let events_a = build(run_a, workflow, artifact);
    let events_b = build(run_b, workflow, artifact);

    let sig_a = semantic_observation_signature(&events_a);
    let sig_b = semantic_observation_signature(&events_b);

    assert_eq!(
        sig_a.digest, sig_b.digest,
        "two runs that differ only in run id must produce identical observation digests",
    );
    assert_eq!(
        sig_a.schema_version, SEMANTIC_OBSERVATION_SCHEMA_VERSION,
        "signature must record the current schema version",
    );
    assert_eq!(
        sig_a.observations.len(),
        sig_b.observations.len(),
        "two equivalent runs must yield the same number of observations",
    );
}

// ---------------------------------------------------------------------------
// 7. observation_detects_digest_divergence
// ---------------------------------------------------------------------------

#[test]
fn observation_detects_digest_divergence() {
    fn build(workflow_byte: u8) -> Vec<JournalEvent> {
        vec![
            JournalEvent::RunAccepted {
                run: RunId::new(20),
                seq: EventSeq::new(0),
                workflow: sample_digest(workflow_byte),
            },
            JournalEvent::RunFinished {
                run: RunId::new(20),
                seq: EventSeq::new(1),
                result: SlotIdx::new(0),
                attempt: 1,
            },
        ]
    }

    let sig_a = semantic_observation_signature(&build(0x01));
    let sig_b = semantic_observation_signature(&build(0x02));

    assert_ne!(
        sig_a.digest, sig_b.digest,
        "one-byte WorkflowDigest difference must produce different observation digests",
    );
}

// ---------------------------------------------------------------------------
// 8. observation_detects_capacity_divergence
// ---------------------------------------------------------------------------

#[test]
fn observation_detects_capacity_divergence() {
    let run = RunId::new(30);

    let build = |capacity: u16| -> Vec<JournalEvent> {
        let ticket = sample_ticket(run, StepIdx::new(4), 1, ActionId::new(55), 1, capacity);
        vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: sample_digest(0x71),
            },
            JournalEvent::ActionScheduledTicket {
                run,
                seq: EventSeq::new(1),
                ticket,
                input: SlotIdx::new(0),
                output: SlotIdx::new(1),
                action_abi_digest: sample_digest(0x72),
            },
            JournalEvent::ActionAbandoned {
                run,
                seq: EventSeq::new(2),
                ticket,
            },
        ]
    };

    let digest_a = digest_of(&build(1));
    let digest_b = digest_of(&build(2));

    assert_ne!(
        digest_a, digest_b,
        "abandoned-action capacity difference must flip observation_digest",
    );
}

// ---------------------------------------------------------------------------
// Supplemental coverage: asks, waits, timers, answers
// ---------------------------------------------------------------------------

#[test]
fn observation_covers_ask_wait_timer_answer_events() {
    let run = RunId::new(40);
    let events = vec![
        JournalEvent::WaitScheduledEvent {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::WaitResolvedEvent {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::AskScheduledEvent {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(2),
            attempt: 1,
        },
        JournalEvent::AskAnsweredEvent {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(2),
            attempt: 1,
        },
        JournalEvent::RunAnswered {
            run,
            seq: EventSeq::new(4),
            slot_idx: SlotIdx::new(5),
            answer: ConstValue::Bool(true),
            timestamp: chrono::Utc::now(),
        },
        JournalEvent::AskTimedOutEvent {
            run,
            seq: EventSeq::new(5),
            step: StepIdx::new(2),
            attempt: 1,
        },
        JournalEvent::RetryScheduledEvent {
            run,
            seq: EventSeq::new(6),
            step: StepIdx::new(2),
            attempt: 2,
        },
    ];

    let observations = observe_journal(&events);
    assert!(observations.iter().any(|o| matches!(
        o,
        JournalObservation::Wait(WaitObservation::Scheduled { .. })
    )));
    assert!(observations.iter().any(|o| matches!(
        o,
        JournalObservation::Wait(WaitObservation::Resolved { .. })
    )));
    assert!(
        observations
            .iter()
            .any(|o| matches!(o, JournalObservation::Ask(AskObservation::Scheduled { .. })))
    );
    assert!(
        observations
            .iter()
            .any(|o| matches!(o, JournalObservation::Ask(AskObservation::Answered { .. })))
    );
    assert!(observations.iter().any(|o| matches!(
        o,
        JournalObservation::Ask(AskObservation::AnswerRecorded {
            answer: ConstAnswerObservation::Bool(true),
            ..
        })
    )));
    assert!(
        observations
            .iter()
            .any(|o| matches!(o, JournalObservation::Ask(AskObservation::TimedOut { .. })))
    );
    assert!(observations.iter().any(|o| matches!(
        o,
        JournalObservation::Timer(TimerObservation::AskTimedOut { .. })
    )));
    assert!(observations.iter().any(|o| matches!(
        o,
        JournalObservation::Timer(TimerObservation::RetryScheduled { .. })
    )));
}

#[test]
fn observation_signature_reports_current_schema_version() {
    let events: Vec<JournalEvent> = vec![];
    let signature = semantic_observation_signature(&events);
    assert_eq!(
        signature.schema_version,
        SEMANTIC_OBSERVATION_SCHEMA_VERSION
    );
    assert!(signature.observations.is_empty());
    // Empty events still produce a deterministic digest.
    let again = semantic_observation_signature(&[]).digest;
    assert_eq!(signature.digest, again);
}

// ---------------------------------------------------------------------------
// 11. allocation_failed_sentinel_collapses_divergent_runs
// ---------------------------------------------------------------------------
//
// Verifies that when `postcard::to_allocvec` returns `Err(_)` (or
// `try_reserve` fails), the capability-set digest collapses to the
// fixed `ALLOCATION_FAILED_SENTINEL` regardless of the input content.
// Two divergent runs that both hit the sentinel must produce the
// same digest so no false-positive divergence is reported.
//
// The test uses the cfg(test) `capability_set_digest_from_bytes`
// helper so the failure path can be exercised deterministically
// without relying on postcard's runtime allocation-failure behavior.

#[test]
fn allocation_failed_sentinel_collapses_divergent_runs() {
    use super::helpers::ALLOCATION_FAILED_SENTINEL;

    // Two different simulated byte payloads — these represent two
    // divergent `CapabilitySet` values that both would fail to
    // serialize in the wild. Both must collapse to the same
    // sentinel-driven digest.
    let payload_a: Result<Vec<u8>, ()> = Err(());
    let payload_b: Result<Vec<u8>, ()> = Err(());

    let digest_a = capability_set_digest_from_bytes(payload_a);
    let digest_b = capability_set_digest_from_bytes(payload_b);

    // Subject must remain `CapabilitySet` even on the failure path
    // so the observation is correctly classified downstream.
    assert_eq!(
        digest_a.subject,
        DigestSubject::CapabilitySet,
        "sentinel digest must preserve the CapabilitySet subject",
    );
    assert_eq!(
        digest_b.subject,
        DigestSubject::CapabilitySet,
        "sentinel digest must preserve the CapabilitySet subject",
    );

    // Both divergent runs must collapse to the exact same bytes,
    // so observation_digest sees them as equivalent.
    assert_eq!(
        digest_a.bytes, digest_b.bytes,
        "two divergent failing runs must collapse to the same sentinel digest",
    );

    // The sentinel bytes must be the documented constant so any
    // change to the sentinel value is a deliberate contract change.
    assert_eq!(
        digest_a.bytes, ALLOCATION_FAILED_SENTINEL,
        "failing-run digest must equal ALLOCATION_FAILED_SENTINEL",
    );
}

#[test]
fn allocation_failed_sentinel_differs_from_success_path() {
    // Positive control: a successful encode must produce a digest
    // that differs from the sentinel so divergence tests can
    // distinguish a real grant-set difference from an
    // allocation-failure collapse.
    let success_payload: Result<Vec<u8>, ()> = Ok(vec![0x01, 0x02, 0x03, 0x04]);
    let fail_payload: Result<Vec<u8>, ()> = Err(());

    let success_digest = capability_set_digest_from_bytes(success_payload);
    let fail_digest = capability_set_digest_from_bytes(fail_payload);

    assert_ne!(
        success_digest.bytes, fail_digest.bytes,
        "successful encode and allocation-failure must produce different digests",
    );
    assert_eq!(
        success_digest.subject,
        DigestSubject::CapabilitySet,
        "successful digest must preserve the CapabilitySet subject",
    );
}
