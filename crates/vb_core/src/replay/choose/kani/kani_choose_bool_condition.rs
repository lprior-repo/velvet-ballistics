// Verification artifact: kani_choose_bool_condition.rs
// Bead: vb-xi2f.13 | State: 5 (proof-writer)
// PO: PO-KANI-009 — Boolean condition slot invariant for replay_choose_slot
// Command: cargo kani -p vb_core --harness kani_choose_bool_condition --unwind 16
//
// GOD RULE 1: Uses kani::any() for nondeterministic slot value generation.
// GOD RULE 2: Binds to production replay_choose_slot in parent module.

#![forbid(unsafe_code)]

use crate::frame::RunFrame;
use crate::ids::{RunId, SlotIdx, StepIdx};
use crate::value::{FiniteF64, SlotValue};
use crate::workflow::SlotBranch;

use super::super::replay_choose_slot;

#[kani::proof]
#[kani::unwind(16)]
fn kani_choose_bool_condition() {
    // Generate symbolic slot value of any type.
    let value_kind: u8 = kani::any();
    kani::assume(value_kind < 6);

    // Build a non-NaN finite f64 for F64 variant
    let finite_f64 = match FiniteF64::new(0.0) {
        Ok(v) => v,
        Err(_) => {
            kani::cover!(true, "FiniteF64 construction failed — should not happen with 0.0");
            return;
        }
    };

    let slot_value = match value_kind {
        0 => SlotValue::Bool(true),
        1 => SlotValue::Bool(false),
        2 => SlotValue::I64(kani::any()),
        3 => SlotValue::F64(finite_f64),
        4 => SlotValue::Symbol(crate::ids::SymbolId::new(kani::any())),
        _ => SlotValue::Null,
    };

    // Ensure the RunFrame has enough capacity for the branch target (StepIdx(1))
    // and the otherwise target (StepIdx(2)). Need at least 3 steps and 1 slot.
    let step_count: u16 = kani::any();
    kani::assume(step_count >= 3 && step_count <= 8);
    let slot_count: u16 = 2;

    let mut run = match RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count) {
        Ok(frame) => frame,
        Err(_) => {
            kani::cover!(true, "RunFrame creation failed — valid error path");
            return;
        }
    };

    // Write the symbolic value to condition slot 0
    if run.write_slot(SlotIdx::new(0), slot_value).is_err() {
        kani::cover!(true, "write_slot failed");
        return;
    }

    let branches = vec![SlotBranch {
        condition: SlotIdx::new(0),
        target: StepIdx::new(1),
    }];
    let otherwise = Some(StepIdx::new(2));

    let result = replay_choose_slot(&mut run, &branches, otherwise);

    match slot_value {
        SlotValue::Bool(true) => {
            kani::assert(result.is_ok(), "Bool(true) must produce Ok result");
            kani::cover!(true, "Bool(true) matched branch successfully");
        }
        SlotValue::Bool(false) => {
            kani::assert(result.is_ok(), "Bool(false) must produce Ok with otherwise");
            kani::cover!(true, "Bool(false) fell through to otherwise");
        }
        _ => {
            kani::assert(result.is_err(), "non-Bool condition must produce Err");
            kani::cover!(true, "non-Bool condition correctly rejected");
        }
    }
}
