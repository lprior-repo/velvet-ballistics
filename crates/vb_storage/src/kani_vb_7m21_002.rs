#![forbid(unsafe_code)]
//! PO-vb-7m21-007
#[kani::proof]
fn vb_7m21_002_harness() {
    let delta: u16 = kani::any();
    kani::assume(delta > 0);
    kani::assume(crate::constants::CURRENT_SCHEMA_VERSION <= u16::MAX - delta);
    let version = crate::constants::CURRENT_SCHEMA_VERSION + delta;
    let mut header = [0_u8; crate::constants::RECORD_HEADER_BYTES];
    header[0..4].copy_from_slice(&crate::constants::MAGIC_JOURNAL_EVENT.to_le_bytes());
    header[4..6].copy_from_slice(&version.to_le_bytes());
    let result = crate::codec::decode_record_header(
        &header,
        crate::constants::MAGIC_JOURNAL_EVENT,
        crate::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    match &result {
        Err(crate::error::JournalError::UnsupportedSchemaVersion { version: v }) => {
            kani::cover(
                *v > crate::constants::CURRENT_SCHEMA_VERSION,
                "future schema path reached",
            );
            kani::assert(
                *v > crate::constants::CURRENT_SCHEMA_VERSION,
                "future schema typed",
            );
        }
        _ => kani::assert(
            false,
            "future schema public header decode rejected with typed error",
        ),
    }
    core::mem::forget(result);
}
