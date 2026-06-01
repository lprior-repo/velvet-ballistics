//! WorkflowError variant and display tests.

use super::super::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstValue, ExprBranch, ExprIdx, ExprProgram,
    SlotIdx, StepIdx, WorkflowError, WorkflowParts,
};
use super::tests::{
    action_tickets_error, assert_budget_detail, assert_workflow_budget_detail, choose_expr_parts,
    choose_slot_parts, fanout_budget_parts, fanout_error, finish_const_parts_with, load,
    nesting_depth_budget_parts, nesting_depth_error, parallel_error, resource_contract,
    result_bytes_budget_parts, result_bytes_error, run_time_error, steps_executable_error,
    total_slots_error, total_steps_budget_parts, total_steps_error,
};
use crate::ids::{ConstIdx, SymbolId, WorkflowDigest};

// -- WorkflowError exact variant assertions --

#[test]
fn workflow_error_empty_nodes_exact_variant() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("empty"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::new([]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, 0, 1, 0, 0),
        step_names: Box::new([]),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::EmptyNodes) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_error_entry_out_of_bounds_exact_variant() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("entry_oob"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(5),
        resource_contract: resource_contract(1, 0, 1, 0, 0),
        step_names: Box::new([]),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::EntryOutOfBounds { entry }) if entry == StepIdx::new(5) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_error_step_out_of_bounds_exact_variant() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("step_oob"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(99)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, 0, 1, 0, 0),
        step_names: Box::new([]),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::StepOutOfBounds { step }) if step == StepIdx::new(99) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_error_slot_out_of_bounds_exact_variant() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(1, 1, 1, 0, 0), Box::new([]));
    parts.nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(5)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }]
    .into_boxed_slice();
    parts.slot_count = 1;

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(5) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_error_const_out_of_bounds_exact_variant() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(1, 0, 1, 0, 0), Box::new([]));
    parts.nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(50),
        },
    }]
    .into_boxed_slice();

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ConstOutOfBounds { constant }) if constant == ConstIdx::new(50) => {
            Ok(())
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_error_node_id_mismatch_exact_variant() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(1, 0, 1, 0, 0), Box::new([]));
    parts.nodes = vec![CompiledNode {
        id: StepIdx::new(7),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }]
    .into_boxed_slice();

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::NodeIdMismatch { expected, actual })
            if expected == StepIdx::new(0) && actual == StepIdx::new(7) =>
        {
            Ok(())
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_error_expression_wrapped_core_error_exact_variant() -> Result<(), String> {
    let expression = ExprProgram::try_from_ops(vec![load(0)].into_boxed_slice())
        .map_err(|error| error.to_string())?;

    let parts = choose_expr_parts(
        vec![ExprBranch {
            condition: ExprIdx::new(1),
            target: StepIdx::new(1),
        }]
        .into_boxed_slice(),
        Some(StepIdx::new(1)),
        vec![expression].into_boxed_slice(),
    );

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::Expression(crate::errors::CoreError::ExprOutOfBounds { expr }))
            if expr == ExprIdx::new(1) =>
        {
            Ok(())
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_error_empty_branch_table_exact_variant() -> Result<(), String> {
    let parts = choose_slot_parts(Box::new([]), None);

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::EmptyBranchTable) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// =========================================================================
// WorkflowError display and equality tests
// =========================================================================

#[test]
fn workflow_error_empty_nodes_display() {
    assert_eq!(
        WorkflowError::EmptyNodes.to_string(),
        "compiled workflow must contain at least one node"
    );
}

#[test]
fn workflow_error_empty_branch_table_display() {
    assert_eq!(
        WorkflowError::EmptyBranchTable.to_string(),
        "branch table must contain a branch or otherwise target"
    );
}

#[test]
fn workflow_error_budget_policy_exceeded_display() {
    let error = WorkflowError::BudgetPolicyExceeded {
        detail: "max_total_steps",
    };
    assert!(error.to_string().contains("max_total_steps"));
}

#[test]
fn workflow_error_step_count_overflow_exact_variant_fields() -> Result<(), String> {
    match (WorkflowError::StepCountOverflow { actual: u64::MAX }) {
        WorkflowError::StepCountOverflow { actual } if actual == u64::MAX => Ok(()),
        other => Err(format!(
            "expected exact StepCountOverflow variant, got {other:?}"
        )),
    }
}

#[test]
fn workflow_error_jump_cycle_exact_variant_fields() -> Result<(), String> {
    match (WorkflowError::JumpCycle {
        step: StepIdx::new(7),
        target: StepIdx::new(3),
    }) {
        WorkflowError::JumpCycle { step, target }
            if step == StepIdx::new(7) && target == StepIdx::new(3) =>
        {
            Ok(())
        }
        other => Err(format!("expected exact JumpCycle variant, got {other:?}")),
    }
}

#[test]
fn validate_budget_maps_every_budget_error_variant_to_exact_detail() -> Result<(), String> {
    assert_budget_detail(total_steps_error(), "max_total_steps")?;
    assert_budget_detail(total_slots_error(), "max_total_slots")?;
    assert_budget_detail(fanout_error(), "max_fanout")?;
    assert_budget_detail(nesting_depth_error(), "max_nesting_depth")?;
    assert_budget_detail(parallel_error(), "max_parallel_in_flight")?;
    assert_budget_detail(action_tickets_error(), "max_action_tickets")?;
    assert_budget_detail(run_time_error(), "max_run_time_seconds")?;
    assert_budget_detail(result_bytes_error(), "max_result_bytes")?;
    assert_budget_detail(steps_executable_error(), "max_steps_executable")?;
    Ok(())
}

#[test]
fn workflow_budget_validation_reports_total_steps_detail() -> Result<(), String> {
    assert_workflow_budget_detail(total_steps_budget_parts(), "max_total_steps")
}

#[test]
fn workflow_budget_validation_reports_fanout_detail() -> Result<(), String> {
    assert_workflow_budget_detail(fanout_budget_parts(), "max_fanout")
}

#[test]
fn workflow_budget_validation_reports_nesting_depth_detail() -> Result<(), String> {
    assert_workflow_budget_detail(nesting_depth_budget_parts(), "max_nesting_depth")
}

#[test]
fn workflow_budget_validation_reports_result_bytes_detail() -> Result<(), String> {
    assert_workflow_budget_detail(result_bytes_budget_parts(), "max_result_bytes")
}

// =========================================================================
// Additional error edge-case tests
// =========================================================================

#[test]
fn workflow_error_backward_edge_display() {
    let error = WorkflowError::BackwardEdge {
        from: StepIdx::new(1),
        to: StepIdx::new(0),
    };
    let s = error.to_string();
    assert!(
        s.contains("backward edge"),
        "expected backward edge message, got: {s}"
    );
}

#[test]
fn workflow_error_symbol_out_of_bounds_display() {
    let error = WorkflowError::SymbolOutOfBounds {
        symbol: SymbolId::new(42),
    };
    let s = error.to_string();
    assert!(s.contains("42"), "expected symbol 42 in message, got: {s}");
}

#[test]
fn workflow_error_accessor_path_too_deep_display() {
    let error = WorkflowError::AccessorPathTooDeep { depth: 5, max: 4 };
    let s = error.to_string();
    assert!(s.contains("depth"), "expected depth in message, got: {s}");
}
