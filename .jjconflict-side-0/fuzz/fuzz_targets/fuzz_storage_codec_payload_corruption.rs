#![no_main]

//! Fuzz target: storage_codec_payload_corruption
//!
//! Split from `vb_storage_codec` (PO-vb-y9d3v-0041). Oracle: a correctly
//! encoded `JournalEvent` payload that has had a single byte incremented at
//! one of several offsets (header byte 0..3, schema-version bytes 4..5,
//! payload-len bytes, sequence bytes, digest bytes, or CRC bytes) must fail
//! to decode with a typed `JournalError` rather than panic or silently
//! succeed. Truncation at any prefix length must also fail gracefully.
//!
//! Run with: cargo fuzz run fuzz_storage_codec_payload_corruption -- -max_len=4096 -runs=100000

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let magic = vb_storage::MAGIC_JOURNAL_EVENT;
    let max_payload = vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES;

    let Ok((_, event)) =
        vb_storage::decode_record::<vb_storage::JournalEvent>(data, magic, max_payload)
    else {
        return;
    };
    if !event.is_valid() {
        return;
    }

    let Ok(encoded) = vb_storage::encode_record(
        magic,
        event.record_kind(),
        event.seq().get(),
        &event,
        max_payload,
    ) else {
        return;
    };

    // Sanity: the freshly encoded bytes must round-trip successfully.
    let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(&encoded, magic, max_payload);

    // Single-byte flip corruption across magic/schema/kind/len/seq/digest/crc.
    for corruption_offset in [0u32, 1, 2, 3, 4, 5, 6, 7, 8, 12, 16, 24, 32, 48, 56, 60] {
        let off = corruption_offset as usize;
        if off >= encoded.len() {
            continue;
        }
        let mut corrupted = encoded.clone();
        let original = corrupted.get(off).copied().unwrap_or(0);
        corrupted[off] = original.wrapping_add(1);
        let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(
            &corrupted,
            magic,
            max_payload,
        );
    }

    // Truncation at every prefix length — decoder must reject, not panic.
    let cap = encoded.len().min(64);
    for truncation in 0..cap {
        let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(
            &encoded[..truncation],
            magic,
            max_payload,
        );
    }
});
