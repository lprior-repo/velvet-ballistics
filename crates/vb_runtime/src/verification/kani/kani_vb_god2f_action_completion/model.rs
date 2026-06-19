#![forbid(unsafe_code)]

use std::vec::Vec;

use vb_core::action::{
    ActionContract, ActionInput, ActionName, ActionTicket, Idempotency, MockMarker, RetrySafety,
    SideEffect,
};
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};

use crate::RuntimeError;

pub(super) const HVR_RUNTIME_RUN_ID: RunId = RunId::new(1);

#[derive(Clone, Copy)]
pub(super) enum CompletionResultKind {
    Ok,
    InvalidActionCompletion,
    OtherErr,
}

#[derive(Clone, Copy)]
pub(super) struct PcHarnessInput {
    pub(super) node_count: u16,
    pub(super) step: u16,
    pub(super) next_mode: u8,
    pub(super) valid_next: u16,
}

pub(super) fn generated_contract(
    input_slot_count: u16,
    max_input_bytes: u32,
) -> Option<ActionContract> {
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

pub(super) fn generated_action_input(action: ActionId, step: StepIdx) -> ActionInput {
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

pub(super) fn generated_pc_input() -> PcHarnessInput {
    let node_count: u16 = kani::any();
    kani::assume(node_count >= 1);
    kani::assume(node_count <= 2);

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

pub(super) fn completion_result_kind(result: &Result<(), RuntimeError>) -> CompletionResultKind {
    match result {
        Ok(()) => CompletionResultKind::Ok,
        Err(RuntimeError::InvalidActionCompletion) => CompletionResultKind::InvalidActionCompletion,
        Err(_) => CompletionResultKind::OtherErr,
    }
}

pub(super) fn completion_result_is_ok(kind: CompletionResultKind) -> bool {
    matches!(kind, CompletionResultKind::Ok)
}

pub(super) fn completion_result_is_invalid_action_completion(kind: CompletionResultKind) -> bool {
    matches!(kind, CompletionResultKind::InvalidActionCompletion)
}

pub(super) fn generated_workflow(input: PcHarnessInput) -> CompiledWorkflow {
    let mut nodes: Vec<CompiledNode> = Vec::with_capacity(usize::from(input.node_count));
    let mut index: u16 = 0;
    while index < input.node_count {
        nodes.push(generated_node(input, index));
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

fn generated_node(input: PcHarnessInput, index: u16) -> CompiledNode {
    let next = if index == input.step {
        match input.next_mode {
            0 => None,
            1 => Some(StepIdx::new(input.valid_next)),
            _ => Some(StepIdx::new(input.node_count)),
        }
    } else {
        None
    };
    CompiledNode {
        id: StepIdx::new(index),
        output: Some(SlotIdx::ZERO),
        next,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(1),
            input: SlotIdx::ZERO,
        },
    }
}
