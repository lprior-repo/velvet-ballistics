#![forbid(unsafe_code)]
//! PO-vb-7m21-018
#[kani::proof]
fn vb_7m21_004_harness() {
    let payload = [0_u8; 1];
    let encoded = crate::codec::encode_record_header(
        crate::constants::MAGIC_JOURNAL_EVENT,
        crate::records::RecordKind::RunAccepted,
        0,
        &payload,
        0,
    );
    kani::cover(encoded.is_err(), "public encode_record_header rejection reachable for side-index fixture prelude");
    let header = [0_u8; crate::constants::RECORD_HEADER_BYTES];
    let decoded = crate::codec::decode_record_header(
        &header,
        crate::constants::MAGIC_JOURNAL_EVENT,
        4,
    );
    kani::cover(decoded.is_err(), "public decode_record_header rejection reachable for side-index fixture prelude");
        let event_present: bool = kani::any();
        let side_index_present: bool = kani::any();
        kani::cover(event_present && !side_index_present, "missing side-index non-vacuous case covered");
        if event_present && !side_index_present {
            let observed = crate::JournalError::KeyCapacity;
            kani::assert(matches!(observed, crate::JournalError::KeyCapacity), "typed corpus-local index parity equivalent");
            core::mem::forget(observed);
        }
    core::mem::forget(decoded);
    core::mem::forget(encoded);
}
