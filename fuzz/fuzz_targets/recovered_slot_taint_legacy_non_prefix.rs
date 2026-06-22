//! Fuzz target: vb-7ol6y / POB-vb-7ol6y-010 / ps-002 non-prefix returns Clean
//!
//! Bead: vb-7ol6y (P0)
//! State: 5 (proof-writer)
//! Verifier: cargo-fuzz
//! Command:
//!   cargo fuzz run recovered_slot_taint_legacy_non_prefix --sanitizer=address \
//!     -- -max_total_time=120 -rss_limit_mb=2048
//!
//! PRODUCTION BINDING:
//!   crates/vb_storage/src/recovery/replay/summary/slots/taint.rs:90-93
//!     legacy_or_corrupt_taint non-prefix arm (UNCONDITIONAL Clean)
//!   crates/vb_storage/src/slot_extra.rs:88
//!     decoder LegacyFrameExtra arm
//!
//! REDO (State 5): real assertions on the decoder invariant. The
//! production non-prefix arm at taint.rs:90-93 is UNCONDITIONAL — it
//! does NOT consult the decoder result. The fuzz target verifies the
//! decoder invariant: non-prefix bytes MUST decode as LegacyFrameExtra.
//!
//! Domain claim: arbitrary non-prefix byte vectors route the production
//! `legacy_or_corrupt_taint` non-prefix arm to
//! `Ok(RecoveredSlotTaint { taint: Taint::Clean, unsupported: false })`.
//! The decoder returns `Ok(LegacyFrameExtra(payload))` for these bytes;
//! the production function ignores the decoder result on the non-prefix
//! branch.

#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_storage::{DecodedSlotWrittenExtra, SLOT_WRITTEN_EXTRA_PREFIX, decode_slot_written_extra};

fuzz_target!(|data: &[u8]| {
    let mut bytes = data.to_vec();
    if bytes.starts_with(SLOT_WRITTEN_EXTRA_PREFIX) && !bytes.is_empty() {
        bytes[0] = 0x00; // Force first byte != 'V' (0x56).
    }

    let decode_result = decode_slot_written_extra(&bytes);

    // Real assertion: the decoder returns Ok(LegacyFrameExtra(payload))
    // for ANY non-prefix bytes (slot_extra.rs:88). The payload MUST equal
    // the input bytes (decoder invariant).
    match decode_result {
        Ok(DecodedSlotWrittenExtra::LegacyFrameExtra(payload)) => {
            debug_assert_eq!(
                payload,
                bytes.as_slice(),
                "LegacyFrameExtra MUST preserve input bytes (decoder invariant at slot_extra.rs:88)"
            );
        }
        Ok(DecodedSlotWrittenExtra::Envelope(_)) => {
            // UNREACHABLE: bytes do NOT start with prefix, so
            // decode_slot_written_extra MUST return LegacyFrameExtra,
            // not Envelope. Reaching this branch means the decoder
            // invariant is violated.
            panic!(
                "decoder invariant violated: non-prefix bytes MUST decode as LegacyFrameExtra, got Envelope for input {:?}",
                bytes
            );
        }
        Err(_) => {
            // UNREACHABLE: bytes do NOT start with prefix, so
            // decode_slot_written_extra MUST return Ok(LegacyFrameExtra).
            // The Err arm is unreachable for non-prefix bytes.
            panic!(
                "decoder invariant violated: non-prefix bytes MUST decode as LegacyFrameExtra, got Err for input {:?}",
                decode_result
            );
        }
    }
});
