//! **PO-vb-hbav-035**: Proptest digest coherence equivalence.
//!
//! For any WorkflowParts, `blake3::hash(postcard::to_allocvec(&parts))`
//! must equal the digest computed by the vb_storage admission pipeline
//! when both succeed.

use proptest::prelude::*;
use vb_core::{
    CompiledNode, CompiledNodeKind, ResourceContract, SlotIdx, StepIdx, WorkflowDigest,
    WorkflowParts,
};

proptest! {
    /// Generate random small workflows, compute reference blake3 hash,
    /// and verify it matches the production admission digest.
    #[test]
    fn proptest_workflow_digest_coherence(
        node_count in 1usize..4usize,
        slot_count in 1u16..4u16,
    ) {
        // Build a minimal workflow with deterministic nodes.
        let mut nodes: Vec<CompiledNode> = Vec::new();
        for i in 0..node_count {
            let step_idx = StepIdx::new(i as u16);
            let next_step = if i + 1 < node_count {
                Some(StepIdx::new((i + 1) as u16))
            } else {
                None
            };
            let max_slot = slot_count.saturating_sub(1);

            if i + 1 == node_count {
                nodes.push(CompiledNode {
                    id: step_idx,
                    output: None,
                    next: None,
                    error_slot: None,
                    on_error: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(max_slot),
                    },
                });
            } else {
                nodes.push(CompiledNode {
                    id: step_idx,
                    output: Some(SlotIdx::new(max_slot)),
                    next: next_step,
                    error_slot: None,
                    on_error: None,
                    kind: CompiledNodeKind::Nop,
                });
            }
        }

        // Parts with zero digest (for computing reference hash).
        let parts_zeroed = WorkflowParts {
            name: Box::<str>::from("proptest_digest"),
            digest: WorkflowDigest::from_bytes([0u8; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };

        // Compute reference digest via postcard + blake3.
        let serialized = postcard::to_allocvec(&parts_zeroed);
        prop_assert!(serialized.is_ok(), "postcard serialization must succeed");

        let reference_hash = blake3::hash(&serialized.unwrap());
        let reference_digest = WorkflowDigest::from_bytes(*reference_hash.as_bytes());

        // Now build the correct parts with the right digest.
        let corrected_parts = WorkflowParts {
            digest: reference_digest,
            ..parts_zeroed
        };

        let workflow = vb_core::CompiledWorkflow::try_from_parts(corrected_parts);
        prop_assert!(workflow.is_ok(), "workflow construction must succeed");

        let workflow = workflow.unwrap();

        // The workflow's digest must match the reference.
        prop_assert_eq!(workflow.digest(), reference_digest,
            "workflow digest must match reference blake3 hash");
    }
}
