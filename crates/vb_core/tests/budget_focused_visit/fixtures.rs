use vb_core::ids::{ActionId, SlotIdx, StepIdx};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledRetryPolicy, ErrorRouting, NodeContinuation,
    NodeOutput, NodeRouting, ResourceContract, SecretResultPolicy,
};

pub(super) fn node(kind: CompiledNodeKind, position: u16, malformed: bool) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(if malformed { u16::MAX } else { position }),
        output: if malformed {
            NodeOutput::Slot(SlotIdx::new(u16::MAX))
        } else {
            NodeOutput::Discard
        },
        routing: NodeRouting {
            next: if malformed {
                NodeContinuation::Step(StepIdx::new(u16::MAX))
            } else {
                NodeContinuation::End
            },
            error: if malformed {
                ErrorRouting::Handler {
                    handler: StepIdx::new(u16::MAX),
                    error_slot: Some(SlotIdx::new(u16::MAX)),
                }
            } else {
                ErrorRouting::None
            },
        },
        retry: if malformed {
            Some(CompiledRetryPolicy {
                max_attempts: u16::MAX,
                delay_ms: u64::MAX,
            })
        } else {
            None
        },
        kind,
    }
}

pub(super) fn words(malformed: bool) -> Vec<Vec<CompiledNode>> {
    let kinds = [
        CompiledNodeKind::Nop,
        CompiledNodeKind::Do {
            action: ActionId::new(u16::MAX),
            input: SlotIdx::new(u16::MAX),
        },
        CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::new(u16::MAX),
        },
        CompiledNodeKind::WaitEvent {
            event: SlotIdx::new(u16::MAX),
            timeout_slot: Some(SlotIdx::new(u16::MAX)),
        },
        CompiledNodeKind::Ask {
            prompt: SlotIdx::new(u16::MAX),
            timeout_slot: None,
        },
        CompiledNodeKind::Finish {
            result: SlotIdx::new(u16::MAX),
        },
    ];
    let mut words = vec![Vec::new()];
    for first in &kinds {
        words.push(vec![node(first.clone(), 0, malformed)]);
        for second in &kinds {
            words.push(vec![
                node(first.clone(), 0, malformed),
                node(second.clone(), 1, malformed),
            ]);
        }
    }
    words
}

pub(super) fn contract(high: bool) -> ResourceContract {
    let byte = if high { u8::MAX } else { 0 };
    let short = if high { u16::MAX } else { 0 };
    let word = if high { u32::MAX } else { 0 };
    let wide = if high { u64::MAX } else { 0 };
    ResourceContract {
        max_steps: short,
        max_slots: short,
        max_constants: short,
        max_accessors: short,
        max_expressions: short,
        max_expr_stack: byte,
        max_step_budget_per_tick: wide,
        max_transitions_per_tick: wide,
        max_input_bytes: word,
        max_output_bytes: word,
        max_blob_bytes: wide,
        max_ipc_payload_bytes: word,
        max_retry_attempts: short,
        max_fanout: short,
        max_collect_items: word,
        max_queue_depth: word,
        max_journal_batch_bytes: word,
        allows_secret_results: if high {
            SecretResultPolicy::Allow
        } else {
            SecretResultPolicy::Deny
        },
    }
}
