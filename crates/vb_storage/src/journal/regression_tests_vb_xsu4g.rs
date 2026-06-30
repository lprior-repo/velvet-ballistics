//! Regression tests for bead vb-xsu4g: P0 durable action events must
//! preserve the full `ActionTicket` (and specifically the
//! `idempotency_key`) through the journal codec. The legacy
//! `ActionCompletedEvent` variant dropped the ticket on the floor, so
//! durable action events could not be matched with their scheduled
//! ticket on replay. The fix introduced `ActionCompletedEnvelope`
//! which carries the full ticket payload. These tests pin the
//! encoding contract so any future field-shape change or
//! `#[serde(...)]` attribute regression is caught by CI.
//!
//! Two layers are exercised:
//!
//! 1. Postcard-level roundtrip through the journal envelope using
//!    `encode_record` + `parse_event`. This is the canonical decode
//!    path for untrusted journal input streams and enforces the
//!    `JournalEvent::is_valid()` and envelope/payload parity checks.
//! 2. Fjall-backed journal roundtrip using `append_strict_batch`
//!    plus `events_for_run`. This is the full durable path the
//!    runtime actually uses when persisting `ActionCompletedEnvelope`.
//!
//! Both layers must preserve every `ActionTicket` field, including
//! `idempotency_key`, otherwise recovery cannot dedupe a replayed
//! completion against the scheduled ticket.

#![forbid(unsafe_code)]

use crate::{
    DurableActionOutcome, EventSeq, JournalEvent,
    codec::encode_record,
    constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES},
    journal::parse_event,
};
use vb_core::{
    ActionId, ActionTicket, RunId, SlotIdx, StepIdx, WorkflowDigest, ids::SeqNo, value::Taint,
};

/// Construct a `ActionCompletedEnvelope` with distinctive, non-default
/// values for every field. In particular the `idempotency_key` is
/// non-zero and arbitrary, so a silent drop of the field during
/// roundtrip will be caught by the equality assertion.
fn sample_completed_envelope() -> JournalEvent {
    JournalEvent::ActionCompletedEnvelope {
        run: RunId::new(0x0123_4567_89AB_CDEF_u64),
        seq: EventSeq::new(1),
        ticket: ActionTicket {
            run: RunId::new(0x0123_4567_89AB_CDEF_u64),
            step: StepIdx::new(0x00C3),
            seq: SeqNo::new(0x0042_1337_9000_0001_u64),
            action: ActionId::new(0x07B7),
            attempt: 2,
            idempotency_key: 0xA5A5_A5A5_5A5A_5A5A_B0B0_B0B0_0B0B_0B0B_u128,
            capacity: 5,
        },
        output: SlotIdx::new(3),
        outcome: DurableActionOutcome::Ready,
        value: vec![0xDE, 0xAD, 0xBE, 0xEF],
        encoded_len: 4,
        taint: Taint::Clean,
        value_digest: [0x5A; 32],
        action_abi_digest: WorkflowDigest::from_bytes([0xC0; 32]),
    }
}

// ----- Postcard-level roundtrip through the journal envelope ------------

#[test]
fn action_completed_envelope_roundtrips_through_parse_event() {
    let original = sample_completed_envelope();

    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        original.record_kind(),
        original.seq().get(),
        &original,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("encode_record must accept ActionCompletedEnvelope");

    let parsed = parse_event(&bytes)
        .expect("parse_event must decode ActionCompletedEnvelope after encode_record");

    // Exact-shape match: this is the precise contract under test.
    match parsed {
        JournalEvent::ActionCompletedEnvelope {
            run,
            seq,
            ticket,
            output,
            outcome,
            value,
            encoded_len,
            taint,
            value_digest,
            action_abi_digest,
        } => {
            assert_eq!(
                run,
                RunId::new(0x0123_4567_89AB_CDEF_u64),
                "run must survive"
            );
            assert_eq!(seq, EventSeq::new(1), "seq must survive");

            // Full ticket must survive verbatim: same run, same step, same
            // seq, same action, same attempt, same idempotency_key, same
            // capacity. A field drop or rename fails this equality.
            let expected_ticket = ActionTicket {
                run: RunId::new(0x0123_4567_89AB_CDEF_u64),
                step: StepIdx::new(0x00C3),
                seq: SeqNo::new(0x0042_1337_9000_0001_u64),
                action: ActionId::new(0x07B7),
                attempt: 2,
                idempotency_key: 0xA5A5_A5A5_5A5A_5A5A_B0B0_B0B0_0B0B_0B0B_u128,
                capacity: 5,
            };
            assert_eq!(
                ticket, expected_ticket,
                "ActionTicket must survive journal encode/decode verbatim"
            );
            assert_eq!(
                ticket.idempotency_key, expected_ticket.idempotency_key,
                "ActionCompletedEnvelope.ticket.idempotency_key must survive roundtrip"
            );

            assert_eq!(output, SlotIdx::new(3), "output slot must survive");
            assert_eq!(outcome, DurableActionOutcome::Ready, "outcome must survive");
            assert_eq!(
                value,
                vec![0xDE, 0xAD, 0xBE, 0xEF],
                "value bytes must survive"
            );
            assert_eq!(encoded_len, 4, "encoded_len must survive");
            assert_eq!(taint, Taint::Clean, "taint must survive");
            assert_eq!(value_digest, [0x5A; 32], "value_digest must survive");
            assert_eq!(
                action_abi_digest,
                WorkflowDigest::from_bytes([0xC0; 32]),
                "action_abi_digest must survive"
            );
        }
        other => panic!("expected ActionCompletedEnvelope, got {other:?}"),
    }
}

// ----- Fjall-backed durable journal roundtrip ----------------------------

fn temp_journal() -> (tempfile::TempDir, crate::FjallJournal) {
    let temp = tempfile::tempdir().expect("tempdir creation must succeed");
    let journal =
        crate::FjallJournal::open(temp.path(), None).expect("FjallJournal open must succeed");
    (temp, journal)
}

#[test]
fn action_completed_envelope_roundtrips_through_fjall_journal() {
    let (temp, journal) = temp_journal();
    let original = sample_completed_envelope();

    // The journal enforces a contiguous per-run sequence starting at 0,
    // so seed a `RunAccepted` event before the completion envelope.
    let run_accepted = JournalEvent::RunAccepted {
        run: original.run_id(),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0xAA; 32]),
    };
    let batch = [run_accepted, original.clone()];
    journal
        .append_strict_batch(&batch)
        .expect("append_strict_batch must accept RunAccepted + ActionCompletedEnvelope");

    let replayed = journal
        .events_for_run(original.run_id())
        .expect("events_for_run must succeed for the run we just appended");

    assert_eq!(replayed.len(), 2, "replay must yield both seeded events");
    let replayed_envelope = replayed
        .iter()
        .find_map(|event| match event {
            JournalEvent::ActionCompletedEnvelope { .. } => Some(event),
            _ => None,
        })
        .expect("replayed stream must contain the ActionCompletedEnvelope");
    assert_eq!(
        replayed_envelope, &original,
        "replayed ActionCompletedEnvelope must equal original including full ticket"
    );

    // Defense in depth: the replayed ticket is the exact ticket, with
    // the exact idempotency_key. This is the field recovery needs to
    // match the completion back to its scheduled action.
    match replayed_envelope {
        JournalEvent::ActionCompletedEnvelope { ticket, .. } => {
            assert_eq!(
                ticket.idempotency_key, 0xA5A5_A5A5_5A5A_5A5A_B0B0_B0B0_0B0B_0B0B_u128,
                "replayed envelope's ticket.idempotency_key must equal the original"
            );
            assert_eq!(
                ticket.run,
                RunId::new(0x0123_4567_89AB_CDEF_u64),
                "replayed envelope's ticket.run must equal the original"
            );
            assert_eq!(
                ticket.seq,
                SeqNo::new(0x0042_1337_9000_0001_u64),
                "replayed envelope's ticket.seq must equal the original"
            );
            assert_eq!(
                ticket.action,
                ActionId::new(0x07B7),
                "ticket.action must survive"
            );
            assert_eq!(
                ticket.step,
                StepIdx::new(0x00C3),
                "ticket.step must survive"
            );
            assert_eq!(ticket.attempt, 2, "ticket.attempt must survive");
            assert_eq!(ticket.capacity, 5, "ticket.capacity must survive");
        }
        other => panic!("expected ActionCompletedEnvelope, got {other:?}"),
    }

    // Hold the temp dir alive until assertions are complete so the
    // Fjall backing store is not dropped mid-test.
    drop(temp);
}
