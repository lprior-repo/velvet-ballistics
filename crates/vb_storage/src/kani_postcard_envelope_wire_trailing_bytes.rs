#![forbid(unsafe_code)]
#![cfg(kani)]
//! Kani harnesses for vb-e7tl trailing-byte envelope strictness.

use crate::codec::payload::trailing_byte_bounds;

/// vb-e7tl-OB-001: equal declared/actual ends are accepted.
#[kani::proof]
#[kani::unwind(8)]
fn vb_e7tl_trailing_bytes_required() {
    let declared_end: usize = kani::any();

    let result = trailing_byte_bounds(declared_end, declared_end);

    match result {
        None => {}
        Some(_) => {
            kani::assert(
                false,
                "equal declared and actual lengths must not produce trailing-byte error",
            );
        }
    }
}

/// vb-e7tl-OB-004: nonzero trailing bytes are rejected with exact offsets.
#[kani::proof]
#[kani::unwind(8)]
fn vb_e7tl_trailing_bytes_rejected() {
    let declared_end: usize = kani::any();
    let extra_len: usize = kani::any();
    kani::assume(extra_len != 0);
    kani::assume(declared_end <= usize::MAX - extra_len);
    kani::cover!(extra_len == 1, "one trailing byte is covered");
    kani::cover!(
        declared_end == 0 && extra_len == usize::MAX,
        "max trailing length is covered"
    );

    let actual_len = declared_end + extra_len;
    let result = trailing_byte_bounds(declared_end, actual_len);

    match result {
        Some((found_declared_end, found_actual_len)) => {
            kani::assert(
                found_declared_end == declared_end,
                "declared_end is preserved in the error",
            );
            kani::assert(
                found_actual_len == actual_len,
                "actual_len is preserved in the error",
            );
        }
        None => {
            kani::assert(
                false,
                "larger actual length must not pass trailing-byte validation",
            );
        }
    }
}
