//! Tests for expression evaluation.

use crate::errors::EngineError;
use crate::ids::{ConstIdx, ExprIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use crate::limits::MAX_EXPRESSION_STACK;
use crate::value::{ConstValue, SlotValue, Taint};
use crate::value_store::{ObjectField, ValueStore};
use crate::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram, ResourceContract,
    WorkflowParts, check_expr_stack_bound,
};

use super::eval_expr_with_store;

fn empty_plan_with_expr(
    ops: Box<[ExprOp]>,
    constants: Box<[ConstValue]>,
) -> Result<CompiledWorkflow, EngineError> {
    let max_stack = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK).map_err(|_| {
        EngineError::InvalidCompiledWorkflow {
            reason: "stack check failed",
        }
    })?;
    let expr = ExprProgram::try_from_parts(ops, max_stack).map_err(|_| {
        EngineError::InvalidCompiledWorkflow {
            reason: "expr parts",
        }
    })?;
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: "test".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            kind: CompiledNodeKind::Nop,
            next: None,
            on_error: None,
            error_slot: None,
            output: None,
        }]
        .into(),
        expressions: vec![expr].into(),
        accessors: vec![].into(),
        constants,
        slot_count: 8,
        symbols_count: 10,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),    })
    .map_err(|_| EngineError::InvalidCompiledWorkflow {
        reason: "workflow parts",
    })
}

fn run_frame_with_slots(slots: Vec<SlotValue>) -> Result<crate::RunFrame, EngineError> {
    let mut run = crate::RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 8)?;
    for (i, value) in slots.iter().enumerate() {
        let idx = SlotIdx::new(u16::try_from(i).map_err(|_| {
            EngineError::InternalInvariantViolation {
                reason: "slot index overflow",
            }
        })?);
        run.write_slot(idx, *value)?;
    }
    Ok(run)
}

fn eval_expr_ops_with_constants(
    ops: &[ExprOp],
    constants: Vec<ConstValue>,
    store: &mut ValueStore,
) -> Result<SlotValue, EngineError> {
    let plan = empty_plan_with_expr(ops.into(), constants.into())?;
    let run = run_frame_with_slots(vec![])?;
    let (value, _) = eval_expr_with_store(&plan, &run, store, ExprIdx::new(0))?;
    Ok(value)
}

fn eval_expr_ops_with_store(
    ops: &[ExprOp],
    slots: Vec<SlotValue>,
    constants: Vec<ConstValue>,
    store: &mut ValueStore,
) -> Result<SlotValue, EngineError> {
    let plan = empty_plan_with_expr(ops.into(), constants.into())?;
    let run = run_frame_with_slots(slots)?;
    let (value, _) = eval_expr_with_store(&plan, &run, store, ExprIdx::new(0))?;
    Ok(value)
}

#[test]
fn contains_finds_substring() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let haystack = store.insert_symbol("hello world")?;
    let needle = store.insert_symbol("world")?;
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Contains,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::Symbol(haystack), ConstValue::Symbol(needle)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn contains_rejects_missing_substring() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let haystack = store.insert_symbol("hello")?;
    let needle = store.insert_symbol("xyz")?;
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Contains,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::Symbol(haystack), ConstValue::Symbol(needle)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn starts_with_matches_prefix() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let text = store.insert_symbol("hello world")?;
    let prefix = store.insert_symbol("hello")?;
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::StartsWith,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::Symbol(text), ConstValue::Symbol(prefix)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn ends_with_matches_suffix() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let text = store.insert_symbol("hello world")?;
    let suffix = store.insert_symbol("world")?;
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::EndsWith,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::Symbol(text), ConstValue::Symbol(suffix)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn has_finds_element_in_list() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let list = store.insert_list(
        vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)].into_boxed_slice(),
    )?;

    let ops = vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::Has,
    ];
    let result = eval_expr_ops_with_store(
        &ops,
        vec![SlotValue::List(list)],
        vec![ConstValue::I64(20)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn exists_checks_object_field() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let sym = store.insert_symbol("key")?;
    let obj = store.insert_object(
        vec![ObjectField {
            key: sym,
            value: SlotValue::Bool(true),
            taint: Taint::Clean,
        }]
        .into_boxed_slice(),
    )?;

    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Exists];
    let result = eval_expr_ops_with_store(&ops, vec![SlotValue::Object(obj)], vec![], &mut store)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn length_counts_list_items() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let list = store.insert_list(
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)].into_boxed_slice(),
    )?;

    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Length];
    let result = eval_expr_ops_with_store(&ops, vec![SlotValue::List(list)], vec![], &mut store)?;
    assert_eq!(result, SlotValue::I64(3));
    Ok(())
}

#[test]
fn empty_detects_empty_list() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let list = store.insert_list(vec![].into_boxed_slice())?;

    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Empty];
    let result = eval_expr_ops_with_store(&ops, vec![SlotValue::List(list)], vec![], &mut store)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn append_adds_to_list() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let list = store.insert_list(vec![SlotValue::I64(1)].into_boxed_slice())?;

    let ops = vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::Append,
    ];
    let result = eval_expr_ops_with_store(
        &ops,
        vec![SlotValue::List(list)],
        vec![ConstValue::I64(2)],
        &mut store,
    )?;
    let result_list_id = match result {
        SlotValue::List(id) => id,
        other => {
            return Err(EngineError::TypeMismatch {
                expected: "list",
                found: other.type_name(),
            });
        }
    };
    let items = store.list(result_list_id)?;
    assert_eq!(items.len(), 2);
    Ok(())
}

#[test]
fn append_if_conditionally_adds() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let list = store.insert_list(vec![SlotValue::I64(1)].into_boxed_slice())?;

    let ops = vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::AppendIf,
    ];
    let result = eval_expr_ops_with_store(
        &ops,
        vec![SlotValue::List(list)],
        vec![ConstValue::I64(2), ConstValue::Bool(true)],
        &mut store,
    )?;
    let result_list_id = match result {
        SlotValue::List(id) => id,
        other => {
            return Err(EngineError::TypeMismatch {
                expected: "list",
                found: other.type_name(),
            });
        }
    };
    let items = store.list(result_list_id)?;
    assert_eq!(items.len(), 2);
    Ok(())
}

#[test]
fn merge_combines_objects() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let sym1 = store.insert_symbol("a")?;
    let sym2 = store.insert_symbol("b")?;
    let obj1 = store.insert_object(
        vec![ObjectField {
            key: sym1,
            value: SlotValue::I64(1),
            taint: Taint::Clean,
        }]
        .into_boxed_slice(),
    )?;
    let obj2 = store.insert_object(
        vec![ObjectField {
            key: sym2,
            value: SlotValue::I64(2),
            taint: Taint::Clean,
        }]
        .into_boxed_slice(),
    )?;

    let ops = vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::Merge,
    ];
    let result = eval_expr_ops_with_store(
        &ops,
        vec![SlotValue::Object(obj1), SlotValue::Object(obj2)],
        vec![],
        &mut store,
    )?;
    let result_obj_id = match result {
        SlotValue::Object(id) => id,
        other => {
            return Err(EngineError::TypeMismatch {
                expected: "object",
                found: other.type_name(),
            });
        }
    };
    let merged = store.object(result_obj_id)?;
    assert_eq!(merged.len(), 2);
    Ok(())
}

#[test]
fn sum_computes_list_total() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let list = store.insert_list(
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)].into_boxed_slice(),
    )?;

    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Sum];
    let result = eval_expr_ops_with_store(&ops, vec![SlotValue::List(list)], vec![], &mut store)?;
    assert_eq!(result, SlotValue::I64(6));
    Ok(())
}

#[test]
fn count_computes_list_length() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let list = store.insert_list(vec![SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice())?;

    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Count];
    let result = eval_expr_ops_with_store(&ops, vec![SlotValue::List(list)], vec![], &mut store)?;
    assert_eq!(result, SlotValue::I64(2));
    Ok(())
}

#[test]
fn unique_removes_duplicates() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let list = store.insert_list(
        vec![SlotValue::I64(1), SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice(),
    )?;

    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Unique];
    let result = eval_expr_ops_with_store(&ops, vec![SlotValue::List(list)], vec![], &mut store)?;
    let result_list_id = match result {
        SlotValue::List(id) => id,
        other => {
            return Err(EngineError::TypeMismatch {
                expected: "list",
                found: other.type_name(),
            });
        }
    };
    let items = store.list(result_list_id)?;
    assert_eq!(items.len(), 2);
    Ok(())
}

// ===== Security regression tests =====

#[test]
fn div_i64_min_div_neg_one_returns_overflow_not_division_by_zero() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Div,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(i64::MIN), ConstValue::I64(-1)],
        &mut store,
    );
    let Err(EngineError::InvalidCompiledWorkflow { reason }) = result else {
        return Err(EngineError::TypeMismatch {
            expected: "InvalidCompiledWorkflow (overflow)",
            found: "wrong error variant",
        });
    };
    assert!(
        reason.contains("overflow"),
        "reason should mention overflow, got: {reason}"
    );
    Ok(())
}

#[test]
fn div_by_zero_still_returns_division_by_zero_error() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Div,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(42), ConstValue::I64(0)],
        &mut store,
    );
    assert_eq!(result, Err(EngineError::DivisionByZero));
    Ok(())
}

#[test]
fn sum_overflow_on_individual_element_is_detected() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let list =
        store.insert_list(vec![SlotValue::I64(i64::MAX), SlotValue::I64(1)].into_boxed_slice())?;
    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Sum];
    let result = eval_expr_ops_with_store(&ops, vec![SlotValue::List(list)], vec![], &mut store);
    let Err(EngineError::InvalidCompiledWorkflow { reason }) = result else {
        return Err(EngineError::TypeMismatch {
            expected: "InvalidCompiledWorkflow (sum overflow)",
            found: "no error or wrong error",
        });
    };
    assert!(
        reason.contains("overflow"),
        "reason should mention overflow, got: {reason}"
    );
    Ok(())
}

// =====================================================================
// Arithmetic operator tests
// =====================================================================

#[test]
fn add_produces_exact_sum() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Add,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(27), ConstValue::I64(15)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::I64(42));
    Ok(())
}

#[test]
fn add_overflow_i64_max_plus_one_returns_overflow_error() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Add,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(i64::MAX), ConstValue::I64(1)],
        &mut store,
    );
    let Err(EngineError::InvalidCompiledWorkflow { reason }) = result else {
        return Err(EngineError::TypeMismatch {
            expected: "overflow error",
            found: "wrong result",
        });
    };
    assert!(
        reason.contains("overflow"),
        "expected overflow, got: {reason}"
    );
    Ok(())
}

#[test]
fn add_with_zero_returns_other_value() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Add,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(99), ConstValue::I64(0)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::I64(99));
    Ok(())
}

#[test]
fn add_with_negative_number_produces_correct_result() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Add,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(10), ConstValue::I64(-3)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::I64(7));
    Ok(())
}

#[test]
fn sub_produces_exact_difference() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Sub,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(100), ConstValue::I64(37)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::I64(63));
    Ok(())
}

#[test]
fn sub_overflow_i64_min_minus_one_returns_overflow_error() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Sub,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(i64::MIN), ConstValue::I64(1)],
        &mut store,
    );
    let Err(EngineError::InvalidCompiledWorkflow { reason }) = result else {
        return Err(EngineError::TypeMismatch {
            expected: "overflow error",
            found: "wrong result",
        });
    };
    assert!(
        reason.contains("overflow"),
        "expected overflow, got: {reason}"
    );
    Ok(())
}

#[test]
fn sub_from_zero_produces_negation() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Sub,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(0), ConstValue::I64(42)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::I64(-42));
    Ok(())
}

#[test]
fn mul_produces_exact_product() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Mul,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(7), ConstValue::I64(6)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::I64(42));
    Ok(())
}

#[test]
fn mul_overflow_i64_max_times_two_returns_overflow_error() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Mul,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(i64::MAX), ConstValue::I64(2)],
        &mut store,
    );
    let Err(EngineError::InvalidCompiledWorkflow { reason }) = result else {
        return Err(EngineError::TypeMismatch {
            expected: "overflow error",
            found: "wrong result",
        });
    };
    assert!(
        reason.contains("overflow"),
        "expected overflow, got: {reason}"
    );
    Ok(())
}

#[test]
fn mul_with_zero_returns_zero() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Mul,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(i64::MAX), ConstValue::I64(0)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::I64(0));
    Ok(())
}

#[test]
fn mul_with_negative_returns_negative_product() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Mul,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(5), ConstValue::I64(-3)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::I64(-15));
    Ok(())
}

#[test]
fn div_produces_exact_quotient() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Div,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(100), ConstValue::I64(4)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::I64(25));
    Ok(())
}

#[test]
fn div_integer_truncation_returns_truncated_result() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Div,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(7), ConstValue::I64(2)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::I64(3));
    Ok(())
}

#[test]
fn div_negative_numerator_truncates_toward_zero() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Div,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(-7), ConstValue::I64(2)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::I64(-3));
    Ok(())
}

#[test]
fn div_negative_denominator_truncates_toward_zero() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Div,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(7), ConstValue::I64(-2)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::I64(-3));
    Ok(())
}

#[test]
fn div_zero_numerator_produces_zero() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Div,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(0), ConstValue::I64(42)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::I64(0));
    Ok(())
}

// =====================================================================
// Comparison operator tests
// =====================================================================

#[test]
fn eq_equal_i64_values_returns_true() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::Eq,
    ];
    let result = eval_expr_ops_with_constants(&ops, vec![ConstValue::I64(42)], &mut store)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn eq_different_i64_values_returns_false() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Eq,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(1), ConstValue::I64(2)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn eq_equal_bool_values_returns_true() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::Eq,
    ];
    let result = eval_expr_ops_with_constants(&ops, vec![ConstValue::Bool(true)], &mut store)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn neq_different_i64_values_returns_true() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::NotEq,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(3), ConstValue::I64(4)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn neq_equal_i64_values_returns_false() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::NotEq,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(5), ConstValue::I64(5)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn neq_different_bool_values_returns_true() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::NotEq,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::Bool(true), ConstValue::Bool(false)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn gt_left_greater_returns_true() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Gt,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(10), ConstValue::I64(3)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn gt_left_less_returns_false() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Gt,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(3), ConstValue::I64(10)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn gt_equal_values_returns_false() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Gt,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(7), ConstValue::I64(7)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn gte_greater_returns_true() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Gte,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(10), ConstValue::I64(3)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn gte_equal_returns_true() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Gte,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(5), ConstValue::I64(5)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn gte_less_returns_false() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Gte,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(3), ConstValue::I64(10)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn lt_left_less_returns_true() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Lt,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(2), ConstValue::I64(9)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn lt_left_greater_returns_false() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Lt,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(9), ConstValue::I64(2)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn lt_equal_values_returns_false() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Lt,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(4), ConstValue::I64(4)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn lte_less_returns_true() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Lte,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(1), ConstValue::I64(8)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn lte_equal_returns_true() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Lte,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(6), ConstValue::I64(6)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn lte_greater_returns_false() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Lte,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(8), ConstValue::I64(1)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

// =====================================================================
// Boolean operator tests - full truth tables
// =====================================================================

#[test]
fn and_true_true_returns_true() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::And,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::Bool(true), ConstValue::Bool(true)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn and_true_false_returns_false() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::And,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::Bool(true), ConstValue::Bool(false)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn and_false_true_returns_false() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::And,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::Bool(false), ConstValue::Bool(true)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn and_false_false_returns_false() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::And,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::Bool(false), ConstValue::Bool(false)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn or_true_true_returns_true() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Or,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::Bool(true), ConstValue::Bool(true)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn or_true_false_returns_true() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Or,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::Bool(true), ConstValue::Bool(false)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn or_false_true_returns_true() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Or,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::Bool(false), ConstValue::Bool(true)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn or_false_false_returns_false() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Or,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::Bool(false), ConstValue::Bool(false)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn not_true_returns_false() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::Not];
    let result = eval_expr_ops_with_constants(&ops, vec![ConstValue::Bool(true)], &mut store)?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn not_false_returns_true() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::Not];
    let result = eval_expr_ops_with_constants(&ops, vec![ConstValue::Bool(false)], &mut store)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

// =====================================================================
// Type mismatch error tests
// =====================================================================

#[test]
fn add_type_mismatch_left_not_number_returns_type_error() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Add,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::Bool(false), ConstValue::I64(1)],
        &mut store,
    );
    assert_eq!(
        result,
        Err(EngineError::TypeMismatch {
            expected: "number",
            found: "boolean",
        })
    );
    Ok(())
}

#[test]
fn mul_type_mismatch_right_not_number_returns_type_error() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Mul,
    ];
    let result =
        eval_expr_ops_with_constants(&ops, vec![ConstValue::I64(1), ConstValue::Null], &mut store);
    assert_eq!(
        result,
        Err(EngineError::TypeMismatch {
            expected: "number",
            found: "null",
        })
    );
    Ok(())
}

#[test]
fn and_type_mismatch_left_not_bool_returns_type_error() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::And,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(0), ConstValue::Bool(true)],
        &mut store,
    );
    assert_eq!(
        result,
        Err(EngineError::TypeMismatch {
            expected: "boolean",
            found: "number",
        })
    );
    Ok(())
}

#[test]
fn or_type_mismatch_right_not_bool_returns_type_error() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Or,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::Bool(true), ConstValue::Null],
        &mut store,
    );
    assert_eq!(
        result,
        Err(EngineError::TypeMismatch {
            expected: "boolean",
            found: "null",
        })
    );
    Ok(())
}

#[test]
fn not_type_mismatch_operand_not_bool_returns_type_error() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::Not];
    let result = eval_expr_ops_with_constants(&ops, vec![ConstValue::I64(42)], &mut store);
    assert_eq!(
        result,
        Err(EngineError::TypeMismatch {
            expected: "boolean",
            found: "number",
        })
    );
    Ok(())
}

// =====================================================================
// Stack overflow / underflow boundary tests
// =====================================================================

#[test]
fn stack_underflow_single_operand_for_add_rejected_at_construction() {
    let mut store = ValueStore::new();
    let ops = vec![ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::Add];
    let result = eval_expr_ops_with_constants(&ops, vec![ConstValue::I64(1)], &mut store);
    assert_eq!(
        result,
        Err(EngineError::InvalidCompiledWorkflow {
            reason: "stack check failed",
        })
    );
}

#[test]
fn stack_underflow_single_operand_for_sub_rejected_at_construction() {
    let mut store = ValueStore::new();
    let ops = vec![ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::Sub];
    let result = eval_expr_ops_with_constants(&ops, vec![ConstValue::I64(1)], &mut store);
    assert_eq!(
        result,
        Err(EngineError::InvalidCompiledWorkflow {
            reason: "stack check failed",
        })
    );
}

// =====================================================================
// Empty expression, single value, null handling
// =====================================================================

#[test]
fn single_load_const_returns_pushed_value() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![ExprOp::LoadConst(ConstIdx::new(0))];
    let result = eval_expr_ops_with_constants(&ops, vec![ConstValue::I64(42)], &mut store)?;
    assert_eq!(result, SlotValue::I64(42));
    Ok(())
}

#[test]
fn single_load_const_null_returns_null() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![ExprOp::LoadConst(ConstIdx::new(0))];
    let result = eval_expr_ops_with_constants(&ops, vec![ConstValue::Null], &mut store)?;
    assert_eq!(result, SlotValue::Null);
    Ok(())
}

#[test]
fn null_equals_null_via_eq_returns_true() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::Eq,
    ];
    let result = eval_expr_ops_with_constants(&ops, vec![ConstValue::Null], &mut store)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn null_not_equals_i64_via_eq_returns_false() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Eq,
    ];
    let result =
        eval_expr_ops_with_constants(&ops, vec![ConstValue::Null, ConstValue::I64(0)], &mut store)?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn load_slot_uninitialized_returns_error() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0))];
    let result = eval_expr_ops_with_store(&ops, vec![], vec![], &mut store);
    assert_eq!(
        result,
        Err(EngineError::SlotUninitialized {
            slot: SlotIdx::new(0)
        })
    );
    Ok(())
}

// =====================================================================
// Operator precedence tests
// =====================================================================

#[test]
fn mul_before_add_in_rpn_produces_correct_precedence() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::LoadConst(ConstIdx::new(2)),
        ExprOp::Mul,
        ExprOp::Add,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(1), ConstValue::I64(2), ConstValue::I64(3)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::I64(7));
    Ok(())
}

#[test]
fn add_before_mul_in_rpn_produces_left_to_right_result() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Add,
        ExprOp::LoadConst(ConstIdx::new(2)),
        ExprOp::Mul,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(1), ConstValue::I64(2), ConstValue::I64(3)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::I64(9));
    Ok(())
}

#[test]
fn comparison_after_arithmetic_produces_correct_result() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Add,
        ExprOp::LoadConst(ConstIdx::new(2)),
        ExprOp::Gt,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(10), ConstValue::I64(5), ConstValue::I64(12)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn arithmetic_chained_with_eq_produces_correct_result() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Mul,
        ExprOp::LoadConst(ConstIdx::new(2)),
        ExprOp::Eq,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(3), ConstValue::I64(4), ConstValue::I64(12)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

// =====================================================================
// Deep nesting near max stack depth
// =====================================================================

#[test]
fn deep_nesting_32_loads_then_31_adds_produces_sum() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let mut ops = Vec::with_capacity(63);
    for _ in 0..32 {
        ops.push(ExprOp::LoadConst(ConstIdx::new(0)));
    }
    for _ in 0..31 {
        ops.push(ExprOp::Add);
    }
    let result = eval_expr_ops_with_constants(&ops, vec![ConstValue::I64(1)], &mut store)?;
    assert_eq!(result, SlotValue::I64(32));
    Ok(())
}

#[test]
fn deep_nesting_alternating_loads_and_ops_near_max_stack() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let mut ops = Vec::with_capacity(39);
    for _ in 0..20 {
        ops.push(ExprOp::LoadConst(ConstIdx::new(0)));
    }
    for _ in 0..19 {
        ops.push(ExprOp::Add);
    }
    let result = eval_expr_ops_with_constants(&ops, vec![ConstValue::I64(2)], &mut store)?;
    assert_eq!(result, SlotValue::I64(40));
    Ok(())
}

// =====================================================================
// proptest: property-based correctness
// =====================================================================

#[cfg(test)]
mod proptests {
    use super::super::eval_expr_with_store;
    use crate::errors::EngineError;
    use crate::ids::{ConstIdx, ExprIdx, RunId, StepIdx, WorkflowDigest};
    use crate::limits::MAX_EXPRESSION_STACK;
    use crate::value::{ConstValue, SlotValue};
    use crate::value_store::ValueStore;
    use crate::workflow::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram, ResourceContract,
        WorkflowParts, check_expr_stack_bound,
    };
    use proptest::prelude::*;

    fn eval_ops_with_constants(
        ops: &[ExprOp],
        constants: Vec<ConstValue>,
    ) -> Result<SlotValue, EngineError> {
        let max_stack = check_expr_stack_bound(ops, MAX_EXPRESSION_STACK).map_err(|_| {
            EngineError::InvalidCompiledWorkflow {
                reason: "stack check",
            }
        })?;
        let expr = ExprProgram::try_from_parts(ops.into(), max_stack).map_err(|_| {
            EngineError::InvalidCompiledWorkflow {
                reason: "expr parts",
            }
        })?;
        let plan = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: "proptest".into(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                kind: CompiledNodeKind::Nop,
                next: None,
                on_error: None,
                error_slot: None,
                output: None,
            }]
            .into(),
            expressions: vec![expr].into(),
            accessors: vec![].into(),
            constants: constants.into(),
            slot_count: 8,
            symbols_count: 10,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        input_slots: Box::new([]),        })
        .map_err(|_| EngineError::InvalidCompiledWorkflow {
            reason: "workflow parts",
        })?;
        let run = crate::RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 8)?;
        let mut store = ValueStore::new();
        let (value, _) = eval_expr_with_store(&plan, &run, &mut store, ExprIdx::new(0))?;
        Ok(value)
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
        fn eq_is_reflexive_for_i64(
            a in any::<i64>(),
        ) {
            let ops = vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::Eq,
            ];
            let result = eval_ops_with_constants(
                &ops,
                vec![ConstValue::I64(a)],
            );
            prop_assert_eq!(result, Ok(SlotValue::Bool(true)));
        }

        #[test]
        fn eq_is_reflexive_for_bool(
            a in any::<bool>(),
        ) {
            let ops = vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::Eq,
            ];
            let result = eval_ops_with_constants(
                &ops,
                vec![ConstValue::Bool(a)],
            );
            prop_assert_eq!(result, Ok(SlotValue::Bool(true)));
        }

        #[test]
        fn arithmetic_operators_dont_panic_for_valid_range(
            a in -10_000i64..10_000,
            b in -10_000i64..10_000,
        ) {
            let add_ops = [ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::LoadConst(ConstIdx::new(1)), ExprOp::Add];
            let sub_ops = [ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::LoadConst(ConstIdx::new(1)), ExprOp::Sub];
            let mul_ops = [ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::LoadConst(ConstIdx::new(1)), ExprOp::Mul];
            let cmp_gt = [ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::LoadConst(ConstIdx::new(1)), ExprOp::Gt];
            let cmp_eq = [ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::LoadConst(ConstIdx::new(1)), ExprOp::Eq];
            let consts = vec![ConstValue::I64(a), ConstValue::I64(b)];

            let _ = eval_ops_with_constants(&add_ops, consts.clone());
            let _ = eval_ops_with_constants(&sub_ops, consts.clone());
            let _ = eval_ops_with_constants(&mul_ops, consts.clone());
            let _ = eval_ops_with_constants(&cmp_gt, consts.clone());
            let _ = eval_ops_with_constants(&cmp_eq, consts.clone());
        }

        #[test]
        fn boolean_operators_dont_panic_for_all_inputs(
            a in any::<bool>(),
            b in any::<bool>(),
        ) {
            let ops_and = [ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::LoadConst(ConstIdx::new(1)), ExprOp::And];
            let ops_or = [ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::LoadConst(ConstIdx::new(1)), ExprOp::Or];
            let ops_not = [ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::Not];
            let consts_and_or = vec![ConstValue::Bool(a), ConstValue::Bool(b)];
            let consts_not = vec![ConstValue::Bool(a)];

            let _ = eval_ops_with_constants(&ops_and, consts_and_or.clone());
            let _ = eval_ops_with_constants(&ops_or, consts_and_or);
            let _ = eval_ops_with_constants(&ops_not, consts_not);
        }

        #[test]
        fn mul_then_div_roundtrips_for_nonzero_divisor(
            a in -1000i64..1000,
            b in -1000i64..1000,
        ) {
            prop_assume!(b != 0);
            prop_assume!(a.checked_mul(b).is_some());

            let ops = vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Mul,
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Div,
            ];
            let result = eval_ops_with_constants(
                &ops,
                vec![ConstValue::I64(a), ConstValue::I64(b)],
            );
            prop_assert_eq!(result, Ok(SlotValue::I64(a)));
        }

        #[test]
        fn and_truth_table_is_commutative(
            a in any::<bool>(),
            b in any::<bool>(),
        ) {
            let ops_ab = [ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::LoadConst(ConstIdx::new(1)), ExprOp::And];
            let ops_ba = [ExprOp::LoadConst(ConstIdx::new(1)), ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::And];
            let consts = vec![ConstValue::Bool(a), ConstValue::Bool(b)];

            let ab = eval_ops_with_constants(&ops_ab, consts.clone()).expect("and should succeed");
            let ba = eval_ops_with_constants(&ops_ba, consts).expect("and should succeed");
            prop_assert_eq!(ab, ba);
        }

        #[test]
        fn or_truth_table_is_commutative(
            a in any::<bool>(),
            b in any::<bool>(),
        ) {
            let ops_ab = [ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::LoadConst(ConstIdx::new(1)), ExprOp::Or];
            let ops_ba = [ExprOp::LoadConst(ConstIdx::new(1)), ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::Or];
            let consts = vec![ConstValue::Bool(a), ConstValue::Bool(b)];

            let ab = eval_ops_with_constants(&ops_ab, consts.clone()).expect("or should succeed");
            let ba = eval_ops_with_constants(&ops_ba, consts).expect("or should succeed");
            prop_assert_eq!(ab, ba);
        }

        #[test]
        fn not_is_an_involution(
            a in any::<bool>(),
        ) {
            let ops = vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::Not,
                ExprOp::Not,
            ];
            let result = eval_ops_with_constants(
                &ops,
                vec![ConstValue::Bool(a)],
            );
            prop_assert_eq!(result, Ok(SlotValue::Bool(a)));
        }
    }
}
