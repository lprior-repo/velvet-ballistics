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
    SLOT_WRITTEN_EXTRA_PREFIX, decode_slot_written_extra,
    encode_slot_written_extra, DecodedSlotWrittenExtra, SlotWrittenExtraError,
};

fuzz_target!(|data: &[u8]| {
    let mode = data.first().copied().unwrap_or(0) % 6;
    let input = match mode {
        0 => {
            // Valid envelope: try to create a proper envelope from the data
            // Use bytes 1.. as the frame_extra payload
            let frame_extra = data.get(1..).map(|b| b.to_vec());
            match encode_slot_written_extra(vb_core::Taint::Clean, frame_extra) {
                Ok(envelope) => envelope,
                Err(_) => return,
            }
        }
        1 => {
            // Corrupt prefix: flip first byte of prefix
            let mut envelope = vec![0; SLOT_WRITTEN_EXTRA_PREFIX.len() + data.len().min(32)];
            for (i, b) in SLOT_WRITTEN_EXTRA_PREFIX.iter().enumerate() {
                if i == 0 {
                    envelope[i] = *b ^ 0x01;
                } else {
                    envelope[i] = *b;
                }
            }
            let payload_start = SLOT_WRITTEN_EXTRA_PREFIX.len();
            let copy_len = data.len().min(envelope.len() - payload_start);
            envelope[payload_start..payload_start + copy_len].copy_from_slice(&data[..copy_len]);
            envelope
        }
        2 => {
            // Truncated envelope: just the prefix with no payload
            SLOT_WRITTEN_EXTRA_PREFIX.to_vec()
        }
        3 => {
            // Legacy bytes: no prefix
            data[1..].to_vec()
        }
        4 => {
            // Valid prefix + truncated postcard payload
            let mut envelope = SLOT_WRITTEN_EXTRA_PREFIX.to_vec();
            let copy_len = data.len().min(16);
            envelope.extend_from_slice(&data[..copy_len]);
            envelope
        }
        5 => {
            // Valid envelope with corrupt payload
            let frame_extra = data.get(1..).map(|b| b.to_vec());
            match encode_slot_written_extra(vb_core::Taint::Clean, frame_extra) {
                Ok(mut env) => {
                    // Corrupt a payload byte
                    if env.len() > SLOT_WRITTEN_EXTRA_PREFIX.len() {
                        let corrupt_at = SLOT_WRITTEN_EXTRA_PREFIX.len()
                            + (env.len() - SLOT_WRITTEN_EXTRA_PREFIX.len()) / 2;
                        env[corrupt_at] ^= 0xFF;
                    }
                    env
                }
                Err(_) => return,
            }
        }
        _ => unreachable!(),
    };

    let result = decode_slot_written_extra(&input);

    // Verify the result is consistent
    match result {
        Ok(DecodedSlotWrittenExtra::Envelope(env)) => {
            // Envelope decoded - verify round-trip
            let encoded = encode_slot_written_extra(env.taint, env.frame_extra.clone());
            assert!(encoded.is_ok(), "Re-encode of decoded envelope must succeed");
        }
        Ok(DecodedSlotWrittenExtra::LegacyFrameExtra(_)) => {
            // Legacy - just accept it
        }
        Err(SlotWrittenExtraError::DecodeFailed) => {
            // Expected for corrupt payloads
        }
        Err(e) => {
            // Shouldn't happen - envelope is fixed format
            panic!("Unexpected error: {:?}", e);
        }
    }
});
