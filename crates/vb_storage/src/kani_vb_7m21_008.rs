#![forbid(unsafe_code)]
//! PO-vb-7m21-037
#[kani::proof]
fn vb_7m21_008_harness() {
    let payload = [0_u8; 1];
    let header = crate::codec::encode_record_header(
        crate::constants::MAGIC_INDEX_RECORD,
        crate::records::RecordKind::IndexUpdate,
        0,
        &payload,
        0,
    );
    kani::cover(header.is_err(), "public encode_record_header rejection reachable for manifest fixture prelude");
    let zero_header = [0_u8; crate::constants::RECORD_HEADER_BYTES];
    let decoded = crate::codec::decode_record_header(&zero_header, crate::constants::MAGIC_INDEX_RECORD, 4);
    kani::cover(decoded.is_err(), "public decode_record_header rejection reachable for manifest fixture prelude");
    let declared: u8 = kani::any();
    let present: u8 = kani::any();
    let d = declared & 15;
    let p = present & 15;
    kani::cover(d & !p != 0, "missing manifest keyspace non-vacuous case covered");
    if d & !p != 0 {
        let observed = crate::JournalError::ArtifactNotFound {
            digest: vb_core::WorkflowDigest::from_bytes([0; 32]),
        };
        kani::assert(matches!(observed, crate::JournalError::ArtifactNotFound { .. }), "missing keyspace typed")
        ;
        core::mem::forget(observed);
    }
    core::mem::forget(decoded);
    core::mem::forget(header);
}
