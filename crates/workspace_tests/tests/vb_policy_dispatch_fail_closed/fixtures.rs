//! Fixtures and helpers for policy dispatch fail-closed tests.

use std::num::NonZeroUsize;

use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::action_pack::CapabilityScopeSpec;
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::policy::{PolicyDigest, RuntimePolicy};
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_runtime::shard::ShardConfig;

#[path = "test_journal.rs"]
mod test_journal;
pub(crate) use test_journal::FailingPolicyPortJournal;

pub(super) fn shard_count(value: usize) -> Result<NonZeroUsize, String> {
    NonZeroUsize::new(value).ok_or_else(|| format!("invalid shard count {value}"))
}

pub(crate) fn strict_config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 32,
        trace_capacity: 64,
        step_budget_per_tick: 16,
        max_active_runs: 8,
        policy: RuntimePolicy::Strict,
    }
}

pub(crate) fn node(
    id: u16,
    output: Option<u16>,
    next: Option<u16>,
    kind: CompiledNodeKind,
) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        output: output.map(SlotIdx::new),
        next: next.map(StepIdx::new),
        on_error: None,
        error_slot: None,
        kind,
    }
}

pub(crate) fn action_then_finish_workflow() -> Result<CompiledWorkflow, String> {
    let mut parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::from("vb_policy_dispatch_fail_closed"),
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
    let hash_bytes = postcard::to_allocvec(&parts).map_err(|err| err.to_string())?;
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&hash_bytes).into());
    CompiledWorkflow::try_from_parts(parts).map_err(|err| err.to_string())
}

pub(crate) fn external_contract(
    action: ActionId,
) -> Result<ActionContract, Box<dyn std::error::Error>> {
    Ok(ActionContract {
        id: action,
        name: ActionName::new("vb_policy_dispatch_fail_closed_action")?,
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

pub(crate) fn scope() -> CapabilityScopeSpec {
    CapabilityScopeSpec {
        namespace: 15,
        key: 29,
    }
}

pub(crate) fn action_grants(action: ActionId) -> CapabilitySet {
    CapabilitySet::from_grants(Box::from([Capability::new(
        "velvet.register.write".into(),
        action,
    )]))
}

/// Compute the expected input digest for a given value, matching the policy
/// evaluator's canonical encoding.
pub(crate) fn compute_expected_input_digest(
    value: i64,
    taint: Taint,
    ticket: &vb_core::action::ActionTicket,
) -> Result<PolicyDigest, String> {
    let input_bytes = postcard::to_allocvec(&SlotValue::I64(value)).map_err(|e| e.to_string())?;
    let taint_byte = match taint {
        Taint::Clean => 0,
        other => return Err(format!("unexpected taint variant: {other:?}")),
    };

    let mut hasher = blake3::Hasher::new();
    hasher.update(b"velvet-ballistics:policy:input:");
    hasher.update(&input_bytes);
    hasher.update(b"velvet-ballistics:policy:taint:");
    hasher.update(&[taint_byte]);
    hasher.update(b"velvet-ballistics:policy:ticket:");
    hasher.update(b"run:");
    hasher.update(&ticket.run.get().to_be_bytes());
    hasher.update(b"step:");
    hasher.update(&ticket.step.get().to_be_bytes());
    hasher.update(b"seq:");
    hasher.update(&ticket.seq.get().to_be_bytes());
    hasher.update(b"action:");
    hasher.update(&ticket.action.get().to_be_bytes());
    hasher.update(b"run_attempt:");
    hasher.update(&ticket.run_attempt.to_be_bytes());
    hasher.update(b"attempt:");
    hasher.update(&ticket.attempt.to_be_bytes());
    hasher.update(b"key:");
    hasher.update(&ticket.idempotency_key.to_be_bytes());
    hasher.update(b"capacity:");
    hasher.update(&ticket.capacity.to_be_bytes());

    let result = hasher.finalize();
    let mut output = [0u8; 32];
    output.copy_from_slice(result.as_bytes());
    Ok(PolicyDigest::from_bytes(output))
}
