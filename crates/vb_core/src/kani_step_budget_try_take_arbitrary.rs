#![cfg(kani)]
#![forbid(unsafe_code)]

//! PO-009: generated structural Kani harness for zero-budget exhaustion.
//!
//! The harness varies `WorkflowParts` and `RunFrame` dimensions with
//! `kani::any`/bounded generators from `kani_workflow_arbitrary`; it does not
//! prove a single hardcoded dummy frame shape.

use crate::engine::StepBudget;
use crate::frame::RunFrame;
use crate::ids::{RunId, StepIdx};

#[derive(Clone, Copy)]
struct WorkflowShape {
    entry_raw: u16,
    node_count: u16,
    slot_count: u16,
    resource_max_steps: u64,
    resource_tick_budget: u64,
}

impl kani::Arbitrary for WorkflowShape {
    fn any() -> Self {
        let node_count: u16 = kani::any();
        kani::assume((1..=2).contains(&node_count));
        let entry_raw: u16 = kani::any();
        kani::assume(entry_raw < node_count);
        let slot_count: u16 = kani::any();
        kani::assume(slot_count <= 2);
        Self {
            entry_raw,
            node_count,
            slot_count,
            resource_max_steps: kani::any(),
            resource_tick_budget: kani::any(),
        }
    }
}

#[derive(Clone, Copy)]
struct FrameShape {
    run_id: RunId,
    first_step: StepIdx,
    step_count: u16,
    slot_count: u16,
}

impl kani::Arbitrary for FrameShape {
    fn any() -> Self {
        let step_count: u16 = kani::any();
        kani::assume((1..=2).contains(&step_count));
        let first_step_raw: u16 = kani::any();
        kani::assume(first_step_raw < step_count);
        let slot_count: u16 = kani::any();
        kani::assume(slot_count <= 2);
        Self {
            run_id: RunId::new(kani::any()),
            first_step: StepIdx::new(first_step_raw),
            step_count,
            slot_count,
        }
    }
}

/// PO-009: zero-budget production budget transition preserves generated actual run frames.
#[kani::proof]
#[kani::unwind(12)]
fn kani_step_budget_try_take_arbitrary() {
    let workflow: WorkflowShape = kani::any();
    let shape = FrameShape {
        run_id: RunId::new(kani::any()),
        first_step: StepIdx::new(workflow.entry_raw),
        step_count: workflow.node_count,
        slot_count: workflow.slot_count,
    };
    let clamped_tick_budget = StepBudget::new(workflow.resource_tick_budget);
    let clamped_max_steps = StepBudget::new(workflow.resource_max_steps);

    let run = match RunFrame::new(
        shape.run_id,
        shape.first_step,
        shape.step_count,
        shape.slot_count,
    ) {
        Ok(value) => value,
        Err(_) => {
            kani::assert(false, "generated run frame shape must be valid");
            return;
        }
    };
    let before_pc = run.pc();
    let before_executed = run.executed();
    let before_step_count = run.step_count();
    let before_slot_count = run.slot_count();
    let before_max_parallel = run.max_parallel_in_flight();
    let before_parallel = run.parallel_in_flight();
    let before_first_step = run.step_state(shape.first_step);

    let mut budget = StepBudget::new(0);
    let result = budget.try_take();

    kani::assert(result == Ok(false), "zero budget reports exhaustion");
    kani::assert(budget.remaining() == 0, "zero budget remains zero");
    kani::assert(
        clamped_tick_budget.remaining() <= StepBudget::MAX.remaining(),
        "tick budget generator clamps",
    );
    kani::assert(
        clamped_max_steps.remaining() <= StepBudget::MAX.remaining(),
        "max steps generator clamps",
    );
    kani::assert(run.pc() == before_pc, "actual pc is preserved");
    kani::assert(
        run.executed() == before_executed,
        "actual execution count is preserved",
    );
    kani::assert(
        run.step_count() == before_step_count,
        "actual step count is preserved",
    );
    kani::assert(
        run.slot_count() == before_slot_count,
        "actual slot count is preserved",
    );
    kani::assert(
        run.max_parallel_in_flight() == before_max_parallel,
        "actual max parallel accounting is preserved",
    );
    kani::assert(
        run.parallel_in_flight() == before_parallel,
        "actual parallel accounting is preserved",
    );
    kani::assert(
        run.step_state(shape.first_step) == before_first_step,
        "actual entry step state is preserved",
    );
}
