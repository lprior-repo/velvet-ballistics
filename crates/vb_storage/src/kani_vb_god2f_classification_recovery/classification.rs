#![forbid(unsafe_code)]

use crate::codec::{
    JournalSemanticDecodeDecision, RecordKindFamilyDecision, classify_journal_semantic_decode,
    classify_record_kind_family,
};
use crate::constants::MAGIC_JOURNAL_EVENT;
use crate::mrwe5_contract::{
    MRWE5_JOURNAL_MAX_KIND_ID, MRWE5_JOURNAL_MIN_KIND_ID, Mrwe5RecordKindFamilyDecision,
    mrwe5_classify_record_kind_family,
};

#[kani::proof]
#[kani::unwind(8)]
fn vb_god2f_storage_kind_family_and_semantic_decode() {
    let magic: u32 = kani::any();
    let kind: u16 = kani::any();
    let payload_kind: u16 = kani::any();
    let event_valid: bool = kani::any();

    let family = classify_record_kind_family(magic, kind);
    let mrwe5_family = mrwe5_classify_record_kind_family(magic, kind);
    kani::cover!(
        magic == MAGIC_JOURNAL_EVENT
            && kind >= MRWE5_JOURNAL_MIN_KIND_ID
            && kind <= MRWE5_JOURNAL_MAX_KIND_ID,
        "journal family accepted branch covered"
    );
    kani::cover!(
        magic != MAGIC_JOURNAL_EVENT,
        "non-journal magic branch covered"
    );
    if matches!(mrwe5_family, Mrwe5RecordKindFamilyDecision::Accepted) {
        kani::assert(
            family == RecordKindFamilyDecision::Accepted,
            "codec accepts MRWE5 journal-family kind",
        );
    }
    if magic == MAGIC_JOURNAL_EVENT
        && (kind < MRWE5_JOURNAL_MIN_KIND_ID || kind > MRWE5_JOURNAL_MAX_KIND_ID)
    {
        kani::assert(
            family == RecordKindFamilyDecision::Rejected,
            "codec rejects out-of-family journal kind",
        );
    }

    let semantic = classify_journal_semantic_decode(kind, payload_kind, event_valid);
    kani::cover!(
        kind == payload_kind && event_valid,
        "semantic success branch covered"
    );
    kani::cover!(
        kind == payload_kind && !event_valid,
        "invalid event branch covered"
    );
    kani::cover!(kind != payload_kind, "kind mismatch branch covered");
    if kind != payload_kind {
        kani::assert(
            semantic == JournalSemanticDecodeDecision::KindPayloadMismatch,
            "kind mismatch is classified before event validity",
        );
    } else if event_valid {
        kani::assert(
            semantic == JournalSemanticDecodeDecision::SemanticSuccess,
            "matching valid event is semantic success",
        );
    } else {
        kani::assert(
            semantic == JournalSemanticDecodeDecision::InvalidEvent,
            "matching invalid event is InvalidEvent",
        );
    }
}
