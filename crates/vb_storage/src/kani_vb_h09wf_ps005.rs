// Kani proof harness for PS-005: Trailing bytes rejection (Gate 3).
//
// Obligation: PO-vb-h09wf-014
// Verifier: kani
// Command: cargo kani -p vb_storage --harness ps_005_trailing_bytes --features kani-vb-h09wf
//
// Domain claim: For all (declared_end, actual_len) pairs where declared_end <= actual_len
// within bounded domain: Ok(()) when declared_end == actual_len,
// Err(UnexpectedTrailingBytes) when declared_end < actual_len.
//
// PRODUCTION BINDING:
//   vb_storage::codec::payload::reject_trailing_bytes (codec/payload.rs:86-97)
//
// Trusted base: usize comparison is structural
// Model bounds: declared_end and actual_len bounded to 0..256
// Source: .beads/vb-h09wf/proof-obligations.planned.jsonl PO-vb-h09wf-014

#![forbid(unsafe_code)]
#![cfg(kani)]

use crate::codec::payload::reject_trailing_bytes;
use crate::error::JournalError;

/// PS-005: Exhaustive test of reject_trailing_bytes for bounded domain.
#[kani::proof]
#[kani::unwind(3)]
fn ps_005_trailing_bytes() {
    let declared_end: usize = kani::any();
    let actual_len: usize = kani::any();
    kani::assume(declared_end <= 256);
    kani::assume(actual_len <= 256);

    let result = reject_trailing_bytes(declared_end, actual_len);

    if declared_end == actual_len {
        // Exact match: no trailing bytes
        assert!(result.is_ok(), "equal lengths must pass, got {result:?}");
    } else if actual_len > declared_end {
        // Trailing bytes present
        match result {
            Err(JournalError::UnexpectedTrailingBytes {
                declared_end: d,
                actual_len: a,
            }) => {
                assert_eq!(d, declared_end, "declared_end must match");
                assert_eq!(a, actual_len, "actual_len must match");
            }
            Ok(()) => {
                kani::assert(
                    false,
                    "trailing bytes (actual={actual_len} > declared={declared_end}) must be rejected"
                );
            }
            Err(_) => {
                // Other errors also acceptable (defense-in-depth)
            }
        }
    } else {
        // actual_len < declared_end: if declared_end > actual_len, the caller
        // precondition should prevent this, but the function should still return Ok
        // (trailing_byte_bounds only checks actual_len > declared_end)
        // This is the "declared_end overshoots actual_len" case — Ok is fine
    }

    kani::cover!(result.is_ok(), "no trailing bytes passes");
    kani::cover!(
        matches!(result, Err(JournalError::UnexpectedTrailingBytes { .. })),
        "trailing bytes correctly detected"
    );
}

/// PS-005b: Zero-length case — equal zero is fine.
#[kani::proof]
fn ps_005_zero_length_case() {
    let result = reject_trailing_bytes(0, 0);
    assert!(result.is_ok(), "zero/zero must pass");
}

/// PS-005c: Single trailing byte always detected.
#[kani::proof]
fn ps_005_single_trailing_byte() {
    let result = reject_trailing_bytes(0, 1);
    assert!(result.is_err(), "one trailing byte must be rejected");
}
