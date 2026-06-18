#![forbid(unsafe_code)]
#![cfg(kani)]
//! Kani proof harnesses for vb-fn4vt PO-011: policy digest binding.
//!
//! Production binding: calls `admission::compute_policy_digest`, which hashes
//! the canonical serialization of `CompiledWorkflow::resource_contract()`.

use crate::admission::compute_policy_digest;
use core::mem::ManuallyDrop;
use vb_core::{CompiledWorkflow, ResourceContract, StepIdx, WorkflowDigest, WorkflowParts};

fn symbolic_resource_contract() -> ResourceContract {
    ResourceContract {
        max_steps: kani::any(),
        max_slots: kani::any(),
        max_constants: kani::any(),
        max_accessors: kani::any(),
        max_expressions: kani::any(),
        max_expr_stack: kani::any(),
        max_step_budget_per_tick: kani::any(),
        max_transitions_per_tick: kani::any(),
        max_input_bytes: kani::any(),
        max_output_bytes: kani::any(),
        max_blob_bytes: kani::any(),
        max_ipc_payload_bytes: kani::any(),
        max_retry_attempts: kani::any(),
        max_fanout: kani::any(),
        max_collect_items: kani::any(),
        max_queue_depth: kani::any(),
        max_journal_batch_bytes: kani::any(),
        allows_secret_results: kani::any(),
    }
}

fn workflow_with_contract(resource_contract: ResourceContract) -> CompiledWorkflow {
    CompiledWorkflow::kani_from_parts_unchecked(WorkflowParts {
        name: Box::from("kani-policy-digest"),
        digest: WorkflowDigest::from_bytes(kani::any()),
        nodes: Box::default(),
        expressions: Box::default(),
        accessors: Box::default(),
        constants: Box::default(),
        slot_count: kani::any(),
        symbols_count: kani::any(),
        entry: StepIdx::new(kani::any()),
        resource_contract,
        step_names: Box::default(),
    })
}

fn policy_digest_bytes(workflow: &CompiledWorkflow) -> Option<[u8; 32]> {
    let result = ManuallyDrop::new(compute_policy_digest(workflow));
    match &*result {
        Ok(digest) => Some(digest.as_bytes()),
        Err(_) => None,
    }
}

#[kani::proof]
fn policy_digest_binding() {
    let contract = symbolic_resource_contract();
    let workflow = workflow_with_contract(contract);
    kani::assert(
        policy_digest_bytes(&workflow).is_some(),
        "policy digest serialization is total",
    );
}

#[kani::proof]
fn compute_policy_digest_no_panic() {
    let workflow = workflow_with_contract(symbolic_resource_contract());
    let _result = policy_digest_bytes(&workflow);
}

#[kani::proof]
fn policy_digest_deterministic() {
    let contract = symbolic_resource_contract();
    let workflow = workflow_with_contract(contract);

    let first = policy_digest_bytes(&workflow);
    let second = policy_digest_bytes(&workflow);

    kani::assert(
        first == second,
        "same workflow policy digest is deterministic",
    );
}
