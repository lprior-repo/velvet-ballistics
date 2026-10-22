#![forbid(unsafe_code)]

use std::fs;
use std::io;
use std::path::PathBuf;

use vb_core::action::ActionTicket;
use vb_core::ids::{ActionId, RunId};
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

fn create_test_dir(name: &str) -> io::Result<PathBuf> {
    let path = PathBuf::from("/tmp").join(format!("vb-pd-{}", name));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn test_decision_record() -> DecisionRecord {
    let digest = [42u8; 32];
    let admission_digest = [99u8; 32];
    let input_digest = [123u8; 32];

    DecisionRecord {
        ticket: ActionTicket {
            run: RunId::new(100),
            attempt: 1,
            step: 2,
            action: ActionId::new(5),
            input_slot: 0,
            output_slot: 1,
            abi_digest: [111u8; 32],
            seq: 7,
        },
        admission: PinnedAdmissionPolicy::Artifact(PolicyDigest::new_verified(digest)),
        current_live: vb_core::policy::CurrentPolicyIdentity::Configured(
            PolicyDigest::new_verified(admission_digest),
        ),
        input_digest: PolicyDigest::new_verified(input_digest),
        taint: Taint::Secret,
        outcome: DecisionCode::Denied,
        layer: PolicyLayer::Emergency,
        emergency_influence: InfluenceState::Active,
        compat_influence: InfluenceState::NotEvaluated,
    }
}

fn allowed_decision_record() -> DecisionRecord {
    let digest = [10u8; 32];
    let admission_digest = [20u8; 32];
    let input_digest = [30u8; 32];

    DecisionRecord {
        ticket: ActionTicket {
            run: RunId::new(200),
            attempt: 2,
            step: 3,
            action: ActionId::new(10),
            input_slot: 2,
            output_slot: 3,
            abi_digest: [40u8; 32],
            seq: 15,
        },
        admission: PinnedAdmissionPolicy::Artifact(PolicyDigest::new_verified(digest)),
        current_live: vb_core::policy::CurrentPolicyIdentity::Configured(
            PolicyDigest::new_verified(admission_digest),
        ),
        input_digest: PolicyDigest::new_verified(input_digest),
        taint: Taint::Clean,
        outcome: DecisionCode::Allowed,
        layer: PolicyLayer::Live,
        emergency_influence: InfluenceState::Inactive,
        compat_influence: InfluenceState::NotEvaluated,
    }
}

fn suspended_decision_record() -> DecisionRecord {
    let digest = [50u8; 32];
    let admission_digest = [60u8; 32];
    let input_digest = [70u8; 32];

    DecisionRecord {
        ticket: ActionTicket {
            run: RunId::new(300),
            attempt: 3,
            step: 4,
            action: ActionId::new(15),
            input_slot: 4,
            output_slot: 5,
            abi_digest: [80u8; 32],
            seq: 22,
        },
        admission: PinnedAdmissionPolicy::Unverified,
        current_live: vb_core::policy::CurrentPolicyIdentity::Unconfigured,
        input_digest: PolicyDigest::new_verified(input_digest),
        taint: Taint::DerivedFromSecret,
        outcome: DecisionCode::Suspended,
        layer: PolicyLayer::Admission,
        emergency_influence: InfluenceState::NotEvaluated,
        compat_influence: InfluenceState::Compatible,
    }
}

#[test]
fn policy_decision_event_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let decision = test_decision_record();
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

    if let JournalEvent::PolicyDecisionEvent { decision, .. } = &decoded {
        assert_eq!(decision.outcome, DecisionCode::Denied);
        assert_eq!(decision.layer, PolicyLayer::Emergency);
        assert_eq!(decision.taint, Taint::Secret);
        assert_eq!(decision.emergency_influence, InfluenceState::Active);
    } else {
        return Err("decoded event is not PolicyDecisionEvent".into());
    }

    Ok(())
}

#[test]
fn all_decision_enum_variants_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let decisions = vec![
        test_decision_record(),
        allowed_decision_record(),
        suspended_decision_record(),
    ];

    for (i, decision) in decisions.into_iter().enumerate() {
        let event = JournalEvent::PolicyDecisionEvent {
            run: RunId::new(i as u64 + 1000),
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
fn record_field_identity_survives_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let decision = test_decision_record();

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

    if let JournalEvent::PolicyDecisionEvent { decision: decoded_decision, .. } = &decoded {
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
        assert_eq!(decoded_decision.ticket.attempt, decision.ticket.attempt);
        assert_eq!(decoded_decision.ticket.step, decision.ticket.step);
        assert_eq!(decoded_decision.ticket.action, decision.ticket.action);
        assert_eq!(
            decoded_decision.ticket.input_slot,
            decision.ticket.input_slot
        );
        assert_eq!(
            decoded_decision.ticket.output_slot,
            decision.ticket.output_slot
        );
    } else {
        return Err("decoded event is not PolicyDecisionEvent".into());
    }

    Ok(())
}

#[test]
fn fjall_persistence_survives_reopen() -> Result<(), Box<dyn std::error::Error>> {
    let dir = create_test_dir("persist")?;

    let decision = test_decision_record();
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

    let key = format!(
        "run:1:evt:0",
    );
    let key_bytes = key.as_bytes();
    let value_bytes = &encoded;

    let mut store = fjall::Store::open(&dir, None)?;
    store.put(key_bytes, value_bytes)?;
    store.sync()?;
    store.close()?;

    let store2 = fjall::Store::open(&dir, None)?;
    let read = store2.get(key_bytes)?;
    assert!(read.is_some());
    let read_bytes = read.unwrap();
    store2.close()?;

    let (_, decoded) = decode_journal_event(
        &read_bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;

    assert_eq!(decoded, original);

    Ok(())
}

#[test]
fn multiple_ordered_policy_events() -> Result<(), Box<dyn std::error::Error>> {
    let decisions = vec![
        test_decision_record(),
        allowed_decision_record(),
        suspended_decision_record(),
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
    let decision = test_decision_record();
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
    let decision = test_decision_record();
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
    let decision = test_decision_record();
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
    let mut decision = test_decision_record();
    decision.ticket.seq = 7;

    let event = JournalEvent::PolicyDecisionEvent {
        run: RunId::new(100),
        seq: EventSeq::new(42),
        decision,
    };

    assert_eq!(event.seq(), EventSeq::new(42));

    if let JournalEvent::PolicyDecisionEvent { decision, .. } = &event {
        assert_eq!(decision.ticket.seq, 7);
        assert_ne!(decision.ticket.seq, 42);
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

    if let JournalEvent::PolicyDecisionEvent { decision, .. } = &decoded {
        assert_eq!(decision.ticket.seq, 7);
        assert_ne!(decision.ticket.seq, 42);
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
    let decision = test_decision_record();

    assert_ne!(decision.input_digest, PolicyDigest::new_verified([0u8; 32]));
    assert_ne!(
        decision.input_digest,
        PolicyDigest::new_verified([1u8; 32])
    );

    Ok(())
}