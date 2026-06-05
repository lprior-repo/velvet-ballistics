#![no_main]
#![forbid(unsafe_code)]

//! Fuzz artifact for `obl-vb-mrwe-5-ps003-fuzz-015`.

use libfuzzer_sys::fuzz_target;
use vb_core::{RunId, SlotIdx, StepIdx};
use vb_storage::{EventSeq, JournalEvent, RecordKind, constants};

fuzz_target!(|data: &[u8]| {
    check_roundtrip(step_event(), RecordKind::StepSucceeded.id());
    check_roundtrip(slot_event(), RecordKind::SlotWritten.id());
    if let Ok(record) = vb_storage::decode_validated_journal_record(
        data,
        constants::MAGIC_JOURNAL_EVENT,
        constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    ) {
        if record.envelope().record_kind != record.event().record_kind_id()
            || !record.parity().is_exact_match()
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

fn check_roundtrip(event: JournalEvent, expected_kind: u16) {
    match vb_storage::encode_journal_event_record(&event).and_then(|bytes| {
        vb_storage::decode_validated_journal_record(
            &bytes,
            constants::MAGIC_JOURNAL_EVENT,
            constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
    }) {
        Ok(record)
            if record.envelope().record_kind == expected_kind
                && record.event().record_kind_id() == expected_kind
                && record.parity().is_exact_match() => {}
        Ok(_) | Err(_) => std::process::abort(),
    }
}
