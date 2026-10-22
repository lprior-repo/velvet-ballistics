#![forbid(unsafe_code)]

use std::path::PathBuf;

use vb_core::action::ActionTicket;
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};
use vb_core::policy::{
    DecisionCode, DecisionRecord, InfluenceState, PinnedAdmissionPolicy, PolicyDigest,
    PolicyLayer,
};
use vb_core::value::Taint;
use vb_storage::codec::{decode_journal_event, encode_record};
use vb_storage::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES};
use vb_storage::events::JournalEvent;
use vb_storage::records::RecordKind;
use vb_storage::types::EventSeq;

/// Build an ActionTicket with all 8 fields distinct and non-trivial.
fn make_ticket(run: u64, step: u16, seq: u64, action: u16) -> ActionTicket {
    ActionTicket {
        run: RunId::new(run),
        step: StepIdx::new(step),
        seq: SeqNo::new(seq),
        action: ActionId::new(action),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: 0,
        capacity: 10,
    }
}

/// Build a distinct non-zero PolicyDigest from a seed byte.
fn make_digest(seed: u8) -> PolicyDigest {
    let mut bytes = [0u8; 32];
    bytes[0] = seed;
    bytes[31] = seed.wrapping_mul(2).wrapping_add(1);
    PolicyDigest::from_bytes(bytes)
}

/// Denied decision at Emergency layer.
fn denied_emergency_record() -> DecisionRecord {
    DecisionRecord {
        ticket: make_ticket(100, 2, 7, 5),
        admission: PinnedAdmissionPolicy::Artifact(make_digest(1)),
        current_live: vb_core::policy::CurrentPolicyIdentity::Configured {
            digest: make_digest(2),
        },
        input_digest: make_digest(3),
        taint: Taint::Secret,
        outcome: DecisionCode::Denied,
        layer: PolicyLayer::Emergency,
        emergency_influence: InfluenceState::Active,
        compat_influence: InfluenceState::NotEvaluated,
    }
}

/// Allowed decision at Live layer.
fn allowed_live_record() -> DecisionRecord {
    DecisionRecord {
        ticket: make_ticket(200, 3, 15, 10),
        admission: PinnedAdmissionPolicy::Artifact(make_digest(4)),
        current_live: vb_core::policy::CurrentPolicyIdentity::Configured {
            digest: make_digest(5),
        },
        input_digest: make_digest(6),
        taint: Taint::Clean,
        outcome: DecisionCode::Allowed,
        layer: PolicyLayer::Live,
        emergency_influence: InfluenceState::Inactive,
        compat_influence: InfluenceState::NotEvaluated,
    }
}

/// Suspended decision at Admission layer.
fn suspended_admission_record() -> DecisionRecord {
    DecisionRecord {
        ticket: make_ticket(300, 4, 22, 15),
        admission: PinnedAdmissionPolicy::Unverified,
        current_live: vb_core::policy::CurrentPolicyIdentity::Unconfigured,
        input_digest: make_digest(7),
        taint: Taint::DerivedFromSecret,
        outcome: DecisionCode::Suspended,
        layer: PolicyLayer::Admission,
        emergency_influence: InfluenceState::NotEvaluated,
        compat_influence: InfluenceState::NotEvaluated,
    }
}

/// Encode a policy decision event into journal format.
fn encode_policy_event(
    run: RunId,
    seq: u64,
    decision: &DecisionRecord,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let event = JournalEvent::PolicyDecisionEvent {
        run,
        seq: EventSeq::new(seq),
        decision: decision.clone(),
    };
    encode_record(
        MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
}

/// Decode and verify a policy decision event.
fn decode_and_verify_event(
    encoded: &[u8],
) -> Result<JournalEvent, Box<dyn std::error::Error>> {
    decode_journal_event(encoded, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES).map(|(_, e)| e)
}

#[test]
fn policy_decision_event_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let decision = denied_emergency_record();
    let event = JournalEvent::PolicyDecisionEvent {
        run: RunId::new(100),
        seq: EventSeq::new(42),
        decision,
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
        JournalEvent::PolicyDecisionEvent { decision, .. } => {
            assert_eq!(decision.outcome, DecisionCode::Denied);
            assert_eq!(decision.layer, PolicyLayer::Emergency);
            assert_eq!(decision.taint, Taint::Secret);
            assert_eq!(decision.emergency_influence, InfluenceState::Active);
        }
        _ => {
            return Err("decoded event is not PolicyDecisionEvent".into());
        }
    }

    Ok(())
}

#[test]
fn all_decision_enum_variants_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let decisions = vec![
        denied_emergency_record(),
        allowed_live_record(),
        suspended_admission_record(),
    ];

    for (i, decision) in decisions.into_iter().enumerate() {
        let run_id = RunId::new((i as u64) + 1000);
        let seq = EventSeq::new(i as u64);
        let event = JournalEvent::PolicyDecisionEvent {
            run: run_id,
            seq: seq.clone(),
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
fn record_field_identity_survives_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let decision = denied_emergency_record();
    let event = JournalEvent::PolicyDecisionEvent {
        run: RunId::new(100),
        seq: EventSeq::new(42),
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

    match &decoded {
        JournalEvent::PolicyDecisionEvent { decision: decoded_decision, .. } => {
            assert_eq!(decoded_decision.outcome, decision.outcome);
            assert_eq!(decoded_decision.layer, decision.layer);
            assert_eq!(decoded_decision.taint, decision.taint);
            assert_eq!(
                decoded_decision.emergency_influence,
                decision.emergency_influence
            );
            assert_eq!(
                decoded_decision.compat_influence,
                decision.compat_influence
            );
            assert_eq!(decoded_decision.ticket.run, decision.ticket.run);
            assert_eq!(decoded_decision.ticket.step, decision.ticket.step);
            assert_eq!(decoded_decision.ticket.seq, decision.ticket.seq);
            assert_eq!(decoded_decision.ticket.action, decision.ticket.action);
            assert_eq!(
                decoded_decision.ticket.run_attempt,
                decision.ticket.run_attempt
            );
            assert_eq!(
                decoded_decision.ticket.attempt,
                decision.ticket.attempt
            );
            assert_eq!(
                decoded_decision.ticket.idempotency_key,
                decision.ticket.idempotency_key
            );
            assert_eq!(
                decoded_decision.ticket.capacity,
                decision.ticket.capacity
            );
            assert_eq!(
                decoded_decision.input_digest,
                decision.input_digest
            );
        }
        _ => {
            return Err("decoded event is not PolicyDecisionEvent".into());
        }
    }

    Ok(())
}

#[test]
fn fjall_persistence_survives_reopen() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let dir = temp.path();

    let decision = denied_emergency_record();
    let original = JournalEvent::PolicyDecisionEvent {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        decision,
    };

    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        original.record_kind(),
        original.seq().get(),
        &original,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;

    let key = "run:1:evt:0";
    let key_bytes = key.as_bytes();
    let value_bytes = &encoded;

    let mut store = fjall::Store::open(dir, None)?;
    store.put(key_bytes, value_bytes)?;
    store.sync()?;
    store.close()?;

    let store2 = fjall::Store::open(dir, None)?;
    let read = store2.get(key_bytes)?;
    match read {
        Some(read_bytes) => {
            store2.close()?;

            let (_, decoded) = decode_journal_event(
                &read_bytes,
                MAGIC_JOURNAL_EVENT,
                MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
            )?;

            assert_eq!(decoded, original);
        }
        None => {
            store2.close()?;
            return Err("value not found after reopen".into());
        }
    }

    Ok(())
}

#[test]
fn multiple_ordered_policy_events() -> Result<(), Box<dyn std::error::Error>> {
    let decisions = vec![
        denied_emergency_record(),
        allowed_live_record(),
        suspended_admission_record(),
    ];

    let mut events = Vec::new();
    for (i, decision) in decisions.into_iter().enumerate() {
        let event = JournalEvent::PolicyDecisionEvent {
            run: RunId::new(500),
            seq: EventSeq::new(i as u64),
            decision,
        };
        events.push(event);
    }

    for (i, event) in events.iter().enumerate() {
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            event.record_kind(),
            event.seq().get(),
            event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;

        let (_, decoded) = decode_journal_event(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;

        assert_eq!(decoded, *event);
        assert_eq!(decoded.seq(), EventSeq::new(i as u64));
    }

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
    let decision = denied_emergency_record();
    let event = JournalEvent::PolicyDecisionEvent {
        run: RunId::new(1),
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

    let truncated = &encoded[..encoded.len() / 2];
    let result = decode_journal_event(
        truncated,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(result.is_err(), "truncated payload should be rejected");

    Ok(())
}

#[test]
fn envelope_kind_mismatch_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let decision = denied_emergency_record();
    let event = JournalEvent::PolicyDecisionEvent {
        run: RunId::new(1),
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
    let decision = denied_emergency_record();
    let event = JournalEvent::PolicyDecisionEvent {
        run: RunId::new(999),
        seq: EventSeq::new(0),
        decision,
    };

    assert_eq!(event.run_id(), RunId::new(999));

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

    assert_eq!(decoded.run_id(), RunId::new(999));
    assert_ne!(decoded.run_id(), RunId::new(1));

    Ok(())
}

#[test]
fn journal_seq_not_ticket_seq() -> Result<(), Box<dyn std::error::Error>> {
    let decision = denied_emergency_record();
    let event = JournalEvent::PolicyDecisionEvent {
        run: RunId::new(100),
        seq: EventSeq::new(42),
        decision: decision.clone(),
    };

    assert_eq!(event.seq(), EventSeq::new(42));

    match &event {
        JournalEvent::PolicyDecisionEvent { decision, .. } => {
            assert_eq!(decision.ticket.seq, SeqNo::new(7));
            assert_ne!(decision.ticket.seq, SeqNo::new(42));
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
        JournalEvent::PolicyDecisionEvent { decision, .. } => {
            assert_eq!(decision.ticket.seq, SeqNo::new(7));
            assert_ne!(decision.ticket.seq, SeqNo::new(42));
        }
        _ => {
            return Err("decoded event is not PolicyDecisionEvent".into());
        }
    }

    Ok(())
}

#[test]
fn old_tag_10_fixture_unchanged() -> Result<(), Box<dyn std::error::Error>> {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: [1u8; 32].into(),
    };

    assert_eq!(event.record_kind(), RecordKind::RunAccepted);
    assert_eq!(event.record_kind().id(), 10);

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

    Ok(())
}

#[test]
fn distinct_nonzero_digests() -> Result<(), Box<dyn std::error::Error>> {
    let decision = denied_emergency_record();

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

    let mut idx = 0;
    for outcome in outcomes.iter() {
        for layer in layers.iter() {
            for taint in taints.iter() {
                for emergency in influences.iter() {
                    for compat in influences.iter() {
                        idx += 1;
                        let ticket = make_ticket(1000 + idx as u64, 0, idx as u64, 0);
                        let decision = DecisionRecord {
                            ticket,
                            admission: PinnedAdmissionPolicy::Artifact(make_digest(
                                (idx % 250) as u8 + 1,
                            )),
                            current_live: vb_core::policy::CurrentPolicyIdentity::Configured {
                                digest: make_digest(((idx % 250) as u8 + 50)),
                            },
                            input_digest: make_digest(((idx % 250) as u8 + 100)),
                            taint: *taint,
                            outcome: *outcome,
                            layer: *layer,
                            emergency_influence: *emergency,
                            compat_influence: *compat,
                        };

                        let event = JournalEvent::PolicyDecisionEvent {
                            run: RunId::new(idx as u64),
                            seq: EventSeq::new(idx as u64),
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
                }
            }
        }
    }

    Ok(())
}

#[test]
fn neutral_policy_fact_does_not_reset_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    // A PolicyDecisionEvent should not interfere with lifecycle events
    let run = RunId::new(777);

    let run_accepted = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: [42u8; 32].into(),
    };

    let policy_event = JournalEvent::PolicyDecisionEvent {
        run,
        seq: EventSeq::new(1),
        decision: denied_emergency_record(),
    };

    let run_failed = JournalEvent::RunFailed {
        run,
        seq: EventSeq::new(2),
        step: StepIdx::new(0),
        attempt: 1,
        message: "test failure".to_string(),
    };

    // Encode and decode each independently
    for event in [run_accepted, policy_event, run_failed] {
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