// Differential proof for vb-x46ae: workflow admission and the public engine
// validator must agree on the resource-contract `TooLarge` corpus.
//
// Strategy under test:
// - engine side: `engine::validate::validate_resource_contract(&parts)`
//   (re-exported as `vb_core::validate_resource_contract`).
// - workflow side: `CompiledWorkflow::try_from_parts(parts)` (runs the private
//   `workflow::validate_resource_contract` among its admission checks).
//
// Agreement corpus: DEFAULT contract, every dimension pinned at exactly its
// hard limit, and every type-reachable dimension pushed to limit + 1.
// Documented intentional divergence (out of scope for unification):
// zero `max_transitions_per_tick` is `Exceeded` under admission but accepted
// by the bounds-only engine pre-check.

#![forbid(unsafe_code)]

use crate::engine::validate::validate_resource_contract;
use crate::ids::{SlotIdx, StepIdx, WorkflowDigest};
use crate::limits::{
    MAX_ACCESSORS, MAX_BLOB_BYTES, MAX_COLLECT_ITEMS, MAX_CONSTANTS, MAX_EXPRESSION_STACK,
    MAX_EXPRESSIONS, MAX_FANOUT, MAX_INPUT_BYTES, MAX_IPC_PAYLOAD_BYTES, MAX_JOURNAL_BATCH_BYTES,
    MAX_OUTPUT_BYTES, MAX_QUEUE_DEPTH, MAX_RETRY_ATTEMPTS, MAX_SLOTS_PER_WORKFLOW, MAX_STEP_BUDGET,
    MAX_STEPS_PER_WORKFLOW,
};
use crate::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ContractResource, ErrorRouting,
    NodeContinuation, NodeOutput, NodeRouting, ResourceContract, WorkflowError, WorkflowParts,
};

fn valid_parts() -> WorkflowParts {
    WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("test"),
        digest: WorkflowDigest::from_bytes([0x00; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: NodeOutput::Discard,
            routing: NodeRouting {
                next: NodeContinuation::End,
                error: ErrorRouting::None,
            },
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

fn engine_result(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    validate_resource_contract(parts)
}

fn workflow_result(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    match CompiledWorkflow::try_from_parts(parts.clone()) {
        Ok(_) => Ok(()),
        Err(error) => Err(error),
    }
}

fn assert_agree(parts: &WorkflowParts) {
    let engine = engine_result(parts);
    let workflow = workflow_result(parts);
    assert_eq!(
        engine, workflow,
        "validators disagree: engine={engine:?} workflow={workflow:?}"
    );
}

/// Baseline: the default contract is accepted by both validators.
#[test]
fn differential_default_contract_is_accepted_by_both() {
    let parts = valid_parts();
    assert_eq!(engine_result(&parts), Ok(()));
    assert_agree(&parts);
}

/// One dimension pinned at exactly its hard limit (every other dimension at
/// its default) is accepted by both validators. Single-dimension variation
/// keeps the downstream budget-policy check out of the picture: pinning every
/// dimension at its limit at once trips `BudgetPolicyExceeded`, which belongs
/// to a different validation layer. `u16`-capped dimensions whose type
/// maximum equals the hard limit (steps, slots, constants) use `u16::MAX`,
/// which is the limit itself.
fn assert_at_limit_agrees(mutate: fn(&mut ResourceContract)) {
    let mut parts = valid_parts();
    mutate(&mut parts.resource_contract);
    assert_eq!(engine_result(&parts), Ok(()));
    assert_agree(&parts);
}

#[test]
fn differential_at_limit_contracts_agree() {
    // Sanity: u16-capped dimensions really sit exactly on their hard limits,
    // so the at-limit corpus cannot pass vacuously on a mistyped conversion.
    assert_eq!(usize::from(u16::MAX), MAX_STEPS_PER_WORKFLOW);
    assert_eq!(usize::from(u16::MAX), MAX_SLOTS_PER_WORKFLOW);
    assert_eq!(usize::from(u16::MAX), MAX_CONSTANTS);
    assert_eq!(
        usize::from(u16::try_from(MAX_ACCESSORS).unwrap_or(u16::MAX)),
        MAX_ACCESSORS
    );
    assert_eq!(
        usize::from(u16::try_from(MAX_EXPRESSIONS).unwrap_or(u16::MAX)),
        MAX_EXPRESSIONS
    );
    assert_eq!(MAX_RETRY_ATTEMPTS, 10);
    assert_eq!(MAX_FANOUT, 256);

    assert_at_limit_agrees(|contract| {
        contract.max_steps = u16::MAX;
    });
    assert_at_limit_agrees(|contract| {
        contract.max_slots = u16::MAX;
    });
    assert_at_limit_agrees(|contract| {
        contract.max_constants = u16::MAX;
    });
    assert_at_limit_agrees(|contract| {
        contract.max_accessors = u16::try_from(MAX_ACCESSORS).unwrap_or(u16::MAX);
    });
    assert_at_limit_agrees(|contract| {
        contract.max_expressions = u16::try_from(MAX_EXPRESSIONS).unwrap_or(u16::MAX);
    });
    assert_at_limit_agrees(|contract| {
        contract.max_expr_stack = MAX_EXPRESSION_STACK;
    });
    assert_at_limit_agrees(|contract| {
        contract.max_step_budget_per_tick = MAX_STEP_BUDGET;
    });
    assert_at_limit_agrees(|contract| {
        contract.max_transitions_per_tick = MAX_STEP_BUDGET;
    });
    assert_at_limit_agrees(|contract| {
        contract.max_retry_attempts = MAX_RETRY_ATTEMPTS;
    });
    assert_at_limit_agrees(|contract| {
        contract.max_fanout = MAX_FANOUT;
    });
    assert_at_limit_agrees(|contract| {
        contract.max_collect_items = MAX_COLLECT_ITEMS;
    });
}

/// Layering boundary: the budget policy caps six payload dimensions strictly
/// below their contract hard limits, so a declaration exactly at the hard
/// limit passes the contract layer (engine: `Ok`) but is refused downstream
/// by admission (`BudgetPolicyExceeded`). These dimensions are therefore
/// outside the `TooLarge` agreement corpus by construction; the over-limit
/// tests above still prove the contract layer agrees on them, because the
/// unified check runs before the budget check in `validate_parts`.
#[test]
fn differential_at_limit_budget_governed_dims_documented() {
    use crate::budget::BudgetDimension;

    let cases: [(fn(&mut ResourceContract), BudgetDimension); 6] = [
        (
            (|contract: &mut ResourceContract| {
                contract.max_input_bytes = MAX_INPUT_BYTES;
            }) as fn(&mut ResourceContract),
            BudgetDimension::InputBytes,
        ),
        (
            (|contract: &mut ResourceContract| {
                contract.max_output_bytes = MAX_OUTPUT_BYTES;
            }) as fn(&mut ResourceContract),
            BudgetDimension::ResultBytes,
        ),
        (
            (|contract: &mut ResourceContract| {
                contract.max_blob_bytes = MAX_BLOB_BYTES;
            }) as fn(&mut ResourceContract),
            BudgetDimension::BlobBytes,
        ),
        (
            (|contract: &mut ResourceContract| {
                contract.max_ipc_payload_bytes = MAX_IPC_PAYLOAD_BYTES;
            }) as fn(&mut ResourceContract),
            BudgetDimension::IpcPayloadBytes,
        ),
        (
            (|contract: &mut ResourceContract| {
                contract.max_queue_depth = MAX_QUEUE_DEPTH;
            }) as fn(&mut ResourceContract),
            BudgetDimension::QueueDepth,
        ),
        (
            (|contract: &mut ResourceContract| {
                contract.max_journal_batch_bytes = MAX_JOURNAL_BATCH_BYTES;
            }) as fn(&mut ResourceContract),
            BudgetDimension::JournalBatchBytes,
        ),
    ];
    for (mutate, dimension) in cases {
        let mut parts = valid_parts();
        mutate(&mut parts.resource_contract);
        assert_eq!(engine_result(&parts), Ok(()));
        assert_eq!(
            workflow_result(&parts),
            Err(WorkflowError::BudgetPolicyExceeded { detail: dimension })
        );
    }
}

/// One dimension over its hard limit: both validators must report the
/// identical `TooLarge` error. `mutate` pushes exactly one dimension to
/// limit + 1 while every other dimension stays at its default.
fn assert_over_limit_agrees(resource: ContractResource, mutate: fn(&mut ResourceContract)) {
    let mut parts = valid_parts();
    mutate(&mut parts.resource_contract);
    let expected = Err(WorkflowError::ResourceContractTooLarge { resource });
    assert_eq!(engine_result(&parts), expected);
    assert_agree(&parts);
}

#[test]
fn differential_accessors_over_limit_agrees() {
    assert_over_limit_agrees(ContractResource::Accessors, |contract| {
        contract.max_accessors = u16::try_from(MAX_ACCESSORS)
            .unwrap_or(u16::MAX)
            .saturating_add(1);
    });
}

#[test]
fn differential_expressions_over_limit_agrees() {
    assert_over_limit_agrees(ContractResource::Expressions, |contract| {
        contract.max_expressions = u16::try_from(MAX_EXPRESSIONS)
            .unwrap_or(u16::MAX)
            .saturating_add(1);
    });
}

#[test]
fn differential_expr_stack_over_limit_agrees() {
    assert_over_limit_agrees(ContractResource::ExprStack, |contract| {
        contract.max_expr_stack = MAX_EXPRESSION_STACK.saturating_add(1);
    });
}

#[test]
fn differential_step_budget_over_limit_agrees() {
    assert_over_limit_agrees(ContractResource::StepBudgetPerTick, |contract| {
        contract.max_step_budget_per_tick = MAX_STEP_BUDGET.saturating_add(1);
    });
}

#[test]
fn differential_transitions_over_limit_agrees() {
    assert_over_limit_agrees(ContractResource::TransitionsPerTick, |contract| {
        contract.max_transitions_per_tick = MAX_STEP_BUDGET.saturating_add(1);
    });
}

#[test]
fn differential_input_bytes_over_limit_agrees() {
    assert_over_limit_agrees(ContractResource::InputBytes, |contract| {
        contract.max_input_bytes = MAX_INPUT_BYTES.saturating_add(1);
    });
}

#[test]
fn differential_output_bytes_over_limit_agrees() {
    assert_over_limit_agrees(ContractResource::OutputBytes, |contract| {
        contract.max_output_bytes = MAX_OUTPUT_BYTES.saturating_add(1);
    });
}

#[test]
fn differential_blob_bytes_over_limit_agrees() {
    assert_over_limit_agrees(ContractResource::BlobBytes, |contract| {
        contract.max_blob_bytes = MAX_BLOB_BYTES.saturating_add(1);
    });
}

#[test]
fn differential_ipc_payload_bytes_over_limit_agrees() {
    assert_over_limit_agrees(ContractResource::IpcPayloadBytes, |contract| {
        contract.max_ipc_payload_bytes = MAX_IPC_PAYLOAD_BYTES.saturating_add(1);
    });
}

#[test]
fn differential_retry_attempts_over_limit_agrees() {
    assert_over_limit_agrees(ContractResource::RetryAttempts, |contract| {
        contract.max_retry_attempts = MAX_RETRY_ATTEMPTS.saturating_add(1);
    });
}

#[test]
fn differential_fanout_over_limit_agrees() {
    assert_over_limit_agrees(ContractResource::Fanout, |contract| {
        contract.max_fanout = MAX_FANOUT.saturating_add(1);
    });
}

#[test]
fn differential_collect_items_over_limit_agrees() {
    assert_over_limit_agrees(ContractResource::CollectItems, |contract| {
        contract.max_collect_items = MAX_COLLECT_ITEMS.saturating_add(1);
    });
}

#[test]
fn differential_queue_depth_over_limit_agrees() {
    assert_over_limit_agrees(ContractResource::QueueDepth, |contract| {
        contract.max_queue_depth = MAX_QUEUE_DEPTH.saturating_add(1);
    });
}

#[test]
fn differential_journal_batch_bytes_over_limit_agrees() {
    assert_over_limit_agrees(ContractResource::JournalBatchBytes, |contract| {
        contract.max_journal_batch_bytes = MAX_JOURNAL_BATCH_BYTES.saturating_add(1);
    });
}

/// Documented intentional divergence: admission requires a non-zero
/// `max_transitions_per_tick` (`Exceeded` on 0) while the bounds-only engine
/// pre-check accepts 0. This semantic is out of scope for the `TooLarge`
/// unification and must keep holding after the merge.
#[test]
fn differential_zero_transitions_divergence_is_pinned() {
    let mut parts = valid_parts();
    parts.resource_contract.max_transitions_per_tick = 0;
    assert_eq!(engine_result(&parts), Ok(()));
    assert_eq!(
        workflow_result(&parts),
        Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::TransitionsPerTick,
        })
    );
}
