//! Byte-flipping mutation testing for postcard binary envelopes.
//!
//! Serializes values via postcard, then flips bytes in the serialized
//! representation to verify that decoding produces either valid values
//! or typed postcard errors — never panics.

#![no_main]
use libfuzzer_sys::fuzz_target;
use vb_core::RunId;
use vb_storage::{
    EventSeq,
    constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RECORD_HEADER_BYTES},
};

fn make_journal_event(data: &[u8]) -> vb_storage::JournalEvent {
    let run_id = if !data.is_empty() {
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

    let digest = vb_core::WorkflowDigest::from_bytes([0x42; 32]);

    vb_storage::JournalEvent::RunAccepted {
        run: run_id,
        seq: EventSeq::new(0),
        workflow: digest,
    }
}

fn test_postcard_corruption(event: &vb_storage::JournalEvent, seed: &[u8]) {
    let serialized = match postcard::to_allocvec(event) {
        Ok(s) => s,
        Err(_) => return,
    };

    if serialized.is_empty() {
        return;
    }

    let corrupted = fuzz_lib::byte_flip_mutation::corrupted_version(&serialized, seed);

    let result = postcard::from_bytes::<vb_storage::JournalEvent>(&corrupted);

    match result {
        Ok(_) => {}
        Err(e) => {
            let _ = e;
        }
    }
}

fn test_field_specific_corruption(event: &vb_storage::JournalEvent, data: &[u8]) {
    let serialized = match postcard::to_allocvec(event) {
        Ok(s) => s,
        Err(_) => return,
    };

    if serialized.is_empty() {
        return;
    }

    let flip_positions = match data.first() {
        Some(first) => {
            let count = (first & 0x07).checked_add(1).unwrap_or(1);
            let mut positions = Vec::new();
            for byte in data.iter().skip(1).take(usize::from(count)) {
                if let Some(pos) = usize::from(*byte).checked_rem(serialized.len()) {
                    positions.push(pos);
                }
            }
            positions
        }
        None => vec![0],
    };

    for pos in flip_positions {
        let mut corrupted = serialized.clone();
        if let Some(byte) = corrupted.get_mut(pos) {
            *byte ^= 0xFF;
        }

        let result = postcard::from_bytes::<vb_storage::JournalEvent>(&corrupted);

        if result.is_err() {}
    }
}

fn test_envelope_with_corrupted_postcard(event: &vb_storage::JournalEvent, data: &[u8]) {
    let postcard_bytes = match postcard::to_allocvec(event) {
        Ok(b) => b,
        Err(_) => return,
    };

    let first_byte = match data.first() {
        Some(byte) => *byte,
        None => 0,
    };
    let mut seed_bytes = vec![first_byte];
    seed_bytes.resize(8, 0);

    let _corrupted_postcard =
        fuzz_lib::byte_flip_mutation::corrupted_version(&postcard_bytes, &seed_bytes);

    let envelope = vb_storage::codec::encode_record(
        MAGIC_JOURNAL_EVENT,
        vb_storage::RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );

    if let Ok(mut envelope_bytes) = envelope {
        let payload_offset = RECORD_HEADER_BYTES;
        if envelope_bytes.len() > payload_offset + 1 {
            let flip_seed = match data.first() {
                Some(byte) => usize::from(*byte),
                None => 0,
            };
            let flip_pos = match envelope_bytes
                .len()
                .checked_sub(payload_offset)
                .and_then(|span| flip_seed.checked_rem(span))
                .and_then(|offset| payload_offset.checked_add(offset))
            {
                Some(pos) => pos,
                None => return,
            };
            if let Some(byte) = envelope_bytes.get_mut(flip_pos) {
                *byte ^= 0xFF;
            }

            let result = vb_storage::codec::decode_record::<vb_storage::JournalEvent>(
                &envelope_bytes,
                MAGIC_JOURNAL_EVENT,
                MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
            );

            match result {
                Ok(_) => {}
                Err(vb_storage::JournalError::PostcardDecodeFailed(_)) => {}
                Err(vb_storage::JournalError::PayloadDigestMismatch) => {}
                Err(vb_storage::JournalError::HeaderChecksumMismatch) => {}
                Err(vb_storage::JournalError::RecordKindPayloadMismatch { .. }) => {}
                Err(vb_storage::JournalError::UnsupportedSchemaVersion { .. }) => {}
                Err(vb_storage::JournalError::UnknownRecordKind { .. }) => {}
                Err(vb_storage::JournalError::HeaderLengthMismatch { .. }) => {}
                Err(vb_storage::JournalError::BadMagic { .. }) => {}
                Err(vb_storage::JournalError::InvalidEvent) => {}
                Err(_) => {}
            }
        }
    }
}

fn test_bit_level_corruption(event: &vb_storage::JournalEvent, data: &[u8]) {
    let serialized = match postcard::to_allocvec(event) {
        Ok(s) => s,
        Err(_) => return,
    };

    if serialized.is_empty() {
        return;
    }

    let byte_pos = match data.first() {
        Some(byte) => usize::from(*byte)
            .checked_rem(serialized.len())
            .unwrap_or(0),
        None => 0,
    };

    let bit_pos = match data.get(1) {
        Some(byte) => byte & 0x07,
        None => 0,
    };

    let corrupted =
        fuzz_lib::byte_flip_mutation::flip_bit_at_byte_offset(&serialized, byte_pos, bit_pos);

    let result = postcard::from_bytes::<vb_storage::JournalEvent>(&corrupted);

    if result.is_err() {}
}

fuzz_target!(|data: &[u8]| {
    let event = make_journal_event(data);

    test_postcard_corruption(&event, data);
    test_field_specific_corruption(&event, data);
    test_envelope_with_corrupted_postcard(&event, data);
    test_bit_level_corruption(&event, data);
});
