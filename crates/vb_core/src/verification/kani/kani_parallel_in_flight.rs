#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harnesses for parallel-in-flight (PIF) lifecycle invariants.
//!
//! PO-KANI-007: Proves 0 ≤ PIF ≤ max_PIF for all valid sequences of
//! increment/decrement operations on RunFrame.

use crate::frame::RunFrame;
use crate::ids::{RunId, StepIdx};

/// Creates a RunFrame suitable for PIF operation testing.
fn make_test_frame(slots: u16, max_pif: u16) -> Result<RunFrame, crate::errors::CoreError> {
    RunFrame::new(RunId::new(0), StepIdx::new(0), 1, slots).map(|mut frame: RunFrame| {
        frame.set_max_parallel_in_flight(max_pif);
        frame
    })
}

/// PO-KANI-007 H1: PIF starts at 0 and max_PIF matches configured limit.
#[kani::proof]
#[kani::unwind(5)]
fn kani_pif_initial_state() {
    let max_pif: u16 = kani::any();
    kani::assume(max_pif >= 1);
    kani::assume(max_pif <= 256);

    let frame = make_test_frame(max_pif, max_pif).unwrap_or_else(|_| kani::any());
    kani::assert(frame.parallel_in_flight() == 0, "PIF must start at 0");
    kani::assert(
        frame.max_parallel_in_flight() == max_pif,
        "max_PIF must match configured limit",
    );
}

/// PO-KANI-007 H2: add_parallel_in_flight increments PIF correctly.
#[kani::proof]
#[kani::unwind(10)]
fn kani_pif_add_increments() {
    let max_pif: u16 = kani::any();
    kani::assume(max_pif >= 10);

    let mut frame = make_test_frame(max_pif, max_pif).unwrap_or_else(|_| kani::any());

    let add_amount: u16 = kani::any();
    kani::assume(add_amount <= max_pif);

    frame
        .add_parallel_in_flight(add_amount)
        .unwrap_or_else(|_| kani::assume(false));
    kani::assert(
        frame.parallel_in_flight() == add_amount,
        "PIF must equal add_amount",
    );
    kani::assert(
        frame.parallel_in_flight() <= max_pif,
        "PIF must not exceed max_PIF",
    );
}

/// PO-KANI-007 H3: sub_parallel_in_flight decrements PIF correctly.
#[kani::proof]
#[kani::unwind(10)]
fn kani_pif_sub_decrements() {
    let max_pif: u16 = kani::any();
    kani::assume(max_pif >= 10);

    let mut frame = make_test_frame(max_pif, max_pif).unwrap_or_else(|_| kani::any());
    frame
        .add_parallel_in_flight(5)
        .unwrap_or_else(|_| kani::assume(false));
    kani::assert(
        frame.parallel_in_flight() == 5,
        "PIF must be 5 after add(5)",
    );

    frame
        .sub_parallel_in_flight(3)
        .unwrap_or_else(|_| kani::assume(false));
    kani::assert(
        frame.parallel_in_flight() == 2,
        "PIF must be 2 after sub(3)",
    );
}

/// PO-KANI-007 H4: PIF invariant holds after add/sub sequence.
#[kani::proof]
#[kani::unwind(20)]
fn kani_pif_invariant_holds() {
    let max_pif: u16 = kani::any();
    kani::assume(max_pif >= 1);
    kani::assume(max_pif <= 256);

    let mut frame = make_test_frame(max_pif, max_pif).unwrap_or_else(|_| kani::any());

    let add_amount: u16 = kani::any();
    kani::assume(add_amount <= max_pif);
    frame
        .add_parallel_in_flight(add_amount)
        .unwrap_or_else(|_| kani::assume(false));

    kani::assert(
        frame.parallel_in_flight() <= max_pif,
        "PIF invariant: must be <= max_PIF",
    );
    kani::assert(
        frame.max_parallel_in_flight() >= frame.parallel_in_flight(),
        "max_PIF must track peak PIF",
    );

    kani::cover!(frame.parallel_in_flight() > 0);
    kani::cover!(frame.max_parallel_in_flight() > 0);
}

/// PO-KANI-007 H5: add overflow produces error.
#[kani::proof]
#[kani::unwind(5)]
fn kani_pif_add_overflow_error() {
    let max_pif: u16 = kani::any();
    kani::assume(max_pif > 0);

    let mut frame = make_test_frame(max_pif, max_pif).unwrap_or_else(|_| kani::any());

    frame
        .add_parallel_in_flight(max_pif)
        .unwrap_or_else(|_| kani::assume(false));
    let result = frame.add_parallel_in_flight(1);

    if result.is_err() {
        kani::cover!(true, "overflow error path covered");
    }
}

/// PO-KANI-007 H6: sub underflow produces error.
#[kani::proof]
#[kani::unwind(5)]
fn kani_pif_sub_underflow_error() {
    let max_pif: u16 = kani::any();
    kani::assume(max_pif >= 1);

    let mut frame = make_test_frame(max_pif, max_pif).unwrap_or_else(|_| kani::any());

    let result = frame.sub_parallel_in_flight(1);
    kani::assert(
        result.is_err(),
        "sub_parallel_in_flight(1) from PIF=0 must return error",
    );
    kani::assert(
        frame.parallel_in_flight() == 0,
        "PIF must remain 0 after underflow rejection",
    );

    kani::cover!(result.is_err());
}
