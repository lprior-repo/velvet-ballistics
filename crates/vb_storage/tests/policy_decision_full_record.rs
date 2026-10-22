#![forbid(unsafe_code)]

use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx, WorkflowDigest};
use vb_core::policy::{
    DecisionCode, DecisionRecord, InfluenceState, PinnedAdmissionPolicy, PolicyDigest,
    PolicyLayer,
};
use vb_core::value::Taint;

#[path = "policy_decision_full_record/pd_helpers.rs"]
mod pd_helpers;
use vb_storage::codec::{decode_journal_event, encode_record};
use vb_storage::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES};
use vb_storage::events::JournalEvent;
use vb_storage::journal::FjallJournal;
use vb_storage::records::RecordKind;
use vb_storage::types::EventSeq;
use vb_core::workflow::LifecycleState;
use vb_storage::derive_lifecycle_state_from_events;





#[test]
fn policy_decision_event_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let decision = pd_helpers::make_decision(100, DecisionCode::Denied, PolicyLayer::Emergency);
    let event = JournalEvent::PolicyDecisionEvent {
        run: RunId::new(100),
        seq: EventSeq::new(42),
        decision: decision.clone(),
    };

    assert_eq!(event.record_kind(), RecordKind::PolicyDecision);
    assert_eq!(event.record_kind().id(), 38);
    assert_eq!(event.run_id(), RunId::new(100));
    assert_eq!(event.seq(), EventSeq::new(42));

    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;

    let (envelope, decoded) = decode_journal_event(
        &encoded,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;

    assert_eq!(envelope.record_kind, 38);
    assert_eq!(decoded, event);

    match &decoded {
        JournalEvent::PolicyDecisionEvent {
            decision: d, run: r, seq: s
        } => {
            assert_eq!(*r, RunId::new(100));
            assert_eq!(*s, EventSeq::new(42));
            assert_eq!(d.outcome, DecisionCode::Denied);
            assert_eq!(d.layer, PolicyLayer::Emergency);
            assert_eq!(d.taint, Taint::Secret);
            assert_eq!(d.emergency_influence, InfluenceState::Active);
            assert_eq!(d.compat_influence, InfluenceState::NotEvaluated);
            assert_eq!(d.ticket.run, RunId::new(100));
            assert_eq!(d.ticket.step, StepIdx::new(2));
            assert_eq!(d.ticket.seq, SeqNo::new(7));
            assert_eq!(d.ticket.action, ActionId::new(5));
            assert_eq!(d.ticket.run_attempt, 1);
            assert_eq!(d.ticket.attempt, 1);
            assert_eq!(d.ticket.idempotency_key, 0);
            assert_eq!(d.ticket.capacity, 10);
            assert_eq!(d.admission, decision.admission);
            assert_eq!(d.current_live, decision.current_live);
            assert_eq!(d.input_digest, decision.input_digest);
        }
        _ => {
            return Err("decoded event is not PolicyDecisionEvent".into());
        }
    }

    Ok(())
}

#[test]
fn all_decision_enum_variants_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let cases = [
        (100, DecisionCode::Denied, PolicyLayer::Emergency, InfluenceState::Active, InfluenceState::NotEvaluated, Taint::Secret),
        (200, DecisionCode::Allowed, PolicyLayer::Live, InfluenceState::Inactive, InfluenceState::NotEvaluated, Taint::Clean),
        (300, DecisionCode::Suspended, PolicyLayer::Admission, InfluenceState::NotEvaluated, InfluenceState::NotEvaluated, Taint::DerivedFromSecret),
    ];

    for (i, (run, outcome, layer, emerg, compat, taint)) in cases.iter().enumerate() {
        let ticket = pd_helpers::make_ticket(*run, 2, 7, 5);
        let decision = DecisionRecord {
            ticket,
            admission: PinnedAdmissionPolicy::Artifact(pd_helpers::make_digest(1)),
            current_live: vb_core::policy::CurrentPolicyIdentity::Configured {
                digest: pd_helpers::make_digest(2),
            },
            input_digest: pd_helpers::make_digest(3),
            taint: *taint,
            outcome: *outcome,
            layer: *layer,
            emergency_influence: *emerg,
            compat_influence: *compat,
        };

        let event = JournalEvent::PolicyDecisionEvent {
            run: RunId::new(*run),
            seq: EventSeq::new(i as u64),
            decision: decision.clone(),
        };

        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            event.record_kind(),
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;

        let (_, decoded) = decode_journal_event(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;

        assert_eq!(decoded, event);
    }

    Ok(())
}

#[test]
fn fjall_persistence_survives_reopen() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let dir = temp.path();

    let run = RunId::new(1);
    let decision = pd_helpers::make_decision(1, DecisionCode::Denied, PolicyLayer::Emergency);
    let event = JournalEvent::PolicyDecisionEvent {
        run,
        seq: EventSeq::new(0),
        decision,
    };

    let journal = FjallJournal::open(dir, None)?;
    journal.append_strict(&event)?;
    drop(journal);

    let reopened = FjallJournal::open(dir, None)?;
    let read_events = reopened.events_for_run(run)?;
    drop(reopened);

    assert_eq!(read_events, vec![event]);

    Ok(())
}

#[test]
fn multiple_ordered_policy_events() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(500);
    let outcomes = [
        DecisionCode::Denied,
        DecisionCode::Allowed,
        DecisionCode::Suspended,
    ];
    let layers = [
        PolicyLayer::Emergency,
        PolicyLayer::Live,
        PolicyLayer::Admission,
    ];

    let mut events = Vec::new();
    for (i, outcome) in outcomes.iter().enumerate() {
        let decision = pd_helpers::make_decision(500, *outcome, layers[i]);
        let event = JournalEvent::PolicyDecisionEvent {
            run,
            seq: EventSeq::new(i as u64),
            decision,
        };
        events.push(event);
    }

    let journal_dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(journal_dir.path(), None)?;
    for event in &events {
        journal.append_strict(event)?;
    }
    drop(journal);

    let reopened = FjallJournal::open(journal_dir.path(), None)?;
    let read_events = reopened.events_for_run(run)?;
    drop(reopened);

    assert_eq!(read_events, events);

    Ok(())
}

#[test]
fn malformed_payload_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::PolicyDecision,
        0,
        &[1u8, 2u8, 3u8],
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;

    let result = decode_journal_event(
        &encoded,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(result.is_err(), "malformed payload should be rejected");

    Ok(())
}

#[test]
fn truncated_payload_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(1);
    let decision = pd_helpers::make_decision(1, DecisionCode::Denied, PolicyLayer::Emergency);
    let event = JournalEvent::PolicyDecisionEvent {
        run,
        seq: EventSeq::new(0),
        decision,
    };

    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;

    let half = encoded.len() / 2;
    let truncated: Vec<u8> = encoded[..half].to_vec();
    let result = decode_journal_event(
        &truncated,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(result.is_err(), "truncated payload should be rejected");

    Ok(())
}

#[test]
fn envelope_kind_mismatch_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(1);
    let decision = pd_helpers::make_decision(1, DecisionCode::Denied, PolicyLayer::Emergency);
    let event = JournalEvent::PolicyDecisionEvent {
        run,
        seq: EventSeq::new(0),
        decision,
    };

    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;

    let result = decode_journal_event(
        &encoded,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(
        result.is_err(),
        "envelope/payload kind mismatch should be rejected"
    );

    Ok(())
}

#[test]
fn run_envelope_mismatch_protected() -> Result<(), Box<dyn std::error::Error>> {
    // Outer run is 999, but ticket.run is 100 — this should be rejected
    let decision = pd_helpers::make_decision(100, DecisionCode::Denied, PolicyLayer::Emergency);
    let event = JournalEvent::PolicyDecisionEvent {
        run: RunId::new(999),
        seq: EventSeq::new(0),
        decision,
    };

    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;

    let result = decode_journal_event(
        &encoded,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );

    // Either encoding or decoding must detect the mismatch
    assert!(
        result.is_err(),
        "run envelope mismatch should be rejected; got Ok"
    );

    Ok(())
}

#[test]
fn journal_seq_not_ticket_seq() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(100);
    let decision = pd_helpers::make_decision(100, DecisionCode::Denied, PolicyLayer::Emergency);
    let event = JournalEvent::PolicyDecisionEvent {
        run,
        seq: EventSeq::new(42),
        decision: decision.clone(),
    };

    assert_eq!(event.seq(), EventSeq::new(42));

    match &event {
        JournalEvent::PolicyDecisionEvent { decision: d, .. } => {
            assert_eq!(d.ticket.seq, SeqNo::new(7));
            assert_ne!(d.ticket.seq, SeqNo::new(42));
        }
        _ => {
            return Err("event is not PolicyDecisionEvent".into());
        }
    }

    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;

    let (_, decoded) = decode_journal_event(
        &encoded,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;

    match &decoded {
        JournalEvent::PolicyDecisionEvent { decision: d, .. } => {
            assert_eq!(d.ticket.seq, SeqNo::new(7));
            assert_ne!(d.ticket.seq, SeqNo::new(42));
        }
        _ => {
            return Err("decoded event is not PolicyDecisionEvent".into());
        }
    }

    Ok(())
}

#[test]
fn old_tag_10_fixture_unchanged() -> Result<(), Box<dyn std::error::Error>> {
    // Legacy tag stability: RunAccepted must still encode with variant tag 10
    let event = JournalEvent::RunAccepted {
        run: RunId::new(16),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([15; 32]),
    };

    assert_eq!(event.record_kind(), RecordKind::RunAccepted);
    assert_eq!(event.record_kind().id(), 10);

    let wire = postcard::to_allocvec(&event)?;
    // The first byte of the postcard payload must be the variant tag 10
    assert_eq!(wire.first().copied(), Some(10));

    let decoded: JournalEvent = postcard::from_bytes(&wire)?;
    assert_eq!(decoded, event);

    Ok(())
}

#[test]
fn distinct_nonzero_digests() -> Result<(), Box<dyn std::error::Error>> {
    let decision = pd_helpers::make_decision(100, DecisionCode::Denied, PolicyLayer::Emergency);

    let zero_digest = PolicyDigest::from_bytes([0u8; 32]);
    assert_ne!(decision.input_digest, zero_digest);

    let other_digest = PolicyDigest::from_bytes([1u8; 32]);
    assert_ne!(decision.input_digest, other_digest);

    Ok(())
}

#[test]
fn influence_state_values_distinct() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        InfluenceState::NotEvaluated,
        InfluenceState::NotEvaluated
    );
    assert_ne!(InfluenceState::NotEvaluated, InfluenceState::Active);
    assert_ne!(InfluenceState::Inactive, InfluenceState::Active);
    assert_ne!(InfluenceState::NotEvaluated, InfluenceState::Inactive);
    Ok(())
}

#[test]
fn all_outcome_layers_taints_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let outcomes = [
        DecisionCode::Allowed,
        DecisionCode::Denied,
        DecisionCode::Suspended,
    ];
    let layers = [
        PolicyLayer::Live,
        PolicyLayer::Admission,
        PolicyLayer::Emergency,
    ];
    let taints = [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ];
    let influences = [
        InfluenceState::NotEvaluated,
        InfluenceState::Inactive,
        InfluenceState::Active,
    ];

    let run = RunId::new(800);
    let journal_dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(journal_dir.path(), None)?;

    let mut seq = 0;
    for outcome in outcomes.iter() {
        for layer in layers.iter() {
            for taint in taints.iter() {
                for emergency in influences.iter() {
                    for compat in influences.iter() {
                        let decision = DecisionRecord {
                            ticket: pd_helpers::make_ticket(800, 2, 7, 5),
                            admission: PinnedAdmissionPolicy::Artifact(pd_helpers::make_digest(1)),
                            current_live: vb_core::policy::CurrentPolicyIdentity::Configured {
                                digest: pd_helpers::make_digest(2),
                            },
                            input_digest: pd_helpers::make_digest(3),
                            taint: *taint,
                            outcome: *outcome,
                            layer: *layer,
                            emergency_influence: *emergency,
                            compat_influence: *compat,
                        };

                        let event = JournalEvent::PolicyDecisionEvent {
                            run,
                            seq: EventSeq::new(seq),
                            decision: decision.clone(),
                        };

                        journal.append_strict(&event)?;
                        seq += 1;
                    }
                }
            }
        }
    }
    drop(journal);

    let reopened = FjallJournal::open(journal_dir.path(), None)?;
    let read_events = reopened.events_for_run(run)?;
    drop(reopened);

    assert_eq!(read_events.len(), 3 * 3 * 5 * 3 * 3);

    Ok(())
}

#[test]
fn neutral_policy_fact_does_not_reset_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    // A PolicyDecisionEvent must not reset terminal lifecycle states.
    // Policy decision is a neutral fact that should preserve the last
    // lifecycle-determining event's state.

    let run = RunId::new(777);
    let decision = pd_helpers::make_decision(777, DecisionCode::Denied, PolicyLayer::Emergency);

    // Case 1: Failed -> Policy -> should remain Failed
    let events_failed = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([42u8; 32]),
        },
        JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(1),
            attempt: 1,
        },
        JournalEvent::PolicyDecisionEvent {
            run,
            seq: EventSeq::new(2),
            decision: decision.clone(),
        },
    ];

    let state_after_failed_policy = derive_lifecycle_state_from_events(&events_failed);
    assert_eq!(
        state_after_failed_policy,
        LifecycleState::Failed,
        "Policy decision after Failed should preserve Failed state"
    );

    // Case 2: Cancelled -> Policy -> should remain Cancelled
    let events_cancelled = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([42u8; 32]),
        },
        JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(1),
            attempt: 1,
            reason: None,
        },
        JournalEvent::PolicyDecisionEvent {
            run,
            seq: EventSeq::new(2),
            decision: decision.clone(),
        },
    ];

    let state_after_cancelled_policy = derive_lifecycle_state_from_events(&events_cancelled);
    assert_eq!(
        state_after_cancelled_policy,
        LifecycleState::Cancelled,
        "Policy decision after Cancelled should preserve Cancelled state"
    );

    // Case 3: Completed -> Policy -> should remain Completed
    let events_completed = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([42u8; 32]),
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(1),
            result: vb_core::ids::SlotIdx::new(0),
            attempt: 1,
        },
        JournalEvent::PolicyDecisionEvent {
            run,
            seq: EventSeq::new(2),
            decision: decision.clone(),
        },
    ];

    let state_after_completed_policy = derive_lifecycle_state_from_events(&events_completed);
    assert_eq!(
        state_after_completed_policy,
        LifecycleState::Completed,
        "Policy decision after Completed should preserve Completed state"
    );

    Ok(())
}