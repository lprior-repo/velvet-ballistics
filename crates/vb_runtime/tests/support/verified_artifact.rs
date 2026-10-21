use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::{
    ActionId, Capability, CompiledNode, CompiledNodeKind, CompiledWorkflow, SlotIdx, StepIdx,
    WorkflowDigest,
};
use vb_storage::admission::{AcceptedArtifact, prepare_compiled_artifact};

pub(super) fn accepted_artifact_with_caps(
    caps: Box<[Capability]>,
) -> Result<AcceptedArtifact, String> {
    let mut parts = minimal_workflow()?.to_parts();
    let first = parts.nodes.first().ok_or("missing fixture node")?.clone();
    let mut nodes = vec![first];
    let mut contracts = Vec::new();
    let actions = caps
        .iter()
        .map(|cap| cap.action_id().get())
        .max()
        .map(|max| 0..=max);
    for action_id in actions.into_iter().flatten() {
        let action = ActionId::new(action_id);
        let id = u16::try_from(nodes.len()).map_err(|e| e.to_string())?;
        let next = id.checked_add(1).ok_or("fixture step overflow")?;
        let after = next.checked_add(1).ok_or("fixture step overflow")?;
        nodes.push(CompiledNode {
            id: StepIdx::new(id),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(next)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action,
                input: SlotIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(next),
            output: None,
            next: Some(StepIdx::new(after)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        });
        contracts.push(ActionContract {
            id: action,
            name: ActionName::new(format!("fixture_{}", action.get()))
                .map_err(|e| e.to_string())?,
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 64,
            max_output_bytes: 64,
            timeout_ms: 1000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: caps
                .iter()
                .filter(|cap| cap.action_id() == action)
                .cloned()
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        });
    }
    nodes.push(CompiledNode {
        id: StepIdx::new(u16::try_from(nodes.len()).map_err(|e| e.to_string())?),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    });
    parts.nodes = nodes.into_boxed_slice();
    parts.slot_count = 2;
    parts.digest = WorkflowDigest::from_bytes([0; 32]);
    parts.digest = WorkflowDigest::from_bytes(
        blake3::hash(&postcard::to_allocvec(&parts).map_err(|e| e.to_string())?).into(),
    );
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())?;
    prepare_compiled_artifact(&workflow, vb_core::RuntimePolicy::Strict, &contracts)
        .map_err(|e| e.to_string())
}

fn minimal_workflow() -> Result<vb_core::CompiledWorkflow, String> {
    use vb_core::value::ConstValue;
    use vb_core::workflow::{ResourceContract, WorkflowParts};
    use vb_core::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, SlotIdx, StepIdx,
        WorkflowDigest,
    };

    let mut parts = WorkflowParts {
        name: Box::<str>::from("test_admission"),
        semantic_source_identity: None,
        input_slots: Box::new([]),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::new([
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([ConstValue::I64(42)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    // Compute the correct BLAKE3 digest from the zeroed-digest serialization.
    let hash_bytes =
        postcard::to_allocvec(&parts).map_err(|e| format!("serialize parts for digest: {e}"))?;
    let computed = blake3::hash(&hash_bytes);
    parts.digest = WorkflowDigest::from_bytes(computed.into());

    CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
}
