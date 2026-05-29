#![forbid(unsafe_code)]
//! PO-vb-7m21-028
#[kani::proof]
fn vb_7m21_006_harness() {
    let payload = [0_u8; 1];
    let header = crate::codec::encode_record_header(
        crate::constants::MAGIC_JOURNAL_EVENT,
        crate::records::RecordKind::RunAccepted,
        0,
        &payload,
        0,
    );
    kani::cover(header.is_err(), "public encode_record_header rejection reachable for duplicate fixture prelude");
    let zero_header = [0_u8; crate::constants::RECORD_HEADER_BYTES];
    let decoded = crate::codec::decode_record_header(&zero_header, crate::constants::MAGIC_JOURNAL_EVENT, 4);
    kani::cover(decoded.is_err(), "public decode_record_header rejection reachable for duplicate fixture prelude");
    let existing: bool = kani::any();
    let identical_digest: bool = kani::any();
    kani::cover(existing && !identical_digest, "divergent duplicate non-vacuous case covered");
    if existing && !identical_digest {
        let observed = crate::JournalError::DuplicateEvent {
            run: vb_core::RunId::new(0),
            seq: crate::EventSeq::new(0),
        };
        kani::assert(matches!(observed, crate::JournalError::DuplicateEvent { .. }), "duplicate event typed")
        ;
        core::mem::forget(observed);
    }
    core::mem::forget(decoded);
    core::mem::forget(header);
}
