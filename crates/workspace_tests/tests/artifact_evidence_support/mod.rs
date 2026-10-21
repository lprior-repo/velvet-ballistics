#![forbid(unsafe_code)]

use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::{
    ActionId, CompiledNode, CompiledNodeKind, CompiledWorkflow, SlotIdx, StepIdx, WorkflowDigest,
    WorkflowParts,
};

pub(super) const SOURCE: &[u8] = b"version: velvet-ballistics/v1\nname: artifact_evidence\nwhen: { manual: {} }\nsteps:\n  - id: make\n    set: { output: answer, value: \"42\" }\n  - id: done\n    finish: { result: answer }\n";

pub(super) fn workflow() -> Result<CompiledWorkflow, Box<dyn std::error::Error>> {
    Ok(vb_compile::compile_workflow(SOURCE)?)
}

pub(super) fn canonicalize(
    mut parts: WorkflowParts,
) -> Result<CompiledWorkflow, Box<dyn std::error::Error>> {
    parts.digest = WorkflowDigest::from_bytes([0; 32]);
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&postcard::to_allocvec(&parts)?).into());
    Ok(CompiledWorkflow::try_from_parts(parts)?)
}

pub(super) fn action_workflow() -> Result<CompiledWorkflow, Box<dyn std::error::Error>> {
    let mut parts = workflow()?.to_parts();
    let first = parts.nodes.first().ok_or("missing first node")?.clone();
    let mut last = parts.nodes.last().ok_or("missing last node")?.clone();
    last.id = StepIdx::new(2);
    last.kind = CompiledNodeKind::Finish {
        result: SlotIdx::new(1),
    };
    parts.nodes = Box::new([
        first,
        CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(7),
                input: SlotIdx::new(0),
            },
        },
        last,
    ]);
    parts.slot_count = 2;
    parts.step_names = Box::new([]);
    canonicalize(parts)
}

pub(super) fn contract() -> Result<ActionContract, Box<dyn std::error::Error>> {
    Ok(ActionContract {
        id: ActionId::new(7),
        name: ActionName::new("artifact_action")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 64,
        max_output_bytes: 64,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::new([]),
    })
}

pub(super) fn rebind(
    artifact: &mut vb_storage::AcceptedArtifact,
) -> Result<(), vb_storage::JournalError> {
    artifact.digest = vb_storage::admission::accepted_artifact_digest(artifact)?;
    artifact.verification.digest = artifact.digest;
    Ok(())
}
