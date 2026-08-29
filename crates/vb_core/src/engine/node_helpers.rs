#![forbid(unsafe_code)]
//! Node execution helper functions.

use crate::errors::EngineError;
use crate::ids::{ConstIdx, SlotIdx, StepIdx};
use crate::workflow::CompiledWorkflow;

#[inline]
pub(super) fn set_const(
    plan: &CompiledWorkflow,
    run: &mut crate::frame::RunFrame,
    node: &crate::workflow::CompiledNode,
    value: ConstIdx,
) -> Result<EngineSignal, EngineError> {
    let constant = plan
        .constant(value)
        .copied()
        .ok_or(EngineError::ConstOutOfBounds { index: value })?;
    let slot_value = constant.to_slot_value()?;
    let output = node
        .output
        .ok_or(EngineError::MissingOutputSlot { step: node.id })?;
    run.write_slot(output, slot_value)?;
    jump_to_next(run, node.next, node.id)
}

#[inline]
pub(super) fn copy_slot(
    run: &mut crate::frame::RunFrame,
    node: &crate::workflow::CompiledNode,
    source: SlotIdx,
) -> Result<EngineSignal, EngineError> {
    let value = *run.read_slot(source)?;
    let taint = run.read_taint(source)?;
    let output = node
        .output
        .ok_or(EngineError::MissingOutputSlot { step: node.id })?;
    run.write_slot_with_taint(output, value, taint)?;
    jump_to_next(run, node.next, node.id)
}

#[inline]
pub(super) fn jump_to_next(
    run: &mut crate::frame::RunFrame,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<EngineSignal, EngineError> {
    let next = next.ok_or(EngineError::MissingNextStep { step })?;
    jump_to(run, next)
}

#[inline]
pub(super) fn jump_to(
    run: &mut crate::frame::RunFrame,
    target: StepIdx,
) -> Result<EngineSignal, EngineError> {
    run.set_pc(target)?;
    run.increment_executed()?;
    Ok(EngineSignal::Continue)
}

#[inline]
pub(super) fn finish_run(
    run: &mut crate::frame::RunFrame,
    result: SlotIdx,
) -> Result<EngineSignal, EngineError> {
    let value = *run.read_slot(result)?;
    let taint = run.read_taint(result)?;
    run.increment_executed()?;
    Ok(EngineSignal::Finished(value, taint))
}

use crate::EngineSignal;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
    use crate::value::{ConstValue, SlotValue, Taint};
    use crate::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, WorkflowParts};

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

    fn test_frame(step_count: u16, slot_count: u16) -> Result<crate::frame::RunFrame, String> {
        crate::frame::RunFrame::new(RunId::new(1), StepIdx::new(0), step_count, slot_count)
            .map_err(|e| e.to_string())
    }

    fn minimal_plan_with_const(value: ConstValue) -> Result<CompiledWorkflow, String> {
        CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("node_helpers_test"),
            digest: WorkflowDigest::from_bytes([0xAA; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![value].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
            step_names: Box::new([]),
        input_slots: Box::new([]),        })
        .map_err(|e| e.to_string())
    }

    // ===== jump_to tests =====

    #[test]
    fn jump_to_sets_pc_and_returns_continue() -> Result<(), String> {
        let mut run = test_frame(3, 1)?;
        let result = jump_to(&mut run, StepIdx::new(2)).map_err(|e| e.to_string())?;
        ensure_equal(result, EngineSignal::Continue)?;
        ensure_equal(run.pc(), StepIdx::new(2))?;
        ensure_equal(run.executed(), 1)
    }

    #[test]
    fn jump_to_rejects_out_of_bounds_target() -> Result<(), String> {
        let mut run = test_frame(2, 1)?;
        let result = jump_to(&mut run, StepIdx::new(99));
        match result {
            Err(EngineError::InvalidProgramCounter { step }) if step == StepIdx::new(99) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // ===== jump_to_next tests =====

    #[test]
    fn jump_to_next_advances_to_next_step() -> Result<(), String> {
        let mut run = test_frame(3, 1)?;
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let result = jump_to_next(&mut run, node.next, node.id).map_err(|e| e.to_string())?;
        ensure_equal(result, EngineSignal::Continue)?;
        ensure_equal(run.pc(), StepIdx::new(1))
    }

    #[test]
    fn jump_to_next_rejects_missing_next() -> Result<(), String> {
        let mut run = test_frame(3, 1)?;
        let result = jump_to_next(&mut run, None, StepIdx::new(0));
        match result {
            Err(EngineError::MissingNextStep { step }) if step == StepIdx::new(0) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // ===== finish_run tests =====

    #[test]
    fn finish_run_returns_finished_signal_with_slot_value() -> Result<(), String> {
        let mut run = test_frame(2, 2)?;
        run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(55), Taint::Clean)
            .map_err(|e| e.to_string())?;
        let result = finish_run(&mut run, SlotIdx::new(1)).map_err(|e| e.to_string())?;
        ensure_equal(
            result,
            EngineSignal::Finished(SlotValue::I64(55), Taint::Clean),
        )?;
        ensure_equal(run.executed(), 1)
    }

    #[test]
    fn finish_run_propagates_secret_taint() -> Result<(), String> {
        let mut run = test_frame(2, 2)?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Bool(true), Taint::Secret)
            .map_err(|e| e.to_string())?;
        let result = finish_run(&mut run, SlotIdx::new(0)).map_err(|e| e.to_string())?;
        ensure_equal(
            result,
            EngineSignal::Finished(SlotValue::Bool(true), Taint::Secret),
        )
    }

    #[test]
    fn finish_run_rejects_uninitialized_slot() -> Result<(), String> {
        let mut run = test_frame(2, 2)?;
        let result = finish_run(&mut run, SlotIdx::new(1));
        match result {
            Err(EngineError::SlotUninitialized { slot }) if slot == SlotIdx::new(1) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // ===== set_const tests =====

    #[test]
    fn set_const_writes_constant_to_output_slot() -> Result<(), String> {
        let plan = minimal_plan_with_const(ConstValue::I64(99))?;
        let mut run = test_frame(2, 1)?;
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let result =
            set_const(&plan, &mut run, &node, ConstIdx::new(0)).map_err(|e| e.to_string())?;
        ensure_equal(result, EngineSignal::Continue)?;
        ensure_equal(
            *run.read_slot(SlotIdx::new(0)).map_err(|e| e.to_string())?,
            SlotValue::I64(99),
        )
    }

    #[test]
    fn set_const_rejects_out_of_bounds_constant() -> Result<(), String> {
        let plan = minimal_plan_with_const(ConstValue::I64(1))?;
        let mut run = test_frame(2, 1)?;
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(5),
            },
        };
        let result = set_const(&plan, &mut run, &node, ConstIdx::new(5));
        match result {
            Err(EngineError::ConstOutOfBounds { index }) if index == ConstIdx::new(5) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // ===== copy_slot tests =====

    #[test]
    fn copy_slot_copies_value_and_taint() -> Result<(), String> {
        let mut run = test_frame(3, 3)?;
        run.write_slot_with_taint(
            SlotIdx::new(0),
            SlotValue::I64(77),
            Taint::DerivedFromSecret,
        )
        .map_err(|e| e.to_string())?;
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(0),
            },
        };
        let result = copy_slot(&mut run, &node, SlotIdx::new(0)).map_err(|e| e.to_string())?;
        ensure_equal(result, EngineSignal::Continue)?;
        ensure_equal(
            *run.read_slot(SlotIdx::new(1)).map_err(|e| e.to_string())?,
            SlotValue::I64(77),
        )?;
        ensure_equal(
            run.read_taint(SlotIdx::new(1)).map_err(|e| e.to_string())?,
            Taint::DerivedFromSecret,
        )
    }

    #[test]
    fn copy_slot_rejects_uninitialized_source() -> Result<(), String> {
        let mut run = test_frame(3, 3)?;
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(2),
            },
        };
        let result = copy_slot(&mut run, &node, SlotIdx::new(2));
        match result {
            Err(EngineError::SlotUninitialized { slot }) if slot == SlotIdx::new(2) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }
}
