#![cfg(all(kani, feature = "vb-god2f-action-completion"))]
#![forbid(unsafe_code)]

//! HVR-PO-RUNTIME-{001,002,006}: production-bound Kani harnesses for
//! action dispatch input limits, PC advancement, and scheduled-attempt state.

use std::vec::Vec;

use vb_core::action::{
    ActionContract, ActionError, ActionInput, ActionName, ActionOutcome, ActionTicket, Idempotency,
    MockMarker, RetrySafety, SideEffect,
};
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};

use crate::RuntimeError;
use crate::shard::helpers::action::scheduled_attempt_after;
use crate::shard::helpers::{
    advance_after_action_completion, make_run_state, record_scheduled_attempt,
};

const HVR_RUNTIME_RUN_ID: RunId = RunId::new(1);

#[derive(Clone, Copy)]
struct PcHarnessInput {
    node_count: u16,
    step: u16,
    next_mode: u8,
    valid_next: u16,
}

fn generated_contract(input_slot_count: u16, max_input_bytes: u32) -> Option<ActionContract> {
    let name = match ActionName::new("a") {
        Ok(value) => value,
        Err(_) => return None,
    };
    Some(ActionContract {
        id: ActionId::new(1),
        name,
        input_slot_count,
        output_slot_count: 1,
        max_input_bytes,
        max_output_bytes: 1,
        timeout_ms: 1,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::Idempotent,
        required_capabilities: Vec::new().into_boxed_slice(),
    })
}

fn generated_action_input(action: ActionId, step: StepIdx) -> ActionInput {
    let seq: u64 = kani::any();
    let attempt: u16 = kani::any();
    let idempotency_key: u128 = kani::any();
    ActionInput {
        run: HVR_RUNTIME_RUN_ID,
        step,
        action,
        input: SlotIdx::ZERO,
        ticket: ActionTicket {
            run: HVR_RUNTIME_RUN_ID,
            step,
            seq: SeqNo::new(seq),
            action,
            attempt,
            idempotency_key,
            capacity: 1,
            mock: MockMarker::HttpGet,
        },
    }
}

fn generated_pc_input() -> PcHarnessInput {
    let node_count: u16 = kani::any();
    kani::assume(node_count >= 1);
    kani::assume(node_count <= 8);

    let step: u16 = kani::any();
    kani::assume(step <= node_count);

    let next_mode: u8 = kani::any();
    kani::assume(next_mode <= 2);

    let valid_next: u16 = kani::any();
    kani::assume(valid_next < node_count);

    PcHarnessInput {
        node_count,
        step,
        next_mode,
        valid_next,
    }
}

fn generated_workflow(input: PcHarnessInput) -> CompiledWorkflow {
    let mut nodes: Vec<CompiledNode> = Vec::with_capacity(usize::from(input.node_count));
    let mut index: u16 = 0;
    while index < input.node_count {
        let next = if index == input.step {
            match input.next_mode {
                0 => None,
                1 => Some(StepIdx::new(input.valid_next)),
                _ => Some(StepIdx::new(input.node_count)),
            }
        } else {
            None
        };
        nodes.push(CompiledNode {
            id: StepIdx::new(index),
            output: Some(SlotIdx::ZERO),
            next,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::ZERO,
            },
        });
        index = match index.checked_add(1) {
            Some(value) => value,
            None => input.node_count,
        };
    }

    CompiledWorkflow::kani_from_parts_unchecked(WorkflowParts {
        name: Box::from("h"),
        digest: WorkflowDigest::from_bytes([0x2F; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::from([]),
    })
}

#[kani::proof]
#[kani::unwind(8)]
fn vb_god2f_validate_input_bytes_contract_boundaries() {
    let input_slot_count: u16 = kani::any();
    let max_input_bytes: u32 = kani::any();
    let contract = match generated_contract(input_slot_count, max_input_bytes) {
        Some(value) => value,
        None => {
            kani::assert(false, "HVR-PO-RUNTIME-001 static action name must be valid");
            return;
        }
    };
    let input = generated_action_input(contract.id, StepIdx::ZERO);
    let result = crate::action::dispatch_generic(&input, &contract);
    let should_reject = max_input_bytes == 0 && input_slot_count > 0;

    kani::cover!(
        should_reject,
        "HVR-PO-RUNTIME-001 covers zero-byte rejection"
    );
    kani::cover!(
        !should_reject,
        "HVR-PO-RUNTIME-001 covers accepted metadata"
    );

    match (should_reject, result) {
        (
            true,
            Err(ActionError::PayloadTooLarge {
                max_bytes,
                actual_bytes,
            }),
        ) => {
            kani::assert(max_bytes == 0, "PayloadTooLarge max is exact");
            kani::assert(
                actual_bytes == 0,
                "PayloadTooLarge actual is structural zero",
            );
        }
        (true, _) => {
            kani::assert(
                false,
                "positive input slots with zero max bytes reject exactly",
            );
        }
        (false, Ok(ActionOutcome::Suspended(ticket))) => {
            kani::assert(ticket.run == input.run, "dispatch preserves run id");
            kani::assert(ticket.step == input.step, "dispatch preserves step id");
            kani::assert(
                ticket.seq == input.ticket.seq,
                "dispatch preserves ticket seq",
            );
            kani::assert(
                ticket.action == input.action,
                "dispatch preserves action id",
            );
            kani::assert(
                ticket.attempt == input.ticket.attempt,
                "dispatch preserves attempt",
            );
            kani::assert(
                ticket.capacity == 1,
                "dispatch assigns structural capacity one",
            );
        }
        (false, _) => {
            kani::assert(
                false,
                "accepted metadata suspends through production dispatch",
            );
        }
    }
}

#[kani::proof]
#[kani::unwind(12)]
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
            matches!(result, Err(RuntimeError::InvalidActionCompletion)),
            "missing node returns InvalidActionCompletion",
        );
        kani::assert(
            state.frame.pc() == before_pc,
            "missing node leaves PC unchanged",
        );
    } else if input.next_mode == 0 {
        kani::assert(result.is_ok(), "terminal completion succeeds");
        kani::assert(
            state.frame.pc() == before_pc,
            "terminal node leaves PC unchanged",
        );
    } else if input.next_mode == 1 {
        kani::assert(result.is_ok(), "valid next completion succeeds");
        kani::assert(
            state.frame.pc() == StepIdx::new(input.valid_next),
            "valid next moves PC to generated target",
        );
    } else {
        kani::assert(
            matches!(result, Err(RuntimeError::InvalidActionCompletion)),
            "invalid next returns InvalidActionCompletion",
        );
        kani::assert(
            state.frame.pc() == before_pc,
            "invalid next leaves PC unchanged",
        );
    }
    std::mem::forget(state);
}

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
