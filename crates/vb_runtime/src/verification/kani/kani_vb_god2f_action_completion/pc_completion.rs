#![forbid(unsafe_code)]

use super::model::{
    HVR_RUNTIME_RUN_ID, completion_result_is_invalid_action_completion, completion_result_is_ok,
    completion_result_kind, generated_pc_input, generated_workflow,
};
use crate::shard::helpers::{advance_after_action_completion, make_run_state};
use vb_core::ids::StepIdx;

#[kani::proof]
#[kani::unwind(5)]
fn vb_god2f_advance_after_action_completion_pc_cases() {
    let input = generated_pc_input();
    let workflow = generated_workflow(input);
    let mut state = match make_run_state(workflow, HVR_RUNTIME_RUN_ID) {
        Some(value) => value,
        None => {
            kani::assert(
                false,
                "HVR-PO-RUNTIME-002 generated workflow builds RunState",
            );
            return;
        }
    };
    let before_pc = state.frame.pc();
    let step = StepIdx::new(input.step);
    let result = advance_after_action_completion(&mut state, step);
    let result_kind = completion_result_kind(&result);
    std::mem::forget(result);

    kani::cover!(
        input.step == input.node_count,
        "missing completion node branch"
    );
    kani::cover!(
        input.step < input.node_count && input.next_mode == 0,
        "terminal node branch"
    );
    kani::cover!(
        input.step < input.node_count && input.next_mode == 1,
        "valid next branch"
    );
    kani::cover!(
        input.step < input.node_count && input.next_mode == 2,
        "invalid next branch"
    );

    if input.step == input.node_count {
        kani::assert(
            completion_result_is_invalid_action_completion(result_kind),
            "missing node returns InvalidActionCompletion",
        );
        kani::assert(
            state.frame.pc() == before_pc,
            "missing node leaves PC unchanged",
        );
    } else if input.next_mode == 0 {
        kani::assert(
            completion_result_is_ok(result_kind),
            "terminal completion succeeds",
        );
        kani::assert(
            state.frame.pc() == before_pc,
            "terminal node leaves PC unchanged",
        );
    } else if input.next_mode == 1 {
        kani::assert(
            completion_result_is_ok(result_kind),
            "valid next completion succeeds",
        );
        kani::assert(
            state.frame.pc() == StepIdx::new(input.valid_next),
            "valid next moves PC to generated target",
        );
    } else {
        kani::assert(
            completion_result_is_invalid_action_completion(result_kind),
            "invalid next returns InvalidActionCompletion",
        );
        kani::assert(
            state.frame.pc() == before_pc,
            "invalid next leaves PC unchanged",
        );
    }
    std::mem::forget(state);
}
