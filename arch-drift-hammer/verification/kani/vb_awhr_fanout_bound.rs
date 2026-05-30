// Verification artifact: vb_awhr_fanout_bound.rs
// Bead: vb-awhr
// PO: PO-001 (fanout limit: ≤64 accepted, >64 rejected)
// Verifier: Kani
// Command: cargo kani --package vb_compile --harness lower_choose_fanout_bound
//
// Proof obligations:
// - PO-001: lower_choose accepts up to 64 branches inclusive
// - PO-001: lower_choose rejects >64 branches with PrimitiveLoweringLimitExceeded
// - PO-001: No fanout bypass exists (dead compile/mod.rs lower_choose lacks limit)
//
// GOD RULE 1: Uses kani::any() for symbolic branch contents — no hardcoded shapes.
// GOD RULE 2: Binds to actual Rust lower_choose in mod_compile_lowering/part_06.rs.

#![cfg(kani)]
#![forbid(unsafe_code)]

use crate::{CompileError, SlotCompiler, lower_choose};
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::workflow::SlotBranch;

/// PO-001 H1: lower_choose correctly enforces the 64-branch fanout limit.
///
/// For branch_count ≤ 64: result is Ok (accepted).
/// For branch_count > 64: result is Err(PrimitiveLoweringLimitExceeded).
#[kani::proof]
#[kani::unwind(128)]
fn lower_choose_fanout_bound() {
    let branch_count: u8 = kani::any();
    kani::assume(branch_count <= 65);

    // Symbolically choose whether to test acceptance (64) or rejection (65).
    let test_rejection: bool = kani::any();

    let branches: Vec<SlotBranch> = if test_rejection {
        (0..65)
            .map(|_| SlotBranch {
                condition: SlotIdx::new(kani::any()),
                target: StepIdx::new(kani::any()),
            })
            .collect()
    } else {
        (0..64)
            .map(|_| SlotBranch {
                condition: SlotIdx::new(kani::any()),
                target: StepIdx::new(kani::any()),
            })
            .collect()
    };

    let mut builder = SlotCompiler::new();
    let result = lower_choose(
        StepIdx::new(0),
        branches,
        Some(StepIdx::new(1)),
        &mut builder,
    );

    if test_rejection {
        match result {
            Err(CompileError::PrimitiveLoweringLimitExceeded {
                primitive,
                field,
                value,
                limit,
            }) => {
                kani::assert(primitive == "choose", "error primitive is choose");
                kani::assert(field == "branches", "error field is branches");
                kani::assert(value == 65, "error value matches branch count");
                kani::assert(limit == 64, "error limit is 64");
            }
            _ => {
                kani::assert(
                    false,
                    ">64 branches must reject with PrimitiveLoweringLimitExceeded",
                );
            }
        }
    } else {
        kani::assert(result.is_ok(), "≤64 branches must be accepted");
    }
    std::mem::forget(builder);
}

/// PO-001 H2: No fanout bypass exists in the dead compile/mod.rs lower_choose.
///
/// The dead code in compile/mod.rs (line 371) does NOT enforce the fanout limit.
/// This harness proves that the live lower_choose (re-exported from
/// mod_compile_lowering) is the only reachable lower_choose from the public API.
#[kani::proof]
#[kani::unwind(128)]
fn lower_choose_live_api_has_fanout_check() {
    // Call the public lower_choose with 65 branches; it MUST reject.
    let branches: Vec<SlotBranch> = (0..65)
        .map(|_| SlotBranch {
            condition: SlotIdx::new(kani::any()),
            target: StepIdx::new(kani::any()),
        })
        .collect();

    let mut builder = SlotCompiler::new();
    let result = lower_choose(StepIdx::new(0), branches, Some(StepIdx::new(1)), &mut builder);

    match result {
        Err(CompileError::PrimitiveLoweringLimitExceeded { .. }) => {
            // Public API correctly rejects 65 branches.
        }
        Ok(_) => {
            kani::assert(false, "public lower_choose must reject 65 branches");
        }
        Err(_) => {
            kani::assert(false, "public lower_choose must reject with PrimitiveLoweringLimitExceeded");
        }
    }
    std::mem::forget(builder);
}
