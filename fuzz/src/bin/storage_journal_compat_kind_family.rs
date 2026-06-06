#![no_main]
#![forbid(unsafe_code)]

//! Fuzz artifact for `obl-vb-mrwe-5-ps004-fuzz-020`.

use libfuzzer_sys::fuzz_target;
use vb_storage::codec::{
    JournalKindCompatibility, RecordKindFamilyDecision, classify_journal_kind_compatibility,
    classify_record_kind_family,
};
use vb_storage::{RecordKind, constants};

fuzz_target!(|data: &[u8]| {
    check_pair(
        RecordKind::StepSucceeded.id(),
        RecordKind::StepSucceeded.id(),
    );
    check_pair(RecordKind::SlotWritten.id(), RecordKind::SlotWritten.id());
    check_pair(RecordKind::SlotWritten.id(), RecordKind::StepSucceeded.id());
    check_pair(RecordKind::StepSucceeded.id(), RecordKind::SlotWritten.id());

    for chunk in data.chunks(2) {
        let kind = match chunk {
            [lo, hi] => u16::from_le_bytes([*lo, *hi]),
            [lo] => u16::from(*lo),
            _ => 0,
        };
        if !vb_storage::codec::is_known_record_kind(kind)
            && matches!(
                classify_record_kind_family(constants::MAGIC_JOURNAL_EVENT, kind),
                RecordKindFamilyDecision::Accepted
            )
        {
            std::process::abort();
        }
        check_pair(kind, RecordKind::StepSucceeded.id());
        check_pair(kind, RecordKind::SlotWritten.id());
    }
});

fn check_pair(envelope_kind: u16, payload_kind: u16) {
    let compatibility = classify_journal_kind_compatibility(envelope_kind, payload_kind);
    match compatibility {
        JournalKindCompatibility::ExactMatch if envelope_kind == payload_kind => {}
        JournalKindCompatibility::RejectedMismatch if envelope_kind != payload_kind => {}
        JournalKindCompatibility::ExactMatch | JournalKindCompatibility::RejectedMismatch => {
            std::process::abort();
        }
    }
}
