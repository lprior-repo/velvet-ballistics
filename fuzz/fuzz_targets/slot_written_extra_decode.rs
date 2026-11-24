//! Fuzz target for SlotWritten extra envelope decoding.
//!
//! Tests `decode_slot_written_extra` with:
//! - Valid v1 envelopes (prefix + postcard payload)
//! - Corrupted prefix bytes
//! - Truncated envelopes
//! - Legacy bytes (no prefix)
//! - Various corrupt payloads after valid prefix

#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_storage::slot_extra::{
    DecodedSlotWrittenExtra, SLOT_WRITTEN_EXTRA_PREFIX, SlotWrittenExtraError,
    decode_slot_written_extra, encode_slot_written_extra,
};

fuzz_target!(|data: &[u8]| {
    let (mode, payload) = match data.split_first() {
        Some((&mode, payload)) => (mode % 6, Some(payload)),
        None => (0, None),
    };
    let input = match mode {
        0 => {
            // Valid envelope: use the remaining bytes as the frame-extra payload.
            match encode_slot_written_extra(vb_core::Taint::Clean, payload.map(<[u8]>::to_vec)) {
                Ok(envelope) => envelope,
                Err(_) => return,
            }
        }
        1 => {
            // Corrupt prefix: flip its first byte.
            let mut envelope = SLOT_WRITTEN_EXTRA_PREFIX.to_vec();
            if let Some(first) = envelope.first_mut() {
                *first ^= 0x01;
            }
            envelope.extend(data.iter().take(32).copied());
            envelope
        }
        2 => {
            // Truncated envelope: just the prefix with no payload
            SLOT_WRITTEN_EXTRA_PREFIX.to_vec()
        }
        3 => {
            // Legacy bytes: no prefix
            payload.map_or_else(Vec::new, <[u8]>::to_vec)
        }
        4 => {
            // Valid prefix + truncated postcard payload
            let mut envelope = SLOT_WRITTEN_EXTRA_PREFIX.to_vec();
            envelope.extend(data.iter().take(16).copied());
            envelope
        }
        _ => {
            // Valid envelope with a corrupted payload byte.
            match encode_slot_written_extra(vb_core::Taint::Clean, payload.map(<[u8]>::to_vec)) {
                Ok(mut envelope) => {
                    if let Some(bytes) = envelope.get_mut(SLOT_WRITTEN_EXTRA_PREFIX.len()..) {
                        let middle = bytes.len() / 2;
                        if let Some(byte) = bytes.get_mut(middle) {
                            *byte ^= 0xFF;
                        }
                    }
                    envelope
                }
                Err(_) => return,
            }
        }
    };

    let result = decode_slot_written_extra(&input);

    // Verify the result is consistent
    match result {
        Ok(DecodedSlotWrittenExtra::Envelope(env)) => {
            // Envelope decoded - verify round-trip
            let encoded = encode_slot_written_extra(env.taint, env.frame_extra);
            assert!(
                encoded.is_ok(),
                "Re-encode of decoded envelope must succeed"
            );
        }
        Ok(DecodedSlotWrittenExtra::LegacyFrameExtra(_)) => {
            // Legacy - just accept it
        }
        Err(err) => assert_eq!(err, SlotWrittenExtraError::DecodeFailed, "Unexpected error"),
    }
});
