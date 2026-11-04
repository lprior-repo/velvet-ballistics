//! Fuzz target for compiled IR using the `arbitrary` crate to generate
//! structured WorkflowParts inputs with controlled boundary violations.

#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue, ResourceContract,
    SlotIdx, StepIdx, WorkflowDigest, WorkflowParts,
};
use vb_core::workflow::{
    ErrorRouting, NodeContinuation, NodeOutput, NodeRouting, SecretResultPolicy,
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
    let slot_count = (case.slot_count % 16) + 1;
    let node_count = (case.node_count % 8) + 1;

    let slot_refs: Vec<SlotIdx> = case
        .slot_refs
        .iter()
        .take(8)
        .map(|&b| SlotIdx::new(u16::from(b)))
        .collect();

    let mut nodes = Vec::with_capacity(node_count as usize);

    for i in 0..node_count {
        let step = StepIdx::new(i as u16);
        let node = match i {
            0 => {
                let const_idx = ConstIdx::new(0);
                let output = NodeOutput::Slot(slot_refs[0]);
                let routing = NodeRouting {
                    next: NodeContinuation::Step(StepIdx::new(1)),
                    error: ErrorRouting::None,
                };
                CompiledNode {
                    id: step,
                    output,
                    routing,
                    kind: CompiledNodeKind::SetConst { value: const_idx },
                }
            }
            _ if i < node_count - 1 => {
                let source = slot_refs[i as usize % slot_refs.len()];
                let output = NodeOutput::Slot(slot_refs[(i + 1) as usize % slot_refs.len()]);
                let routing = NodeRouting {
                    next: NodeContinuation::Step(StepIdx::new(i as u16 + 1)),
                    error: ErrorRouting::None,
                };
                CompiledNode {
                    id: step,
                    output,
                    routing,
                    kind: CompiledNodeKind::Copy { source },
                }
            }
            _ => {
                let result = slot_refs[i as usize % slot_refs.len()];
                let output = NodeOutput::Slot(result);
                let routing = NodeRouting {
                    next: NodeContinuation::End,
                    error: ErrorRouting::None,
                };
                CompiledNode {
                    id: step,
                    output,
                    routing,
                    kind: CompiledNodeKind::Finish { result },
                }
            }
        };
        nodes.push(node);
    }

    let constants = vec![ConstValue::I64(0)];

    let parts = WorkflowParts {
        name: "fuzz_arbitrary".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        nodes: nodes.clone().into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: constants.into_boxed_slice(),
        slot_count: slot_count as u16,
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
        step_names: nodes
            .iter()
            .enumerate()
            .map(|(i, _)| format!("step_{i}").into())
            .collect::<Vec<_>>()
            .into_boxed_slice(),
        input_slots: Box::new([]),
    };

    let result = CompiledWorkflow::try_from_parts(parts);

    match result {
        Ok(workflow) => {
            assert!(workflow.node_count() >= 1, "node count must be >= 1");
            assert_eq!(workflow.slot_count(), slot_count, "slot count must be preserved");

            for i in 0..workflow.node_count() {
                let step = StepIdx::new(i);
                let node = match workflow.node(step) {
                    Some(n) => n,
                    None => panic!("node {} must exist", i),
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
                    CompiledNodeKind::SetConst { .. }
                    | CompiledNodeKind::Nop
                    | CompiledNodeKind::EvalExpr { .. }
                    | CompiledNodeKind::BuildObject { .. }
                    | CompiledNodeKind::BuildList { .. }
                    | CompiledNodeKind::Do { .. }
                    | CompiledNodeKind::Choose { .. }
                    | CompiledNodeKind::ChooseSlot { .. }
                    | CompiledNodeKind::ForEachStart { .. }
                    | CompiledNodeKind::ForEachNext { .. }
                    | CompiledNodeKind::ForEachJoin { .. }
                    | CompiledNodeKind::TogetherStart { .. }
                    | CompiledNodeKind::TogetherBranch { .. }
                    | CompiledNodeKind::TogetherJoin { .. }
                    | CompiledNodeKind::CollectStart { .. }
                    | CompiledNodeKind::CollectPage { .. }
                    | CompiledNodeKind::CollectNext { .. }
                    | CompiledNodeKind::CollectFinish { .. }
                    | CompiledNodeKind::ReduceStart { .. }
                    | CompiledNodeKind::ReduceNext { .. }
                    | CompiledNodeKind::ReduceFinish { .. }
                    | CompiledNodeKind::RepeatStart { .. }
                    | CompiledNodeKind::RepeatAttempt { .. }
                    | CompiledNodeKind::RepeatCheck { .. }
                    | CompiledNodeKind::RepeatFinish { .. }
                    | CompiledNodeKind::WaitUntil { .. }
                    | CompiledNodeKind::WaitEvent { .. }
                    | CompiledNodeKind::Ask { .. }
                    | CompiledNodeKind::AskResume { .. }
                    | CompiledNodeKind::RetryCheck { .. }
                    | CompiledNodeKind::ErrorHandler { .. }
                    | CompiledNodeKind::Jump { .. }
                    | _ => {}
                }
            }
        }
        Err(e) => {
            let _ = format!("{:?}", e);
        }
    }
});
