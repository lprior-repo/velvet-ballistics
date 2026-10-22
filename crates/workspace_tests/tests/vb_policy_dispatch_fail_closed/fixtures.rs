//! Fixtures and helpers for policy dispatch fail-closed tests.

use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};

use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx};
use vb_core::policy::{
    CompiledNodeKind, PolicyDigest, PolicyLayer, PolicyRule, DecisionCode,
};
use vb_core::value::{SlotValue, Taint};
use vb_runtime::admission::{RunAdmission, CapabilitySet, Capability};
use vb_runtime::{
    Runtime, RuntimePolicy, ShardConfig, StorageRuntimeJournal,
};
use vb_storage::{FjallConfig, FjallJournal};
use vb_storage::executor::{ExecutorCatalog, RecoveryClass};

#[path = "test_journal.rs"]
mod test_journal;
pub use test_journal::FailingPolicyPortJournal;

fn shard_count(value: usize) -> Result<NonZeroUsize, String> {
    NonZeroUsize::new(value).ok_or_else(|| format!("invalid shard count {value}"))
}

pub fn strict_config() -> ShardConfig {
    ShardConfig {
        policy: RuntimePolicy::Strict,
        ..ShardConfig::default()
    }
}

pub fn node(
    id: u16,
    output: Option<u16>,
    next: Option<u16>,
    kind: CompiledNodeKind,
) -> vb_core::CompiledNode {
    vb_core::CompiledNode {
        id: StepIdx::new(id),
        output: output.map(SlotIdx::new),
        next: next.map(StepIdx::new),
        kind,
    }
}

pub fn action_then_finish_workflow() -> Result<vb_core::CompiledWorkflow, String> {
    let nodes = Box::from([
        node(0, Some(0), Some(1), CompiledNodeKind::Do),
        node(1, None, None, CompiledNodeKind::Finish),
    ]);
    let workflow = vb_core::CompiledWorkflow::build(
        StepIdx::new(0),
        nodes,
        Box::from([]),
    );
    match workflow {
        Ok(w) => Ok(w),
        Err(e) => Err(format!("failed to build workflow: {e:?}")),
    }
}

pub fn external_contract(action: ActionId) -> Result<vb_core::action::ActionContract, Box<dyn std::error::Error>> {
    let contract = vb_core::action::ActionContract::new(
        action,
        256,
        512,
        vb_core::action::RetryPolicy::NonRetryable,
    );
    Ok(contract)
}

pub fn scope() -> vb_runtime::admission::CapabilityScopeSpec {
    vb_runtime::admission::CapabilityScopeSpec {
        capabilities: vec![
            vb_runtime::admission::CapabilitySpec {
                name: "velvet.register.write".into(),
                action: Some(ActionId::new(0)),
            },
        ],
    }
}

pub fn action_grants(action: ActionId) -> CapabilitySet {
    CapabilitySet::from_grants(Box::from([Capability::new(
        "velvet.register.write".into(),
        action,
    )]))
}

/// Compute the expected input digest for a given value, matching the policy
/// evaluator's canonical encoding.
pub fn compute_expected_input_digest(
    value: i64,
    taint: Taint,
    ticket: &vb_core::ActionTicket,
) -> Result<PolicyDigest, String> {
    let input_bytes = postcard::to_allocvec(&SlotValue::I64(value))
        .map_err(|e| format!("failed to serialize input: {e}"))?;
    let taint_byte = vb_core::policy::encode_taint_canonical(taint)
        .ok_or_else(|| "unexpected taint variant".to_string())?;

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