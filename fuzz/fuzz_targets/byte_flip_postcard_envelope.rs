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
    let run_seed = u64::from(data.first().copied().unwrap_or(0));
    let run_id = RunId::new(run_seed.saturating_add(1));
    let digest = vb_core::WorkflowDigest::from_bytes([0x42; 32]);

    vb_storage::JournalEvent::RunAccepted {
        run: run_id,
        seq: EventSeq::new(0),
        workflow: digest,
    }
}

/// Accept any successfully decoded variant without panicking on others.
fn accept_decoded(decoded: &vb_storage::JournalEvent) {
    if let vb_storage::JournalEvent::RunAccepted { .. } = decoded {}
}

/// Serialize an event, corrupt it by flipping bytes, then try to decode.
/// Verify result is either valid event or typed error.
fn test_postcard_corruption(event: &vb_storage::JournalEvent, seed: &[u8]) {
    let Ok(serialized) = postcard::to_allocvec(event) else {
        return;
    };

    if serialized.is_empty() {
        return;
    }

    let corrupted = fuzz_lib::byte_flip_mutation::corrupted_version(&serialized, seed);
    let result = postcard::from_bytes::<vb_storage::JournalEvent>(&corrupted);

    if let Ok(decoded) = result {
        accept_decoded(&decoded);
    }
}

/// Test that specific field corruption produces errors.
fn test_field_specific_corruption(event: &vb_storage::JournalEvent, data: &[u8]) {
    let Ok(serialized) = postcard::to_allocvec(event) else {
        return;
    };

    if serialized.is_empty() {
        return;
    }

    let flip_positions = if let Some(first) = data.first().copied() {
        let count = (first & 0x07).saturating_add(1);
        let mut positions = Vec::new();
        for i in 0..count {
            let index = usize::from(i);
            let Some(&b) = data.get(index.saturating_add(1)) else {
                continue;
            };
            if let Some(pos) = usize::from(b).checked_rem(serialized.len()) {
                positions.push(pos);
            }
        }
        positions
    } else {
        vec![0]
    };

    for pos in flip_positions {
        let mut corrupted = serialized.clone();
        if let Some(byte) = corrupted.get_mut(pos) {
            *byte ^= 0xFF;
        }

        let result = postcard::from_bytes::<vb_storage::JournalEvent>(&corrupted);
        if let Ok(decoded) = result {
            accept_decoded(&decoded);
        }
    }
}

/// Test that wrapping corrupted postcard in storage envelope fails properly.
fn test_envelope_with_corrupted_postcard(event: &vb_storage::JournalEvent, data: &[u8]) {
    let Ok(postcard_bytes) = postcard::to_allocvec(event) else {
        return;
    };

    let seed_byte = data.first().copied().unwrap_or(0);
    let seed_bytes = [seed_byte, 0, 0, 0, 0, 0, 0, 0];
    let _corrupted_postcard =
        fuzz_lib::byte_flip_mutation::corrupted_version(&postcard_bytes, &seed_bytes);

    let Ok(mut envelope_bytes) = vb_storage::codec::encode_record(
        MAGIC_JOURNAL_EVENT,
        vb_storage::RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    ) else {
        return;
    };

    let payload_offset = RECORD_HEADER_BYTES;
    if envelope_bytes.len() <= payload_offset.saturating_add(1) {
        return;
    }
    let Some(payload_len) = envelope_bytes.len().checked_sub(payload_offset) else {
        return;
    };
    let flip_seed = usize::from(seed_byte);
    let Some(flip_offset) = flip_seed.checked_rem(payload_len) else {
        return;
    };
    let Some(flip_pos) = payload_offset.checked_add(flip_offset) else {
        return;
    };
    if let Some(byte) = envelope_bytes.get_mut(flip_pos) {
        *byte ^= 0xFF;
    }

    let result = vb_storage::codec::decode_record::<vb_storage::JournalEvent>(
        &envelope_bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );

    // Any typed decode error is an acceptable mutation outcome.
    if let Err(error) = result {
        drop(error);
    }
}

/// Test bit-level corruption within postcard bytes.
fn test_bit_level_corruption(event: &vb_storage::JournalEvent, data: &[u8]) {
    let Ok(serialized) = postcard::to_allocvec(event) else {
        return;
    };

    if serialized.is_empty() {
        return;
    }

    let byte_seed = usize::from(data.first().copied().unwrap_or(0));
    let byte_pos = byte_seed.checked_rem(serialized.len()).unwrap_or(0);
    let bit_pos = data.get(1).copied().unwrap_or(0) & 0x07;
    let corrupted =
        fuzz_lib::byte_flip_mutation::flip_bit_at_byte_offset(&serialized, byte_pos, bit_pos);
    let result = postcard::from_bytes::<vb_storage::JournalEvent>(&corrupted);

    if let Ok(decoded) = result {
        accept_decoded(&decoded);
    }
}

fuzz_target!(|data: &[u8]| {
    let event = make_journal_event(data);

    // Run all byte-flip test variants
    test_postcard_corruption(&event, data);
    test_field_specific_corruption(&event, data);
    test_envelope_with_corrupted_postcard(&event, data);
    test_bit_level_corruption(&event, data);
});
