#![no_main]
#![forbid(unsafe_code)]

//! Fuzz artifact for `obl-vb-mrwe-5-ps001-fuzz-005`.

use libfuzzer_sys::fuzz_target;
use vb_core::{RunId, SlotIdx, StepIdx};
use vb_storage::codec::{decode_validated_journal_record, encode_journal_event_record};
use vb_storage::{EventSeq, JournalEvent, RecordKind, constants};

fuzz_target!(|data: &[u8]| {
    check_canonical_event(step_event(), RecordKind::StepSucceeded.id());
    check_canonical_event(slot_event(), RecordKind::SlotWritten.id());
    if let Ok(record) = decode_validated_journal_record(
        data,
        constants::MAGIC_JOURNAL_EVENT,
        constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    ) {
        let parity = record.parity();
        if !parity.is_exact_match()
            || parity.envelope_kind() != record.event().record_kind_id()
            || parity.payload_kind() != record.event().record_kind_id()
        {
            std::process::abort();
        }
    }
});

fn step_event() -> JournalEvent {
    JournalEvent::StepSucceeded {
        run: RunId::new(1),
        seq: EventSeq::new(1),
        step: StepIdx::new(2),
        output: SlotIdx::new(3),
    }
}

fn slot_event() -> JournalEvent {
    JournalEvent::SlotWrittenEvent {
        run: RunId::new(1),
        seq: EventSeq::new(2),
        slot: SlotIdx::new(3),
        value: None,
        extra: None,
        attempt: 1,
    }
}

fn check_canonical_event(event: JournalEvent, expected_kind: u16) {
    if event.record_kind_id() != expected_kind {
        std::process::abort();
    }
    match encode_journal_event_record(&event).and_then(|bytes| {
        decode_validated_journal_record(
            &bytes,
            constants::MAGIC_JOURNAL_EVENT,
            constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
    }) {
        Ok(record)
            if record.parity().is_exact_match()
                && record.envelope().record_kind == expected_kind => {}
        Ok(_) | Err(_) => std::process::abort(),
    }
}
