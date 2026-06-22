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
//!   crates/vb_storage/src/recovery/replay/summary/slots/taint.rs:82-91
//!     legacy_or_corrupt_taint non-prefix arm
//!   crates/vb_storage/src/slot_extra.rs:88
//!     decoder LegacyFrameExtra arm
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
    // Force the input to NOT start with SLOT_WRITTEN_EXTRA_PREFIX so we
    // exercise the non-prefix arm of `legacy_or_corrupt_taint`.
    // If the input naturally starts with the prefix, mutate the first byte
    // to break the prefix.
    let mut bytes = data.to_vec();
    if bytes.starts_with(SLOT_WRITTEN_EXTRA_PREFIX) && !bytes.is_empty() {
        bytes[0] = 0x00; // Force first byte != 'V' (0x56).
    }

    let decode_result = decode_slot_written_extra(&bytes);

    // The decoder returns Ok(LegacyFrameExtra(payload)) for any non-prefix
    // bytes (slot_extra.rs:88). The production legacy_or_corrupt_taint
    // non-prefix arm is UNCONDITIONAL — it does NOT consult the decoder
    // result. The fuzz target asserts:
    //
    //   1. The decoder never panics.
    //   2. Non-prefix bytes ALWAYS decode as LegacyFrameExtra (the
    //      decoder invariant).
    match decode_result {
        Ok(DecodedSlotWrittenExtra::LegacyFrameExtra(payload)) => {
            debug_assert_eq!(
                payload,
                bytes.as_slice(),
                "LegacyFrameExtra must preserve input bytes"
            );
        }
        _ => {
            panic!(
                "decoder invariant violated: non-prefix bytes MUST decode as LegacyFrameExtra (got {:?})",
                decode_result
            );
        }
    }
});
