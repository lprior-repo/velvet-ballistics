#![forbid(unsafe_code)]
//! PO-vb-7m21-002
#[kani::proof]
fn vb_7m21_001_harness() {
    let max: u32 = kani::any();
    let delta: u32 = kani::any();
    kani::assume(delta > 0);
    kani::assume(delta <= u32::MAX - max);
    let len = max + delta;
    kani::assume(len <= 64);
    let payload = [0_u8; 64];
    let result = crate::codec::encode_record_header(
        crate::constants::MAGIC_JOURNAL_EVENT,
        crate::records::RecordKind::RunAccepted,
        0,
        &payload[..len as usize],
        max,
    );
    match &result {
        Err(crate::error::JournalError::PayloadTooLarge { len: l, max: m }) => {
            kani::cover(l > m, "real encode_record_header reached PayloadTooLarge");
            kani::assert(l > m, "oversized typed");
        }
        _ => kani::assert(
            false,
            "oversized public header encode rejected with typed error",
        ),
    }
    core::mem::forget(result);
}
