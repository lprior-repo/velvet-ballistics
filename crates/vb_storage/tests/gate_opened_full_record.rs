//! Wire contract for the durable manual-gate opening event (§77.15).
//!
//! `JournalEvent::GateOpenedEvent` carries the gate definition as opened,
//! its opening time, and the requesting operator under the stable
//! `RecordKind::GateOpened` tag (48). The codec round-trip, the run-match
//! enforcement, Fjall persistence across reopen, and the lifecycle-neutral
//! classification are all pinned here.

#![forbid(unsafe_code)]

use vb_core::ids::{ActionId, RunId};
use vb_core::manual_gate::{
    ApproverId, ApproverRole, GateDigest, GateId, ManualGate, RiskClass, SeparationOfDuties,
};
use vb_core::workflow::LifecycleState;
use vb_storage::codec::{decode_journal_event, encode_record};
use vb_storage::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES};
use vb_storage::derive_lifecycle_state_from_events;
use vb_storage::events::JournalEvent;
use vb_storage::journal::FjallJournal;
use vb_storage::records::RecordKind;
use vb_storage::types::EventSeq;

fn digest(byte: u8) -> GateDigest {
    GateDigest::from_bytes([byte; 32])
}

fn fixture_gate(run: RunId) -> ManualGate {
    ManualGate {
        id: GateId::new(7),
        run,
        action: ActionId::new(9),
        risk: RiskClass::High,
        compensatable: false,
        required_role: ApproverRole::new("deploy-approver".to_string()).unwrap(),
        separation_of_duties: SeparationOfDuties::DistinctFromRequester,
        evidence_bundle_digest: digest(1),
        prompt_digest: digest(2),
        timeout_ms: 1_000,
        escalation_policy_digest: digest(3),
        break_glass_allowed: true,
        approval_reason_required: true,
    }
}

fn opened_event(run: RunId, seq: u64) -> JournalEvent {
    JournalEvent::GateOpenedEvent {
        run,
        seq: EventSeq::new(seq),
        gate: fixture_gate(run),
        opened_at_ms: 100,
        requester: ApproverId::new("bob".to_string()).unwrap(),
    }
}

#[test]
fn gate_opened_event_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let event = opened_event(RunId::new(100), 42);

    assert_eq!(event.record_kind(), RecordKind::GateOpened);
    assert_eq!(event.record_kind().id(), 48);
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

    assert_eq!(envelope.record_kind, 48);
    assert_eq!(decoded, event);

    match &decoded {
        JournalEvent::GateOpenedEvent {
            gate,
            opened_at_ms,
            requester,
            run: r,
            seq: s,
        } => {
            assert_eq!(*r, RunId::new(100));
            assert_eq!(*s, EventSeq::new(42));
            assert_eq!(*gate, fixture_gate(RunId::new(100)));
            assert_eq!(*opened_at_ms, 100);
            assert_eq!(requester.value(), "bob");
        }
        _ => {
            return Err("decoded event is not GateOpenedEvent".into());
        }
    }

    Ok(())
}

#[test]
fn gate_opened_event_rejects_run_mismatch() {
    let mut event = opened_event(RunId::new(100), 0);
    if let JournalEvent::GateOpenedEvent { ref mut gate, .. } = event {
        gate.run = RunId::new(101);
    }
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(
        encoded.is_err(),
        "outer run must equal gate.run on the wire"
    );
}

#[test]
fn fjall_persistence_survives_reopen() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let dir = temp.path();

    let run = RunId::new(1);
    let event = opened_event(run, 0);

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
fn gate_opened_event_is_lifecycle_neutral() {
    let opened = opened_event(RunId::new(100), 0);
    assert_eq!(
        derive_lifecycle_state_from_events(std::slice::from_ref(&opened)),
        LifecycleState::Pending,
        "an opened gate alone leaves lifecycle pending"
    );

    let failed = JournalEvent::RunFailedEvent {
        run: RunId::new(100),
        seq: EventSeq::new(1),
        attempt: 1,
    };
    assert_eq!(
        derive_lifecycle_state_from_events(&[failed, opened]),
        LifecycleState::Failed,
        "a later opened gate must not overwrite a failed lifecycle"
    );
}
