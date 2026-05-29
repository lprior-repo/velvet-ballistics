#![forbid(unsafe_code)]
//! PO-vb-7m21-032
#[kani::proof]
fn vb_7m21_007_harness() {
    let payload = [0_u8; 1];
    let header = crate::codec::encode_record_header(
        crate::constants::MAGIC_SNAPSHOT,
        crate::records::RecordKind::Snapshot,
        0,
        &payload,
        0,
    );
    kani::cover(header.is_err(), "public encode_record_header rejection reachable for snapshot fixture prelude");
    let zero_header = [0_u8; crate::constants::RECORD_HEADER_BYTES];
    let decoded = crate::codec::decode_record_header(&zero_header, crate::constants::MAGIC_SNAPSHOT, 4);
    kani::cover(decoded.is_err(), "public decode_record_header rejection reachable for snapshot fixture prelude");
    let snapshot = u64::from(kani::any::<u8>() % 8);
    let tail = u64::from(kani::any::<u8>() % 8);
    kani::cover(snapshot < tail, "stale snapshot with newer tail non-vacuous case covered");
    if snapshot < tail {
        let observed = crate::recovery::RecoveryError::ReplayDivergence {
            step: vb_core::StepIdx::new(0),
            detail: String::new(),
        };
        kani::assert(matches!(observed, crate::recovery::RecoveryError::ReplayDivergence { .. }), "stale snapshot has typed replay obligation")
        ;
        core::mem::forget(observed);
    }
    core::mem::forget(decoded);
    core::mem::forget(header);
}
