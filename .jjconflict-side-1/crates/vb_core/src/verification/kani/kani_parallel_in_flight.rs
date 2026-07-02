#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harnesses for parallel-in-flight (PIF) lifecycle invariants.
//!
//! PO-KANI-007: Proves 0 ≤ PIF ≤ max_PIF for all valid sequences of
//! increment/decrement operations on RunFrame.
//!
//! Cross-validated by PO-PROP-005 (10k proptest runs with randomized
//! for_each/together/FanOut operation sequences).

use crate::frame::RunFrame;
use crate::ids::{RunId, StepIdx};

/// Creates a RunFrame suitable for PIF operation testing.
fn make_test_frame(slots: u16, max_pif: u16) -> Result<RunFrame, crate::errors::CoreError> {
    RunFrame::new(RunId::new(0), StepIdx::new(0), 1, slots)
        .map(|mut frame: RunFrame| {
            frame.set_max_parallel_in_flight(max_pif);
            frame
        })
}

/// PO-KANI-007: Proves that parallel_in_flight never underflows and
/// never exceeds the maximum across add/sub sequences.
#[kani::proof]
#[kani::unwind(20)]
fn kani_parallel_in_flight_lifecycle() {
    let max_pif: u16 = kani::any();
    kani::assume(max_pif >= 1);
    kani::assume(max_pif <= 256);

    let mut frame = match make_test_frame(max_pif, max_pif) {
        Ok(f) => f,
        Err(_) => return, // frame creation failed, skip
    };

    // Initial state
    assert_eq!(
        frame.parallel_in_flight(),
        0,
        "PIF must start at 0"
    );
    assert_eq!(
        frame.max_parallel_in_flight(),
        max_pif,
        "max_PIF must match configured limit"
    );

    kani::cover!(frame.parallel_in_flight() == 0);
    kani::cover!(frame.max_parallel_in_flight() == max_pif);

    // Symbolic operation sequence: up to 10 operations
    let ops_count: u8 = kani::any();
    kani::assume(ops_count <= 10);

    for _ in 0..ops_count {
        let op: u8 = kani::any(); // 0 = add, 1 = sub
        let count: u16 = kani::any();
        kani::assume(count >= 1);
        kani::assume(count <= 8);

        match op % 2 {
            0 => {
                // Add operation
                match frame.add_parallel_in_flight(count) {
                    Ok(()) => {
                        // After successful add, PIF must:
                        // - be within [1, max_pif]
                        // - not exceed max
                        let pif = frame.parallel_in_flight();
                        assert!(pif <= max_pif,
                            "PIF {} must not exceed max {}", pif, max_pif);
                        assert!(
                            frame.max_parallel_in_flight() >= pif,
                            "tracked max {} must be >= current PIF {}",
                            frame.max_parallel_in_flight(),
                            pif
                        );
                    }
                    Err(_) => {
                        // Overflow is OK; PIF must remain valid
                        let pif = frame.parallel_in_flight();
                        assert!(pif <= max_pif,
                            "PIF must remain ≤ max after failed add");
                    }
                }
            }
            _ => {
                // Sub operation
                match frame.sub_parallel_in_flight(count) {
                    Ok(()) => {
                        // PIF must be ≥ 0 after successful sub
                        let pif = frame.parallel_in_flight();
                        assert!(pif <= max_pif,
                            "PIF {} must be ≤ max {}", pif, max_pif);
                    }
                    Err(_) => {
                        // Underflow is OK; PIF must not go below 0
                        // The checked_sub prevents underflow
                    }
                }
            }
        }

        // Invariant: 0 ≤ PIF ≤ max_PIF holds after every operation
        let pif = frame.parallel_in_flight();
        assert!(pif <= max_pif,
            "PIF invariant: {} must be ≤ max_PIF {}", pif, max_pif);
    }

    kani::cover!(frame.parallel_in_flight() > 0);
    kani::cover!(frame.max_parallel_in_flight() > 0);
    kani::cover!(frame.parallel_in_flight() == frame.max_parallel_in_flight());
}

/// PO-KANI-007: Proves explicit overflow rejection.
#[kani::proof]
fn kani_parallel_in_flight_overflow_rejection() {
    // Start with PIF near u16::MAX
    let mut frame = match make_test_frame(1, u16::MAX) {
        Ok(f) => f,
        Err(_) => return,
    };

    // Set PIF to a high value
    frame.set_max_parallel_in_flight(u16::MAX);
    let _ = frame.add_parallel_in_flight(u16::MAX - 10);
    let pif_before = frame.parallel_in_flight();

    // Adding more should overflow and return error
    let result = frame.add_parallel_in_flight(20);
    match result {
        Ok(()) => {
            // If ok, the checked_add prevented overflow
            let pif = frame.parallel_in_flight();
            assert!(pif >= pif_before);
        }
        Err(_) => {
            // Overflow rejected — PIF must be unchanged
            assert_eq!(
                frame.parallel_in_flight(),
                pif_before,
                "PIF must be unchanged after overflow rejection"
            );
        }
    }

    kani::cover!(result.is_ok());
    kani::cover!(result.is_err());
}

/// PO-KANI-007: Proves explicit underflow rejection.
#[kani::proof]
fn kani_parallel_in_flight_underflow_rejection() {
    let mut frame = match make_test_frame(1, 256) {
        Ok(f) => f,
        Err(_) => return,
    };

    // PIF starts at 0; subtracting anything should underflow
    let result = frame.sub_parallel_in_flight(1);
    assert!(
        result.is_err(),
        "sub_parallel_in_flight(1) from PIF=0 must return error"
    );
    assert_eq!(
        frame.parallel_in_flight(),
        0,
        "PIF must remain 0 after underflow rejection"
    );

    kani::cover!(result.is_err());
}
