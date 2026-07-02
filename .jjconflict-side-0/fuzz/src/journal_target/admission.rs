//! Admission and artifact fuzz target bodies.

use super::errors::assert_typed_journal_error;

pub fn fuzz_admission_flow(data: &[u8]) {
    if data.len() < 2 {
        return;
    }
    let Some(&byte0) = data.first() else {
        return;
    };
    let node_count = usize::from(byte0.wrapping_rem(4)).saturating_add(1);
    let slot_count = u16::from(byte0.wrapping_rem(4)).saturating_add(1);
    let max_slot = slot_count.saturating_sub(1);
    let mut nodes: Vec<vb_core::CompiledNode> = Vec::new();
    for i in 0..node_count {
        let step_idx = vb_core::StepIdx::new(u16::try_from(i).unwrap_or(0));
        let next_step = if i.saturating_add(1) < node_count {
            Some(vb_core::StepIdx::new(
                u16::try_from(i).unwrap_or(0).saturating_add(1),
            ))
        } else {
            None
        };
        nodes.push(admission_node(step_idx, next_step, max_slot));
    }
    let parts_zeroed = workflow_parts("fuzz_admission", [0u8; 32], nodes.into_boxed_slice(), slot_count);
    let Ok(hash_bytes) = postcard::to_allocvec(&parts_zeroed) else {
        return;
    };
    let computed = blake3::hash(&hash_bytes);
    let correct_parts = vb_core::WorkflowParts {
        digest: vb_core::WorkflowDigest::from_bytes(*computed.as_bytes()),
        ..parts_zeroed
    };
    let Ok(workflow) = vb_core::CompiledWorkflow::try_from_parts(correct_parts) else {
        return;
    };
    let Ok(temp_dir) = tempfile::tempdir() else {
        return;
    };
    let Ok(journal) = vb_storage::FjallJournal::open(temp_dir.path(), None) else {
        return;
    };
    submit_with_all_policies(&journal, &workflow);
    let corrupted_parts = vb_core::WorkflowParts {
        digest: vb_core::WorkflowDigest::from_bytes([0xFF; 32]),
        ..workflow.to_parts()
    };
    if let Ok(corrupted) = vb_core::CompiledWorkflow::try_from_parts(corrupted_parts) {
        let strict_result =
            vb_storage::submit_artifact(&journal, &corrupted, vb_core::RuntimePolicy::Strict);
        if let Err(error) = strict_result {
            assert_typed_journal_error(error);
        }
    }
}

fn admission_node(
    step_idx: vb_core::StepIdx,
    next_step: Option<vb_core::StepIdx>,
    max_slot: u16,
) -> vb_core::CompiledNode {
    if next_step.is_none() {
        vb_core::CompiledNode {
            id: step_idx,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: vb_core::CompiledNodeKind::Finish {
                result: vb_core::SlotIdx::new(max_slot),
            },
        }
    } else {
        vb_core::CompiledNode {
            id: step_idx,
            output: Some(vb_core::SlotIdx::new(max_slot)),
            next: next_step,
            error_slot: None,
            on_error: None,
            kind: vb_core::CompiledNodeKind::Nop,
        }
    }
}

fn workflow_parts(
    name: &'static str,
    digest: [u8; 32],
    nodes: Box<[vb_core::CompiledNode]>,
    slot_count: u16,
) -> vb_core::WorkflowParts {
    vb_core::WorkflowParts {
        name: Box::<str>::from(name),
        digest: vb_core::WorkflowDigest::from_bytes(digest),
        nodes,
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![vb_core::ConstValue::Bool(true)].into_boxed_slice(),
        slot_count,
        symbols_count: 0,
        entry: vb_core::StepIdx::ZERO,
        resource_contract: vb_core::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

fn submit_with_all_policies(journal: &vb_storage::FjallJournal, workflow: &vb_core::CompiledWorkflow) {
    for policy in [
        vb_core::RuntimePolicy::Relaxed,
        vb_core::RuntimePolicy::Journaled,
        vb_core::RuntimePolicy::Strict,
    ] {
        match vb_storage::submit_artifact(journal, workflow, policy) {
            Ok(artifact) => {
                assert!(artifact.accepted_at_seq.get() >= 1);
                assert!(artifact.verification.gate_count > 0);
                let _ = artifact.digest;
            }
            Err(error) => assert_typed_journal_error(error),
        }
    }
}

pub fn fuzz_admission_fuzz(data: &[u8]) {
    let Ok(parts) = postcard::from_bytes::<vb_core::WorkflowParts>(data) else {
        return;
    };
    let Ok(workflow) = vb_core::CompiledWorkflow::try_from_parts(parts) else {
        return;
    };
    let Ok(temp_dir) = tempfile::tempdir() else {
        return;
    };
    let Ok(journal) = vb_storage::FjallJournal::open(temp_dir.path(), None) else {
        return;
    };
    submit_with_all_policies(&journal, &workflow);
}

pub fn fuzz_strict_artifact_decoder(data: &[u8]) {
    if let Ok(artifact) = postcard::from_bytes::<vb_storage::admission::AcceptedArtifact>(data) {
        assert!(artifact.verification.gate_count > 0);
        assert!(artifact.accepted_at_seq.get() >= 1);
    }
    if let Ok(parts) = postcard::from_bytes::<vb_core::WorkflowParts>(data) {
        assert!(parts.nodes.len() <= usize::from(u16::MAX));
    }
    let artifact_decode = postcard::from_bytes::<vb_storage::admission::AcceptedArtifact>(data);
    let parts_decode = postcard::from_bytes::<vb_core::WorkflowParts>(data);
    let _ = artifact_decode.is_ok();
    let _ = parts_decode.is_ok();
}

pub fn fuzz_digest_coherence(data: &[u8]) {
    let digest_bytes: [u8; 32] = match data.get(..32).and_then(|slice| slice.try_into().ok()) {
        Some(arr) => arr,
        None => return,
    };
    let temp_dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(_) => return,
    };
    let Ok(journal) = vb_storage::FjallJournal::open(temp_dir.path(), None) else {
        return;
    };
    let nodes: Box<[vb_core::CompiledNode]> = Box::new([vb_core::CompiledNode {
        id: vb_core::StepIdx::ZERO,
        output: Some(vb_core::SlotIdx::ZERO),
        next: None,
        on_error: None,
        error_slot: None,
        kind: vb_core::CompiledNodeKind::Finish {
            result: vb_core::SlotIdx::ZERO,
        },
    }]);
    let parts = workflow_parts("fuzz_digest_test", digest_bytes, nodes.clone(), 1);
    let Ok(_workflow) = vb_core::CompiledWorkflow::try_from_parts(parts) else {
        return;
    };
    let mut reference_parts = workflow_parts("fuzz_digest_test", [0u8; 32], nodes, 1);
    if let Ok(serialized) = postcard::to_allocvec(&reference_parts) {
        let reference_digest = vb_core::WorkflowDigest::from_bytes(*blake3::hash(&serialized).as_bytes());
        reference_parts.digest = reference_digest;
        let coherent_workflow = match vb_core::CompiledWorkflow::try_from_parts(reference_parts) {
            Ok(wf) => wf,
            Err(_) => return,
        };
        let result =
            vb_storage::submit_artifact(&journal, &coherent_workflow, vb_core::RuntimePolicy::Strict);
        match result {
            Ok(artifact) => assert_eq!(artifact.digest, reference_digest),
            Err(error) => assert_typed_journal_error(error),
        }
    }
}
