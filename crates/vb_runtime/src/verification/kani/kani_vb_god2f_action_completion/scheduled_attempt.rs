#![forbid(unsafe_code)]

use super::model::{HVR_RUNTIME_RUN_ID, generated_pc_input, generated_workflow};
use crate::shard::helpers::action::scheduled_attempt_after;
use crate::shard::helpers::{make_run_state, record_scheduled_attempt};
use vb_core::action::{ActionTicket, MockMarker};
use vb_core::ids::{ActionId, SeqNo, StepIdx};

#[kani::proof]
#[kani::unwind(12)]
fn vb_god2f_record_scheduled_attempt_monotonic() {
    let input = generated_pc_input();
    let workflow = generated_workflow(input);
    let mut state = match make_run_state(workflow, HVR_RUNTIME_RUN_ID) {
        Some(value) => value,
        None => {
            kani::assert(
                false,
                "HVR-PO-RUNTIME-006 generated workflow builds RunState",
            );
            return;
        }
    };

    let current: u16 = kani::any();
    let incoming: u16 = kani::any();
    let ticket = ActionTicket {
        run: HVR_RUNTIME_RUN_ID,
        step: StepIdx::new(input.step),
        seq: SeqNo::ZERO,
        action: ActionId::new(1),
        attempt: incoming,
        idempotency_key: kani::any(),
        capacity: u16::MAX,
        mock: MockMarker::HttpGet,
    };

    if let Some(slot) = state.action_attempts.get_mut(ticket.step.as_usize()) {
        *slot = current;
    }
    let before = state.action_attempts.clone();

    record_scheduled_attempt(&mut state, ticket);

    kani::cover!(incoming == 0, "zero incoming attempt branch");
    kani::cover!(incoming > current, "future attempt branch");
    kani::cover!(
        incoming <= current && incoming != 0,
        "stale/current attempt branch"
    );
    kani::cover!(
        ticket.step.as_usize() >= before.len(),
        "out-of-bounds step branch"
    );

    if ticket.step.as_usize() >= before.len() {
        kani::assert(
            state.action_attempts.len() == before.len(),
            "out-of-bounds scheduled attempt preserves attempt array length",
        );
    } else {
        let observed = state.action_attempts.get(ticket.step.as_usize()).copied();
        let expected = scheduled_attempt_after(Some(current), incoming);
        kani::assert(
            observed == expected,
            "recorded attempt matches pure scheduled kernel",
        );
        if let Some(after) = observed {
            kani::assert(
                after >= current,
                "scheduled attempts never decrease current counter",
            );
            if incoming != 0 {
                kani::assert(
                    after >= incoming || current >= incoming,
                    "attempt is monotonic max",
                );
            }
        } else {
            kani::assert(
                false,
                "in-bounds scheduled attempt always remains observable",
            );
        }
    }
    std::mem::forget(before);
    std::mem::forget(state);
}
