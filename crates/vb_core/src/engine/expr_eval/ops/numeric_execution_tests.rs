// Numeric operator execution and symbolic property coverage
// Uses validated EvalExpr->Finish drive through real workflow.

use crate::errors::EngineError;
use crate::ids::{ConstIdx, ExprIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use crate::limits::MAX_EXPRESSION_STACK;
use crate::value::{ConstValue, FiniteF64, SlotValue, Taint};
use crate::workflow::{
    CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram, ResourceContract, WorkflowParts,
    check_expr_stack_bound, node,
};

fn f64_const(value: f64) -> Result<ConstValue, String> {
    FiniteF64::new(value)
        .map(ConstValue::F64)
        .map_err(|e| e.to_string())
}

fn drive_numeric_expr(
    ops: Vec<ExprOp>,
    constants: Vec<ConstValue>,
    slots: Vec<(SlotValue, Taint)>,
) -> Result<crate::EngineSignal, EngineError> {
    let max_stack = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK)?;
    let expr = ExprProgram::try_from_parts(ops.into_boxed_slice(), max_stack)?;
    let slot_count = if slots.is_empty() {
        1
    } else {
        u16::try_from(slots.len()).map_err(|_| EngineError::InternalInvariantViolation {
            reason: "slot count overflow",
        })?
    };
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("numeric_execution_test"),
        digest: WorkflowDigest::from_bytes([0xFA; 32]),
        nodes: vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::EvalExpr {
                    expr: ExprIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ]
        .into_boxed_slice(),
        expressions: vec![expr].into_boxed_slice(),
        accessors: Box::new([]),
        constants: constants.into_boxed_slice(),
        slot_count,
        symbols_count: 10,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|_| EngineError::InternalInvariantViolation {
        reason: "workflow construction failed",
    })?;
    let mut run = crate::frame::RunFrame::new(RunId::new(1), StepIdx::new(0), 2, slot_count)?;
    for (i, (value, taint)) in slots.iter().enumerate() {
        let idx = u16::try_from(i).map_err(|_| EngineError::InternalInvariantViolation {
            reason: "slot index overflow",
        })?;
        run.write_slot_with_taint(SlotIdx::new(idx), *value, *taint)?;
    }
    let mut store = crate::value_store::ValueStore::new();
    let mut budget = crate::StepBudget::MAX;
    let signal = crate::drive_deterministic(&workflow, &mut run, &mut budget, &mut store)?;
    if matches!(&signal, crate::EngineSignal::Finished(_, _)) {
        assert_eq!(run.pc(), StepIdx::new(1));
        assert_eq!(
            run.step_state(StepIdx::new(0))?,
            crate::StepState::Succeeded
        );
        assert_eq!(
            run.step_state(StepIdx::new(1))?,
            crate::StepState::Succeeded
        );
    }
    Ok(signal)
}

#[test]
fn numeric_ops_all_pairings_through_drive() -> Result<(), String> {
    let scenarios = [
        // Add: I64/I64
        (
            "add_i64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Add,
            ],
            vec![ConstValue::I64(6), ConstValue::I64(2)],
            SlotValue::I64(8),
        ),
        // Add: F64/F64
        (
            "add_f64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Add,
            ],
            vec![f64_const(6.0)?, f64_const(2.0)?],
            SlotValue::F64(FiniteF64::new(8.0).map_err(|e| e.to_string())?),
        ),
        // Add: I64/F64
        (
            "add_i64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Add,
            ],
            vec![ConstValue::I64(6), f64_const(2.0)?],
            SlotValue::F64(FiniteF64::new(8.0).map_err(|e| e.to_string())?),
        ),
        // Add: F64/I64
        (
            "add_f64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Add,
            ],
            vec![f64_const(2.0)?, ConstValue::I64(6)],
            SlotValue::F64(FiniteF64::new(8.0).map_err(|e| e.to_string())?),
        ),
        // Sub: I64/I64
        (
            "sub_i64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Sub,
            ],
            vec![ConstValue::I64(6), ConstValue::I64(2)],
            SlotValue::I64(4),
        ),
        // Sub: F64/F64
        (
            "sub_f64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Sub,
            ],
            vec![f64_const(6.0)?, f64_const(2.0)?],
            SlotValue::F64(FiniteF64::new(4.0).map_err(|e| e.to_string())?),
        ),
        // Sub: I64/F64
        (
            "sub_i64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Sub,
            ],
            vec![ConstValue::I64(6), f64_const(2.0)?],
            SlotValue::F64(FiniteF64::new(4.0).map_err(|e| e.to_string())?),
        ),
        // Sub: F64/I64
        (
            "sub_f64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Sub,
            ],
            vec![f64_const(6.0)?, ConstValue::I64(2)],
            SlotValue::F64(FiniteF64::new(4.0).map_err(|e| e.to_string())?),
        ),
        // Mul: I64/I64
        (
            "mul_i64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Mul,
            ],
            vec![ConstValue::I64(6), ConstValue::I64(2)],
            SlotValue::I64(12),
        ),
        // Mul: F64/F64
        (
            "mul_f64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Mul,
            ],
            vec![f64_const(6.0)?, f64_const(2.0)?],
            SlotValue::F64(FiniteF64::new(12.0).map_err(|e| e.to_string())?),
        ),
        // Mul: I64/F64
        (
            "mul_i64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Mul,
            ],
            vec![ConstValue::I64(6), f64_const(2.0)?],
            SlotValue::F64(FiniteF64::new(12.0).map_err(|e| e.to_string())?),
        ),
        // Mul: F64/I64
        (
            "mul_f64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Mul,
            ],
            vec![f64_const(6.0)?, ConstValue::I64(2)],
            SlotValue::F64(FiniteF64::new(12.0).map_err(|e| e.to_string())?),
        ),
        // Div: I64/I64
        (
            "div_i64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Div,
            ],
            vec![ConstValue::I64(6), ConstValue::I64(2)],
            SlotValue::I64(3),
        ),
        // Div: F64/F64
        (
            "div_f64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Div,
            ],
            vec![f64_const(6.0)?, f64_const(2.0)?],
            SlotValue::F64(FiniteF64::new(3.0).map_err(|e| e.to_string())?),
        ),
        // Div: I64/F64
        (
            "div_i64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Div,
            ],
            vec![ConstValue::I64(6), f64_const(2.0)?],
            SlotValue::F64(FiniteF64::new(3.0).map_err(|e| e.to_string())?),
        ),
        // Div: F64/I64
        (
            "div_f64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Div,
            ],
            vec![f64_const(6.0)?, ConstValue::I64(2)],
            SlotValue::F64(FiniteF64::new(3.0).map_err(|e| e.to_string())?),
        ),
        // Gt: I64/I64
        (
            "gt_i64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Gt,
            ],
            vec![ConstValue::I64(6), ConstValue::I64(2)],
            SlotValue::Bool(true),
        ),
        // Gt: F64/F64
        (
            "gt_f64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Gt,
            ],
            vec![f64_const(6.0)?, f64_const(2.0)?],
            SlotValue::Bool(true),
        ),
        // Gt: I64/F64
        (
            "gt_i64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Gt,
            ],
            vec![ConstValue::I64(6), f64_const(2.0)?],
            SlotValue::Bool(true),
        ),
        // Gt: F64/I64
        (
            "gt_f64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Gt,
            ],
            vec![f64_const(6.0)?, ConstValue::I64(2)],
            SlotValue::Bool(true),
        ),
        // Gte: I64/I64 equal
        (
            "gte_i64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Gte,
            ],
            vec![ConstValue::I64(4), ConstValue::I64(4)],
            SlotValue::Bool(true),
        ),
        // Gte: F64/F64 equal
        (
            "gte_f64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Gte,
            ],
            vec![f64_const(4.0)?, f64_const(4.0)?],
            SlotValue::Bool(true),
        ),
        // Gte: I64/F64 equal
        (
            "gte_i64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Gte,
            ],
            vec![ConstValue::I64(4), f64_const(4.0)?],
            SlotValue::Bool(true),
        ),
        // Gte: F64/I64 equal
        (
            "gte_f64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Gte,
            ],
            vec![f64_const(4.0)?, ConstValue::I64(4)],
            SlotValue::Bool(true),
        ),
        // Lt: I64/I64
        (
            "lt_i64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Lt,
            ],
            vec![ConstValue::I64(2), ConstValue::I64(6)],
            SlotValue::Bool(true),
        ),
        // Lt: F64/F64
        (
            "lt_f64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Lt,
            ],
            vec![f64_const(2.0)?, f64_const(6.0)?],
            SlotValue::Bool(true),
        ),
        // Lt: I64/F64
        (
            "lt_i64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Lt,
            ],
            vec![ConstValue::I64(2), f64_const(6.0)?],
            SlotValue::Bool(true),
        ),
        // Lt: F64/I64
        (
            "lt_f64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Lt,
            ],
            vec![f64_const(2.0)?, ConstValue::I64(6)],
            SlotValue::Bool(true),
        ),
        // Lte: I64/I64 equal
        (
            "lte_i64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Lte,
            ],
            vec![ConstValue::I64(4), ConstValue::I64(4)],
            SlotValue::Bool(true),
        ),
        // Lte: F64/F64 equal
        (
            "lte_f64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Lte,
            ],
            vec![f64_const(4.0)?, f64_const(4.0)?],
            SlotValue::Bool(true),
        ),
        // Lte: I64/F64 equal
        (
            "lte_i64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Lte,
            ],
            vec![ConstValue::I64(4), f64_const(4.0)?],
            SlotValue::Bool(true),
        ),
        // Lte: F64/I64 equal
        (
            "lte_f64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Lte,
            ],
            vec![f64_const(4.0)?, ConstValue::I64(4)],
            SlotValue::Bool(true),
        ),
    ];
    for (name, ops, constants, expected) in scenarios {
        let signal =
            drive_numeric_expr(ops, constants, vec![]).map_err(|e| format!("{name}: {e}"))?;
        match signal {
            crate::EngineSignal::Finished(value, Taint::Clean) => {
                if value != expected {
                    return Err(format!("{name}: expected {expected:?}, got {value:?}"));
                }
            }
            other => {
                return Err(format!("{name}: unexpected signal {other:?}"));
            }
        }
    }
    Ok(())
}

#[test]
fn taint_joins_to_secret_through_arithmetic_drive() -> Result<(), String> {
    let signal = drive_numeric_expr(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Add,
        ],
        vec![],
        vec![
            (SlotValue::I64(6), Taint::Clean),
            (SlotValue::I64(2), Taint::Secret),
        ],
    )
    .map_err(|e| e.to_string())?;
    match signal {
        crate::EngineSignal::Finished(SlotValue::I64(8), Taint::Secret) => Ok(()),
        other => Err(format!("unexpected: {other:?}")),
    }
}

#[test]
fn taint_joins_to_secret_through_comparison_drive() -> Result<(), String> {
    let signal = drive_numeric_expr(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Gt,
        ],
        vec![],
        vec![
            (SlotValue::I64(6), Taint::Clean),
            (SlotValue::I64(2), Taint::Secret),
        ],
    )
    .map_err(|e| e.to_string())?;
    match signal {
        crate::EngineSignal::Finished(SlotValue::Bool(true), Taint::Secret) => Ok(()),
        other => Err(format!("unexpected: {other:?}")),
    }
}

#[test]
fn i64_min_by_neg_one_division_overflow_through_drive() -> Result<(), String> {
    let signal = drive_numeric_expr(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Div,
        ],
        vec![ConstValue::I64(i64::MIN), ConstValue::I64(-1)],
        vec![],
    );
    match signal {
        Err(EngineError::InvalidCompiledWorkflow {
            reason: "integer division overflow",
        }) => Ok(()),
        other => Err(format!("expected division overflow, got {other:?}")),
    }
}

#[test]
fn div_by_zero_i64_through_drive() -> Result<(), String> {
    let signal = drive_numeric_expr(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Div,
        ],
        vec![ConstValue::I64(10), ConstValue::I64(0)],
        vec![],
    );
    match signal {
        Err(EngineError::DivisionByZero) => Ok(()),
        other => Err(format!("expected DivisionByZero, got {other:?}")),
    }
}

#[test]
fn f64_zero_division_returns_non_finite_error_through_drive() -> Result<(), String> {
    let signal = drive_numeric_expr(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Div,
        ],
        vec![f64_const(1.0)?, f64_const(0.0)?],
        vec![],
    );
    match signal {
        Err(EngineError::NonFiniteNumber) => Ok(()),
        other => Err(format!("expected NonFiniteNumber, got {other:?}")),
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    fn eval_ops_with_constants(
        ops: &[ExprOp],
        constants: Vec<ConstValue>,
    ) -> Result<SlotValue, String> {
        let signal = super::drive_numeric_expr(ops.to_vec(), constants, vec![])
            .map_err(|e| e.to_string())?;
        match signal {
            crate::EngineSignal::Finished(value, _) => Ok(value),
            other => Err(format!("unexpected signal: {other:?}")),
        }
    }

    proptest! {
        #[test]
        fn add_then_sub_roundtrips_to_original(
            a in -1_000_000i64..1_000_000,
            b in -1_000_000i64..1_000_000,
        ) {
            let ops = vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Add,
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Sub,
            ];
            let result = eval_ops_with_constants(
                &ops,
                vec![ConstValue::I64(a), ConstValue::I64(b)],
            );
            prop_assert_eq!(result, Ok(SlotValue::I64(a)));
        }

        #[test]
        fn checked_add_property_matches_independent_oracle(
            a in -1000i32..1000,
            b in -1000i32..1000,
        ) {
            let expected = a.checked_add(b).ok_or_else(|| TestCaseError::fail("bounded sum overflowed"))?;
            let ops = vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Add,
            ];
            let result = eval_ops_with_constants(
                &ops,
                vec![ConstValue::I64(i64::from(a)), ConstValue::I64(i64::from(b))],
            );
            prop_assert_eq!(result, Ok(SlotValue::I64(i64::from(expected))));
        }

        #[test]
        fn mixed_add_property_matches_independent_f64_oracle(
            a in -1000i32..1000,
            b in -1000i32..1000,
        ) {
            let expected = f64::from(a.checked_add(b).ok_or_else(|| TestCaseError::fail("bounded sum overflowed"))?);
            let ops = vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Add,
            ];
            let result = eval_ops_with_constants(
                &ops,
                vec![
                    ConstValue::I64(i64::from(a)),
                    ConstValue::F64(FiniteF64::new(f64::from(b)).map_err(|e| TestCaseError::fail(e.to_string()))?),
                ],
            );
            match result {
                Ok(SlotValue::F64(v)) => {
                    prop_assert_eq!(v.get(), expected);
                }
                other => prop_assert!(false, "unexpected: {other:?}"),
            }
        }

        #[test]
        fn mixed_lt_property_matches_independent_comparison(
            a in -1000i32..1000,
            b in -1000i32..1000,
        ) {
            let expected = f64::from(a) < f64::from(b);
            let ops = vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Lt,
            ];
            let result = eval_ops_with_constants(
                &ops,
                vec![
                    ConstValue::I64(i64::from(a)),
                    ConstValue::F64(FiniteF64::new(f64::from(b)).map_err(|e| TestCaseError::fail(e.to_string()))?),
                ],
            );
            prop_assert_eq!(result, Ok(SlotValue::Bool(expected)));
        }
    }
}

#[cfg(test)]
#[path = "numeric_boundary_tests.rs"]
mod numeric_boundary;
