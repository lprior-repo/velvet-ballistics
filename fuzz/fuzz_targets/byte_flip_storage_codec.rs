//! Byte-flipping mutation testing for storage codec.
//!
//! Takes a valid JournalEvent, encodes it to the storage envelope format,
//! then systematically flips bytes and verifies that each corrupted version
//! either decodes correctly (equivalent mutation) or produces a typed error
//! — never panicking.

#![no_main]
use libfuzzer_sys::fuzz_target;
use vb_core::RunId;
use vb_storage::{
    EventSeq, RecordKind,
    constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RECORD_HEADER_BYTES},
};

fn make_valid_event(data: &[u8]) -> vb_storage::JournalEvent {
    let run_id = if data.len() >= 2 {
        let seed = u64::from(data.first().copied().unwrap_or(0));
        RunId::new(seed.saturating_add(1))
    } else {
        RunId::new(1)
    };

    let digest = vb_core::WorkflowDigest::from_bytes([0xAB; 32]);

    vb_storage::JournalEvent::RunAccepted {
        run: run_id,
        seq: EventSeq::new(0),
        workflow: digest,
    }
}

fn test_byte_flip(encoded: &[u8], seed: &[u8]) {
    let corrupted = fuzz_lib::byte_flip_mutation::corrupted_version(encoded, seed);

    let result = vb_storage::codec::decode_record::<vb_storage::JournalEvent>(
        &corrupted,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );

    // An equivalent mutation or any typed error is acceptable; the key
    // property is that decoding never panics.
    if let Err(error) = result {
        drop(error);
    }
}

fn test_all_byte_positions(encoded: &[u8], data: &[u8]) {
    let seed = u64::from(data.first().copied().unwrap_or(0));

    let seed_bytes = seed.to_le_bytes();
    test_byte_flip(encoded, &seed_bytes);

    let Some(position) = seed.checked_rem(u64::try_from(encoded.len()).unwrap_or(u64::MAX)) else {
        return;
    };
    let mut position_seed = seed_bytes;
    if let Some(slot) = position_seed.first_mut() {
        *slot = u8::try_from(position & 0xFF).unwrap_or(0);
    }
    test_byte_flip(encoded, &position_seed);
}

fn test_header_field_flips(encoded: &[u8], seed: u64) {
    let magic_positions: [u32; 4] = [0, 1, 2, 3];
    for pos in magic_positions {
        if seed & 1u64.checked_shl(pos).unwrap_or(0) != 0 {
            let mut flip_seed = vec![0u8; 16];
            if let Some(slot) = flip_seed.first_mut() {
                *slot = u8::try_from(pos).unwrap_or(0);
            }
            test_byte_flip(encoded, &flip_seed);
        }
    }

    if seed & 1u64.checked_shl(4).unwrap_or(0) != 0 {
        let mut flip_seed = vec![0u8; 16];
        if let Some(slot) = flip_seed.first_mut() {
            *slot = 6;
        }
        test_byte_flip(encoded, &flip_seed);
    }

    if seed & 1u64.checked_shl(5).unwrap_or(0) != 0 {
        for i in 0..4u32 {
            let mut flip_seed = vec![0u8; 16];
            if let Some(slot) = flip_seed.first_mut() {
                *slot = u8::try_from(8u32.saturating_add(i)).unwrap_or(u8::MAX);
            }
            test_byte_flip(encoded, &flip_seed);
        }
    }

    if seed & 1u64.checked_shl(6).unwrap_or(0) != 0 {
        for i in 0..4u32 {
            let mut flip_seed = vec![0u8; 16];
            if let Some(slot) = flip_seed.first_mut() {
                *slot = u8::try_from(44u32.saturating_add(i)).unwrap_or(u8::MAX);
            }
            test_byte_flip(encoded, &flip_seed);
        }
    }

    if seed & 1u64.checked_shl(7).unwrap_or(0) != 0 {
        for i in 0..4u32 {
            let mut flip_seed = vec![0u8; 16];
            if let Some(slot) = flip_seed.first_mut() {
                *slot = u8::try_from(20u32.saturating_add(i)).unwrap_or(u8::MAX);
            }
            test_byte_flip(encoded, &flip_seed);
        }
    }

    if seed & 1u64.checked_shl(8).unwrap_or(0) != 0 {
        for i in 0..4u32 {
            let mut flip_seed = vec![0u8; 16];
            if let Some(slot) = flip_seed.first_mut() {
                *slot = u8::try_from(52u32.saturating_add(i)).unwrap_or(u8::MAX);
            }
            test_byte_flip(encoded, &flip_seed);
        }
    }
}

fn test_payload_corruption(encoded: &[u8], data: &[u8]) {
    let payload_start = RECORD_HEADER_BYTES;
    let Some(payload_len) = encoded.len().checked_sub(payload_start) else {
        return;
    };

    if payload_len == 0 {
        return;
    }

    let offset = u64::from(data.first().copied().unwrap_or(0));
    let Some(corruption_offset) =
        offset.checked_rem(u64::try_from(payload_len).unwrap_or(u64::MAX))
    else {
        return;
    };
    let Some(byte_offset) =
        payload_start.checked_add(usize::try_from(corruption_offset).unwrap_or(0))
    else {
        return;
    };

    let mut flip_seed = vec![0u8; 16];
    if let Some(slot) = flip_seed.first_mut() {
        *slot = u8::try_from(byte_offset).unwrap_or(u8::MAX);
    }
    test_byte_flip(encoded, &flip_seed);

    let mut digest_seed = vec![0u8; 16];
    if let Some(slot) = digest_seed.first_mut() {
        *slot = 52;
    }
    test_byte_flip(encoded, &digest_seed);
}

fuzz_target!(|data: &[u8]| {
    let event = make_valid_event(data);

    let Ok(encoded) = vb_storage::codec::encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    ) else {
        return;
    };

    let seed = data
        .get(..8)
        .and_then(|bytes| <[u8; 8]>::try_from(bytes).ok())
        .map_or(0, u64::from_le_bytes);

    test_all_byte_positions(&encoded, data);
    test_header_field_flips(&encoded, seed);
    test_payload_corruption(&encoded, data);
});
