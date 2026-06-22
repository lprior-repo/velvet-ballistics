#![cfg(test)]
#![forbid(unsafe_code)]

//! HVR-PO-RUNTIME-{003,004,005}: generated executable properties for
//! action-completion attempt counters, input-byte contract metadata, and PC moves.

use proptest::prelude::*;
use proptest::strategy::Strategy;
use std::vec::Vec;

use vb_core::action::{
    ActionContract, ActionError, ActionInput, ActionName, ActionOutcome, ActionTicket, Idempotency,
    MockMarker, RetrySafety, SideEffect,
};
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::limits::MAX_INPUT_BYTES;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};

use crate::RuntimeError;
use crate::shard::helpers::action::scheduled_attempt_after;
use crate::shard::helpers::{
    advance_after_action_completion, make_run_state, record_scheduled_attempt,
};

const HVR_RUN_ID: RunId = RunId::new(1);

#[derive(Debug, Clone, Copy)]
enum NextCase {
    Terminal,
    Valid { next: u16 },
    Invalid,
}

fn expected_scheduled_after(current: Option<u16>, incoming: u16) -> Option<u16> {
    match (current, incoming) {
        (value, 0) => value,
        (None, value) => Some(value),
        (Some(existing), value) if existing == 0 || value > existing => Some(value),
        (Some(existing), _) => Some(existing),
    }
}

fn generated_contract(
    input_slot_count: u16,
    max_input_bytes: u32,
) -> Result<ActionContract, TestCaseError> {
    let name = match ActionName::new("vb_god2f_action") {
        Ok(value) => value,
        Err(error) => {
            return Err(TestCaseError::fail(format!(
                "HVR-PO-RUNTIME-004 static action name failed validation: {error:?}"
            )));
        }
    };
    Ok(ActionContract {
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

fn generated_action_input(action: ActionId, step: StepIdx, seq: u64, attempt: u16) -> ActionInput {
    ActionInput {
        run: HVR_RUN_ID,
        step,
        action,
        input: SlotIdx::ZERO,
        ticket: ActionTicket {
            run: HVR_RUN_ID,
            step,
            seq: SeqNo::new(seq),
            action,
            attempt,
            idempotency_key: 0xA5A5,
            capacity: 1,
            mock: MockMarker::HttpGet,
        },
    }
}

fn pc_case_strategy() -> impl Strategy<Value = (u16, u16, NextCase)> {
    (1u16..=32).prop_flat_map(|node_count| {
        (
            Just(node_count),
            0u16..=node_count,
            0u8..3,
            0u16..node_count,
        )
            .prop_map(|(count, step, mode, next)| {
                let next_case = match mode {
                    0 => NextCase::Terminal,
                    1 => NextCase::Valid { next },
                    _ => NextCase::Invalid,
                };
                (count, step, next_case)
            })
    })
}

fn generated_workflow(node_count: u16, step: u16, next_case: NextCase) -> CompiledWorkflow {
    let mut nodes = Vec::new();
    let mut index: u16 = 0;
    while index < node_count {
        let next = if index == step {
            match next_case {
                NextCase::Terminal => None,
                NextCase::Valid { next } => Some(StepIdx::new(next)),
                NextCase::Invalid => Some(StepIdx::new(node_count)),
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
            None => node_count,
        };
    }

    CompiledWorkflow::from_parts_unchecked(WorkflowParts {
        name: Box::from("hvr_po_runtime_pc"),
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

proptest! {
    #[test]
    fn vb_god2f_action_completion_attempt_properties(
        node_count in 1u16..=32,
        step in 0u16..=32,
        current in any::<u16>(),
        incoming in any::<u16>(),
    ) {
        prop_assume!(step <= node_count);
        let workflow = generated_workflow(node_count, 0, NextCase::Terminal);
        let mut state = match make_run_state(workflow, HVR_RUN_ID) {
            Some(value) => value,
            None => return Err(TestCaseError::fail("HVR-PO-RUNTIME-003 generated RunState failed")),
        };

        if let Some(slot) = state.action_attempts.get_mut(usize::from(step)) {
            *slot = current;
        }
        let before = state.action_attempts.clone();
        let ticket = ActionTicket {
            run: HVR_RUN_ID,
            step: StepIdx::new(step),
            seq: SeqNo::ZERO,
            action: ActionId::new(1),
            attempt: incoming,
            idempotency_key: 0,
            capacity: u16::MAX,
            mock: MockMarker::HttpGet,
        };

        record_scheduled_attempt(&mut state, ticket);
        let production_kernel = scheduled_attempt_after(Some(current), incoming);
        let independent_expected = expected_scheduled_after(Some(current), incoming);
        prop_assert_eq!(production_kernel, independent_expected);

        if usize::from(step) >= before.len() {
            prop_assert_eq!(state.action_attempts.len(), before.len());
        } else {
            prop_assert_eq!(
                state.action_attempts.get(usize::from(step)).copied(),
                independent_expected
            );
        }
    }

    #[test]
    fn vb_god2f_action_completion_input_properties(
        input_slot_count in any::<u16>(),
        max_input_bytes in any::<u32>(),
        seq in any::<u64>(),
        attempt in any::<u16>(),
    ) {
        let contract = generated_contract(input_slot_count, max_input_bytes)?;
        let input = generated_action_input(contract.id, StepIdx::ZERO, seq, attempt);
        let result = crate::action::dispatch_generic(&input, &contract);
        // HVR-PO-RUNTIME-004 (vb-c34qm): construction-time enforcement requires
        // the declared limit to satisfy 0 < max_input_bytes <= MAX_INPUT_BYTES.
        // Zero is a placeholder and the sentinel u32::MAX would defeat any
        // boundary check, so both are rejected before a ticket is issued.
        let expected_zero = ActionError::PayloadTooLarge {
            max_bytes: 0,
            actual_bytes: 0,
        };
        let expected_overflow = ActionError::PayloadTooLarge {
            max_bytes: MAX_INPUT_BYTES,
            actual_bytes: max_input_bytes,
        };

        if max_input_bytes == 0 {
            prop_assert_eq!(result, Err(expected_zero));
        } else if max_input_bytes > MAX_INPUT_BYTES {
            prop_assert_eq!(result, Err(expected_overflow));
        } else {
            match result {
                Ok(ActionOutcome::Suspended(ticket)) => {
                    prop_assert_eq!(ticket.run, input.run);
                    prop_assert_eq!(ticket.step, input.step);
                    prop_assert_eq!(ticket.seq, input.ticket.seq);
                    prop_assert_eq!(ticket.action, input.action);
                    prop_assert_eq!(ticket.attempt, input.ticket.attempt);
                    prop_assert_eq!(ticket.capacity, 1);
                }
                other => {
                    return Err(TestCaseError::fail(format!(
                        "HVR-PO-RUNTIME-004 expected suspended outcome, got {other:?}"
                    )));
                }
            }
        }
    }

    #[test]
    fn vb_god2f_action_completion_pc_properties((node_count, step, next_case) in pc_case_strategy()) {
        let workflow = generated_workflow(node_count, step, next_case);
        let mut state = match make_run_state(workflow, HVR_RUN_ID) {
            Some(value) => value,
            None => return Err(TestCaseError::fail("HVR-PO-RUNTIME-005 generated RunState failed")),
        };
        let before = state.frame.pc();
        let result = advance_after_action_completion(&mut state, StepIdx::new(step));

        if step == node_count {
            prop_assert!(matches!(result, Err(RuntimeError::InvalidActionCompletion)));
            prop_assert_eq!(state.frame.pc(), before);
        } else {
            match next_case {
                NextCase::Terminal => {
                    prop_assert_eq!(result, Ok(()));
                    prop_assert_eq!(state.frame.pc(), before);
                }
                NextCase::Valid { next } => {
                    prop_assert_eq!(result, Ok(()));
                    prop_assert_eq!(state.frame.pc(), StepIdx::new(next));
                }
                NextCase::Invalid => {
                    prop_assert!(matches!(result, Err(RuntimeError::InvalidActionCompletion)));
                    prop_assert_eq!(state.frame.pc(), before);
                }
            }
        }
    }
}
