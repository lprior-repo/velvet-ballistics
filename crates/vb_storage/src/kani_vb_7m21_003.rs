#![forbid(unsafe_code)]
//! PO-vb-7m21-012
#[kani::proof]
fn vb_7m21_003_harness() {
    let len: usize = usize::from(kani::any::<u8>() % (crate::constants::RECORD_HEADER_BYTES as u8));
    let bytes = [0_u8; crate::constants::RECORD_HEADER_BYTES];
    let result = crate::codec::decode_record_header(
        &bytes[..len],
        crate::constants::MAGIC_JOURNAL_EVENT,
        crate::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    kani::cover(
        len == crate::constants::RECORD_HEADER_BYTES - 1,
        "maximal truncated header checked",
    );
    kani::assert(
        matches!(&result, Err(crate::error::JournalError::UnexpectedEof)),
        "short public header decode returns UnexpectedEof",
    );
    core::mem::forget(result);
}
