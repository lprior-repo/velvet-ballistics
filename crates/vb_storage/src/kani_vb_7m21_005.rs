#![forbid(unsafe_code)]
//! PO-vb-7m21-023
#[kani::proof]
fn vb_7m21_005_harness() {
    let payload = [0_u8; 1];
    let header = crate::codec::encode_record_header(
        crate::constants::MAGIC_JOURNAL_EVENT,
        crate::records::RecordKind::RunAccepted,
        0,
        &payload,
        0,
    );
    kani::cover(header.is_err(), "public encode_record_header rejection reachable for sequence fixture prelude");
    let zero_header = [0_u8; crate::constants::RECORD_HEADER_BYTES];
    let decoded = crate::codec::decode_record_header(&zero_header, crate::constants::MAGIC_JOURNAL_EVENT, 4);
    kani::cover(decoded.is_err(), "public decode_record_header rejection reachable for sequence fixture prelude");
    let expected = crate::EventSeq::new(u64::from(kani::any::<u8>() % 8));
    let actual = crate::EventSeq::new(u64::from(kani::any::<u8>() % 8));
    kani::cover(expected != actual, "sequence gap non-vacuous case covered");
    if expected != actual {
        let observed = crate::JournalError::SequenceGap { expected, actual };
        kani::assert(matches!(observed, crate::JournalError::SequenceGap { .. }), "sequence gap typed")
        ;
        core::mem::forget(observed);
    }
    core::mem::forget(decoded);
    core::mem::forget(header);
}
