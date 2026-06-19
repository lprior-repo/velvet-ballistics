#![cfg(all(kani, feature = "kani-vb-god2f-proof-kernels"))]
#![forbid(unsafe_code)]

//! HVR-PO-CORE-004: production StepBudget and resource-contract replacement harness.

use std::vec::Vec;

use crate::engine::StepBudget;
use crate::ids::{ActionId, SlotIdx, StepIdx, WorkflowDigest};
use crate::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};
use crate::{BoundednessPolicy, WholeWorkflowBudget};

const HVR_PO_CORE_004_MAX_WORKFLOW_NODES: u8 = 4;

fn symbolic_contract() -> ResourceContract {
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

fn symbolic_parts(contract: ResourceContract) -> WorkflowParts {
    let node_count = symbolic_node_count();
    let nodes = symbolic_nodes(node_count);
    WorkflowParts {
        name: Box::from("hvr_po_core_resource"),
        digest: WorkflowDigest::from_bytes([0x43; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: u16::from(kani::any::<u8>() % 8),
        symbols_count: 0,
        entry: bounded_step_for_node_count(node_count),
        resource_contract: contract,
        step_names: Box::from([]),
    }
}

fn symbolic_node_count() -> u8 {
    kani::any::<u8>() % 5
}

fn symbolic_nodes(node_count: u8) -> Vec<CompiledNode> {
    let mut nodes = Vec::new();
    if node_count >= 1 {
        nodes.push(symbolic_node(0, node_count));
    }
    if node_count >= 2 {
        nodes.push(symbolic_node(1, node_count));
    }
    if node_count >= 3 {
        nodes.push(symbolic_node(2, node_count));
    }
    if node_count >= HVR_PO_CORE_004_MAX_WORKFLOW_NODES {
        nodes.push(symbolic_node(3, node_count));
    }
    nodes
}

fn symbolic_node(position: u16, node_count: u8) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(position),
        output: maybe_slot(),
        next: maybe_step(node_count),
        on_error: maybe_step(node_count),
        error_slot: maybe_slot(),
        kind: symbolic_node_kind(),
    }
}

fn symbolic_node_kind() -> CompiledNodeKind {
    match kani::any::<u8>() % 6 {
        0 => CompiledNodeKind::Nop,
        1 => CompiledNodeKind::Do {
            action: ActionId::new(kani::any()),
            input: bounded_slot(),
        },
        2 => CompiledNodeKind::WaitUntil {
            deadline_slot: bounded_slot(),
        },
        3 => CompiledNodeKind::WaitEvent {
            event: bounded_slot(),
            timeout_slot: maybe_slot(),
        },
        4 => CompiledNodeKind::Ask {
            prompt: bounded_slot(),
            timeout_slot: maybe_slot(),
        },
        _ => CompiledNodeKind::Finish {
            result: bounded_slot(),
        },
    }
}

fn maybe_step(node_count: u8) -> Option<StepIdx> {
    if kani::any::<bool>() {
        Some(bounded_step_for_node_count(node_count))
    } else {
        None
    }
}

fn bounded_step_for_node_count(node_count: u8) -> StepIdx {
    match node_count {
        0 | 1 => StepIdx::ZERO,
        2 => StepIdx::new(u16::from(kani::any::<u8>() % 2)),
        3 => StepIdx::new(u16::from(kani::any::<u8>() % 3)),
        _ => StepIdx::new(u16::from(kani::any::<u8>() % 4)),
    }
}

fn maybe_slot() -> Option<SlotIdx> {
    if kani::any::<bool>() {
        Some(bounded_slot())
    } else {
        None
    }
}

fn bounded_slot() -> SlotIdx {
    SlotIdx::new(u16::from(kani::any::<u8>() % 8))
}

#[kani::proof]
#[kani::unwind(12)]
fn vb_god2f_core_resource_budget_replacement() {
    let budget_input: u64 = kani::any();
    let mut step_budget = StepBudget::new(budget_input);
    let clamped = if budget_input > crate::limits::MAX_STEP_BUDGET {
        crate::limits::MAX_STEP_BUDGET
    } else {
        budget_input
    };
    kani::assert(
        step_budget.remaining() == clamped,
        "StepBudget::new clamps to MAX_STEP_BUDGET",
    );
    let take_result = step_budget.try_take();
    if clamped == 0 {
        kani::assert(matches!(take_result, Ok(false)), "zero budget is exhausted");
    } else {
        kani::assert(
            matches!(take_result, Ok(true)),
            "positive budget consumes one step",
        );
        kani::assert(
            step_budget.remaining() == clamped.saturating_sub(1),
            "try_take decrements clamped budget by one",
        );
    }

    let contract = symbolic_contract();
    let parts = symbolic_parts(contract);
    kani::cover!(
        parts.nodes.is_empty(),
        "workflow generator covers zero nodes"
    );
    kani::cover!(parts.nodes.len() == 1, "workflow generator covers one node");
    kani::cover!(
        parts.nodes.len() == 2,
        "workflow generator covers two nodes"
    );
    kani::cover!(
        parts.nodes.len() == 3,
        "workflow generator covers three nodes"
    );
    kani::cover!(
        parts.nodes.len() == 4,
        "workflow generator covers four nodes"
    );
    kani::assert(
        parts.nodes.len() <= usize::from(HVR_PO_CORE_004_MAX_WORKFLOW_NODES),
        "symbolic WorkflowParts node count respects HVR-PO-CORE-004 bound",
    );
    let validation = crate::validate_resource_contract(&parts);
    kani::cover!(
        validation.is_ok(),
        "resource contract accepted branch covered"
    );
    kani::cover!(
        validation.is_err(),
        "resource contract rejected branch covered"
    );
    if validation.is_ok() {
        kani::assert(
            parts.nodes.len() <= usize::from(contract.max_steps),
            "accepted contract admits node count",
        );
        kani::assert(
            usize::from(parts.slot_count) <= usize::from(contract.max_slots),
            "accepted contract admits slot count",
        );
        kani::assert(
            contract.max_transitions_per_tick > 0,
            "accepted contract has nonzero transition budget",
        );
        kani::assert(
            contract.max_transitions_per_tick <= crate::limits::MAX_STEP_BUDGET,
            "accepted contract respects transition hard limit",
        );
    }

    let budget_result =
        WholeWorkflowBudget::compute(parts.nodes.as_ref(), parts.entry, &parts.resource_contract);
    kani::cover!(
        budget_result.is_ok(),
        "workflow budget compute accepted branch covered"
    );
    kani::cover!(
        budget_result.is_err(),
        "workflow budget compute rejected branch covered"
    );
    if let Ok(computed) = budget_result {
        let policy_result = BoundednessPolicy::DEFAULT.validate(&computed);
        kani::cover!(
            policy_result.is_ok(),
            "boundedness policy accepted branch covered"
        );
        kani::cover!(
            policy_result.is_err(),
            "boundedness policy rejected branch covered"
        );
    }
}
