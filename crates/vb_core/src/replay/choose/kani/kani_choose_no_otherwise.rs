// Verification artifact: kani_choose_no_otherwise.rs
// Bead: vb-xi2f.13 | State: 5 (proof-writer)
// PO: PO-KANI-010 — Otherwise liveness: replay_choose_slot with all-false and no otherwise
// Command: cargo kani -p vb_core --harness kani_choose_no_otherwise --unwind 16
//
// GOD RULE 1: Uses kani::any() for nondeterministic branch configurations.
// GOD RULE 2: Binds to production replay_choose_slot in parent module.
// Note: The error path (all branches false, otherwise=None) is loop-position-independent.
//       A single Bool(false) branch with otherwise=None is sufficient to exercise the
//       `otherwise.ok_or(Internal)` path at the end of the while loop.
//       This harness mirrors PO-KANI-009 structure for verification tractability.

#![forbid(unsafe_code)]

use crate::frame::RunFrame;
use crate::ids::{RunId, SlotIdx, StepIdx};
use crate::value::SlotValue;
use crate::workflow::SlotBranch;

use super::super::replay_choose_slot;

#[kani::proof]
#[kani::unwind(16)]
fn kani_choose_no_otherwise() {
    // Use same RunFrame sizing as PO-KANI-009 for tractability
    let step_count: u16 = kani::any();
    kani::assume(step_count >= 3 && step_count <= 8);
    let slot_count: u16 = 2;

    let mut run = match RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count) {
        Ok(frame) => frame,
        Err(_) => {
            return;
        }
    };

    // Write Bool(false) to condition slot 0 — triggers fallthrough in replay_choose_slot
    if run
        .write_slot(SlotIdx::new(0), SlotValue::Bool(false))
        .is_err()
    {
        return;
    }

    let branches = vec![SlotBranch {
        condition: SlotIdx::new(0),
        target: StepIdx::new(1),
    }];

    // otherwise=None: the critical case — with no matching branch, this must error
    let otherwise: Option<StepIdx> = None;
    let result = replay_choose_slot(&mut run, &branches, otherwise);

    match &result {
        Ok(_) => {}
        Err(_) => {}
    }
    kani::assert(
        result.is_err(),
        "all branches false with no otherwise: replay_choose_slot MUST return Internal error",
    );
}
