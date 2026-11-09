use crate::common::workflow_fixture::node;
use std::num::NonZeroUsize;

use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::action_pack::CapabilityScopeSpec;
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts};

pub(super) fn shard_count(value: usize) -> Result<NonZeroUsize, String> {
    NonZeroUsize::new(value).ok_or_else(|| format!("invalid shard count {value}"))
}

pub(super) fn build_workflow() -> Result<CompiledWorkflow, String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::from("vb_c4hzw_emergency_reopen"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::from([
            node(
                0,
                Some(1),
                Some(1),
                CompiledNodeKind::Do {
                    action: ActionId::new(0),
                    input: SlotIdx::ZERO,
                },
            ),
            node(
                1,
                None,
                None,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
            ),
        ]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    let hash = postcard::to_allocvec(&parts).map_err(|e| e.to_string())?;
    let mut parts = parts;
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&hash).into());
    CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
}

pub(super) fn build_contract(
    action: ActionId,
) -> Result<ActionContract, Box<dyn std::error::Error>> {
    Ok(ActionContract {
        id: action,
        name: ActionName::new("vb_c4hzw_emergency_reopen_action")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 64,
        max_output_bytes: 64,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::new([Capability::new("velvet.register.write".into(), action)]),
    })
}

pub(super) fn scope() -> CapabilityScopeSpec {
    CapabilityScopeSpec {
        namespace: 15,
        key: 29,
    }
}

pub(super) fn action_grants(action: ActionId) -> CapabilitySet {
    CapabilitySet::from_grants(Box::from([Capability::new(
        "velvet.register.write".into(),
        action,
    )]))
}
