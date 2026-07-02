// Fuzz target: choose_lowering_fuzzer.rs
// Bead: vb-njib
// PO: ps-01 (fuzzing for choose lowering)
// Verifier: cargo-fuzz
// Command: cargo fuzz run choose_lowering_fuzzer
//
// Fuzzing strategy:
// - Generate arbitrary SlotBranch vectors with condition and target values
// - Test that lower_choose handles all cases without panicking
// - Cover branch counts from 0 to 128 to find boundary issues
//
// This is the ps-01 fuzz target for the choose lowering bug fix.

#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use vb_compile::{lower_choose, SlotCompiler};
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::workflow::SlotBranch;

/// Fuzz target for lower_choose panic-freedom.
///
/// Generates arbitrary branch counts and SlotBranch values to test:
/// - 0 branches with/without otherwise
/// - 1..64 branches (should succeed)
/// - 65+ branches (should return error, not panic)
/// - Arbitrary condition and target values
fn fuzz_choose_lowering(
    branch_count: u8,
    conditions: &[u16],
    targets: &[u16],
    otherwise_val: Option<u16>,
) {
    // Limit branch_count to avoid OOM
    let count = if branch_count > 128 { 128 } else { branch_count } as usize;

    // Empty conditions or targets would trigger RemByZero panics in the modulo.
    // Lowering empty branches is a degenerate case; skip it instead of crashing.
    if conditions.is_empty() || targets.is_empty() {
        return;
    }

    // Build branches from fuzzer data
    let branches: Vec<SlotBranch> = (0..count)
        .map(|i| {
            let cond_idx = i % conditions.len();
            let tgt_idx = i % targets.len();
            SlotBranch {
                condition: SlotIdx::new(conditions[cond_idx]),
                target: StepIdx::new(targets[tgt_idx]),
            }
        })
        .collect();

    let otherwise = otherwise_val.map(StepIdx::new);

    let mut builder = SlotCompiler::new();
    let _result = lower_choose(StepIdx::new(0), branches, otherwise, &mut builder);
    // We don't care about the result - we just want to ensure no panic
}

fuzz_target!(|data: (u8, Vec<u16>, Vec<u16>, Option<u16>)| {
    let (branch_count, conditions, targets, otherwise_val) = data;
    fuzz_choose_lowering(branch_count, &conditions, &targets, otherwise_val);
});
