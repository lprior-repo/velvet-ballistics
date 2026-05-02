//! Synchronous in-memory state-machine loop.

pub(crate) mod choose;
pub(crate) mod expr_eval;
pub(crate) mod node_helpers;
pub(crate) mod object_list;
pub(crate) mod run_loop;
pub(crate) mod signals;
pub(crate) mod step;
pub(crate) mod validate;

pub use crate::errors::EngineError;
pub use crate::frame::RunFrame;
pub use crate::value_store::ValueStore;
pub use crate::workflow::CompiledWorkflow;
pub use expr_eval::eval_accessor;
pub use expr_eval::eval_accessor_with_store;
pub use expr_eval::eval_expr;
pub use expr_eval::eval_expr_with_store;
pub use object_list::build_list;
pub use object_list::build_list as build_list_impl;
pub use object_list::build_object;
pub use object_list::build_object as build_object_impl;
pub use run_loop::{drive_deterministic, run_until_blocked};
pub use signals::{EngineSignal, StepBudget};
pub use step::step_once;
pub use validate::{
    validate_compiled_workflow, validate_node_bounds, validate_resource_contract,
    validate_transition_target,
};

use crate::ids::RunId;

/// Creates a run frame for a compiled workflow.
pub fn new_run_frame(run_id: RunId, workflow: &CompiledWorkflow) -> Result<RunFrame, EngineError> {
    RunFrame::new(
        run_id,
        workflow.entry(),
        workflow.node_count(),
        workflow.slot_count(),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        EngineError, EngineSignal, RunFrame, StepBudget, build_list_impl, build_object_impl,
        eval_accessor, eval_accessor_with_store, eval_expr, new_run_frame, run_until_blocked,
        step_once,
    };
    use crate::frame::StepState;
    use crate::ids::{
        AccessorIdx, ActionId, ConstIdx, ExprIdx, ListId, ObjectId, RunId, SlotIdx, StepIdx,
        SymbolId, WorkflowDigest,
    };
    use crate::value::{ConstValue, SlotValue, Taint, join_taint};
    use crate::value_store::{ObjectField, ValueStore};
    use crate::workflow::{
        AccessorProgram, CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprBranch, ExprOp,
        ExprProgram, PathSegment, SlotBranch, WorkflowParts,
    };

    fn test_store() -> ValueStore {
        ValueStore::new()
    }

    #[test]
    fn set_chain_finishes_with_slot_value() -> Result<(), String> {
        let workflow = tiny_workflow(ConstValue::I64(42)).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(7), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);

        ensure_equal(
            result,
            Ok(EngineSignal::Finished(SlotValue::I64(42), Taint::Clean)),
        )?;
        ensure_equal(run.executed(), 2)?;
        Ok(())
    }

    #[test]
    fn set_chain_finishes_with_object_slot_value() -> Result<(), String> {
        let workflow = tiny_workflow(ConstValue::Bool(true)).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(8), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);

        ensure_equal(
            result,
            Ok(EngineSignal::Finished(SlotValue::Bool(true), Taint::Clean)),
        )?;
        Ok(())
    }

    #[test]
    fn const_finish_returns_constant_pool_value() -> Result<(), String> {
        let workflow = tiny_workflow(ConstValue::Bool(true)).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(9), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|error| error.to_string())?;

        if result == EngineSignal::Finished(SlotValue::Bool(true), Taint::Clean) {
            Ok(())
        } else {
            Err(format!("unexpected const finish result: {result:?}"))
        }
    }

    #[test]
    fn set_const_rejects_missing_constant() -> Result<(), String> {
        let result = missing_constant_workflow(ConstIdx::new(1));

        match result {
            Err(crate::WorkflowError::ConstOutOfBounds { constant })
                if constant == ConstIdx::new(1) =>
            {
                Ok(())
            }
            other => Err(format!("unexpected const validation result: {other:?}")),
        }
    }

    #[test]
    fn zero_budget_exhausts_without_execution() -> Result<(), String> {
        let workflow = tiny_workflow(ConstValue::I64(42)).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(7), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::new(0), &mut store);

        ensure_equal(result, Ok(EngineSignal::StepBudgetExhausted))?;
        ensure_equal(run.executed(), 0)?;
        ensure_equal(run.pc(), StepIdx::new(0))?;
        ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Pending))?;
        Ok(())
    }

    #[test]
    fn step_budget_try_take_consumes_exactly_one_transition() -> Result<(), String> {
        let mut budget = StepBudget::new(1);

        ensure_equal(budget.try_take().map_err(|error| error.to_string())?, true)?;
        ensure_equal(budget.remaining(), 0)?;
        ensure_equal(budget.try_take().map_err(|error| error.to_string())?, false)?;
        ensure_equal(budget.remaining(), 0)?;
        Ok(())
    }

    #[test]
    fn one_budget_executes_one_transition_and_exhausts() -> Result<(), String> {
        let workflow = tiny_workflow(ConstValue::I64(42)).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(17), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::new(1), &mut store);

        ensure_equal(result, Ok(EngineSignal::StepBudgetExhausted))?;
        ensure_equal(run.executed(), 1)?;
        ensure_equal(run.pc(), StepIdx::new(1))?;
        ensure_equal(run.read_slot(SlotIdx::new(0)), Ok(&SlotValue::I64(42)))?;
        Ok(())
    }

    #[test]
    fn copy_preserves_value_and_taint() -> Result<(), String> {
        let workflow = copy_workflow(Some(SlotIdx::new(1))).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(18), &workflow)?;
        run.write_slot_with_taint(
            SlotIdx::new(0),
            SlotValue::I64(77),
            Taint::DerivedFromSecret,
        )
        .map_err(|error| error.to_string())?;

        let mut store = test_store();
        let signal =
            step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

        ensure_equal(signal, EngineSignal::Continue)?;
        ensure_equal(run.read_slot(SlotIdx::new(1)), Ok(&SlotValue::I64(77)))?;
        ensure_equal(
            run.read_taint(SlotIdx::new(1)),
            Ok(Taint::DerivedFromSecret),
        )?;
        Ok(())
    }

    #[test]
    fn failed_node_is_marked_failed_on_typed_error() -> Result<(), String> {
        let workflow = copy_workflow(None).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(19), &workflow)?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(77), Taint::Clean)
            .map_err(|error| error.to_string())?;
        let mut store = test_store();

        let result = step_once(&workflow, &mut run, &mut store);

        ensure_equal(
            result,
            Err(EngineError::MissingOutputSlot {
                step: StepIdx::new(0),
            }),
        )?;
        ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;
        Ok(())
    }

    #[test]
    fn choose_slot_takes_first_true_branch() -> Result<(), String> {
        let workflow = choose_slot_workflow().map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(8), &workflow)?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Bool(true), Taint::Clean)
            .map_err(|error| error.to_string())?;
        run.write_slot_with_taint(SlotIdx::new(1), SlotValue::Bool(true), Taint::Clean)
            .map_err(|error| error.to_string())?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|error| error.to_string())?;

        if result == EngineSignal::Finished(SlotValue::I64(11), Taint::Clean) {
            Ok(())
        } else {
            Err(format!("unexpected result: {result:?}"))
        }
    }

    #[test]
    fn choose_slot_takes_later_true_branch() -> Result<(), String> {
        let workflow = choose_slot_workflow().map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(10), &workflow)?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Bool(false), Taint::Clean)
            .map_err(|error| error.to_string())?;
        run.write_slot_with_taint(SlotIdx::new(1), SlotValue::Bool(true), Taint::Clean)
            .map_err(|error| error.to_string())?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|error| error.to_string())?;

        if result == EngineSignal::Finished(SlotValue::I64(22), Taint::Clean) {
            Ok(())
        } else {
            Err(format!("unexpected result: {result:?}"))
        }
    }

    #[test]
    fn choose_slot_takes_otherwise_when_no_branch_matches() -> Result<(), String> {
        let workflow = choose_slot_workflow().map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(9), &workflow)?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Bool(false), Taint::Clean)
            .map_err(|error| error.to_string())?;
        run.write_slot_with_taint(SlotIdx::new(1), SlotValue::Bool(false), Taint::Clean)
            .map_err(|error| error.to_string())?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|error| error.to_string())?;

        if result == EngineSignal::Finished(SlotValue::I64(99), Taint::Clean) {
            Ok(())
        } else {
            Err(format!("unexpected result: {result:?}"))
        }
    }

    #[test]
    fn choose_slot_rejects_non_bool_condition_with_type_mismatch() -> Result<(), String> {
        let workflow = choose_slot_workflow().map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(11), &workflow)?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean)
            .map_err(|error| error.to_string())?;
        let mut store = test_store();

        match run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store) {
            Err(EngineError::TypeMismatch {
                expected: "boolean",
                found: "number",
            }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn choose_slot_otherwise_taken_when_no_branch_matches() -> Result<(), String> {
        let workflow =
            choose_slot_without_otherwise_workflow().map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(12), &workflow)?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Bool(false), Taint::Clean)
            .map_err(|error| error.to_string())?;
        run.write_slot_with_taint(SlotIdx::new(1), SlotValue::Bool(false), Taint::Clean)
            .map_err(|error| error.to_string())?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|error| error.to_string())?;

        if result == EngineSignal::Finished(SlotValue::I64(99), Taint::Clean) {
            Ok(())
        } else {
            Err(format!("unexpected result: {result:?}"))
        }
    }

    #[test]
    fn choose_expr_takes_first_true_branch() -> Result<(), String> {
        let workflow = choose_expr_workflow().map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(13), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|error| error.to_string())?;

        if result == EngineSignal::Finished(SlotValue::I64(11), Taint::Clean) {
            Ok(())
        } else {
            Err(format!("unexpected result: {result:?}"))
        }
    }

    #[test]
    fn choose_expr_takes_later_true_branch() -> Result<(), String> {
        let workflow = choose_expr_workflow_with(
            ConstValue::Bool(false),
            ConstValue::Bool(true),
            Some(StepIdx::new(3)),
        )
        .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(20), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|error| error.to_string())?;

        ensure_equal(
            result,
            EngineSignal::Finished(SlotValue::I64(22), Taint::Clean),
        )?;
        Ok(())
    }

    #[test]
    fn choose_expr_takes_otherwise_when_all_false() -> Result<(), String> {
        let workflow = choose_expr_workflow_with(
            ConstValue::Bool(false),
            ConstValue::Bool(false),
            Some(StepIdx::new(3)),
        )
        .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(21), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|error| error.to_string())?;

        ensure_equal(
            result,
            EngineSignal::Finished(SlotValue::I64(99), Taint::Clean),
        )?;
        Ok(())
    }

    #[test]
    fn choose_expr_rejects_non_bool_condition() -> Result<(), String> {
        let workflow = choose_expr_workflow_with(
            ConstValue::I64(1),
            ConstValue::Bool(true),
            Some(StepIdx::new(3)),
        )
        .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(22), &workflow)?;
        let mut store = test_store();

        match run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store) {
            Err(EngineError::TypeMismatch {
                expected: "boolean",
                found: "number",
            }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn choose_expr_otherwise_taken_when_no_branch_matches() -> Result<(), String> {
        let workflow =
            choose_expr_workflow_with(ConstValue::Bool(false), ConstValue::Bool(false), Some(StepIdx::new(3)))
                .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(25), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|error| error.to_string())?;

        if result == EngineSignal::Finished(SlotValue::I64(99), Taint::Clean) {
            Ok(())
        } else {
            Err(format!("unexpected result: {result:?}"))
        }
    }

    #[test]
    fn public_eval_expr_returns_exact_value() -> Result<(), String> {
        let workflow = eval_add_workflow().map_err(|error| error.to_string())?;
        let run = test_frame(RunId::new(23), &workflow)?;

        let (value, _taint) =
            eval_expr(&workflow, &run, ExprIdx::new(0)).map_err(|error| error.to_string())?;

        ensure_equal(value, SlotValue::I64(42))?;
        Ok(())
    }

    #[test]
    fn public_eval_expr_rejects_invalid_expr_index() -> Result<(), String> {
        let workflow = eval_add_workflow().map_err(|error| error.to_string())?;
        let run = test_frame(RunId::new(26), &workflow)?;

        match eval_expr(&workflow, &run, ExprIdx::new(1)) {
            Err(EngineError::ExprOutOfBounds { expr }) if expr == ExprIdx::new(1) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn public_eval_accessor_loads_root_value() -> Result<(), String> {
        let workflow = accessor_workflow(Box::new([])).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(24), &workflow)?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(77), Taint::Clean)
            .map_err(|error| error.to_string())?;

        let value = eval_accessor(&workflow, &run, AccessorIdx::new(0))
            .map_err(|error| error.to_string())?;

        ensure_equal(value, SlotValue::I64(77))?;
        Ok(())
    }

    #[test]
    fn eval_accessor_identity_path_returns_root_handle_without_store() -> Result<(), String> {
        let workflow = accessor_workflow(Box::new([])).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(120), &workflow)?;
        run.write_slot_with_taint(
            SlotIdx::new(0),
            SlotValue::Object(ObjectId::new(42)),
            Taint::Clean,
        )
        .map_err(|error| error.to_string())?;

        let value = eval_accessor(&workflow, &run, AccessorIdx::new(0))
            .map_err(|error| error.to_string())?;

        ensure_equal(value, SlotValue::Object(ObjectId::new(42)))?;
        Ok(())
    }

    #[test]
    fn public_eval_accessor_rejects_invalid_accessor_index() -> Result<(), String> {
        let workflow = accessor_workflow(Box::new([])).map_err(|error| error.to_string())?;
        let run = test_frame(RunId::new(27), &workflow)?;

        match eval_accessor(&workflow, &run, AccessorIdx::new(1)) {
            Err(EngineError::InvalidCompiledWorkflow {
                reason: "accessor index out of bounds",
            }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn eval_expr_node_uses_fixed_stack_and_writes_output() -> Result<(), String> {
        let workflow = eval_add_workflow().map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(14), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|error| error.to_string())?;

        if result == EngineSignal::Finished(SlotValue::I64(42), Taint::Clean) {
            Ok(())
        } else {
            Err(format!("unexpected result: {result:?}"))
        }
    }

    #[test]
    fn load_accessor_with_empty_path_loads_root_slot() -> Result<(), String> {
        let workflow = accessor_workflow(Box::new([])).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(15), &workflow)?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(77), Taint::Clean)
            .map_err(|error| error.to_string())?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|error| error.to_string())?;

        if result == EngineSignal::Finished(SlotValue::I64(77), Taint::Clean) {
            Ok(())
        } else {
            Err(format!("unexpected result: {result:?}"))
        }
    }

    #[test]
    fn public_eval_accessor_reports_typed_error_without_store() -> Result<(), String> {
        let workflow =
            accessor_workflow(vec![PathSegment::Field(SymbolId::new(0))].into_boxed_slice())
                .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(16), &workflow)?;
        run.write_slot_with_taint(
            SlotIdx::new(0),
            SlotValue::Object(ObjectId::new(0)),
            Taint::Clean,
        )
        .map_err(|error| error.to_string())?;

        match eval_accessor(&workflow, &run, AccessorIdx::new(0)) {
            Err(EngineError::UnsupportedAccessorTraversal {
                segment: "field",
                found: "object",
            }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn load_accessor_reads_object_field_through_store() -> Result<(), String> {
        let workflow =
            accessor_workflow(vec![PathSegment::Field(SymbolId::new(7))].into_boxed_slice())
                .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(28), &workflow)?;
        let mut store = test_store();
        let object = store
            .insert_object(
                vec![ObjectField {
                    key: SymbolId::new(7),
                    value: SlotValue::I64(123),
                }]
                .into_boxed_slice(),
            )
            .map_err(|error| error.to_string())?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(object), Taint::Clean)
            .map_err(|error| error.to_string())?;

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|error| error.to_string())?;

        ensure_equal(
            result,
            EngineSignal::Finished(SlotValue::I64(123), Taint::Clean),
        )?;
        Ok(())
    }

    #[test]
    fn eval_accessor_reads_list_item_through_store() -> Result<(), String> {
        let workflow = accessor_workflow(vec![PathSegment::Index(1)].into_boxed_slice())
            .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(29), &workflow)?;
        let mut store = test_store();
        let list = store
            .insert_list(vec![SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice())
            .map_err(|error| error.to_string())?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(list), Taint::Clean)
            .map_err(|error| error.to_string())?;

        let value = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
            .map_err(|error| error.to_string())?;

        ensure_equal(value, SlotValue::I64(2))?;
        Ok(())
    }

    #[test]
    fn eval_accessor_reports_missing_field_precisely() -> Result<(), String> {
        let workflow =
            accessor_workflow(vec![PathSegment::Field(SymbolId::new(9))].into_boxed_slice())
                .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(30), &workflow)?;
        let mut store = test_store();
        let object = store
            .insert_object(Vec::<ObjectField>::new().into_boxed_slice())
            .map_err(|error| error.to_string())?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(object), Taint::Clean)
            .map_err(|error| error.to_string())?;

        match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
            Err(EngineError::ObjectFieldNotFound { field }) if field == SymbolId::new(9) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn eval_accessor_reports_list_index_precisely() -> Result<(), String> {
        let workflow = accessor_workflow(vec![PathSegment::Index(4)].into_boxed_slice())
            .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(31), &workflow)?;
        let mut store = test_store();
        let list = store
            .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
            .map_err(|error| error.to_string())?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(list), Taint::Clean)
            .map_err(|error| error.to_string())?;

        match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
            Err(EngineError::ListIndexOutOfBounds { index: 4 }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn eval_accessor_rejects_field_traversal_on_scalar_value() -> Result<(), String> {
        let workflow =
            accessor_workflow(vec![PathSegment::Field(SymbolId::new(7))].into_boxed_slice())
                .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(121), &workflow)?;
        let mut store = test_store();
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(11), Taint::Clean)
            .map_err(|error| error.to_string())?;

        match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
            Err(EngineError::UnsupportedAccessorTraversal {
                segment: "field",
                found: "number",
            }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn eval_accessor_reports_object_handle_bounds() -> Result<(), String> {
        let workflow =
            accessor_workflow(vec![PathSegment::Field(SymbolId::new(3))].into_boxed_slice())
                .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(122), &workflow)?;
        let mut store = test_store();
        run.write_slot_with_taint(
            SlotIdx::new(0),
            SlotValue::Object(ObjectId::new(99)),
            Taint::Clean,
        )
        .map_err(|error| error.to_string())?;

        match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
            Err(EngineError::ObjectOutOfBounds { object }) if object == ObjectId::new(99) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn eval_accessor_reports_list_handle_bounds() -> Result<(), String> {
        let workflow = accessor_workflow(vec![PathSegment::Index(0)].into_boxed_slice())
            .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(123), &workflow)?;
        let mut store = test_store();
        run.write_slot_with_taint(
            SlotIdx::new(0),
            SlotValue::List(ListId::new(88)),
            Taint::Clean,
        )
        .map_err(|error| error.to_string())?;

        match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
            Err(EngineError::ListOutOfBounds { list }) if list == ListId::new(88) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn build_list_copies_slot_values_in_exact_item_order() -> Result<(), String> {
        let mut store = test_store();
        let mut run = RunFrame::new(RunId::new(32), StepIdx::new(0), 1, 3)
            .map_err(|error| error.to_string())?;
        run.write_slot(SlotIdx::new(0), SlotValue::I64(10))
            .map_err(|error| error.to_string())?;
        run.write_slot(SlotIdx::new(1), SlotValue::Bool(true))
            .map_err(|error| error.to_string())?;
        run.write_slot(SlotIdx::new(2), SlotValue::Null)
            .map_err(|error| error.to_string())?;

        let list = build_list_impl(
            &mut store,
            &run,
            &[SlotIdx::new(1), SlotIdx::new(0), SlotIdx::new(2)],
        )
        .map_err(|error| error.to_string())?;
        let items = store.list(list).map_err(|error| error.to_string())?;

        ensure_equal(items.len(), 3)?;
        ensure_equal(items.first().copied(), Some(SlotValue::Bool(true)))?;
        ensure_equal(items.get(1).copied(), Some(SlotValue::I64(10)))?;
        ensure_equal(items.get(2).copied(), Some(SlotValue::Null))?;
        Ok(())
    }

    #[test]
    fn build_object_preserves_field_order_and_first_duplicate_lookup() -> Result<(), String> {
        let mut store = test_store();
        let mut run = RunFrame::new(RunId::new(33), StepIdx::new(0), 1, 3)
            .map_err(|error| error.to_string())?;
        run.write_slot(SlotIdx::new(0), SlotValue::I64(100))
            .map_err(|error| error.to_string())?;
        run.write_slot(SlotIdx::new(1), SlotValue::I64(200))
            .map_err(|error| error.to_string())?;
        run.write_slot(SlotIdx::new(2), SlotValue::Bool(false))
            .map_err(|error| error.to_string())?;
        let duplicate_key = SymbolId::new(7);
        let tail_key = SymbolId::new(9);

        let object = build_object_impl(
            &mut store,
            &run,
            &[
                (duplicate_key, SlotIdx::new(0)),
                (duplicate_key, SlotIdx::new(1)),
                (tail_key, SlotIdx::new(2)),
            ],
        )
        .map_err(|error| error.to_string())?;
        let fields = store.object(object).map_err(|error| error.to_string())?;

        ensure_equal(fields.len(), 3)?;
        ensure_equal(
            fields.first().copied(),
            Some(ObjectField {
                key: duplicate_key,
                value: SlotValue::I64(100),
            }),
        )?;
        ensure_equal(
            fields.get(1).copied(),
            Some(ObjectField {
                key: duplicate_key,
                value: SlotValue::I64(200),
            }),
        )?;
        ensure_equal(
            fields.get(2).copied(),
            Some(ObjectField {
                key: tail_key,
                value: SlotValue::Bool(false),
            }),
        )?;
        ensure_equal(
            store
                .object_field(object, duplicate_key)
                .map_err(|error| error.to_string())?,
            SlotValue::I64(100),
        )?;
        Ok(())
    }

    #[test]
    fn build_list_rejects_unreadable_item_slot_without_inserting() -> Result<(), String> {
        let mut store = test_store();
        let run = RunFrame::new(RunId::new(34), StepIdx::new(0), 1, 1)
            .map_err(|error| error.to_string())?;

        match build_list_impl(&mut store, &run, &[SlotIdx::new(1)]) {
            Err(EngineError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(1) => {
                ensure_equal(store.list_count(), 0)
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn build_object_rejects_unreadable_field_slot_without_inserting() -> Result<(), String> {
        let mut store = test_store();
        let run = RunFrame::new(RunId::new(35), StepIdx::new(0), 1, 1)
            .map_err(|error| error.to_string())?;

        match build_object_impl(&mut store, &run, &[(SymbolId::new(1), SlotIdx::new(1))]) {
            Err(EngineError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(1) => {
                ensure_equal(store.object_count(), 0)
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn build_nodes_finish_with_constructed_handles() -> Result<(), String> {
        let workflow = construction_workflow().map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(36), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|error| error.to_string())?;
        let object = match result {
            EngineSignal::Finished(SlotValue::Object(object), Taint::Clean) => object,
            other => return Err(format!("unexpected result: {other:?}")),
        };
        let list = match store.object_field(object, SymbolId::new(1)) {
            Ok(SlotValue::List(list)) => list,
            other => return Err(format!("unexpected object field: {other:?}")),
        };
        let items = store.list(list).map_err(|error| error.to_string())?;

        ensure_equal(items.first().copied(), Some(SlotValue::I64(11)))?;
        ensure_equal(items.get(1).copied(), Some(SlotValue::I64(22)))?;
        ensure_equal(
            store.object_field(object, SymbolId::new(2)),
            Ok(SlotValue::I64(11)),
        )?;
        Ok(())
    }

    fn tiny_workflow(value: ConstValue) -> Result<CompiledWorkflow, crate::WorkflowError> {
        CompiledWorkflow::try_from_parts(tiny_workflow_parts(value))
    }

    fn tiny_workflow_parts(value: ConstValue) -> WorkflowParts {
        WorkflowParts {
            name: Box::<str>::from("tiny"),
            digest: WorkflowDigest::from_bytes([1; 32]),
            nodes: tiny_workflow_nodes(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![value].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        }
    }

    fn missing_constant_workflow(
        constant: ConstIdx,
    ) -> Result<CompiledWorkflow, crate::WorkflowError> {
        let mut parts = tiny_workflow_parts(ConstValue::Null);
        parts.nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                kind: CompiledNodeKind::SetConst { value: constant },
            },
            tiny_finish_node(),
        ]
        .into_boxed_slice();
        CompiledWorkflow::try_from_parts(parts)
    }

    fn tiny_workflow_nodes() -> Box<[CompiledNode]> {
        vec![tiny_set_const_node(), tiny_finish_node()].into_boxed_slice()
    }

    fn tiny_set_const_node() -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        }
    }

    fn tiny_finish_node() -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }
    }

    fn choose_slot_workflow() -> Result<CompiledWorkflow, crate::WorkflowError> {
        choose_slot_workflow_with_otherwise(Some(StepIdx::new(3)))
    }

    fn choose_slot_without_otherwise_workflow() -> Result<CompiledWorkflow, crate::WorkflowError> {
        choose_slot_workflow_with_otherwise(Some(StepIdx::new(3)))
    }

    fn choose_slot_workflow_with_otherwise(
        otherwise: Option<StepIdx>,
    ) -> Result<CompiledWorkflow, crate::WorkflowError> {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                kind: CompiledNodeKind::ChooseSlot {
                    branches: vec![
                        SlotBranch {
                            condition: SlotIdx::new(0),
                            target: StepIdx::new(1),
                        },
                        SlotBranch {
                            condition: SlotIdx::new(1),
                            target: StepIdx::new(2),
                        },
                    ]
                    .into_boxed_slice(),
                    otherwise,
                },
            },
            set_const_node(1, 2, 0),
            set_const_node(2, 2, 1),
            set_const_node(3, 2, 2),
            CompiledNode {
                id: StepIdx::new(4),
                output: None,
                next: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
            },
        ];
        CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("choose_slot"),
            digest: WorkflowDigest::from_bytes([5; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![
                ConstValue::I64(11),
                ConstValue::I64(22),
                ConstValue::I64(99),
            ]
            .into_boxed_slice(),
            slot_count: 3,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        })
    }

    fn choose_expr_workflow() -> Result<CompiledWorkflow, crate::WorkflowError> {
        choose_expr_workflow_with(
            ConstValue::Bool(true),
            ConstValue::Bool(false),
            Some(StepIdx::new(3)),
        )
    }

    fn choose_expr_workflow_with(
        first: ConstValue,
        second: ConstValue,
        otherwise: Option<StepIdx>,
    ) -> Result<CompiledWorkflow, crate::WorkflowError> {
        let true_expr =
            ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(0))].into_boxed_slice())
                .map_err(crate::WorkflowError::Expression)?;
        let false_expr =
            ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(1))].into_boxed_slice())
                .map_err(crate::WorkflowError::Expression)?;
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
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
                    otherwise,
                },
            },
            set_const_node(1, 2, 2),
            set_const_node(2, 2, 3),
            set_const_node(3, 2, 4),
            CompiledNode {
                id: StepIdx::new(4),
                output: None,
                next: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
            },
        ];
        CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("choose_expr"),
            digest: WorkflowDigest::from_bytes([6; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: vec![true_expr, false_expr].into_boxed_slice(),
            accessors: Box::new([]),
            constants: vec![
                first,
                second,
                ConstValue::I64(11),
                ConstValue::I64(22),
                ConstValue::I64(99),
            ]
            .into_boxed_slice(),
            slot_count: 3,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        })
    }

    fn copy_workflow(output: Option<SlotIdx>) -> Result<CompiledWorkflow, crate::WorkflowError> {
        CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("copy"),
            digest: WorkflowDigest::from_bytes([9; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output,
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::Copy {
                        source: SlotIdx::new(0),
                    },
                },
                tiny_finish_node(),
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 2,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        })
    }

    fn eval_add_workflow() -> Result<CompiledWorkflow, crate::WorkflowError> {
        let expression = ExprProgram::try_from_ops(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Add,
            ]
            .into_boxed_slice(),
        )
        .map_err(crate::WorkflowError::Expression)?;
        CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("eval_add"),
            digest: WorkflowDigest::from_bytes([7; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::EvalExpr {
                        expr: ExprIdx::new(0),
                    },
                },
                tiny_finish_node(),
            ]
            .into_boxed_slice(),
            expressions: vec![expression].into_boxed_slice(),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(19), ConstValue::I64(23)].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        })
    }

    fn accessor_workflow(
        path: Box<[PathSegment]>,
    ) -> Result<CompiledWorkflow, crate::WorkflowError> {
        let expression = ExprProgram::try_from_ops(
            vec![ExprOp::LoadAccessor(AccessorIdx::new(0))].into_boxed_slice(),
        )
        .map_err(crate::WorkflowError::Expression)?;
        CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("accessor"),
            digest: WorkflowDigest::from_bytes([8; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::EvalExpr {
                        expr: ExprIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(1),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: vec![expression].into_boxed_slice(),
            accessors: vec![AccessorProgram {
                root: SlotIdx::new(0),
                path,
            }]
            .into_boxed_slice(),
            constants: Box::new([]),
            slot_count: 2,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        })
    }

    fn construction_workflow() -> Result<CompiledWorkflow, crate::WorkflowError> {
        CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("construction"),
            digest: WorkflowDigest::from_bytes([0x36; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(2)),
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(1),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: Some(SlotIdx::new(2)),
                    next: Some(StepIdx::new(3)),
                    kind: CompiledNodeKind::BuildList {
                        items: vec![SlotIdx::new(0), SlotIdx::new(1)].into_boxed_slice(),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(3),
                    output: Some(SlotIdx::new(3)),
                    next: Some(StepIdx::new(4)),
                    kind: CompiledNodeKind::BuildObject {
                        fields: vec![
                            (SymbolId::new(1), SlotIdx::new(2)),
                            (SymbolId::new(2), SlotIdx::new(0)),
                        ]
                        .into_boxed_slice(),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(4),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(3),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(11), ConstValue::I64(22)].into_boxed_slice(),
            slot_count: 4,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        })
    }

    fn set_const_node(id: u16, output: u16, constant: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(id),
            output: Some(SlotIdx::new(output)),
            next: Some(StepIdx::new(4)),
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(constant),
            },
        }
    }

    fn test_frame(run_id: RunId, workflow: &CompiledWorkflow) -> Result<RunFrame, String> {
        new_run_frame(run_id, workflow).map_err(|error| error.to_string())
    }

    fn eval_expr_value(
        ops: Box<[ExprOp]>,
        constants: Box<[ConstValue]>,
    ) -> Result<SlotValue, String> {
        let expression = ExprProgram::try_from_ops(ops).map_err(|error| error.to_string())?;
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("operator_expr"),
            digest: WorkflowDigest::from_bytes([0x5A; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: vec![expression].into_boxed_slice(),
            accessors: Box::new([]),
            constants,
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        })
        .map_err(|error| error.to_string())?;
        let run = test_frame(RunId::new(117), &workflow)?;

        let (value, _taint) =
            eval_expr(&workflow, &run, ExprIdx::new(0)).map_err(|error| error.to_string())?;
        Ok(value)
    }

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

    #[test]
    fn budget_zero_drive_deterministic_returns_step_budget_exhausted_without_touching_frame()
    -> Result<(), String> {
        let workflow = tiny_workflow(ConstValue::I64(1)).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(100), &workflow)?;
        let mut store = test_store();
        let initial_executed = run.executed();
        let initial_pc = run.pc();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::new(0), &mut store);

        ensure_equal(result, Ok(EngineSignal::StepBudgetExhausted))?;
        ensure_equal(run.executed(), initial_executed)?;
        ensure_equal(run.pc(), initial_pc)?;
        ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Pending))?;
        Ok(())
    }

    #[test]
    fn budget_one_executes_exactly_one_transition_then_exhausts() -> Result<(), String> {
        let workflow = tiny_workflow(ConstValue::I64(7)).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(101), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::new(1), &mut store);

        ensure_equal(result, Ok(EngineSignal::StepBudgetExhausted))?;
        ensure_equal(run.executed(), 1)?;
        ensure_equal(run.pc(), StepIdx::new(1))?;
        ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;
        ensure_equal(run.step_state(StepIdx::new(1)), Ok(StepState::Pending))?;
        Ok(())
    }

    #[test]
    fn budget_two_completes_two_step_workflow_with_finish() -> Result<(), String> {
        let workflow = tiny_workflow(ConstValue::I64(55)).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(102), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::new(2), &mut store);

        ensure_equal(
            result,
            Ok(EngineSignal::Finished(SlotValue::I64(55), Taint::Clean)),
        )?;
        ensure_equal(run.executed(), 2)?;
        Ok(())
    }

    #[test]
    fn step_budget_try_take_returns_false_after_depletion_without_error() -> Result<(), String> {
        let mut budget = StepBudget::new(0);
        let first = budget.try_take().map_err(|error| error.to_string())?;
        ensure_equal(first, false)?;
        ensure_equal(budget.remaining(), 0)?;

        let mut budget_one = StepBudget::new(1);
        let take1 = budget_one.try_take().map_err(|error| error.to_string())?;
        ensure_equal(take1, true)?;
        ensure_equal(budget_one.remaining(), 0)?;
        let take2 = budget_one.try_take().map_err(|error| error.to_string())?;
        ensure_equal(take2, false)?;
        ensure_equal(budget_one.remaining(), 0)?;
        Ok(())
    }

    #[test]
    fn step_budget_max_does_not_overflow_on_consecutive_takes() -> Result<(), String> {
        let mut budget = StepBudget::MAX;
        ensure_equal(budget.remaining(), crate::limits::MAX_STEP_BUDGET)?;
        let take = budget.try_take().map_err(|error| error.to_string())?;
        ensure_equal(take, true)?;
        ensure_equal(budget.remaining(), crate::limits::MAX_STEP_BUDGET - 1)?;
        Ok(())
    }

    #[test]
    fn step_once_with_invalid_pc_rejected_by_set_pc() -> Result<(), String> {
        let workflow = tiny_workflow(ConstValue::I64(1)).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(103), &workflow)?;

        // set_pc now validates bounds, rejecting out-of-bounds step 99
        let result = run.set_pc(StepIdx::new(99));

        match result {
            Err(EngineError::InvalidProgramCounter { step }) if step == StepIdx::new(99) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn nop_without_next_returns_missing_next_step() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("nop_no_next"),
            digest: WorkflowDigest::from_bytes([0xAA; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                kind: CompiledNodeKind::Nop,
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 0,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        let workflow =
            CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(104), &workflow)?;
        let mut store = test_store();

        let result = step_once(&workflow, &mut run, &mut store);

        match result {
            Err(EngineError::MissingNextStep { step }) if step == StepIdx::new(0) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn set_const_without_output_slot_returns_missing_output_slot() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("set_const_no_output"),
            digest: WorkflowDigest::from_bytes([0xBB; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(1)].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        let workflow =
            CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(105), &workflow)?;
        let mut store = test_store();

        let result = step_once(&workflow, &mut run, &mut store);

        match result {
            Err(EngineError::MissingOutputSlot { step }) if step == StepIdx::new(0) => {
                ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;
                Ok(())
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn finish_with_uninitialized_result_slot_returns_slot_out_of_bounds() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("finish_empty_slot"),
            digest: WorkflowDigest::from_bytes([0xCC; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        let workflow =
            CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(106), &workflow)?;
        let mut store = test_store();

        let result = step_once(&workflow, &mut run, &mut store);

        match result {
            Err(EngineError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(0) => {
                ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;
                Ok(())
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn failed_step_is_marked_failed_in_frame_after_engine_error() -> Result<(), String> {
        let workflow = copy_workflow(None).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(107), &workflow)?;
        run.write_slot(SlotIdx::new(0), SlotValue::I64(1))
            .map_err(|error| error.to_string())?;
        let mut store = test_store();

        let result = step_once(&workflow, &mut run, &mut store);

        ensure_equal(
            result,
            Err(EngineError::MissingOutputSlot {
                step: StepIdx::new(0),
            }),
        )?;
        ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;
        Ok(())
    }

    #[test]
    fn set_pc_to_out_of_bounds_target_returns_invalid_program_counter()
    -> Result<(), String> {
        let workflow = tiny_workflow(ConstValue::I64(1)).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(108), &workflow)?;

        // set_pc now validates bounds, rejecting out-of-bounds step 200
        let result = run.set_pc(StepIdx::new(200));

        match result {
            Err(EngineError::InvalidProgramCounter { step }) if step == StepIdx::new(200) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn copy_from_uninitialized_source_slot_returns_slot_out_of_bounds() -> Result<(), String> {
        let workflow = copy_workflow(Some(SlotIdx::new(1))).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(109), &workflow)?;
        let mut store = test_store();

        let result = step_once(&workflow, &mut run, &mut store);

        match result {
            Err(EngineError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(0) => {
                ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;
                Ok(())
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn drive_deterministic_stops_on_awaiting_action_signal() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("do_node"),
            digest: WorkflowDigest::from_bytes([0xDD; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: None,
                kind: CompiledNodeKind::Do {
                    action: ActionId::new(1),
                    input: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        let workflow =
            CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(110), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);

        ensure_equal(result, Ok(EngineSignal::AwaitingAction))?;
        ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Running))?;
        Ok(())
    }

    #[test]
    fn drive_deterministic_stops_on_awaiting_wait_signal() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("wait_node"),
            digest: WorkflowDigest::from_bytes([0xEE; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                kind: CompiledNodeKind::WaitUntil {
                    deadline_slot: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        let workflow =
            CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(111), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);

        ensure_equal(result, Ok(EngineSignal::AwaitingWait))?;
        ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Waiting))?;
        Ok(())
    }

    #[test]
    fn drive_deterministic_stops_on_awaiting_ask_signal() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("ask_node"),
            digest: WorkflowDigest::from_bytes([0xFF; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                kind: CompiledNodeKind::Ask {
                    prompt: SlotIdx::new(0),
                    timeout_slot: None,
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        let workflow =
            CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(112), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);

        ensure_equal(result, Ok(EngineSignal::AwaitingAsk))?;
        ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Asking))?;
        Ok(())
    }

    #[test]
    fn eval_expr_division_by_zero_returns_division_by_zero_error() -> Result<(), String> {
        let expression = ExprProgram::try_from_ops(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Div,
            ]
            .into_boxed_slice(),
        )
        .map_err(|error| error.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("div_zero"),
            digest: WorkflowDigest::from_bytes([0x11; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: ExprIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: vec![expression].into_boxed_slice(),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(10), ConstValue::I64(0)].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        let workflow =
            CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(113), &workflow)?;
        let mut store = test_store();

        let result = step_once(&workflow, &mut run, &mut store);

        match result {
            Err(EngineError::DivisionByZero) => {
                ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;
                Ok(())
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn eval_expr_integer_overflow_returns_invalid_compiled_workflow() -> Result<(), String> {
        let expression = ExprProgram::try_from_ops(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::Mul,
            ]
            .into_boxed_slice(),
        )
        .map_err(|error| error.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("int_overflow"),
            digest: WorkflowDigest::from_bytes([0x22; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: ExprIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: vec![expression].into_boxed_slice(),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(i64::MAX)].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        let workflow =
            CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(114), &workflow)?;
        let mut store = test_store();

        let result = step_once(&workflow, &mut run, &mut store);

        match result {
            Err(EngineError::InvalidCompiledWorkflow {
                reason: "integer arithmetic overflow",
            }) => {
                ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;
                Ok(())
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn eval_expr_not_on_non_bool_returns_type_mismatch() -> Result<(), String> {
        let expression = ExprProgram::try_from_ops(
            vec![ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::Not].into_boxed_slice(),
        )
        .map_err(|error| error.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("not_on_int"),
            digest: WorkflowDigest::from_bytes([0x33; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: ExprIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: vec![expression].into_boxed_slice(),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(42)].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        let workflow =
            CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(115), &workflow)?;
        let mut store = test_store();

        let result = step_once(&workflow, &mut run, &mut store);

        match result {
            Err(EngineError::TypeMismatch {
                expected: "boolean",
                found: "number",
            }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn eval_expr_operator_truth_table_is_exact() -> Result<(), String> {
        ensure_equal(
            eval_expr_value(
                vec![
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::Eq,
                ]
                .into_boxed_slice(),
                vec![ConstValue::I64(5)].into_boxed_slice(),
            )?,
            SlotValue::Bool(true),
        )?;
        ensure_equal(
            eval_expr_value(
                vec![
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::LoadConst(ConstIdx::new(1)),
                    ExprOp::NotEq,
                ]
                .into_boxed_slice(),
                vec![ConstValue::I64(5), ConstValue::I64(6)].into_boxed_slice(),
            )?,
            SlotValue::Bool(true),
        )?;
        ensure_equal(
            eval_expr_value(
                vec![
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::LoadConst(ConstIdx::new(1)),
                    ExprOp::And,
                ]
                .into_boxed_slice(),
                vec![ConstValue::Bool(true), ConstValue::Bool(false)].into_boxed_slice(),
            )?,
            SlotValue::Bool(false),
        )?;
        ensure_equal(
            eval_expr_value(
                vec![
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::LoadConst(ConstIdx::new(1)),
                    ExprOp::Or,
                ]
                .into_boxed_slice(),
                vec![ConstValue::Bool(false), ConstValue::Bool(true)].into_boxed_slice(),
            )?,
            SlotValue::Bool(true),
        )?;
        ensure_equal(
            eval_expr_value(
                vec![ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::Not].into_boxed_slice(),
                vec![ConstValue::Bool(false)].into_boxed_slice(),
            )?,
            SlotValue::Bool(true),
        )?;
        ensure_equal(
            eval_expr_value(
                vec![
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::LoadConst(ConstIdx::new(1)),
                    ExprOp::Add,
                ]
                .into_boxed_slice(),
                vec![ConstValue::I64(7), ConstValue::I64(4)].into_boxed_slice(),
            )?,
            SlotValue::I64(11),
        )?;
        ensure_equal(
            eval_expr_value(
                vec![
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::LoadConst(ConstIdx::new(1)),
                    ExprOp::Sub,
                ]
                .into_boxed_slice(),
                vec![ConstValue::I64(7), ConstValue::I64(4)].into_boxed_slice(),
            )?,
            SlotValue::I64(3),
        )?;
        ensure_equal(
            eval_expr_value(
                vec![
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::LoadConst(ConstIdx::new(1)),
                    ExprOp::Mul,
                ]
                .into_boxed_slice(),
                vec![ConstValue::I64(7), ConstValue::I64(4)].into_boxed_slice(),
            )?,
            SlotValue::I64(28),
        )?;
        ensure_equal(
            eval_expr_value(
                vec![
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::LoadConst(ConstIdx::new(1)),
                    ExprOp::Div,
                ]
                .into_boxed_slice(),
                vec![ConstValue::I64(20), ConstValue::I64(4)].into_boxed_slice(),
            )?,
            SlotValue::I64(5),
        )?;
        ensure_equal(
            eval_expr_value(
                vec![
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::LoadConst(ConstIdx::new(1)),
                    ExprOp::Gt,
                ]
                .into_boxed_slice(),
                vec![ConstValue::I64(7), ConstValue::I64(4)].into_boxed_slice(),
            )?,
            SlotValue::Bool(true),
        )?;
        ensure_equal(
            eval_expr_value(
                vec![
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::LoadConst(ConstIdx::new(1)),
                    ExprOp::Gte,
                ]
                .into_boxed_slice(),
                vec![ConstValue::I64(4), ConstValue::I64(4)].into_boxed_slice(),
            )?,
            SlotValue::Bool(true),
        )?;
        ensure_equal(
            eval_expr_value(
                vec![
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::LoadConst(ConstIdx::new(1)),
                    ExprOp::Lt,
                ]
                .into_boxed_slice(),
                vec![ConstValue::I64(3), ConstValue::I64(4)].into_boxed_slice(),
            )?,
            SlotValue::Bool(true),
        )?;
        ensure_equal(
            eval_expr_value(
                vec![
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::LoadConst(ConstIdx::new(1)),
                    ExprOp::Lte,
                ]
                .into_boxed_slice(),
                vec![ConstValue::I64(4), ConstValue::I64(4)].into_boxed_slice(),
            )?,
            SlotValue::Bool(true),
        )?;
        ensure_equal(
            eval_expr_value(
                vec![
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::LoadConst(ConstIdx::new(1)),
                    ExprOp::Gt,
                ]
                .into_boxed_slice(),
                vec![ConstValue::I64(3), ConstValue::I64(4)].into_boxed_slice(),
            )?,
            SlotValue::Bool(false),
        )?;
        ensure_equal(
            eval_expr_value(
                vec![
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::LoadConst(ConstIdx::new(1)),
                    ExprOp::Lt,
                ]
                .into_boxed_slice(),
                vec![ConstValue::I64(4), ConstValue::I64(4)].into_boxed_slice(),
            )?,
            SlotValue::Bool(false),
        )?;
        Ok(())
    }

    // =========================================================================
    // Phase 43 adversarial tests -- taint propagation
    // =========================================================================

    #[test]
    fn eval_expr_with_secret_tainted_slot_produces_derived_from_secret_taint() -> Result<(), String> {
        let expression = ExprProgram::try_from_ops(vec![ExprOp::LoadSlot(SlotIdx::new(0))].into_boxed_slice())
            .map_err(|error| error.to_string())?;
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("taint_eval_expr"),
            digest: WorkflowDigest::from_bytes([0x43; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::EvalExpr {
                        expr: ExprIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(1),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: vec![expression].into_boxed_slice(),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(42)].into_boxed_slice(),
            slot_count: 2,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        })
        .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(200), &workflow)?;
        // Write a secret-tainted value into slot 0 that the expression reads.
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(99), Taint::Secret)
            .map_err(|error| error.to_string())?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);

        match result {
            Ok(EngineSignal::Finished(SlotValue::I64(99), Taint::Secret)) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn eval_expr_with_clean_slot_produces_clean_taint() -> Result<(), String> {
        let expression = ExprProgram::try_from_ops(vec![ExprOp::LoadSlot(SlotIdx::new(0))].into_boxed_slice())
            .map_err(|error| error.to_string())?;
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("taint_eval_clean"),
            digest: WorkflowDigest::from_bytes([0x43; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::EvalExpr {
                        expr: ExprIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(1),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: vec![expression].into_boxed_slice(),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(42)].into_boxed_slice(),
            slot_count: 2,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        })
        .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(201), &workflow)?;
        // Write a clean value into slot 0.
        run.write_slot(SlotIdx::new(0), SlotValue::I64(10))
            .map_err(|error| error.to_string())?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);

        match result {
            Ok(EngineSignal::Finished(SlotValue::I64(10), Taint::Clean)) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn build_object_joins_taint_from_all_field_slots() -> Result<(), String> {
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("taint_build_object"),
            digest: WorkflowDigest::from_bytes([0x43; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(2)),
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(1),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: Some(SlotIdx::new(2)),
                    next: Some(StepIdx::new(3)),
                    kind: CompiledNodeKind::BuildObject {
                        fields: vec![
                            (SymbolId::new(1), SlotIdx::new(0)),
                            (SymbolId::new(2), SlotIdx::new(1)),
                        ]
                        .into_boxed_slice(),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(3),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(2),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(10), ConstValue::I64(20)].into_boxed_slice(),
            slot_count: 3,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        })
        .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(202), &workflow)?;
        let mut store = test_store();

        // Step 0: SetConst I64(10) into slot 0 (Clean).
        let s0 = step_once(&workflow, &mut run, &mut store);
        match s0 {
            Ok(EngineSignal::Continue) => {}
            other => return Err(format!("expected Continue from step 0, got {other:?}")),
        }
        // Step 1: SetConst I64(20) into slot 1 (Clean). Override to Secret.
        let s1 = step_once(&workflow, &mut run, &mut store);
        match s1 {
            Ok(EngineSignal::Continue) => {}
            other => return Err(format!("expected Continue from step 1, got {other:?}")),
        }
        // Now override slot 1 taint to Secret before BuildObject reads it.
        run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(20), Taint::Secret)
            .map_err(|error| error.to_string())?;
        // Step 2: BuildObject joins taint from slot 0 (Clean) + slot 1 (Secret) -> Secret.
        let s2 = step_once(&workflow, &mut run, &mut store);
        match s2 {
            Ok(EngineSignal::Continue) => {}
            other => return Err(format!("expected Continue from step 2, got {other:?}")),
        }
        // Verify the output slot 2 has Secret taint.
        let slot2_taint = run.read_taint(SlotIdx::new(2)).map_err(|error| error.to_string())?;
        ensure_equal(slot2_taint, Taint::Secret)?;
        // Step 3: Finish carries the taint from slot 2.
        let s3 = step_once(&workflow, &mut run, &mut store);
        match s3 {
            Ok(EngineSignal::Finished(SlotValue::Object(_), Taint::Secret)) => Ok(()),
            Ok(EngineSignal::Finished(_, other_taint)) => {
                Err(format!("expected Secret taint, got {other_taint:?}"))
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn build_object_with_all_clean_slots_produces_clean_taint() -> Result<(), String> {
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("taint_build_object_clean"),
            digest: WorkflowDigest::from_bytes([0x43; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(2)),
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(1),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: Some(SlotIdx::new(2)),
                    next: Some(StepIdx::new(3)),
                    kind: CompiledNodeKind::BuildObject {
                        fields: vec![
                            (SymbolId::new(1), SlotIdx::new(0)),
                            (SymbolId::new(2), SlotIdx::new(1)),
                        ]
                        .into_boxed_slice(),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(3),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(2),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(10), ConstValue::I64(20)].into_boxed_slice(),
            slot_count: 3,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        })
        .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(203), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);

        match result {
            Ok(EngineSignal::Finished(SlotValue::Object(_), Taint::Clean)) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn build_list_joins_taint_from_all_item_slots() -> Result<(), String> {
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("taint_build_list"),
            digest: WorkflowDigest::from_bytes([0x43; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(2)),
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(1),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: Some(SlotIdx::new(2)),
                    next: Some(StepIdx::new(3)),
                    kind: CompiledNodeKind::BuildList {
                        items: vec![SlotIdx::new(0), SlotIdx::new(1)].into_boxed_slice(),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(3),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(2),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(11), ConstValue::I64(22)].into_boxed_slice(),
            slot_count: 3,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        })
        .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(204), &workflow)?;
        let mut store = test_store();

        // Step 0: SetConst I64(11) into slot 0.
        let s0 = step_once(&workflow, &mut run, &mut store);
        match s0 {
            Ok(EngineSignal::Continue) => {}
            other => return Err(format!("expected Continue from step 0, got {other:?}")),
        }
        // Override slot 0 taint to DerivedFromSecret.
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(11), Taint::DerivedFromSecret)
            .map_err(|error| error.to_string())?;
        // Step 1: SetConst I64(22) into slot 1 (Clean).
        let s1 = step_once(&workflow, &mut run, &mut store);
        match s1 {
            Ok(EngineSignal::Continue) => {}
            other => return Err(format!("expected Continue from step 1, got {other:?}")),
        }
        // Step 2: BuildList joins taint from slot 0 (DerivedFromSecret) + slot 1 (Clean).
        let s2 = step_once(&workflow, &mut run, &mut store);
        match s2 {
            Ok(EngineSignal::Continue) => {}
            other => return Err(format!("expected Continue from step 2, got {other:?}")),
        }
        // Verify output slot 2 has DerivedFromSecret taint.
        let slot2_taint = run.read_taint(SlotIdx::new(2)).map_err(|error| error.to_string())?;
        ensure_equal(slot2_taint, Taint::DerivedFromSecret)?;
        // Step 3: Finish carries the taint from slot 2.
        let s3 = step_once(&workflow, &mut run, &mut store);
        match s3 {
            Ok(EngineSignal::Finished(SlotValue::List(_), Taint::DerivedFromSecret)) => Ok(()),
            Ok(EngineSignal::Finished(_, other_taint)) => {
                Err(format!("expected DerivedFromSecret taint, got {other_taint:?}"))
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn build_list_with_all_secret_slots_produces_secret_taint() -> Result<(), String> {
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("taint_build_list_all_secret"),
            digest: WorkflowDigest::from_bytes([0x43; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(2)),
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(1),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: Some(SlotIdx::new(2)),
                    next: Some(StepIdx::new(3)),
                    kind: CompiledNodeKind::BuildList {
                        items: vec![SlotIdx::new(0), SlotIdx::new(1)].into_boxed_slice(),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(3),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(2),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(11), ConstValue::I64(22)].into_boxed_slice(),
            slot_count: 3,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        })
        .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(205), &workflow)?;
        let mut store = test_store();

        // Step 0: SetConst I64(11) into slot 0. Override to Secret.
        let s0 = step_once(&workflow, &mut run, &mut store);
        match s0 {
            Ok(EngineSignal::Continue) => {}
            other => return Err(format!("expected Continue from step 0, got {other:?}")),
        }
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(11), Taint::Secret)
            .map_err(|error| error.to_string())?;
        // Step 1: SetConst I64(22) into slot 1. Override to Secret.
        let s1 = step_once(&workflow, &mut run, &mut store);
        match s1 {
            Ok(EngineSignal::Continue) => {}
            other => return Err(format!("expected Continue from step 1, got {other:?}")),
        }
        run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(22), Taint::Secret)
            .map_err(|error| error.to_string())?;
        // Step 2: BuildList joins taint from slot 0 (Secret) + slot 1 (Secret) -> Secret.
        let s2 = step_once(&workflow, &mut run, &mut store);
        match s2 {
            Ok(EngineSignal::Continue) => {}
            other => return Err(format!("expected Continue from step 2, got {other:?}")),
        }
        let slot2_taint = run.read_taint(SlotIdx::new(2)).map_err(|error| error.to_string())?;
        ensure_equal(slot2_taint, Taint::Secret)?;
        // Step 3: Finish carries Secret taint.
        let s3 = step_once(&workflow, &mut run, &mut store);
        match s3 {
            Ok(EngineSignal::Finished(SlotValue::List(_), Taint::Secret)) => Ok(()),
            Ok(EngineSignal::Finished(_, other_taint)) => {
                Err(format!("expected Secret taint, got {other_taint:?}"))
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn engine_signal_finished_carries_correct_secret_taint() -> Result<(), String> {
        // Write a secret-tainted value into slot 0 via SetConst, then override taint.
        let workflow = tiny_workflow(ConstValue::I64(77)).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(206), &workflow)?;
        let mut store = test_store();

        // First step sets slot 0 to I64(77) with Clean taint. Override to Secret.
        let first = step_once(&workflow, &mut run, &mut store);
        match first {
            Ok(EngineSignal::Continue) => {}
            other => return Err(format!("expected Continue from first step, got {other:?}")),
        }
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(77), Taint::Secret)
            .map_err(|error| error.to_string())?;

        let second = step_once(&workflow, &mut run, &mut store);
        match second {
            Ok(EngineSignal::Finished(SlotValue::I64(77), Taint::Secret)) => Ok(()),
            Ok(EngineSignal::Finished(value, taint)) => {
                Err(format!("expected Finished(I64(77), Secret), got ({value:?}, {taint:?})"))
            }
            other => Err(format!("expected Finished, got {other:?}")),
        }
    }

    #[test]
    fn engine_signal_finished_carries_correct_derived_from_secret_taint() -> Result<(), String> {
        let workflow = tiny_workflow(ConstValue::Bool(true)).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(207), &workflow)?;
        let mut store = test_store();

        let first = step_once(&workflow, &mut run, &mut store);
        match first {
            Ok(EngineSignal::Continue) => {}
            other => return Err(format!("expected Continue from first step, got {other:?}")),
        }
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Bool(true), Taint::DerivedFromSecret)
            .map_err(|error| error.to_string())?;

        let second = step_once(&workflow, &mut run, &mut store);
        match second {
            Ok(EngineSignal::Finished(SlotValue::Bool(true), Taint::DerivedFromSecret)) => Ok(()),
            Ok(EngineSignal::Finished(value, taint)) => {
                Err(format!("expected Finished(Bool(true), DerivedFromSecret), got ({value:?}, {taint:?})"))
            }
            other => Err(format!("expected Finished, got {other:?}")),
        }
    }

    // =========================================================================
    // Comprehensive taint propagation tests -- every node kind and ExprOp
    // =========================================================================

    /// Helper: evaluate an expression and return (value, taint) from the expression engine.
    /// For tests that need a store (list/object handles), pass a pre-populated store.
    fn taint_eval_expr(
        ops: Box<[ExprOp]>,
        constants: Box<[ConstValue]>,
        slots: Vec<(SlotValue, Taint)>,
        accessors: Box<[AccessorProgram]>,
    ) -> Result<(SlotValue, Taint), String> {
        let mut store = test_store();
        taint_eval_expr_with_store(ops, constants, slots, accessors, &mut store)
    }

    /// Helper: evaluate an expression with a caller-provided store (for list/object tests).
    fn taint_eval_expr_with_store(
        ops: Box<[ExprOp]>,
        constants: Box<[ConstValue]>,
        slots: Vec<(SlotValue, Taint)>,
        accessors: Box<[AccessorProgram]>,
        store: &mut ValueStore,
    ) -> Result<(SlotValue, Taint), String> {
        let expression = ExprProgram::try_from_ops(ops).map_err(|error| error.to_string())?;
        let slot_count = u16::try_from(slots.len()).map_err(|_| "too many slots")?
            .max(1);
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("taint_test"),
            digest: WorkflowDigest::from_bytes([0x54; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: vec![expression].into_boxed_slice(),
            accessors,
            constants,
            slot_count,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        })
        .map_err(|error| error.to_string())?;
        let mut run = RunFrame::new(RunId::new(250), StepIdx::new(0), 1, slot_count)
            .map_err(|error| error.to_string())?;
        for (i, (value, taint)) in slots.iter().enumerate() {
            let idx = SlotIdx::new(i as u16);
            run.write_slot_with_taint(idx, *value, *taint)
                .map_err(|error| error.to_string())?;
        }
        super::eval_expr_with_store(&workflow, &run, store, ExprIdx::new(0))
            .map(|(v, t)| (v, t))
            .map_err(|error| error.to_string())
    }

    // ----- join_taint completeness -----

    #[test]
    fn join_taint_clean_plus_clean_is_clean() {
        assert_eq!(join_taint(Taint::Clean, Taint::Clean), Taint::Clean);
    }

    #[test]
    fn join_taint_clean_plus_secret_is_secret() {
        assert_eq!(join_taint(Taint::Clean, Taint::Secret), Taint::Secret);
    }

    #[test]
    fn join_taint_secret_plus_clean_is_secret() {
        assert_eq!(join_taint(Taint::Secret, Taint::Clean), Taint::Secret);
    }

    #[test]
    fn join_taint_clean_plus_derived_from_secret_is_derived_from_secret() {
        assert_eq!(
            join_taint(Taint::Clean, Taint::DerivedFromSecret),
            Taint::DerivedFromSecret
        );
    }

    #[test]
    fn join_taint_derived_from_secret_plus_clean_is_derived_from_secret() {
        assert_eq!(
            join_taint(Taint::DerivedFromSecret, Taint::Clean),
            Taint::DerivedFromSecret
        );
    }

    #[test]
    fn join_taint_secret_plus_derived_from_secret_is_derived_from_secret() {
        assert_eq!(
            join_taint(Taint::Secret, Taint::DerivedFromSecret),
            Taint::DerivedFromSecret
        );
    }

    #[test]
    fn join_taint_derived_from_secret_plus_secret_is_derived_from_secret() {
        assert_eq!(
            join_taint(Taint::DerivedFromSecret, Taint::Secret),
            Taint::DerivedFromSecret
        );
    }

    #[test]
    fn join_taint_secret_plus_secret_is_secret() {
        assert_eq!(join_taint(Taint::Secret, Taint::Secret), Taint::Secret);
    }

    #[test]
    fn join_taint_derived_plus_derived_is_derived_from_secret() {
        assert_eq!(
            join_taint(Taint::DerivedFromSecret, Taint::DerivedFromSecret),
            Taint::DerivedFromSecret
        );
    }

    // ----- SetConst always Clean -----

    #[test]
    fn set_const_produces_clean_taint_on_output_slot() -> Result<(), String> {
        let workflow = tiny_workflow(ConstValue::I64(42)).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(300), &workflow)?;
        let mut store = test_store();

        let first = step_once(&workflow, &mut run, &mut store);
        match first {
            Ok(EngineSignal::Continue) => {}
            other => return Err(format!("expected Continue, got {other:?}")),
        }
        ensure_equal(run.read_taint(SlotIdx::new(0)), Ok(Taint::Clean))?;
        ensure_equal(run.read_slot(SlotIdx::new(0)), Ok(&SlotValue::I64(42)))?;
        Ok(())
    }

    // ----- Copy propagates taint -----

    #[test]
    fn copy_propagates_secret_taint() -> Result<(), String> {
        let workflow = copy_workflow(Some(SlotIdx::new(1))).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(301), &workflow)?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(55), Taint::Secret)
            .map_err(|error| error.to_string())?;
        let mut store = test_store();

        let signal = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
        ensure_equal(signal, EngineSignal::Continue)?;
        ensure_equal(run.read_slot(SlotIdx::new(1)), Ok(&SlotValue::I64(55)))?;
        ensure_equal(run.read_taint(SlotIdx::new(1)), Ok(Taint::Secret))?;
        Ok(())
    }

    #[test]
    fn copy_propagates_derived_from_secret_taint() -> Result<(), String> {
        let workflow = copy_workflow(Some(SlotIdx::new(1))).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(302), &workflow)?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(33), Taint::DerivedFromSecret)
            .map_err(|error| error.to_string())?;
        let mut store = test_store();

        let signal = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
        ensure_equal(signal, EngineSignal::Continue)?;
        ensure_equal(run.read_taint(SlotIdx::new(1)), Ok(Taint::DerivedFromSecret))?;
        Ok(())
    }

    #[test]
    fn copy_clean_slot_stays_clean() -> Result<(), String> {
        let workflow = copy_workflow(Some(SlotIdx::new(1))).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(303), &workflow)?;
        run.write_slot(SlotIdx::new(0), SlotValue::I64(10))
            .map_err(|error| error.to_string())?;
        let mut store = test_store();

        let signal = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
        ensure_equal(signal, EngineSignal::Continue)?;
        ensure_equal(run.read_taint(SlotIdx::new(1)), Ok(Taint::Clean))?;
        Ok(())
    }

    // ----- EvalExpr with ExprOp taint propagation -----

    #[test]
    fn eval_expr_load_slot_carries_secret_taint() -> Result<(), String> {
        let (value, taint) = taint_eval_expr(
            vec![ExprOp::LoadSlot(SlotIdx::new(0))].into_boxed_slice(),
            Box::new([]),
            vec![(SlotValue::I64(99), Taint::Secret)],
            Box::new([]),
        )?;
        ensure_equal(value, SlotValue::I64(99))?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_expr_load_const_is_clean() -> Result<(), String> {
        let (value, taint) = taint_eval_expr(
            vec![ExprOp::LoadConst(ConstIdx::new(0))].into_boxed_slice(),
            vec![ConstValue::I64(42)].into_boxed_slice(),
            vec![],
            Box::new([]),
        )?;
        ensure_equal(value, SlotValue::I64(42))?;
        ensure_equal(taint, Taint::Clean)
    }

    #[test]
    fn eval_expr_add_preserves_secret_taint() -> Result<(), String> {
        let (value, taint) = taint_eval_expr(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::Add,
            ]
            .into_boxed_slice(),
            vec![ConstValue::I64(5)].into_boxed_slice(),
            vec![(SlotValue::I64(10), Taint::Secret)],
            Box::new([]),
        )?;
        ensure_equal(value, SlotValue::I64(15))?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_expr_sub_preserves_secret_taint() -> Result<(), String> {
        let (value, taint) = taint_eval_expr(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::Sub,
            ]
            .into_boxed_slice(),
            vec![ConstValue::I64(3)].into_boxed_slice(),
            vec![(SlotValue::I64(10), Taint::Secret)],
            Box::new([]),
        )?;
        ensure_equal(value, SlotValue::I64(7))?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_expr_mul_preserves_secret_taint() -> Result<(), String> {
        let (value, taint) = taint_eval_expr(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::Mul,
            ]
            .into_boxed_slice(),
            vec![ConstValue::I64(4)].into_boxed_slice(),
            vec![(SlotValue::I64(7), Taint::DerivedFromSecret)],
            Box::new([]),
        )?;
        ensure_equal(value, SlotValue::I64(28))?;
        ensure_equal(taint, Taint::DerivedFromSecret)
    }

    #[test]
    fn eval_expr_div_preserves_secret_taint() -> Result<(), String> {
        let (value, taint) = taint_eval_expr(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::Div,
            ]
            .into_boxed_slice(),
            vec![ConstValue::I64(2)].into_boxed_slice(),
            vec![(SlotValue::I64(20), Taint::Secret)],
            Box::new([]),
        )?;
        ensure_equal(value, SlotValue::I64(10))?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_expr_eq_preserves_secret_taint() -> Result<(), String> {
        let (value, taint) = taint_eval_expr(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::Eq,
            ]
            .into_boxed_slice(),
            vec![ConstValue::I64(10)].into_boxed_slice(),
            vec![(SlotValue::I64(10), Taint::Secret)],
            Box::new([]),
        )?;
        ensure_equal(value, SlotValue::Bool(true))?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_expr_not_eq_preserves_secret_taint() -> Result<(), String> {
        let (value, taint) = taint_eval_expr(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::NotEq,
            ]
            .into_boxed_slice(),
            vec![ConstValue::I64(99)].into_boxed_slice(),
            vec![(SlotValue::I64(10), Taint::Secret)],
            Box::new([]),
        )?;
        ensure_equal(value, SlotValue::Bool(true))?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_expr_gt_preserves_secret_taint() -> Result<(), String> {
        let (value, taint) = taint_eval_expr(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::Gt,
            ]
            .into_boxed_slice(),
            vec![ConstValue::I64(3)].into_boxed_slice(),
            vec![(SlotValue::I64(10), Taint::Secret)],
            Box::new([]),
        )?;
        ensure_equal(value, SlotValue::Bool(true))?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_expr_gte_preserves_secret_taint() -> Result<(), String> {
        let (value, taint) = taint_eval_expr(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::Gte,
            ]
            .into_boxed_slice(),
            vec![ConstValue::I64(10)].into_boxed_slice(),
            vec![(SlotValue::I64(10), Taint::Secret)],
            Box::new([]),
        )?;
        ensure_equal(value, SlotValue::Bool(true))?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_expr_lt_preserves_secret_taint() -> Result<(), String> {
        let (value, taint) = taint_eval_expr(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::Lt,
            ]
            .into_boxed_slice(),
            vec![ConstValue::I64(100)].into_boxed_slice(),
            vec![(SlotValue::I64(10), Taint::Secret)],
            Box::new([]),
        )?;
        ensure_equal(value, SlotValue::Bool(true))?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_expr_lte_preserves_secret_taint() -> Result<(), String> {
        let (value, taint) = taint_eval_expr(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::Lte,
            ]
            .into_boxed_slice(),
            vec![ConstValue::I64(10)].into_boxed_slice(),
            vec![(SlotValue::I64(10), Taint::Secret)],
            Box::new([]),
        )?;
        ensure_equal(value, SlotValue::Bool(true))?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_expr_and_preserves_secret_taint() -> Result<(), String> {
        let (value, taint) = taint_eval_expr(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::And,
            ]
            .into_boxed_slice(),
            vec![ConstValue::Bool(true)].into_boxed_slice(),
            vec![(SlotValue::Bool(true), Taint::Secret)],
            Box::new([]),
        )?;
        ensure_equal(value, SlotValue::Bool(true))?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_expr_or_preserves_secret_taint() -> Result<(), String> {
        let (value, taint) = taint_eval_expr(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::Or,
            ]
            .into_boxed_slice(),
            vec![ConstValue::Bool(false)].into_boxed_slice(),
            vec![(SlotValue::Bool(true), Taint::Secret)],
            Box::new([]),
        )?;
        ensure_equal(value, SlotValue::Bool(true))?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_expr_not_preserves_secret_taint() -> Result<(), String> {
        let (value, taint) = taint_eval_expr(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Not].into_boxed_slice(),
            Box::new([]),
            vec![(SlotValue::Bool(false), Taint::Secret)],
            Box::new([]),
        )?;
        ensure_equal(value, SlotValue::Bool(true))?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_expr_two_secret_slots_join_to_secret() -> Result<(), String> {
        let (value, taint) = taint_eval_expr(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadSlot(SlotIdx::new(1)),
                ExprOp::Add,
            ]
            .into_boxed_slice(),
            Box::new([]),
            vec![
                (SlotValue::I64(3), Taint::Secret),
                (SlotValue::I64(4), Taint::Secret),
            ],
            Box::new([]),
        )?;
        ensure_equal(value, SlotValue::I64(7))?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_expr_secret_and_derived_joins_to_derived_from_secret() -> Result<(), String> {
        let (value, taint) = taint_eval_expr(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadSlot(SlotIdx::new(1)),
                ExprOp::Add,
            ]
            .into_boxed_slice(),
            Box::new([]),
            vec![
                (SlotValue::I64(3), Taint::Secret),
                (SlotValue::I64(4), Taint::DerivedFromSecret),
            ],
            Box::new([]),
        )?;
        ensure_equal(value, SlotValue::I64(7))?;
        ensure_equal(taint, Taint::DerivedFromSecret)
    }

    #[test]
    fn eval_expr_clean_and_secret_joins_to_secret() -> Result<(), String> {
        let (value, taint) = taint_eval_expr(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadSlot(SlotIdx::new(1)),
                ExprOp::Add,
            ]
            .into_boxed_slice(),
            Box::new([]),
            vec![
                (SlotValue::I64(3), Taint::Clean),
                (SlotValue::I64(4), Taint::Secret),
            ],
            Box::new([]),
        )?;
        ensure_equal(value, SlotValue::I64(7))?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_expr_all_clean_stays_clean() -> Result<(), String> {
        let (value, taint) = taint_eval_expr(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Add,
            ]
            .into_boxed_slice(),
            vec![ConstValue::I64(3), ConstValue::I64(4)].into_boxed_slice(),
            vec![],
            Box::new([]),
        )?;
        ensure_equal(value, SlotValue::I64(7))?;
        ensure_equal(taint, Taint::Clean)
    }

    // ----- ExprOp operators requiring store (text/list/object) with taint -----

    #[test]
    fn eval_expr_length_preserves_secret_taint_on_list() -> Result<(), String> {
        let mut store = test_store();
        let list = store.insert_list(vec![SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice())
            .map_err(|error| error.to_string())?;
        let (value, taint) = taint_eval_expr_with_store(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Length].into_boxed_slice(),
            Box::new([]),
            vec![(SlotValue::List(list), Taint::Secret)],
            Box::new([]),
            &mut store,
        )?;
        ensure_equal(value, SlotValue::I64(2))?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_expr_empty_preserves_secret_taint_on_list() -> Result<(), String> {
        let mut store = test_store();
        let list = store.insert_list(vec![].into_boxed_slice())
            .map_err(|error| error.to_string())?;
        let (value, taint) = taint_eval_expr_with_store(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Empty].into_boxed_slice(),
            Box::new([]),
            vec![(SlotValue::List(list), Taint::Secret)],
            Box::new([]),
            &mut store,
        )?;
        ensure_equal(value, SlotValue::Bool(true))?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_expr_contains_preserves_secret_taint() -> Result<(), String> {
        let mut store = test_store();
        let haystack = store.insert_symbol("hello world").map_err(|error| error.to_string())?;
        let needle = store.insert_symbol("world").map_err(|error| error.to_string())?;
        let (value, taint) = taint_eval_expr_with_store(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadSlot(SlotIdx::new(1)),
                ExprOp::Contains,
            ]
            .into_boxed_slice(),
            Box::new([]),
            vec![
                (SlotValue::Symbol(haystack), Taint::Secret),
                (SlotValue::Symbol(needle), Taint::Clean),
            ],
            Box::new([]),
            &mut store,
        )?;
        ensure_equal(value, SlotValue::Bool(true))?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_expr_starts_with_preserves_secret_taint() -> Result<(), String> {
        let mut store = test_store();
        let text = store.insert_symbol("hello world").map_err(|error| error.to_string())?;
        let prefix = store.insert_symbol("hello").map_err(|error| error.to_string())?;
        let (value, taint) = taint_eval_expr_with_store(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadSlot(SlotIdx::new(1)),
                ExprOp::StartsWith,
            ]
            .into_boxed_slice(),
            Box::new([]),
            vec![
                (SlotValue::Symbol(text), Taint::Secret),
                (SlotValue::Symbol(prefix), Taint::Clean),
            ],
            Box::new([]),
            &mut store,
        )?;
        ensure_equal(value, SlotValue::Bool(true))?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_expr_ends_with_preserves_secret_taint() -> Result<(), String> {
        let mut store = test_store();
        let text = store.insert_symbol("hello world").map_err(|error| error.to_string())?;
        let suffix = store.insert_symbol("world").map_err(|error| error.to_string())?;
        let (value, taint) = taint_eval_expr_with_store(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadSlot(SlotIdx::new(1)),
                ExprOp::EndsWith,
            ]
            .into_boxed_slice(),
            Box::new([]),
            vec![
                (SlotValue::Symbol(text), Taint::Secret),
                (SlotValue::Symbol(suffix), Taint::Clean),
            ],
            Box::new([]),
            &mut store,
        )?;
        ensure_equal(value, SlotValue::Bool(true))?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_expr_has_preserves_secret_taint() -> Result<(), String> {
        let mut store = test_store();
        let list = store.insert_list(vec![SlotValue::I64(10), SlotValue::I64(20)].into_boxed_slice())
            .map_err(|error| error.to_string())?;
        let (value, taint) = taint_eval_expr_with_store(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::Has,
            ]
            .into_boxed_slice(),
            vec![ConstValue::I64(20)].into_boxed_slice(),
            vec![(SlotValue::List(list), Taint::Secret)],
            Box::new([]),
            &mut store,
        )?;
        ensure_equal(value, SlotValue::Bool(true))?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_expr_exists_preserves_secret_taint() -> Result<(), String> {
        let mut store = test_store();
        let sym = store.insert_symbol("key").map_err(|error| error.to_string())?;
        let obj = store.insert_object(
            vec![ObjectField { key: sym, value: SlotValue::Bool(true) }].into_boxed_slice(),
        )
        .map_err(|error| error.to_string())?;
        let (value, taint) = taint_eval_expr_with_store(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Exists].into_boxed_slice(),
            Box::new([]),
            vec![(SlotValue::Object(obj), Taint::Secret)],
            Box::new([]),
            &mut store,
        )?;
        ensure_equal(value, SlotValue::Bool(true))?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_expr_sum_preserves_secret_taint() -> Result<(), String> {
        let mut store = test_store();
        let list = store.insert_list(vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)].into_boxed_slice())
            .map_err(|error| error.to_string())?;
        let (value, taint) = taint_eval_expr_with_store(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Sum].into_boxed_slice(),
            Box::new([]),
            vec![(SlotValue::List(list), Taint::Secret)],
            Box::new([]),
            &mut store,
        )?;
        ensure_equal(value, SlotValue::I64(6))?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_expr_count_preserves_secret_taint() -> Result<(), String> {
        let mut store = test_store();
        let list = store.insert_list(vec![SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice())
            .map_err(|error| error.to_string())?;
        let (value, taint) = taint_eval_expr_with_store(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Count].into_boxed_slice(),
            Box::new([]),
            vec![(SlotValue::List(list), Taint::Secret)],
            Box::new([]),
            &mut store,
        )?;
        ensure_equal(value, SlotValue::I64(2))?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_expr_append_preserves_secret_taint() -> Result<(), String> {
        let mut store = test_store();
        let list = store.insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
            .map_err(|error| error.to_string())?;
        let (value, taint) = taint_eval_expr_with_store(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::Append,
            ]
            .into_boxed_slice(),
            vec![ConstValue::I64(2)].into_boxed_slice(),
            vec![(SlotValue::List(list), Taint::Secret)],
            Box::new([]),
            &mut store,
        )?;
        ensure_equal(value, SlotValue::List(ListId::new(1)))?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_expr_append_if_preserves_secret_taint() -> Result<(), String> {
        let mut store = test_store();
        let list = store.insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
            .map_err(|error| error.to_string())?;
        let (value, taint) = taint_eval_expr_with_store(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::AppendIf,
            ]
            .into_boxed_slice(),
            vec![ConstValue::I64(2), ConstValue::Bool(true)].into_boxed_slice(),
            vec![(SlotValue::List(list), Taint::Secret)],
            Box::new([]),
            &mut store,
        )?;
        ensure_equal(value, SlotValue::List(ListId::new(1)))?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_expr_unique_preserves_secret_taint() -> Result<(), String> {
        let mut store = test_store();
        let list = store.insert_list(vec![SlotValue::I64(1), SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice())
            .map_err(|error| error.to_string())?;
        let (value, taint) = taint_eval_expr_with_store(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Unique].into_boxed_slice(),
            Box::new([]),
            vec![(SlotValue::List(list), Taint::Secret)],
            Box::new([]),
            &mut store,
        )?;
        ensure_equal(value, SlotValue::List(ListId::new(1)))?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_expr_merge_preserves_secret_taint_from_left() -> Result<(), String> {
        let mut store = test_store();
        let sym1 = store.insert_symbol("a").map_err(|error| error.to_string())?;
        let sym2 = store.insert_symbol("b").map_err(|error| error.to_string())?;
        let obj1 = store.insert_object(
            vec![ObjectField { key: sym1, value: SlotValue::I64(1) }].into_boxed_slice(),
        )
        .map_err(|error| error.to_string())?;
        let obj2 = store.insert_object(
            vec![ObjectField { key: sym2, value: SlotValue::I64(2) }].into_boxed_slice(),
        )
        .map_err(|error| error.to_string())?;
        let (value, taint) = taint_eval_expr_with_store(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadSlot(SlotIdx::new(1)),
                ExprOp::Merge,
            ]
            .into_boxed_slice(),
            Box::new([]),
            vec![
                (SlotValue::Object(obj1), Taint::Secret),
                (SlotValue::Object(obj2), Taint::Clean),
            ],
            Box::new([]),
            &mut store,
        )?;
        ensure_equal(value, SlotValue::Object(ObjectId::new(2)))?;
        ensure_equal(taint, Taint::Secret)
    }

    // ----- Accessor taint propagation -----

    #[test]
    fn accessor_load_propagates_root_secret_taint() -> Result<(), String> {
        let expression = ExprProgram::try_from_ops(
            vec![ExprOp::LoadAccessor(AccessorIdx::new(0))].into_boxed_slice(),
        )
        .map_err(|error| error.to_string())?;
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("accessor_taint"),
            digest: WorkflowDigest::from_bytes([0x56; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::EvalExpr {
                        expr: ExprIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(1),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: vec![expression].into_boxed_slice(),
            accessors: vec![AccessorProgram {
                root: SlotIdx::new(0),
                path: Box::new([]),
            }]
            .into_boxed_slice(),
            constants: Box::new([]),
            slot_count: 2,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        })
        .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(350), &workflow)?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(77), Taint::Secret)
            .map_err(|error| error.to_string())?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);
        match result {
            Ok(EngineSignal::Finished(SlotValue::I64(77), Taint::Secret)) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn accessor_with_object_field_propagates_secret_taint() -> Result<(), String> {
        let expression = ExprProgram::try_from_ops(
            vec![ExprOp::LoadAccessor(AccessorIdx::new(0))].into_boxed_slice(),
        )
        .map_err(|error| error.to_string())?;
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("accessor_field_taint"),
            digest: WorkflowDigest::from_bytes([0x57; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(2)),
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::EvalExpr {
                        expr: ExprIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(2),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: vec![expression].into_boxed_slice(),
            accessors: vec![AccessorProgram {
                root: SlotIdx::new(0),
                path: vec![PathSegment::Field(SymbolId::new(7))].into_boxed_slice(),
            }]
            .into_boxed_slice(),
            constants: Box::new([]),
            slot_count: 3,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        })
        .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(351), &workflow)?;
        let mut store = test_store();
        let obj = store
            .insert_object(
                vec![ObjectField {
                    key: SymbolId::new(7),
                    value: SlotValue::I64(123),
                }]
                .into_boxed_slice(),
            )
            .map_err(|error| error.to_string())?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(obj), Taint::Secret)
            .map_err(|error| error.to_string())?;

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);
        match result {
            Ok(EngineSignal::Finished(SlotValue::I64(123), Taint::Secret)) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // ----- Jump node: no taint effect -----

    #[test]
    fn jump_node_preserves_existing_slot_taint() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("jump_test"),
            digest: WorkflowDigest::from_bytes([0x58; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Jump {
                        target: StepIdx::new(1),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(42)].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(360), &workflow)?;
        let mut store = test_store();
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(42), Taint::Secret)
            .map_err(|error| error.to_string())?;

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);
        match result {
            Ok(EngineSignal::Finished(SlotValue::I64(42), Taint::Secret)) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // ----- Nop node: no taint effect -----

    #[test]
    fn nop_node_preserves_existing_slot_taint() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("nop_taint_test"),
            digest: WorkflowDigest::from_bytes([0x59; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: Some(StepIdx::new(2)),
                    kind: CompiledNodeKind::Nop,
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(42)].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(361), &workflow)?;
        let mut store = test_store();

        let s0 = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
        ensure_equal(s0, EngineSignal::Continue)?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(42), Taint::Secret)
            .map_err(|error| error.to_string())?;
        let s1 = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
        ensure_equal(s1, EngineSignal::Continue)?;
        ensure_equal(run.read_taint(SlotIdx::new(0)), Ok(Taint::Secret))?;
        let s2 = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
        match s2 {
            EngineSignal::Finished(SlotValue::I64(42), Taint::Secret) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // ----- Choose nodes: control flow only, taint does not leak into branch selection -----

    #[test]
    fn choose_slot_does_not_propagate_condition_taint_into_result() -> Result<(), String> {
        let workflow = choose_slot_workflow().map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(370), &workflow)?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Bool(true), Taint::Secret)
            .map_err(|error| error.to_string())?;
        run.write_slot_with_taint(SlotIdx::new(1), SlotValue::Bool(false), Taint::Clean)
            .map_err(|error| error.to_string())?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|error| error.to_string())?;
        match result {
            EngineSignal::Finished(SlotValue::I64(11), Taint::Clean) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn choose_expr_does_not_propagate_condition_taint_into_result() -> Result<(), String> {
        let workflow = choose_expr_workflow().map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(371), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|error| error.to_string())?;
        match result {
            EngineSignal::Finished(SlotValue::I64(11), Taint::Clean) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // ----- Do (Action) node: suspends, no taint propagation at this stage -----

    #[test]
    fn do_node_preserves_input_slot_taint_on_suspend() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("do_taint_test"),
            digest: WorkflowDigest::from_bytes([0x5A; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: None,
                kind: CompiledNodeKind::Do {
                    action: ActionId::new(1),
                    input: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(42)].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(380), &workflow)?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(42), Taint::Secret)
            .map_err(|error| error.to_string())?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);
        ensure_equal(result, Ok(EngineSignal::AwaitingAction))?;
        ensure_equal(run.read_taint(SlotIdx::new(0)), Ok(Taint::Secret))?;
        Ok(())
    }

    // ----- Ask node: suspends, no secret input -----

    #[test]
    fn ask_node_suspends_without_taint_effect() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("ask_taint_test"),
            digest: WorkflowDigest::from_bytes([0x5B; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                kind: CompiledNodeKind::Ask {
                    prompt: SlotIdx::new(0),
                    timeout_slot: None,
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(381), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);
        ensure_equal(result, Ok(EngineSignal::AwaitingAsk))?;
        Ok(())
    }

    // ----- BuildObject and BuildList taint join with mixed taint levels -----

    #[test]
    fn build_object_with_clean_and_derived_produces_derived_taint() -> Result<(), String> {
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("build_obj_mixed_taint"),
            digest: WorkflowDigest::from_bytes([0x5C; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(2)),
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(1),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: Some(SlotIdx::new(2)),
                    next: Some(StepIdx::new(3)),
                    kind: CompiledNodeKind::BuildObject {
                        fields: vec![
                            (SymbolId::new(1), SlotIdx::new(0)),
                            (SymbolId::new(2), SlotIdx::new(1)),
                        ]
                        .into_boxed_slice(),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(3),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(2),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(10), ConstValue::I64(20)].into_boxed_slice(),
            slot_count: 3,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        })
        .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(390), &workflow)?;
        let mut store = test_store();

        let s0 = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
        ensure_equal(s0, EngineSignal::Continue)?;
        let s1 = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
        ensure_equal(s1, EngineSignal::Continue)?;
        run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(20), Taint::DerivedFromSecret)
            .map_err(|error| error.to_string())?;
        let s2 = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
        ensure_equal(s2, EngineSignal::Continue)?;
        ensure_equal(run.read_taint(SlotIdx::new(2)), Ok(Taint::DerivedFromSecret))?;
        let s3 = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
        match s3 {
            EngineSignal::Finished(SlotValue::Object(_), Taint::DerivedFromSecret) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn build_list_with_clean_and_secret_produces_secret_taint() -> Result<(), String> {
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("build_list_mixed_taint"),
            digest: WorkflowDigest::from_bytes([0x5D; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(2)),
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(1),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: Some(SlotIdx::new(2)),
                    next: Some(StepIdx::new(3)),
                    kind: CompiledNodeKind::BuildList {
                        items: vec![SlotIdx::new(0), SlotIdx::new(1)].into_boxed_slice(),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(3),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(2),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(11), ConstValue::I64(22)].into_boxed_slice(),
            slot_count: 3,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        })
        .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(391), &workflow)?;
        let mut store = test_store();

        let s0 = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
        ensure_equal(s0, EngineSignal::Continue)?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(11), Taint::Secret)
            .map_err(|error| error.to_string())?;
        let s1 = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
        ensure_equal(s1, EngineSignal::Continue)?;
        let s2 = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
        ensure_equal(s2, EngineSignal::Continue)?;
        ensure_equal(run.read_taint(SlotIdx::new(2)), Ok(Taint::Secret))?;
        let s3 = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
        match s3 {
            EngineSignal::Finished(SlotValue::List(_), Taint::Secret) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // ----- Multi-step expression: joins from multiple secret slots -----

    #[test]
    fn eval_expr_complex_expression_joins_multiple_secret_slots() -> Result<(), String> {
        let (value, taint) = taint_eval_expr(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadSlot(SlotIdx::new(1)),
                ExprOp::Add,
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::Mul,
            ]
            .into_boxed_slice(),
            vec![ConstValue::I64(2)].into_boxed_slice(),
            vec![
                (SlotValue::I64(3), Taint::Secret),
                (SlotValue::I64(4), Taint::Secret),
            ],
            Box::new([]),
        )?;
        ensure_equal(value, SlotValue::I64(14))?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_expr_complex_expression_joins_secret_and_derived() -> Result<(), String> {
        let (value, taint) = taint_eval_expr(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadSlot(SlotIdx::new(1)),
                ExprOp::Add,
            ]
            .into_boxed_slice(),
            Box::new([]),
            vec![
                (SlotValue::I64(3), Taint::Secret),
                (SlotValue::I64(4), Taint::DerivedFromSecret),
            ],
            Box::new([]),
        )?;
        ensure_equal(value, SlotValue::I64(7))?;
        ensure_equal(taint, Taint::DerivedFromSecret)
    }
}
