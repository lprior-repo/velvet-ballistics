// Kani proof harness for PS-003: size-bound rejection (Gate 1).

#![forbid(unsafe_code)]
#![cfg(kani)]

use crate::admission::reject_oversized_compiled_ir_value;
use crate::constants::MAX_COMPILED_IR_BYTES;
use crate::error::JournalError;

fn max_compiled_ir_usize() -> usize {
    match usize::try_from(MAX_COMPILED_IR_BYTES) {
        Ok(value) => value,
        Err(_) => {
            kani::assume(false);
            0
        }
    }
}

/// PS-003: bounded exhaustive verification of the compiled-IR size gate.
#[kani::proof]
#[kani::unwind(4)]
fn ps_003_size_bound() {
    let len: usize = kani::any();
    let max_usize = max_compiled_ir_usize();
    let upper = match max_usize.checked_add(1_024) {
        Some(value) => value,
        None => {
            kani::assume(false);
            return;
        }
    };
    kani::assume(len <= upper);

    let result = reject_oversized_compiled_ir_value(len);

    if len <= max_usize {
        kani::assert(result.is_ok(), "len within max must be accepted");
    } else {
        match result {
            Err(JournalError::PayloadTooLarge { len: reported, max }) => {
                let converted = match u32::try_from(len) {
                    Ok(value) => value,
                    Err(_) => u32::MAX,
                };
                kani::assert(reported == converted, "PayloadTooLarge.len matches input");
                kani::assert(
                    max == MAX_COMPILED_IR_BYTES,
                    "PayloadTooLarge.max matches cap",
                );
            }
            Ok(()) => kani::assert(false, "oversized payload must be rejected"),
            Err(_) => kani::assert(false, "size gate must use PayloadTooLarge"),
        }
    }
}

/// PS-003b: zero, max, and every value up to max convert to u32 safely.
#[kani::proof]
fn ps_003_u32_conversion_safe() {
    let max_usize = max_compiled_ir_usize();
    kani::assert(u32::try_from(0_usize).is_ok(), "zero converts");
    kani::assert(u32::try_from(max_usize).is_ok(), "max converts");

    let value: usize = kani::any();
    kani::assume(value <= max_usize);
    kani::assert(u32::try_from(value).is_ok(), "bounded value converts");
}

/// PS-003c: usize::MAX is rejected by the size gate.
#[kani::proof]
fn ps_003_usize_max_rejected() {
    let result = reject_oversized_compiled_ir_value(usize::MAX);
    kani::assert(result.is_err(), "usize::MAX must be rejected");
}
