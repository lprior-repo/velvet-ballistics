//! Blake3 digest coherence tests (B25-B27).

use super::{
    CompiledNode, CompiledNodeKind, ConstValue, ResourceContract, SlotIdx, StepIdx, WorkflowDigest,
    WorkflowParts,
};

fn make_minimal_workflow_parts(name: &str, entry: StepIdx, slot_count: u16) -> WorkflowParts {
    let digest = WorkflowDigest::from_bytes([0u8; 32]);
    WorkflowParts {
        name: name.into(),
        digest,
        nodes: Box::new([CompiledNode {
            id: entry,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
        }]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count,
        symbols_count: 0,
        entry,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

#[test]
fn blake3_digest_is_deterministic_for_identical_parts() {
    let parts1 = make_minimal_workflow_parts("alpha", StepIdx::ZERO, 1);
    let parts2 = make_minimal_workflow_parts("alpha", StepIdx::ZERO, 1);
    let bytes1 = postcard::to_allocvec(&parts1).expect("serialize should succeed");
    let bytes2 = postcard::to_allocvec(&parts2).expect("serialize should succeed");
    let hash1 = blake3::hash(&bytes1);
    let hash2 = blake3::hash(&bytes2);
    assert_eq!(
        hash1.as_bytes(),
        hash2.as_bytes(),
        "identical WorkflowParts must produce identical digests"
    );
}

#[test]
fn blake3_digest_differs_when_name_differs() {
    let parts_alpha = make_minimal_workflow_parts("alpha", StepIdx::ZERO, 1);
    let parts_beta = make_minimal_workflow_parts("beta", StepIdx::ZERO, 1);
    let bytes_alpha = postcard::to_allocvec(&parts_alpha).expect("serialize should succeed");
    let bytes_beta = postcard::to_allocvec(&parts_beta).expect("serialize should succeed");
    let hash_alpha = blake3::hash(&bytes_alpha);
    let hash_beta = blake3::hash(&bytes_beta);
    assert_ne!(
        hash_alpha.as_bytes(),
        hash_beta.as_bytes(),
        "different name must produce different digest"
    );
}

#[test]
fn blake3_digest_differs_when_node_count_differs() {
    let digest = WorkflowDigest::from_bytes([0u8; 32]);
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
    };
    let parts1 = WorkflowParts {
        name: "test".into(),
        digest,
        nodes: Box::new([node.clone()]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let node2 = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
    };
    let parts2 = WorkflowParts {
        name: "test".into(),
        digest,
        nodes: Box::new([node, node2]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let hash1 = blake3::hash(&postcard::to_allocvec(&parts1).expect("serialize should succeed"));
    let hash2 = blake3::hash(&postcard::to_allocvec(&parts2).expect("serialize should succeed"));
    assert_ne!(
        hash1.as_bytes(),
        hash2.as_bytes(),
        "different node_count must produce different digest"
    );
}

#[test]
fn blake3_digest_differs_when_entry_step_differs() {
    let parts_entry0 = make_minimal_workflow_parts("test", StepIdx::ZERO, 1);
    let parts_entry1 = make_minimal_workflow_parts("test", StepIdx::new(1), 1);
    let hash0 =
        blake3::hash(&postcard::to_allocvec(&parts_entry0).expect("serialize should succeed"));
    let hash1 =
        blake3::hash(&postcard::to_allocvec(&parts_entry1).expect("serialize should succeed"));
    assert_ne!(
        hash0.as_bytes(),
        hash1.as_bytes(),
        "different entry step must produce different digest"
    );
}

#[test]
fn blake3_digest_valid_for_zero_slot_workflow() {
    let parts = make_minimal_workflow_parts("zero_slot", StepIdx::ZERO, 0);
    let bytes = postcard::to_allocvec(&parts).expect("serialize should succeed");
    let hash = blake3::hash(&bytes);
    let hash_bytes = hash.as_bytes();
    assert_eq!(hash_bytes.len(), 32, "blake3 must produce 32-byte hash");
    assert_ne!(hash_bytes, &[0u8; 32], "hash must not be all zeros");
}
