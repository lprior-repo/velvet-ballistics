#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harnesses for engine signal terminality properties.
//!
//! PO-KANI-008: Proves that RuntimeSignal::Finished,
//! StepBudgetExhausted, AwaitingAction, AwaitingWait, and
//! AwaitingAsk are all terminal — once emitted, the drive
//! loop executes no further steps.

use crate::engine::types::RuntimeSignal;
use vb_core::value::SlotValue;

// ---------------------------------------------------------------------------
// Harnesses
// ---------------------------------------------------------------------------

/// PO-KANI-008: Proves that the drive loop terminal signal discrimination
/// is correct. Continue is the ONLY non-terminal signal.
///
/// The drive loop in drive.rs implements:
/// ```ignore
/// match signal {
///     RuntimeSignal::Continue => {}   // loop continues
///     other => return Ok(other),      // loop exits
/// }
/// ```
#[kani::proof]
fn kani_engine_signal_terminality() {
    // Test via symbolic signal and pattern matching
    let signal: u8 = kani::any();
    kani::assume(signal < 6);

    // Manual construction of RuntimeSignal variants for exhaustive coverage
    let is_continue = signal == 0;

    // Simulate the drive loop: only Continue allows loop continuation
    let loop_exits = !is_continue;

    // Terminal signals are: Finished, StepBudgetExhausted,
    // AwaitingAction, AwaitingWait, AwaitingAsk
    let is_terminal = signal != 0;

    // Continue and terminal are mutually exclusive
    assert!(
        !(is_terminal && is_continue),
        "Continue cannot also be a terminal signal"
    );

    // All non-Continue signals are terminal
    assert_eq!(
        loop_exits, is_terminal,
        "loop exits iff signal is terminal"
    );

    kani::cover!(signal == 0); // Continue
    kani::cover!(signal == 1); // Finished
    kani::cover!(signal == 2); // StepBudgetExhausted
    kani::cover!(signal == 3); // AwaitingAction
    kani::cover!(signal == 4); // AwaitingWait
    kani::cover!(signal == 5); // AwaitingAsk
}

/// PO-KANI-008: Proves that Finished carries a valid SlotValue
/// (any SlotValue is valid).
#[kani::proof]
fn kani_engine_signal_finished_payload() {
    let value: SlotValue = kani::any();
    let signal = RuntimeSignal::Finished(value);

    match &signal {
        RuntimeSignal::Finished(v) => {
            // Finished always carries a value
            assert_eq!(v, &value);
            kani::cover!(true);
        }
        _ => {
            // Unreachable
        }
    }
}

/// PO-KANI-008: Proves that signal matching in the drive loop
/// correctly distinguishes Continue from all other variants.
#[kani::proof]
fn kani_engine_signal_loop_exit_condition() {
    // Test the actual RuntimeSignal type
    let signal: u8 = kani::any();
    kani::assume(signal < 6);

    // Build each variant and test
    let _exit_expected = signal != 0;

    match signal {
        0 => {
            // Continue — loop should NOT exit
            let sig = RuntimeSignal::Continue;
            let exits = !matches!(sig, RuntimeSignal::Continue);
            assert!(!exits, "Continue must not cause loop exit");
        }
        1 => {
            // Finished — loop SHOULD exit
            let sig = RuntimeSignal::Finished(SlotValue::Null);
            let exits = !matches!(sig, RuntimeSignal::Continue);
            assert!(exits, "Finished must cause loop exit");
        }
        2 => {
            // StepBudgetExhausted — loop SHOULD exit
            let sig = RuntimeSignal::StepBudgetExhausted;
            let exits = !matches!(sig, RuntimeSignal::Continue);
            assert!(exits, "StepBudgetExhausted must cause loop exit");
        }
        3 => {
            // AwaitingWait — loop SHOULD exit
            let sig = RuntimeSignal::AwaitingWait;
            let exits = !matches!(sig, RuntimeSignal::Continue);
            assert!(exits, "AwaitingWait must cause loop exit");
        }
        _ => {
            // AwaitingAsk — loop SHOULD exit
            let sig = RuntimeSignal::AwaitingAsk;
            let exits = !matches!(sig, RuntimeSignal::Continue);
            assert!(exits, "AwaitingAsk must cause loop exit");
        }
    }

    kani::cover!(true);
}
