//! Branch-selection error-path tests.
//!
//! Exercises the error branches in `choose_expr_branch` and `choose_slot_branch`:
//! - Non-boolean expression result (GAP-ERROR-003)
//! - Empty branches with no otherwise (GAP-ERROR-004)
//! - All-branch false with no otherwise (GAP-ERROR-005)

use crate::engine::choose::{choose_expr_branch, choose_slot_branch};
use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::ids::{ConstIdx, ExprIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use crate::value::ConstValue;
use crate::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprBranch, ExprOp, ExprProgram,
    ResourceContract, SlotBranch, WorkflowParts,
};

// ---------------------------------------------------------------------------
// 3. choose_expr_branch — non-boolean result (I64 → TypeMismatch)
// ---------------------------------------------------------------------------

/// GAP-ERROR-003: When a Choose node's expression evaluates to I64 instead
/// of Bool, `choose_expr_branch` returns `EngineError::TypeMismatch`.
///
/// The expression `LoadConst(I64(42))` pushes `SlotValue::I64(42)` onto the
/// stack. The branch target checker sees a non-boolean and returns
/// `TypeMismatch { expected: "boolean", found: "number" }`.
fn i64_expr_plan() -> Result<CompiledWorkflow, String> {
    let expr =
        ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(0))].into_boxed_slice())
            .map_err(crate::WorkflowError::Expression)
            .map_err(|e| e.to_string())?;

    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("choose_i64_expr"),
        digest: WorkflowDigest::from_bytes([0xF3; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Choose {
                    branches: vec![ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(1),
                    }]
                    .into_boxed_slice(),
                    otherwise: None,
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
        ]
        .into_boxed_slice(),
        expressions: vec![expr].into_boxed_slice(),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(42)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())
}

#[test]
fn choose_expr_branch_non_bool_i64_returns_type_mismatch() -> Result<(), String> {
    let plan = i64_expr_plan()?;
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 10, 1).unwrap();
    let mut store = crate::value_store::ValueStore::new();

    let branches = vec![ExprBranch {
        condition: ExprIdx::new(0),
        target: StepIdx::new(1),
    }];

    let result = choose_expr_branch(
        &plan,
        &mut run,
        &mut store,
        &branches,
        Some(StepIdx::new(2)),
    );

    match result {
        Err(EngineError::TypeMismatch {
            expected: "boolean",
            found: "number",
        }) => Ok(()),
        other => Err(format!(
            "expected TypeMismatch(boolean, number), got {other:?}"
        ))?,
    }
}

// ---------------------------------------------------------------------------
// 4. choose_slot_branch — empty branches + no otherwise → MissingNextStep
// ---------------------------------------------------------------------------

/// GAP-ERROR-004: When a ChooseSlot node has zero branches and no `otherwise`
/// slot, `choose_slot_branch` returns `EngineError::MissingNextStep`.
///
/// The empty branches loop terminates immediately, falling through to
/// `otherwise.ok_or(EngineError::MissingNextStep { step: run.pc() })`.
#[test]
fn choose_slot_branch_empty_no_otherwise_returns_missing_next_step() -> Result<(), String> {
    let branches: Vec<SlotBranch> = vec![];

    let result = choose_slot_branch(
        &mut RunFrame::new(RunId::new(1), StepIdx::new(0), 10, 1).unwrap(),
        &branches,
        None,
    );

    match result {
        Err(EngineError::MissingNextStep { step: _ }) => Ok(()),
        other => Err(format!("expected MissingNextStep, got {other:?}"))?,
    }
}

// ---------------------------------------------------------------------------
// 5. choose_expr_branch — all false + no otherwise → MissingNextStep
// ---------------------------------------------------------------------------

/// GAP-ERROR-005: When every branch in a Choose node evaluates to false and
/// there is no `otherwise`, `choose_expr_branch` returns
/// `EngineError::MissingNextStep`.
fn all_false_expr_plan() -> Result<CompiledWorkflow, String> {
    // Two expressions, both loading Bool(false)
    let expr0 =
        ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(0))].into_boxed_slice())
            .map_err(crate::WorkflowError::Expression)
            .map_err(|e| e.to_string())?;
    let expr1 =
        ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(1))].into_boxed_slice())
            .map_err(crate::WorkflowError::Expression)
            .map_err(|e| e.to_string())?;

    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("choose_all_false"),
        digest: WorkflowDigest::from_bytes([0xF5; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Choose {
                    branches: vec![
                        ExprBranch {
                            condition: ExprIdx::new(0),
                            target: StepIdx::new(1),
                        },
                        ExprBranch {
                            condition: ExprIdx::new(1),
                            target: StepIdx::new(2),
                        },
                    ]
                    .into_boxed_slice(),
                    otherwise: None, // No fallback
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
        ]
        .into_boxed_slice(),
        expressions: vec![expr0, expr1].into_boxed_slice(),
        accessors: Box::new([]),
        constants: vec![ConstValue::Bool(false), ConstValue::Bool(false)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())
}

#[test]
fn choose_expr_branch_all_false_no_otherwise_returns_missing_next_step() -> Result<(), String> {
    let plan = all_false_expr_plan()?;
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 10, 1).unwrap();
    let mut store = crate::value_store::ValueStore::new();

    let branches = vec![
        ExprBranch {
            condition: ExprIdx::new(0),
            target: StepIdx::new(1),
        },
        ExprBranch {
            condition: ExprIdx::new(1),
            target: StepIdx::new(2),
        },
    ];

    let result = choose_expr_branch(&plan, &mut run, &mut store, &branches, None);

    match result {
        Err(EngineError::MissingNextStep { step: _ }) => Ok(()),
        other => Err(format!("expected MissingNextStep, got {other:?}"))?,
    }
}
