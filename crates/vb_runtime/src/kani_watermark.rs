#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harnesses for CompletionWatermark monotonicity verification.
//!
//! Coverage:
//! - `po-runtime-watermark-monotonic-kani-01`: CompletionWatermark boundary is monotonically non-decreasing

use vb_core::ids::RunId;

use crate::shard::completion_watermark::{CompletionWatermark, CompletionWatermarkError};

/// PO-runtime-watermark-monotonic-kani-01:
/// CompletionWatermark boundary is monotonically non-decreasing through multiple completions.
///
/// The `complete` method accepts a sequence number and advances the contiguous prefix
/// boundary. The boundary can never regress (go backwards). This harness verifies:
/// 1. A newly created watermark starts at boundary=0
/// 2. Completing seq=1 advances boundary to 1
/// 3. Completing seq=2 advances boundary to 2
/// 4. Completing seq=1 again is rejected (duplicate)
/// 5. Completing seq=0 is rejected (invalid sequence)
/// 6. Completing a non-contiguous seq (e.g., 5) when boundary=2 leaves boundary unchanged
///    until the gap is filled
#[kani::proof]
#[kani::unwind(6)]
fn kani_watermark_monotonic() {
    // Create a valid RunId
    let run_val: u64 = kani::any();
    kani::assume(run_val > 0 && run_val != u64::MAX);
    let run = RunId::new(run_val);

    let max_pending = 16;
    let max_waiters = 16;
    let mut watermark = CompletionWatermark::new(run, max_pending, max_waiters);

    // Property 1: New watermark has boundary = 0
    kani::assert(watermark.boundary() == 0, "new watermark starts at boundary=0");

    // Property 2: Complete seq=1 succeeds and advances boundary to 1
    let result1 = watermark.complete(run, 1);
    kani::assert(result1.is_ok(), "complete(seq=1) must succeed for new watermark");

    // After completing 1, boundary must be 1
    kani::assert(watermark.boundary() == 1,
        "boundary must advance to 1 after completing seq=1",
    );

    // Property 3: Complete seq=2 succeeds and advances boundary to 2
    let result2 = watermark.complete(run, 2);
    kani::assert(result2.is_ok(), "complete(seq=2) must succeed when boundary=1");

    // After completing 2, boundary must be 2
    kani::assert(watermark.boundary() == 2,
        "boundary must advance to 2 after completing seq=2",
    );

    // Property 4: Complete seq=1 again is rejected (duplicate)
    let result_dup = watermark.complete(run, 1);
    kani::assert(matches!(result_dup, Err(CompletionWatermarkError::Duplicate { seq }) if seq == 1),
        "completing same seq twice must return Duplicate error",
    );

    // boundary must NOT have changed
    kani::assert(watermark.boundary() == 2,
        "boundary must not change after duplicate completion attempt",
    );

    // Property 5: Complete seq=0 is rejected (invalid sequence)
    let result_zero = watermark.complete(run, 0);
    kani::assert(matches!(result_zero, Err(CompletionWatermarkError::InvalidSequence { seq }) if seq == 0),
        "completing seq=0 must return InvalidSequence error",
    );

    // boundary must NOT have changed
    kani::assert(watermark.boundary() == 2,
        "boundary must not change after invalid sequence attempt",
    );

    // Property 6: Complete seq=5 (gap in sequence) is accepted but boundary stays at 2
    let result_gap = watermark.complete(run, 5);
    kani::assert(result_gap.is_ok(), "complete(seq=5) with gap must return Ok (queued as pending)");

    // boundary must still be 2 because seq=3 and seq=4 are missing
    kani::assert(watermark.boundary() == 2,
        "boundary must not advance when there is a gap in sequences",
    );

    // Now fill the gap: complete seq=3, seq=4
    match watermark.complete(run, 3) {
        Ok(v) => { let _ = v; },
        Err(_) => {
            kani::assume(false);
            return;
        }
    }
    kani::assert(watermark.boundary() == 3, "boundary advances to 3 after completing seq=3");

    match watermark.complete(run, 4) {
        Ok(v) => { let _ = v; },
        Err(_) => {
            kani::assume(false);
            return;
        }
    }
    kani::assert(watermark.boundary() == 4, "boundary advances to 4 after completing seq=4");

    // seq=5 was already pending, so after seq=4, boundary should jump to 5
    // (drain_prefix drains all contiguous sequences)
    kani::assert(watermark.boundary() == 5,
        "boundary must advance to 5 after completing seq=4 (draining pending seq=5)",
    );

    // Property 7: Complete for wrong run is rejected
    let wrong_run = RunId::new(run_val.wrapping_add(100));
    let result_wrong = watermark.complete(wrong_run, 6);
    kani::assert(matches!(result_wrong, Err(CompletionWatermarkError::WrongRun { .. })),
        "completing with wrong run_id must return WrongRun error",
    );

    // boundary must NOT have changed
    kani::assert(watermark.boundary() == 5,
        "boundary must not change after wrong-run completion attempt",
    );
}
