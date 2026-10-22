use blake3::Hasher;
use vb_core::ids::{ActionId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};

fn node(id: u16, output: Option<u16>, next: Option<u16>, kind: CompiledNodeKind) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        output: output.map(SlotIdx::new),
        next: next.map(StepIdx::new),
        on_error: None,
        error_slot: None,
        kind,
    }
}

/// Workflow with genuine Ask step (slot 0), AskResume (slot 1),
/// Do action (slot 1→2), Finish (slot 2). Three slots.
pub(super) fn legacy_ask_action_workflow(action: ActionId) -> Result<CompiledWorkflow, String> {
    let mut parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::from("vb_c4hzw_legacy_admission"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::from([
            node(
                0,
                None,
                Some(1),
                CompiledNodeKind::Ask {
                    prompt: SlotIdx::ZERO,
                    timeout_slot: None,
                },
            ),
            node(
                1,
                None,
                Some(2),
                CompiledNodeKind::AskResume {
                    answer: SlotIdx::new(1),
                },
            ),
            node(
                2,
                Some(2),
                Some(3),
                CompiledNodeKind::Do {
                    action,
                    input: SlotIdx::new(1),
                },
            ),
            node(
                3,
                None,
                None,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
            ),
        ]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };

    let mut hasher = Hasher::new();
    let parts_bytes = postcard::to_allocvec(&parts).map_err(|err| err.to_string())?;
    hasher.update(&parts_bytes);
    let result = hasher.finalize();
    let mut digest_bytes = [0u8; 32];
    digest_bytes.copy_from_slice(result.as_bytes());
    parts.digest = WorkflowDigest::from_bytes(digest_bytes);

    CompiledWorkflow::try_from_parts(parts).map_err(|err| err.to_string())
}
