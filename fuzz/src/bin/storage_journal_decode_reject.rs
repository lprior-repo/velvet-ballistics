#![no_main]
#![forbid(unsafe_code)]

//! Fuzz artifact for `obl-vb-mrwe-5-ps002-fuzz-010`.

use libfuzzer_sys::fuzz_target;
use vb_storage::{JournalEvent, JournalSemanticDecodeDecision, constants};

fuzz_target!(|data: &[u8]| {
    let generic = vb_storage::decode_record::<JournalEvent>(
        data,
        constants::MAGIC_JOURNAL_EVENT,
        constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    if let Ok((envelope, event)) = generic {
        let decision = vb_storage::classify_journal_semantic_decode(
            envelope.record_kind,
            event.record_kind_id(),
            event.is_valid(),
        );
        match decision {
            JournalSemanticDecodeDecision::SemanticSuccess => {
                if vb_storage::decode_validated_journal_record(
                    data,
                    constants::MAGIC_JOURNAL_EVENT,
                    constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
                )
                .is_err()
                {
                    std::process::abort();
                }
            }
            JournalSemanticDecodeDecision::KindPayloadMismatch
            | JournalSemanticDecodeDecision::InvalidEvent => {
                if vb_storage::decode_validated_journal_record(
                    data,
                    constants::MAGIC_JOURNAL_EVENT,
                    constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
                )
                .is_ok()
                {
                    std::process::abort();
                }
            }
        }
    }
});
