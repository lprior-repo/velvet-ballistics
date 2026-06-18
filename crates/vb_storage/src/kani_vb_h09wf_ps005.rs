// Kani proof harness for PS-005: trailing-byte rejection (Gate 3).

#![forbid(unsafe_code)]
#![cfg(kani)]

use crate::codec::payload::reject_trailing_bytes;
use crate::error::JournalError;

/// PS-005: bounded check of reject_trailing_bytes for all pairs up to 256.
#[kani::proof]
fn ps_005_trailing_bytes() {
    let declared_end: usize = kani::any();
    let actual_len: usize = kani::any();
    kani::assume(declared_end <= 256);
    kani::assume(actual_len <= 256);

    let result = reject_trailing_bytes(declared_end, actual_len);

    if actual_len > declared_end {
        match result {
            Err(JournalError::UnexpectedTrailingBytes {
                declared_end: reported_declared,
                actual_len: reported_actual,
            }) => {
                kani::assert(reported_declared == declared_end, "declared_end is preserved");
                kani::assert(reported_actual == actual_len, "actual_len is preserved");
            }
            Ok(()) => kani::assert(false, "trailing bytes must be rejected"),
            Err(_) => kani::assert(false, "trailing bytes use typed error"),
        }
    } else {
        kani::assert(result.is_ok(), "no trailing bytes must pass");
    }
}

/// PS-005b: zero-length exact match passes.
#[kani::proof]
fn ps_005_zero_length_case() {
    let result = reject_trailing_bytes(0, 0);
    kani::assert(result.is_ok(), "zero/zero must pass");
}

/// PS-005c: one trailing byte is detected and reported exactly.
#[kani::proof]
fn ps_005_single_trailing_byte() {
    let result = reject_trailing_bytes(0, 1);
    match result {
        Err(JournalError::UnexpectedTrailingBytes {
            declared_end,
            actual_len,
        }) => {
            kani::assert(declared_end == 0, "declared_end is zero");
            kani::assert(actual_len == 1, "actual_len is one");
        }
        Ok(()) => kani::assert(false, "one trailing byte must be rejected"),
        Err(_) => kani::assert(false, "one trailing byte uses typed error"),
    }
}
