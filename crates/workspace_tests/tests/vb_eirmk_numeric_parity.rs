//! Numeric operator parity: vb_expr frontend vs live core engine.
//!
//! Each case compiles via the public vb_expr API, evaluates via the frontend
//! evaluator, then drives the identical program through the core engine via
//! a compiled workflow. All results are compared against independent expected
//! values, never just against each other.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{ExprIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::limits::MAX_EXPRESSION_STACK;
use vb_core::value::{ConstValue, FiniteF64, SlotValue, Taint};
use vb_core::workflow::{
    CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram, ResourceContract, WorkflowParts,
    check_expr_stack_bound, node,
};
use vb_expr::{compile_expr, eval_expr_program};

fn expected_f64(value: f64) -> Result<SlotValue, String> {
    FiniteF64::new(value)
        .map(SlotValue::F64)
        .map_err(|e| e.to_string())
}

struct RejectingResolver;

impl vb_expr::ReferenceResolver for RejectingResolver {
    fn resolve_reference(&self, _reference: &str) -> Option<SlotIdx> {
        None
    }
}

fn build_expr_program(program: &ExprProgram) -> Result<ExprProgram, EngineError> {
    let max_stack = check_expr_stack_bound(&program.ops, MAX_EXPRESSION_STACK)?;
    ExprProgram::try_from_parts(program.ops.to_vec().into_boxed_slice(), max_stack)
}

fn workflow_nodes() -> Box<[vb_core::workflow::CompiledNode]> {
    vec![
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
    .into_boxed_slice()
}

fn build_workflow_parts(expr: ExprProgram, constants: &[ConstValue]) -> WorkflowParts {
    WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("vb_eirmk_numeric_parity"),
        digest: WorkflowDigest::from_bytes([0xE1; 32]),
        nodes: workflow_nodes(),
        expressions: vec![expr].into_boxed_slice(),
        accessors: Box::new([]),
        constants: constants.to_vec().into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

fn build_workflow(
    expr: ExprProgram,
    constants: &[ConstValue],
) -> Result<CompiledWorkflow, EngineError> {
    let parts = build_workflow_parts(expr, constants);
    CompiledWorkflow::try_from_parts(parts).map_err(|_| EngineError::InvalidCompiledWorkflow {
        reason: "CompiledWorkflow construction failed",
    })
}

fn drive_workflow(workflow: &CompiledWorkflow) -> Result<vb_core::EngineSignal, EngineError> {
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 1)?;
    let mut store = vb_core::value_store::ValueStore::new();
    let mut budget = vb_core::StepBudget::MAX;
    let signal = vb_core::drive_deterministic(&workflow, &mut run, &mut budget, &mut store)?;
    if matches!(&signal, vb_core::EngineSignal::Finished(_, _)) {
        assert_eq!(run.pc(), StepIdx::new(1));
        assert_eq!(
            run.step_state(StepIdx::new(0))?,
            vb_core::StepState::Succeeded
        );
        assert_eq!(
            run.step_state(StepIdx::new(1))?,
            vb_core::StepState::Succeeded
        );
    }
    Ok(signal)
}

fn run_parity_case(source: &str, expected: SlotValue, expected_op: ExprOp) -> Result<(), String> {
    let resolver = RejectingResolver;
    let (program, constants) =
        compile_expr(source, &resolver).map_err(|e| format!("compile {source}: {e}"))?;
    let op_found = program.ops.iter().any(|op| *op == expected_op);
    assert!(
        op_found,
        "{source}: expected op {:?} not found in bytecode",
        expected_op
    );
    let frontend = eval_expr_program(&program, &[], &constants)
        .map_err(|e| format!("frontend {source}: {e}"))?;
    assert_eq!(frontend, expected, "{source}: frontend result mismatch");
    let expr = build_expr_program(&program).map_err(|e| format!("{source}: {e}"))?;
    let workflow = build_workflow(expr, &constants).map_err(|e| format!("{source}: {e}"))?;
    let core = drive_workflow(&workflow).map_err(|e| format!("{source}: {e}"))?;
    assert_eq!(
        core,
        vb_core::EngineSignal::Finished(expected, Taint::Clean),
        "{source}: core result mismatch"
    );
    Ok(())
}

#[test]
fn arithmetic_operations_all_type_pairings() -> Result<(), String> {
    let scenarios = [
        ("6 + 2", ExprOp::Add, SlotValue::I64(8)),
        ("6.0 + 2.0", ExprOp::Add, expected_f64(8.0)?),
        ("6 + 2.0", ExprOp::Add, expected_f64(8.0)?),
        ("6.0 + 2", ExprOp::Add, expected_f64(8.0)?),
        ("6 - 2", ExprOp::Sub, SlotValue::I64(4)),
        ("6.0 - 2.0", ExprOp::Sub, expected_f64(4.0)?),
        ("6 - 2.0", ExprOp::Sub, expected_f64(4.0)?),
        ("6.0 - 2", ExprOp::Sub, expected_f64(4.0)?),
        ("6 * 2", ExprOp::Mul, SlotValue::I64(12)),
        ("6.0 * 2.0", ExprOp::Mul, expected_f64(12.0)?),
        ("6 * 2.0", ExprOp::Mul, expected_f64(12.0)?),
        ("6.0 * 2", ExprOp::Mul, expected_f64(12.0)?),
        ("6 / 2", ExprOp::Div, SlotValue::I64(3)),
        ("6.0 / 2.0", ExprOp::Div, expected_f64(3.0)?),
        ("6 / 2.0", ExprOp::Div, expected_f64(3.0)?),
        ("6.0 / 2", ExprOp::Div, expected_f64(3.0)?),
    ];
    for (source, op, expected) in scenarios {
        run_parity_case(source, expected, op)?;
    }
    Ok(())
}

#[test]
fn ordered_comparison_operations_all_type_pairings() -> Result<(), String> {
    let scenarios = [
        ("6 > 2", ExprOp::Gt, SlotValue::Bool(true)),
        ("6.0 > 2.0", ExprOp::Gt, SlotValue::Bool(true)),
        ("6 > 2.0", ExprOp::Gt, SlotValue::Bool(true)),
        ("6.0 > 2", ExprOp::Gt, SlotValue::Bool(true)),
        ("4 >= 4", ExprOp::Gte, SlotValue::Bool(true)),
        ("4.0 >= 4.0", ExprOp::Gte, SlotValue::Bool(true)),
        ("4 >= 4.0", ExprOp::Gte, SlotValue::Bool(true)),
        ("4.0 >= 4", ExprOp::Gte, SlotValue::Bool(true)),
        ("2 < 6", ExprOp::Lt, SlotValue::Bool(true)),
        ("2.0 < 6.0", ExprOp::Lt, SlotValue::Bool(true)),
        ("2 < 6.0", ExprOp::Lt, SlotValue::Bool(true)),
        ("2.0 < 6", ExprOp::Lt, SlotValue::Bool(true)),
        ("4 <= 4", ExprOp::Lte, SlotValue::Bool(true)),
        ("4.0 <= 4.0", ExprOp::Lte, SlotValue::Bool(true)),
        ("4 <= 4.0", ExprOp::Lte, SlotValue::Bool(true)),
        ("4.0 <= 4", ExprOp::Lte, SlotValue::Bool(true)),
    ];
    for (source, op, expected) in scenarios {
        run_parity_case(source, expected, op)?;
    }
    Ok(())
}

#[test]
fn ordered_comparisons_including_equal() -> Result<(), String> {
    let tuples = [
        (["6", "6.0"], ["2", "2.0"], [true, true, false, false]),
        (["2", "2.0"], ["6", "6.0"], [false, false, true, true]),
        (["2", "2.0"], ["2", "2.0"], [false, true, false, true]),
    ];
    let ops = [
        (">", ExprOp::Gt),
        (">=", ExprOp::Gte),
        ("<", ExprOp::Lt),
        ("<=", ExprOp::Lte),
    ];
    for (left_spellings, right_spellings, expectations) in tuples {
        for ((symbol, op), expected) in ops.into_iter().zip(expectations) {
            for left in left_spellings {
                for right in right_spellings {
                    let source = format!("{left} {symbol} {right}");
                    run_parity_case(&source, SlotValue::Bool(expected), op)?;
                }
            }
        }
    }
    Ok(())
}

fn check_error(
    source: &str,
    frontend_err: vb_expr::ExprError,
    core_err: EngineError,
) -> Result<(), String> {
    let resolver = RejectingResolver;
    let (program, constants) =
        compile_expr(source, &resolver).map_err(|e| format!("compile {source}: {e}"))?;
    let fe = eval_expr_program(&program, &[], &constants);
    assert_eq!(fe, Err(frontend_err), "{source}: frontend error mismatch");
    let expr = build_expr_program(&program).map_err(|e| format!("{source}: {e}"))?;
    let workflow = build_workflow(expr, &constants).map_err(|e| format!("{source}: {e}"))?;
    let ce = drive_workflow(&workflow);
    assert_eq!(ce, Err(core_err), "{source}: core error mismatch");
    Ok(())
}

fn check_overflow(source: &str, reason: &'static str) -> Result<(), String> {
    check_error(
        source,
        vb_expr::ExprError::IntegerOverflow,
        EngineError::InvalidCompiledWorkflow { reason },
    )
}

fn check_division_by_zero(source: &str) -> Result<(), String> {
    check_error(
        source,
        vb_expr::ExprError::DivisionByZero,
        EngineError::DivisionByZero,
    )
}

fn check_non_finite(source: &str) -> Result<(), String> {
    check_error(
        source,
        vb_expr::ExprError::NonFiniteFloat,
        EngineError::NonFiniteNumber,
    )
}

fn check_type_mismatch(source: &str) -> Result<(), String> {
    check_error(
        source,
        vb_expr::ExprError::TypeMismatch {
            expected: "number".to_string(),
            found: "boolean".to_string(),
        },
        EngineError::TypeMismatch {
            expected: "number",
            found: "boolean",
        },
    )
}

#[test]
fn error_cases_frontend_and_core_separately() -> Result<(), String> {
    for (source, reason) in [
        ("9223372036854775807 + 1", "integer arithmetic overflow"),
        (
            "(-9223372036854775807 - 1) - 1",
            "integer arithmetic overflow",
        ),
        ("9223372036854775807 * 2", "integer arithmetic overflow"),
        (
            "(-9223372036854775807 - 1) / -1",
            "integer division overflow",
        ),
    ] {
        check_overflow(source, reason)?;
    }
    check_division_by_zero("1 / 0")?;
    for source in ["1.0 / 0", "1 / 0.0", "1.0 / 0.0"] {
        check_non_finite(source)?;
    }
    for op in ["+", "-", "*", "/", ">", "<", ">=", "<="] {
        check_type_mismatch(&format!("true {op} 2"))?;
        check_type_mismatch(&format!("2 {op} true"))?;
    }
    Ok(())
}
