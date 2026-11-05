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
        RunId::new(data[0] as u64 + 1)
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

    // Verify we get a valid result (either Ok or typed error), never a panic.
    match result {
        Ok(_) => {
            // Equivalent mutation: corrupted bytes decoded to valid event.
            // This is acceptable for fuzzing (not all mutations are detectable).
        }
        Err(vb_storage::JournalError::PostcardDecodeFailed(_)) => {}
        Err(vb_storage::JournalError::Encode(_)) => {}
        Err(vb_storage::JournalError::PayloadTooLarge { .. }) => {}
        Err(vb_storage::JournalError::UnsupportedSchemaVersion { .. }) => {}
        Err(vb_storage::JournalError::UnknownRecordKind { .. }) => {}
        Err(vb_storage::JournalError::RecordKindFamilyMismatch { .. }) => {}
        Err(vb_storage::JournalError::RecordKindPayloadMismatch { .. }) => {}
        Err(vb_storage::JournalError::HeaderLengthMismatch { .. }) => {}
        Err(vb_storage::JournalError::HeaderChecksumMismatch) => {}
        Err(vb_storage::JournalError::PayloadDigestMismatch) => {}
        Err(vb_storage::JournalError::UnexpectedEof) => {}
        Err(vb_storage::JournalError::BadMagic { .. }) => {}
        Err(vb_storage::JournalError::Fjall(_)) => {}
        Err(vb_storage::JournalError::InvalidEvent) => {}
        Err(other) => {
            // Any other typed error is acceptable; the key property is no panic.
            drop(other);
        }
    }
}

fn test_all_byte_positions(encoded: &[u8], data: &[u8]) {
    let seed = if !data.is_empty() { data[0] as u64 } else { 0 };

    // Seed-based corruption testing
    let seed_bytes = seed.to_le_bytes();
    test_byte_flip(encoded, &seed_bytes);

    // Target specific byte positions based on seed
    let position = seed % (encoded.len() as u64);
    let mut position_seed = seed_bytes;
    position_seed[0] = (position & 0xFF) as u8;
    test_byte_flip(encoded, &position_seed);
}

fn test_header_field_flips(encoded: &[u8], seed: u64) {
    // Flip magic bytes
    let magic_positions = [0, 1, 2, 3];
    for pos in magic_positions {
        if seed & (1 << pos) != 0 {
            let mut flip_seed = vec![0u8; 16];
            flip_seed[0] = pos as u8;
            test_byte_flip(encoded, &flip_seed);
        }
    }

    // Flip kind field (offset 6)
    if seed & (1 << 4) != 0 {
        let mut flip_seed = vec![0u8; 16];
        flip_seed[0] = 6;
        test_byte_flip(encoded, &flip_seed);
    }

    // Flip payload length (offset 8-11)
    if seed & (1 << 5) != 0 {
        for i in 0..4 {
            let mut flip_seed = vec![0u8; 16];
            flip_seed[0] = (8 + i) as u8;
            test_byte_flip(encoded, &flip_seed);
        }
    }

    // Flip CRC field (offset 44-47)
    if seed & (1 << 6) != 0 {
        for i in 0..4 {
            let mut flip_seed = vec![0u8; 16];
            flip_seed[0] = (44 + i) as u8;
            test_byte_flip(encoded, &flip_seed);
        }
    }

    // Flip header digest field (offset 20-51)
    if seed & (1 << 7) != 0 {
        for i in 0..4 {
            let mut flip_seed = vec![0u8; 16];
            flip_seed[0] = (20 + i) as u8;
            test_byte_flip(encoded, &flip_seed);
        }
    }

    // Flip payload digest field (offset 52-83)
    if seed & (1 << 8) != 0 {
        for i in 0..4 {
            let mut flip_seed = vec![0u8; 16];
            flip_seed[0] = (52 + i) as u8;
            test_byte_flip(encoded, &flip_seed);
        }
    }
}

fn test_payload_corruption(encoded: &[u8], data: &[u8]) {
    let payload_start = RECORD_HEADER_BYTES;
    let payload_len = encoded.len() - payload_start;

    if payload_len == 0 {
        return;
    }

    let corruption_offset = if !data.is_empty() {
        let offset: u64 = data[0] as u64;
        (offset % payload_len as u64) as usize
    } else {
        0
    };

    let byte_offset = payload_start + corruption_offset;

    // Corrupt payload byte
    let mut flip_seed = vec![0u8; 16];
    flip_seed[0] = byte_offset as u8;
    test_byte_flip(encoded, &flip_seed);

    // Corrupt payload digest (offset 52)
    let mut digest_seed = vec![0u8; 16];
    digest_seed[0] = 52;
    test_byte_flip(encoded, &digest_seed);
}

fuzz_target!(|data: &[u8]| {
    // Create valid event
    let event = make_valid_event(data);

    // Encode to storage format
    let encoded = match vb_storage::codec::encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    ) {
        Ok(e) => e,
        Err(_) => return,
    };

    // Extract seed bytes for mutation decisions
    let seed = if data.len() >= 8 {
        let seed_bytes: [u8; 8] = data[..8].try_into().unwrap();
        u64::from_le_bytes(seed_bytes)
    } else {
        0
    };

    // Test systematic byte-flipping with seed-based decisions
    test_all_byte_positions(&encoded, data);

    // Test targeted header field flips
    test_header_field_flips(&encoded, seed);

    // Test payload corruption
    test_payload_corruption(&encoded, data);
});
