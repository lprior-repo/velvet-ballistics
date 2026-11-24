//! Fuzz target for compiled IR using the `arbitrary` crate to generate
//! structured WorkflowParts inputs with controlled boundary violations.

#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_core::workflow::{
    ErrorRouting, NodeContinuation, NodeOutput, NodeRouting, SecretResultPolicy,
};
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue, ResourceContract,
    SlotIdx, StepIdx, WorkflowDigest, WorkflowParts,
};

/// Arbitrary structured test cases for compiled IR validation.
#[derive(Debug)]
struct CompiledIrTestCase {
    slot_count: u16,
    node_count: u8,
    slot_refs: [u8; 32],
}

impl<'a> arbitrary::Arbitrary<'a> for CompiledIrTestCase {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self {
            slot_count: u.int_in_range(0..=65535)?,
            node_count: u8::arbitrary(u)?,
            slot_refs: u.arbitrary()?,
        })
    }
}

fuzz_target!(|case: CompiledIrTestCase| {
    let slot_count = (case.slot_count % 16).saturating_add(1);
    let node_count = u16::from((case.node_count % 8).saturating_add(1));

    let slots = case
        .slot_refs
        .iter()
        .copied()
        .take(8)
        .map(|byte| SlotIdx::new(u16::from(byte)));

    let mut nodes = Vec::with_capacity(usize::from(node_count));
    for (i, (slot, next_slot)) in (0..node_count).zip(slots.clone().zip(slots.cycle().skip(1))) {
        let step = StepIdx::new(i);
        let node = match i {
            0 => {
                let const_idx = ConstIdx::new(0);
                let output = NodeOutput::Slot(slot);
                let routing = NodeRouting {
                    next: NodeContinuation::Step(StepIdx::new(1)),
                    error: ErrorRouting::None,
                };
                CompiledNode {
                    id: step,
                    output,
                    routing,
                    kind: CompiledNodeKind::SetConst { value: const_idx },
                    retry: None,
                }
            }
            _ if i < node_count.saturating_sub(1) => {
                let output = NodeOutput::Slot(next_slot);
                let routing = NodeRouting {
                    next: NodeContinuation::Step(StepIdx::new(i.saturating_add(1))),
                    error: ErrorRouting::None,
                };
                CompiledNode {
                    id: step,
                    output,
                    routing,
                    kind: CompiledNodeKind::Copy { source: slot },
                    retry: None,
                }
            }
            _ => {
                let output = NodeOutput::Slot(slot);
                let routing = NodeRouting {
                    next: NodeContinuation::End,
                    error: ErrorRouting::None,
                };
                CompiledNode {
                    id: step,
                    output,
                    routing,
                    kind: CompiledNodeKind::Finish { result: slot },
                    retry: None,
                }
            }
        };
        nodes.push(node);
    }

    let step_names = (0..node_count)
        .map(|i| format!("step_{i}").into())
        .collect();

    let parts = WorkflowParts {
        name: "fuzz_arbitrary".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([ConstValue::I64(0)]),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract {
            max_steps: u16::MAX,
            max_slots: u16::MAX,
            max_constants: u16::MAX,
            max_accessors: u16::MAX,
            max_expressions: u16::MAX,
            max_expr_stack: u8::MAX,
            max_step_budget_per_tick: u64::MAX,
            max_transitions_per_tick: u64::MAX,
            max_input_bytes: u32::MAX,
            max_output_bytes: u32::MAX,
            max_blob_bytes: u64::MAX,
            max_ipc_payload_bytes: u32::MAX,
            max_retry_attempts: u16::MAX,
            max_fanout: u16::MAX,
            max_collect_items: u32::MAX,
            max_queue_depth: u32::MAX,
            max_journal_batch_bytes: u32::MAX,
            allows_secret_results: SecretResultPolicy::Allow,
        },
        step_names,
        input_slots: Box::new([]),
    };

    // Rejection is expected for generated invalid parts.
    let Ok(workflow) = CompiledWorkflow::try_from_parts(parts) else {
        return;
    };
    assert!(workflow.node_count() >= 1, "node count must be >= 1");
    assert_eq!(
        workflow.slot_count(),
        slot_count,
        "slot count must be preserved"
    );

    for i in 0..workflow.node_count() {
        let node = workflow.node(StepIdx::new(i));
        assert!(node.is_some(), "node {i} must exist");
        let Some(node) = node else {
            continue;
        };

        if let Some(output) = node.output.slot() {
            assert!(
                output.get() < slot_count,
                "node {} output slot {} out of bounds (slot_count={})",
                i,
                output.get(),
                slot_count
            );
        }

        match &node.kind {
            CompiledNodeKind::Copy { source } => {
                assert!(
                    source.get() < slot_count,
                    "node {} Copy source slot {} out of bounds (slot_count={})",
                    i,
                    source.get(),
                    slot_count
                );
            }
            CompiledNodeKind::Finish { result } => {
                assert!(
                    result.get() < slot_count,
                    "node {} Finish result slot {} out of bounds (slot_count={})",
                    i,
                    result.get(),
                    slot_count
                );
            }
            _ => {}
        }
    }
});
