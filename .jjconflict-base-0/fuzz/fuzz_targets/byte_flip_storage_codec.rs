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
        match data
            .first()
            .and_then(|byte| u64::from(*byte).checked_add(1))
        {
            Some(value) => RunId::new(value),
            None => RunId::new(1),
        }
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

    match result {
        Ok(_) => {}
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
        Err(_) => {}
    }
}

fn test_all_byte_positions(encoded: &[u8], data: &[u8]) {
    let seed = match data.first() {
        Some(byte) => u64::from(*byte),
        None => 0,
    };

    let seed_bytes = seed.to_le_bytes();
    test_byte_flip(encoded, &seed_bytes);

    let position = u64::try_from(encoded.len())
        .ok()
        .and_then(|len| seed.checked_rem(len))
        .unwrap_or(0);
    let low_byte = match position.to_le_bytes().first() {
        Some(byte) => *byte,
        None => 0,
    };
    let mut position_seed = seed_bytes;
    if let Some(slot) = position_seed.first_mut() {
        *slot = low_byte;
    }
    test_byte_flip(encoded, &position_seed);
}

fn test_header_field_flips(encoded: &[u8], seed: u64) {
    let magic_positions: [u8; 4] = [0, 1, 2, 3];
    for pos in magic_positions {
        if seed & (1 << pos) != 0 {
            let mut flip_seed = vec![pos];
            flip_seed.resize(16, 0);
            test_byte_flip(encoded, &flip_seed);
        }
    }

    if seed & (1 << 4) != 0 {
        let mut flip_seed = vec![6];
        flip_seed.resize(16, 0);
        test_byte_flip(encoded, &flip_seed);
    }

    if seed & (1 << 5) != 0 {
        for offset in 8u8..12u8 {
            let mut flip_seed = vec![offset];
            flip_seed.resize(16, 0);
            test_byte_flip(encoded, &flip_seed);
        }
    }

    if seed & (1 << 6) != 0 {
        for offset in 44u8..48u8 {
            let mut flip_seed = vec![offset];
            flip_seed.resize(16, 0);
            test_byte_flip(encoded, &flip_seed);
        }
    }

    if seed & (1 << 7) != 0 {
        for offset in 20u8..24u8 {
            let mut flip_seed = vec![offset];
            flip_seed.resize(16, 0);
            test_byte_flip(encoded, &flip_seed);
        }
    }

    if seed & (1 << 8) != 0 {
        for offset in 52u8..56u8 {
            let mut flip_seed = vec![offset];
            flip_seed.resize(16, 0);
            test_byte_flip(encoded, &flip_seed);
        }
    }
}

fn test_payload_corruption(encoded: &[u8], data: &[u8]) {
    let payload_start = RECORD_HEADER_BYTES;
    let payload_len = match encoded.len().checked_sub(payload_start) {
        Some(len) => len,
        None => return,
    };

    if payload_len == 0 {
        return;
    }

    let corruption_offset = data
        .first()
        .and_then(|byte| usize::from(*byte).checked_rem(payload_len))
        .unwrap_or(0);

    let byte_offset = match payload_start.checked_add(corruption_offset) {
        Some(offset) => offset,
        None => return,
    };

    let seed_byte = u8::try_from(byte_offset % 256).unwrap_or(0);
    let mut flip_seed = vec![seed_byte];
    flip_seed.resize(16, 0);
    test_byte_flip(encoded, &flip_seed);

    let mut digest_seed = vec![52];
    digest_seed.resize(16, 0);
    test_byte_flip(encoded, &digest_seed);
}

fuzz_target!(|data: &[u8]| {
    let event = make_valid_event(data);

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

    let seed = if data.len() >= 8 {
        match data
            .get(..8)
            .and_then(|slice| <[u8; 8]>::try_from(slice).ok())
        {
            Some(seed_bytes) => u64::from_le_bytes(seed_bytes),
            None => 0,
        }
    } else {
        0
    };

    test_all_byte_positions(&encoded, data);

    test_header_field_flips(&encoded, seed);

    test_payload_corruption(&encoded, data);
});
