//! Expression operator evaluation tests.

#![forbid(unsafe_code)]

use crate::ids::{ConstIdx, ExprIdx, RunId, SlotIdx, StepIdx, SymbolId, WorkflowDigest};
use crate::limits::MAX_EXPRESSION_STACK;
use crate::value::{ConstValue, SlotValue, Taint};
use crate::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram, ResourceContract,
    WorkflowParts, check_expr_stack_bound,
};

use super::ops::eval_expr_operator;
use super::stack::ExprStack;
use crate::value_store::ValueStore;

fn ensure_equal<T>(actual: T, expected: T) -> Result<(), String>
where
    T: core::fmt::Debug + PartialEq,
{
    if actual == expected {
        Ok(())
    } else {
        Err(format!("expected {expected:?}, found {actual:?}"))
    }
}

/// Evaluates a sequence of ops as a single expression, returning the result.
fn eval_ops(
    ops: Vec<ExprOp>,
    constants: Vec<ConstValue>,
    store: &mut ValueStore,
) -> Result<SlotValue, String> {
    eval_ops_with_slots(ops, vec![], constants, store)
}

/// Evaluates a sequence of ops with pre-populated slot values.
fn eval_ops_with_slots(
    ops: Vec<ExprOp>,
    slots: Vec<SlotValue>,
    constants: Vec<ConstValue>,
    store: &mut ValueStore,
) -> Result<SlotValue, String> {
    let max_stack =
        check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK).map_err(|e| e.to_string())?;
    let expr = ExprProgram::try_from_parts(ops.into_boxed_slice(), max_stack)
        .map_err(|e| e.to_string())?;
    let slot_count = if slots.is_empty() {
        1
    } else {
        u16::try_from(slots.len()).map_err(|_| "slot count overflow")?
    };
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("ops_test"),
        digest: WorkflowDigest::from_bytes([0xFA; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }]
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
    .map_err(|e| e.to_string())?;
    let mut run = crate::frame::RunFrame::new(RunId::new(1), StepIdx::new(0), 1, slot_count)
        .map_err(|e| e.to_string())?;
    for (i, value) in slots.iter().enumerate() {
        let idx = u16::try_from(i).map_err(|_| "slot index overflow")?;
        run.write_slot(SlotIdx::new(idx), *value)
            .map_err(|e| e.to_string())?;
    }
    let (value, _taint) =
        crate::engine::expr_eval::eval_expr_with_store(&workflow, &run, store, ExprIdx::new(0))
            .map_err(|e| e.to_string())?;
    Ok(value)
}

// ===== Eq / NotEq =====

#[test]
fn eq_same_values_produces_true() -> Result<(), String> {
    let mut store = ValueStore::new();
    let result = eval_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::Eq,
        ],
        vec![ConstValue::I64(5)],
        &mut store,
    )?;
    ensure_equal(result, SlotValue::Bool(true))
}

#[test]
fn eq_different_values_produces_false() -> Result<(), String> {
    let mut store = ValueStore::new();
    let result = eval_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Eq,
        ],
        vec![ConstValue::I64(5), ConstValue::I64(6)],
        &mut store,
    )?;
    ensure_equal(result, SlotValue::Bool(false))
}

#[test]
fn not_eq_different_values_produces_true() -> Result<(), String> {
    let mut store = ValueStore::new();
    let result = eval_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::NotEq,
        ],
        vec![ConstValue::Bool(true), ConstValue::Bool(false)],
        &mut store,
    )?;
    ensure_equal(result, SlotValue::Bool(true))
}

// ===== And / Or / Not =====

#[test]
fn and_true_true_produces_true() -> Result<(), String> {
    let mut store = ValueStore::new();
    let result = eval_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::And,
        ],
        vec![ConstValue::Bool(true), ConstValue::Bool(true)],
        &mut store,
    )?;
    ensure_equal(result, SlotValue::Bool(true))
}

#[test]
fn and_true_false_produces_false() -> Result<(), String> {
    let mut store = ValueStore::new();
    let result = eval_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::And,
        ],
        vec![ConstValue::Bool(true), ConstValue::Bool(false)],
        &mut store,
    )?;
    ensure_equal(result, SlotValue::Bool(false))
}

#[test]
fn or_false_true_produces_true() -> Result<(), String> {
    let mut store = ValueStore::new();
    let result = eval_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Or,
        ],
        vec![ConstValue::Bool(false), ConstValue::Bool(true)],
        &mut store,
    )?;
    ensure_equal(result, SlotValue::Bool(true))
}

#[test]
fn or_false_false_produces_false() -> Result<(), String> {
    let mut store = ValueStore::new();
    let result = eval_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Or,
        ],
        vec![ConstValue::Bool(false), ConstValue::Bool(false)],
        &mut store,
    )?;
    ensure_equal(result, SlotValue::Bool(false))
}

#[test]
fn not_true_produces_false() -> Result<(), String> {
    let mut store = ValueStore::new();
    let result = eval_ops(
        vec![ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::Not],
        vec![ConstValue::Bool(true)],
        &mut store,
    )?;
    ensure_equal(result, SlotValue::Bool(false))
}

#[test]
fn not_false_produces_true() -> Result<(), String> {
    let mut store = ValueStore::new();
    let result = eval_ops(
        vec![ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::Not],
        vec![ConstValue::Bool(false)],
        &mut store,
    )?;
    ensure_equal(result, SlotValue::Bool(true))
}

// ===== Arithmetic =====

#[test]
fn add_produces_sum() -> Result<(), String> {
    let mut store = ValueStore::new();
    let result = eval_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Add,
        ],
        vec![ConstValue::I64(10), ConstValue::I64(20)],
        &mut store,
    )?;
    ensure_equal(result, SlotValue::I64(30))
}

#[test]
fn sub_produces_difference() -> Result<(), String> {
    let mut store = ValueStore::new();
    let result = eval_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Sub,
        ],
        vec![ConstValue::I64(20), ConstValue::I64(7)],
        &mut store,
    )?;
    ensure_equal(result, SlotValue::I64(13))
}

#[test]
fn mul_produces_product() -> Result<(), String> {
    let mut store = ValueStore::new();
    let result = eval_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Mul,
        ],
        vec![ConstValue::I64(6), ConstValue::I64(7)],
        &mut store,
    )?;
    ensure_equal(result, SlotValue::I64(42))
}

#[test]
fn div_produces_quotient() -> Result<(), String> {
    let mut store = ValueStore::new();
    let result = eval_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Div,
        ],
        vec![ConstValue::I64(20), ConstValue::I64(4)],
        &mut store,
    )?;
    ensure_equal(result, SlotValue::I64(5))
}

#[test]
fn div_by_zero_returns_error() -> Result<(), String> {
    let mut store = ValueStore::new();
    let result = eval_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Div,
        ],
        vec![ConstValue::I64(10), ConstValue::I64(0)],
        &mut store,
    );
    match result {
        Err(msg) if msg.contains("DivisionByZero") || msg.contains("division") => Ok(()),
        Err(msg) => Err(format!("wrong error: {msg}")),
        Ok(val) => Err(format!("expected error, got {val:?}")),
    }
}

#[test]
fn div_truncates_toward_zero() -> Result<(), String> {
    let mut store = ValueStore::new();
    let result = eval_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Div,
        ],
        vec![ConstValue::I64(-7), ConstValue::I64(2)],
        &mut store,
    )?;
    ensure_equal(result, SlotValue::I64(-3))?;
    let result = eval_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Div,
        ],
        vec![ConstValue::I64(-7), ConstValue::I64(-2)],
        &mut store,
    )?;
    ensure_equal(result, SlotValue::I64(3))?;
    let result = eval_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Div,
        ],
        vec![ConstValue::I64(7), ConstValue::I64(-2)],
        &mut store,
    )?;
    ensure_equal(result, SlotValue::I64(-3))
}

#[test]
fn mul_overflow_returns_error() -> Result<(), String> {
    let mut store = ValueStore::new();
    let result = eval_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::Mul,
        ],
        vec![ConstValue::I64(i64::MAX)],
        &mut store,
    );
    match result {
        Err(msg) if msg.contains("overflow") => Ok(()),
        other => Err(format!("expected overflow error, got {other:?}")),
    }
}

#[test]
fn ut_i64_overflow_propagates_correctly() -> Result<(), String> {
    let mut store = ValueStore::new();
    let add_overflow = eval_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Add,
        ],
        vec![ConstValue::I64(i64::MAX), ConstValue::I64(1)],
        &mut store,
    );
    assert!(
        add_overflow.is_err() && add_overflow.as_ref().unwrap_err().contains("overflow"),
        "add overflow should propagate"
    );
    let sub_overflow = eval_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Sub,
        ],
        vec![ConstValue::I64(i64::MIN), ConstValue::I64(1)],
        &mut store,
    );
    assert!(
        sub_overflow.is_err() && sub_overflow.as_ref().unwrap_err().contains("overflow"),
        "sub overflow should propagate"
    );
    let mul_overflow = eval_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Mul,
        ],
        vec![ConstValue::I64(i64::MAX), ConstValue::I64(2)],
        &mut store,
    );
    assert!(
        mul_overflow.is_err() && mul_overflow.as_ref().unwrap_err().contains("overflow"),
        "mul overflow should propagate"
    );
    Ok(())
}

// ===== Comparisons =====

#[test]
fn gt_7_gt_4_is_true() -> Result<(), String> {
    let mut store = ValueStore::new();
    let result = eval_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Gt,
        ],
        vec![ConstValue::I64(7), ConstValue::I64(4)],
        &mut store,
    )?;
    ensure_equal(result, SlotValue::Bool(true))
}

#[test]
fn gte_equal_values_is_true() -> Result<(), String> {
    let mut store = ValueStore::new();
    let result = eval_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Gte,
        ],
        vec![ConstValue::I64(5), ConstValue::I64(5)],
        &mut store,
    )?;
    ensure_equal(result, SlotValue::Bool(true))
}

#[test]
fn lt_3_lt_4_is_true() -> Result<(), String> {
    let mut store = ValueStore::new();
    let result = eval_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Lt,
        ],
        vec![ConstValue::I64(3), ConstValue::I64(4)],
        &mut store,
    )?;
    ensure_equal(result, SlotValue::Bool(true))
}

#[test]
fn lte_equal_values_is_true() -> Result<(), String> {
    let mut store = ValueStore::new();
    let result = eval_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Lte,
        ],
        vec![ConstValue::I64(5), ConstValue::I64(5)],
        &mut store,
    )?;
    ensure_equal(result, SlotValue::Bool(true))
}

// ===== Exists =====

#[test]
fn exists_null_produces_false() -> Result<(), String> {
    let mut store = ValueStore::new();
    let result = eval_ops(
        vec![ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::Exists],
        vec![ConstValue::Null],
        &mut store,
    )?;
    ensure_equal(result, SlotValue::Bool(false))
}

#[test]
fn exists_non_empty_object_produces_true() -> Result<(), String> {
    let mut store = ValueStore::new();
    let obj = store
        .insert_object(
            vec![crate::value_store::ObjectField {
                key: SymbolId::new(0),
                value: SlotValue::I64(1),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let result = eval_ops_with_slots(
        vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Exists],
        vec![SlotValue::Object(obj)],
        vec![],
        &mut store,
    )?;
    ensure_equal(result, SlotValue::Bool(true))
}

#[test]
fn exists_empty_object_produces_false() -> Result<(), String> {
    let mut store = ValueStore::new();
    let obj = store
        .insert_object(Vec::<crate::value_store::ObjectField>::new().into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let result = eval_ops_with_slots(
        vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Exists],
        vec![SlotValue::Object(obj)],
        vec![],
        &mut store,
    )?;
    ensure_equal(result, SlotValue::Bool(false))
}

// ===== Merge =====

#[test]
fn merge_combines_two_objects_with_right_overlapping_key() -> Result<(), String> {
    let mut store = ValueStore::new();
    let sym_a = store.insert_symbol("a").map_err(|e| e.to_string())?;
    let obj1 = store
        .insert_object(
            vec![crate::value_store::ObjectField {
                key: sym_a,
                value: SlotValue::I64(1),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let obj2 = store
        .insert_object(
            vec![crate::value_store::ObjectField {
                key: sym_a,
                value: SlotValue::I64(99),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;

    let result = eval_ops_with_slots(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Merge,
        ],
        vec![SlotValue::Object(obj1), SlotValue::Object(obj2)],
        vec![],
        &mut store,
    )?;
    let merged_id = match result {
        SlotValue::Object(id) => id,
        other => return Err(format!("expected Object, got {other:?}")),
    };
    let fields = store.object(merged_id).map_err(|e| e.to_string())?;
    ensure_equal(fields.len(), 1)?;
    // Right side overwrites
    ensure_equal(fields[0].value, SlotValue::I64(99))
}

// ===== LoadSlot/LoadConst dispatch rejection =====

#[test]
fn eval_expr_operator_rejects_load_slot_op() -> Result<(), String> {
    let mut stack = ExprStack::new(4).map_err(|e| e.to_string())?;
    let mut store = ValueStore::new();
    let result = eval_expr_operator(ExprOp::LoadSlot(SlotIdx::new(0)), &mut stack, &mut store);
    match result {
        Err(crate::errors::EngineError::InternalInvariantViolation { reason })
            if reason.contains("load ops") =>
        {
            Ok(())
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}
