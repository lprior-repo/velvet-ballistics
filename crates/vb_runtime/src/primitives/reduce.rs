//! Reduce accumulation primitive handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{ConstIdx, SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledWorkflow;

/// Executes ReduceStart: initializes accumulator from constant pool,
/// reads input list, binds first item, writes remaining tail to
/// iterator slot.
#[allow(clippy::too_many_arguments)]
pub fn reduce_start(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    input: SlotIdx,
    accumulator: SlotIdx,
    initial: ConstIdx,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    let init_value = plan
        .constant(initial)
        .copied()
        .ok_or(EngineError::ConstOutOfBounds { index: initial })?
        .to_slot_value()?;
    run.write_slot(accumulator, init_value)?;
    let list_id = expect_list(*run.read_slot(input)?)?;
    let items = store.list(list_id)?;
    if items.is_empty() {
        return jump_to(run, done);
    }
    let iter_output = output.ok_or(EngineError::MissingOutputSlot { step: run.pc() })?;
    let first = items
        .first()
        .copied()
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "reduce items checked nonempty",
        })?;
    run.write_slot(input, first)?;
    let tail = tail_items(items)?;
    let tail_id = store.insert_list(tail)?;
    run.write_slot(iter_output, SlotValue::List(tail_id))?;
    jump_to(run, body)
}

/// Executes ReduceNext: advances iterator, binds next item to
/// iterator slot, or exits when all items consumed.
pub fn reduce_next(
    run: &mut RunFrame,
    store: &mut ValueStore,
    iterator_slot: SlotIdx,
    _accumulator: SlotIdx,
    body: StepIdx,
    done: StepIdx,
    output: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    let remaining_id = expect_list(*run.read_slot(iterator_slot)?)?;
    let remaining = store.list(remaining_id)?;
    if remaining.is_empty() {
        return jump_to(run, done);
    }
    let item_output = output.ok_or(EngineError::MissingOutputSlot { step: run.pc() })?;
    let first = remaining
        .first()
        .copied()
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "reduce next items checked nonempty",
        })?;
    run.write_slot(item_output, first)?;
    let tail = tail_items(remaining)?;
    let tail_id = store.insert_list(tail)?;
    run.write_slot(iterator_slot, SlotValue::List(tail_id))?;
    jump_to(run, body)
}

/// Executes ReduceFinish: writes the final accumulator to output.
pub fn reduce_finish(
    run: &mut RunFrame,
    accumulator: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let value = *run.read_slot(accumulator)?;
    let out = output.ok_or(EngineError::MissingOutputSlot { step })?;
    run.write_slot(out, value)?;
    jump_to_next(run, next, step)
}

fn expect_list(value: SlotValue) -> Result<vb_core::ids::ListId, EngineError> {
    match value {
        SlotValue::List(id) => Ok(id),
        other => Err(EngineError::TypeMismatch {
            expected: "list",
            found: other.type_name(),
        }),
    }
}

fn tail_items(items: &[SlotValue]) -> Result<Box<[SlotValue]>, EngineError> {
    if items.len() <= 1 {
        return Ok(Vec::<SlotValue>::new().into_boxed_slice());
    }
    Ok(items
        .get(1..)
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "tail_items start index checked",
        })?
        .to_vec()
        .into_boxed_slice())
}

fn jump_to(run: &mut RunFrame, target: StepIdx) -> Result<vb_core::EngineSignal, EngineError> {
    run.set_pc(target)?;
    run.increment_executed()?;
    Ok(vb_core::EngineSignal::Continue)
}

fn jump_to_next(
    run: &mut RunFrame,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let target = next.ok_or(EngineError::MissingNextStep { step })?;
    jump_to(run, target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::RunId;
    use vb_core::value::{ConstValue, SlotValue};
    use vb_core::value_store::ValueStore;
    use vb_core::workflow::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
    };

    fn fresh_frame() -> RunFrame {
        RunFrame::new(RunId::new(1), StepIdx::ZERO, 4, 8)
            .ok()
            .unwrap_or_else(|| panic!("frame creation must succeed"))
    }

    fn minimal_workflow_with_constant(cv: ConstValue) -> CompiledWorkflow {
        let parts = WorkflowParts {
            name: Box::from("reduce_test"),
            digest: vb_core::ids::WorkflowDigest::from_bytes([5; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::ZERO,
                output: None,
                next: None,
                kind: CompiledNodeKind::Nop,
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![cv].into_boxed_slice(),
            slot_count: 8,
            entry: StepIdx::ZERO,
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts)
            .ok()
            .unwrap_or_else(|| panic!("workflow must compile"))
    }

    fn list_in_slot(
        run: &mut RunFrame,
        store: &mut ValueStore,
        slot: SlotIdx,
        items: Vec<SlotValue>,
    ) {
        let id = store
            .insert_list(items.into_boxed_slice())
            .ok()
            .unwrap_or_else(|| panic!("list insertion must succeed"));
        run.write_slot(slot, SlotValue::List(id))
            .ok()
            .unwrap_or_else(|| panic!("slot write must succeed"));
    }

    #[test]
    fn reduce_start_writes_initial_accumulator() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let plan = minimal_workflow_with_constant(ConstValue::I64(100));
        let input = SlotIdx::new(0);
        let accumulator = SlotIdx::new(1);
        let output = SlotIdx::new(2);
        let initial = ConstIdx::new(0);
        let body = StepIdx::new(1);
        let done = StepIdx::new(2);
        list_in_slot(
            &mut run,
            &mut store,
            input,
            vec![SlotValue::I64(1), SlotValue::I64(2)],
        );

        let result = reduce_start(
            &plan,
            &mut run,
            &mut store,
            input,
            accumulator,
            initial,
            body,
            done,
            Some(output),
        );

        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), body);
        assert_eq!(
            *run.read_slot(accumulator)
                .ok()
                .unwrap_or_else(|| panic!("read must succeed")),
            SlotValue::I64(100)
        );
    }

    #[test]
    fn reduce_next_applies_item_to_accumulator() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let iterator_slot = SlotIdx::new(0);
        let accumulator = SlotIdx::new(1);
        let output = SlotIdx::new(2);
        let body = StepIdx::new(1);
        let done = StepIdx::new(2);
        list_in_slot(
            &mut run,
            &mut store,
            iterator_slot,
            vec![SlotValue::I64(5), SlotValue::I64(6)],
        );

        let result = reduce_next(
            &mut run,
            &mut store,
            iterator_slot,
            accumulator,
            body,
            done,
            Some(output),
        );

        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), body);
        assert_eq!(
            *run.read_slot(output)
                .ok()
                .unwrap_or_else(|| panic!("read must succeed")),
            SlotValue::I64(5)
        );
    }

    #[test]
    fn reduce_finish_writes_final_accumulator_to_output() {
        let mut run = fresh_frame();
        let accumulator = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        let next_step = StepIdx::new(3);
        run.write_slot(accumulator, SlotValue::I64(42))
            .ok()
            .unwrap_or_else(|| panic!("slot write must succeed"));

        let result = reduce_finish(
            &mut run,
            accumulator,
            Some(output),
            Some(next_step),
            StepIdx::ZERO,
        );

        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), next_step);
        assert_eq!(
            *run.read_slot(output)
                .ok()
                .unwrap_or_else(|| panic!("read must succeed")),
            SlotValue::I64(42)
        );
    }

    // BDD tests for reduce primitives

    #[test]
    fn reduce_start_returns_error_when_input_is_not_list() {
        // Given a frame with a non-list in input slot
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let plan = minimal_workflow_with_constant(ConstValue::I64(0));
        run.write_slot(SlotIdx::new(0), SlotValue::Bool(true))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        // When calling reduce_start
        let result = reduce_start(
            &plan,
            &mut run,
            &mut store,
            SlotIdx::new(0),
            SlotIdx::new(1),
            ConstIdx::new(0),
            StepIdx::new(1),
            StepIdx::new(2),
            Some(SlotIdx::new(2)),
        );
        // Then it returns TypeMismatch
        match result {
            Err(EngineError::TypeMismatch { expected, found }) => {
                assert_eq!(expected, "list");
                assert_eq!(found, "boolean");
            }
            other => {
                assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
            }
        }
    }

    #[test]
    fn reduce_start_returns_error_when_output_missing() {
        // Given a frame with a list but no output slot
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let plan = minimal_workflow_with_constant(ConstValue::I64(0));
        list_in_slot(
            &mut run,
            &mut store,
            SlotIdx::new(0),
            vec![SlotValue::I64(1)],
        );
        // When calling reduce_start with output=None
        let result = reduce_start(
            &plan,
            &mut run,
            &mut store,
            SlotIdx::new(0),
            SlotIdx::new(1),
            ConstIdx::new(0),
            StepIdx::new(1),
            StepIdx::new(2),
            None,
        );
        // Then it returns MissingOutputSlot
        match result {
            Err(EngineError::MissingOutputSlot { step }) => {
                assert_eq!(step, StepIdx::ZERO);
            }
            other => {
                assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
            }
        }
    }

    #[test]
    fn reduce_start_jumps_to_done_when_list_empty() {
        // Given a frame with an empty list
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let plan = minimal_workflow_with_constant(ConstValue::I64(100));
        let done = StepIdx::new(3);
        list_in_slot(&mut run, &mut store, SlotIdx::new(0), vec![]);
        // When calling reduce_start
        let result = reduce_start(
            &plan,
            &mut run,
            &mut store,
            SlotIdx::new(0),
            SlotIdx::new(1),
            ConstIdx::new(0),
            StepIdx::new(1),
            done,
            Some(SlotIdx::new(2)),
        );
        // Then it jumps to done
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), done);
    }

    #[test]
    fn reduce_start_returns_error_when_constant_missing() {
        // Given a frame with a list but invalid constant index
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let plan = minimal_workflow_with_constant(ConstValue::I64(0));
        list_in_slot(
            &mut run,
            &mut store,
            SlotIdx::new(0),
            vec![SlotValue::I64(1)],
        );
        // When calling reduce_start with ConstIdx out of bounds
        let result = reduce_start(
            &plan,
            &mut run,
            &mut store,
            SlotIdx::new(0),
            SlotIdx::new(1),
            ConstIdx::new(99),
            StepIdx::new(1),
            StepIdx::new(2),
            Some(SlotIdx::new(2)),
        );
        // Then it returns ConstOutOfBounds
        match result {
            Err(EngineError::ConstOutOfBounds { index }) => {
                assert_eq!(index, ConstIdx::new(99));
            }
            other => {
                assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
            }
        }
    }

    #[test]
    fn reduce_next_binds_first_remaining_item() {
        // Given a frame with remaining items in iterator
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let iterator_slot = SlotIdx::new(0);
        let output = SlotIdx::new(2);
        let body = StepIdx::new(1);
        list_in_slot(
            &mut run,
            &mut store,
            iterator_slot,
            vec![SlotValue::I64(42), SlotValue::I64(99)],
        );
        // When calling reduce_next
        let result = reduce_next(
            &mut run,
            &mut store,
            iterator_slot,
            SlotIdx::new(1),
            body,
            StepIdx::new(2),
            Some(output),
        );
        // Then output has first remaining item
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(
            *run.read_slot(output)
                .ok()
                .unwrap_or_else(|| panic!("read must succeed")),
            SlotValue::I64(42)
        );
    }

    #[test]
    fn reduce_next_jumps_to_done_when_remaining_empty() {
        // Given a frame with empty remaining items
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let iterator_slot = SlotIdx::new(0);
        let done = StepIdx::new(3);
        list_in_slot(&mut run, &mut store, iterator_slot, vec![]);
        // When calling reduce_next
        let result = reduce_next(
            &mut run,
            &mut store,
            iterator_slot,
            SlotIdx::new(1),
            StepIdx::new(1),
            done,
            Some(SlotIdx::new(2)),
        );
        // Then it jumps to done
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), done);
    }

    #[test]
    fn reduce_finish_returns_error_when_output_missing() {
        // Given a frame with accumulator
        let mut run = fresh_frame();
        run.write_slot(SlotIdx::new(0), SlotValue::I64(42))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        // When calling reduce_finish with output=None
        let result = reduce_finish(
            &mut run,
            SlotIdx::new(0),
            None,
            Some(StepIdx::new(1)),
            StepIdx::ZERO,
        );
        // Then it returns MissingOutputSlot
        match result {
            Err(EngineError::MissingOutputSlot { step }) => {
                assert_eq!(step, StepIdx::ZERO);
            }
            other => {
                assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
            }
        }
    }

    #[test]
    fn reduce_finish_returns_error_when_next_missing() {
        // Given a frame
        let mut run = fresh_frame();
        run.write_slot(SlotIdx::new(0), SlotValue::I64(42))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        // When calling reduce_finish with next=None
        let result = reduce_finish(
            &mut run,
            SlotIdx::new(0),
            Some(SlotIdx::new(1)),
            None,
            StepIdx::ZERO,
        );
        // Then it returns MissingNextStep
        match result {
            Err(EngineError::MissingNextStep { step }) => {
                assert_eq!(step, StepIdx::ZERO);
            }
            other => {
                assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
            }
        }
    }

    #[test]
    fn reduce_start_writes_initial_accumulator_from_constant() {
        // Given a frame with a list and constant value 100
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let plan = minimal_workflow_with_constant(ConstValue::I64(100));
        let accumulator = SlotIdx::new(1);
        list_in_slot(
            &mut run,
            &mut store,
            SlotIdx::new(0),
            vec![SlotValue::I64(1), SlotValue::I64(2)],
        );
        // When calling reduce_start
        let result = reduce_start(
            &plan,
            &mut run,
            &mut store,
            SlotIdx::new(0),
            accumulator,
            ConstIdx::new(0),
            StepIdx::new(1),
            StepIdx::new(2),
            Some(SlotIdx::new(2)),
        );
        // Then accumulator has the initial constant value
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(
            *run.read_slot(accumulator)
                .ok()
                .unwrap_or_else(|| panic!("read must succeed")),
            SlotValue::I64(100)
        );
    }

    #[test]
    fn reduce_next_returns_error_when_output_missing() {
        // Given a frame with items but no output
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let iterator_slot = SlotIdx::new(0);
        list_in_slot(&mut run, &mut store, iterator_slot, vec![SlotValue::I64(1)]);
        // When calling reduce_next with output=None
        let result = reduce_next(
            &mut run,
            &mut store,
            iterator_slot,
            SlotIdx::new(1),
            StepIdx::new(1),
            StepIdx::new(2),
            None,
        );
        // Then it returns MissingOutputSlot
        match result {
            Err(EngineError::MissingOutputSlot { step }) => {
                assert_eq!(step, StepIdx::ZERO);
            }
            other => {
                assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
            }
        }
    }

    #[test]
    fn reduce_start_increments_executed_counter() {
        // Given a frame with a list
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let plan = minimal_workflow_with_constant(ConstValue::I64(0));
        list_in_slot(
            &mut run,
            &mut store,
            SlotIdx::new(0),
            vec![SlotValue::I64(1)],
        );
        let before = run.executed();
        // When calling reduce_start
        let result = reduce_start(
            &plan,
            &mut run,
            &mut store,
            SlotIdx::new(0),
            SlotIdx::new(1),
            ConstIdx::new(0),
            StepIdx::new(1),
            StepIdx::new(2),
            Some(SlotIdx::new(2)),
        );
        // Then executed counter incremented
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.executed(), before + 1);
    }

    #[test]
    fn reduce_next_increments_executed_counter() {
        // Given a frame with remaining items
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let iterator_slot = SlotIdx::new(0);
        list_in_slot(&mut run, &mut store, iterator_slot, vec![SlotValue::I64(1)]);
        let before = run.executed();
        // When calling reduce_next
        let result = reduce_next(
            &mut run,
            &mut store,
            iterator_slot,
            SlotIdx::new(1),
            StepIdx::new(1),
            StepIdx::new(2),
            Some(SlotIdx::new(2)),
        );
        // Then executed counter incremented
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.executed(), before + 1);
    }

    #[test]
    fn reduce_finish_increments_executed_counter() {
        // Given a frame with accumulator
        let mut run = fresh_frame();
        run.write_slot(SlotIdx::new(0), SlotValue::I64(42))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        let before = run.executed();
        // When calling reduce_finish
        let result = reduce_finish(
            &mut run,
            SlotIdx::new(0),
            Some(SlotIdx::new(1)),
            Some(StepIdx::new(3)),
            StepIdx::ZERO,
        );
        // Then executed counter incremented
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.executed(), before + 1);
    }

    #[test]
    fn reduce_next_returns_error_when_not_list() {
        // Given a frame with non-list in iterator slot
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let iterator_slot = SlotIdx::new(0);
        run.write_slot(iterator_slot, SlotValue::Bool(true))
            .ok()
            .unwrap_or_else(|| panic!("write must succeed"));
        // When calling reduce_next
        let result = reduce_next(
            &mut run,
            &mut store,
            iterator_slot,
            SlotIdx::new(1),
            StepIdx::new(1),
            StepIdx::new(2),
            Some(SlotIdx::new(2)),
        );
        // Then it returns TypeMismatch
        match result {
            Err(EngineError::TypeMismatch { expected, found }) => {
                assert_eq!(expected, "list");
                assert_eq!(found, "boolean");
            }
            other => {
                assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
            }
        }
    }

    #[test]
    fn reduce_start_writes_tail_to_iterator_slot() {
        // Given a frame with a 3-item list
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let plan = minimal_workflow_with_constant(ConstValue::I64(0));
        let output = SlotIdx::new(2);
        list_in_slot(
            &mut run,
            &mut store,
            SlotIdx::new(0),
            vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)],
        );
        // When calling reduce_start
        let result = reduce_start(
            &plan,
            &mut run,
            &mut store,
            SlotIdx::new(0),
            SlotIdx::new(1),
            ConstIdx::new(0),
            StepIdx::new(1),
            StepIdx::new(2),
            Some(output),
        );
        // Then the output slot has a tail list with 2 remaining items
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        match *run
            .read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed"))
        {
            SlotValue::List(id) => {
                let items = store
                    .list(id)
                    .ok()
                    .unwrap_or_else(|| panic!("list read must succeed"));
                assert_eq!(items.len(), 2);
            }
            other => {
                assert_eq!(other, SlotValue::I64(0));
            }
        }
    }

    #[test]
    fn reduce_next_writes_tail_to_iterator_slot() {
        // Given a frame with 3 remaining items
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let iterator_slot = SlotIdx::new(0);
        let output = SlotIdx::new(2);
        list_in_slot(
            &mut run,
            &mut store,
            iterator_slot,
            vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
        );
        // When calling reduce_next
        let result = reduce_next(
            &mut run,
            &mut store,
            iterator_slot,
            SlotIdx::new(1),
            StepIdx::new(1),
            StepIdx::new(2),
            Some(output),
        );
        // Then the iterator slot has a tail list with 2 remaining items
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        match *run
            .read_slot(iterator_slot)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed"))
        {
            SlotValue::List(id) => {
                let items = store
                    .list(id)
                    .ok()
                    .unwrap_or_else(|| panic!("list read must succeed"));
                assert_eq!(items.len(), 2);
            }
            other => {
                assert_eq!(other, SlotValue::I64(0));
            }
        }
    }

    // ── Adversarial BDD tests for reduce ────────────────────────────────

    #[test]
    fn reduce_start_empty_list_with_initial_value_jumps_to_done() {
        // Given a frame with empty input list and initial value 100
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let plan = minimal_workflow_with_constant(ConstValue::I64(100));
        let accumulator = SlotIdx::new(1);
        let done = StepIdx::new(3);
        list_in_slot(&mut run, &mut store, SlotIdx::new(0), vec![]);
        // When calling reduce_start
        let result = reduce_start(
            &plan,
            &mut run,
            &mut store,
            SlotIdx::new(0),
            accumulator,
            ConstIdx::new(0),
            StepIdx::new(1),
            done,
            Some(SlotIdx::new(2)),
        );
        // Then it jumps to done with accumulator = 100
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), done);
        assert_eq!(
            *run.read_slot(accumulator)
                .ok()
                .unwrap_or_else(|| panic!("must read")),
            SlotValue::I64(100)
        );
    }

    #[test]
    fn reduce_start_single_item_list_writes_tail_and_jumps_to_body() {
        // Given a frame with single-item input list
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let plan = minimal_workflow_with_constant(ConstValue::I64(0));
        let body = StepIdx::new(1);
        let output = SlotIdx::new(2);
        list_in_slot(
            &mut run,
            &mut store,
            SlotIdx::new(0),
            vec![SlotValue::I64(42)],
        );
        // When calling reduce_start
        let result = reduce_start(
            &plan,
            &mut run,
            &mut store,
            SlotIdx::new(0),
            SlotIdx::new(1),
            ConstIdx::new(0),
            body,
            StepIdx::new(2),
            Some(output),
        );
        // Then it jumps to body and output has an empty tail
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), body);
        match *run
            .read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("must read"))
        {
            SlotValue::List(id) => {
                assert_eq!(
                    store
                        .list(id)
                        .ok()
                        .unwrap_or_else(|| panic!("must read"))
                        .len(),
                    0
                );
            }
            other => {
                assert_eq!(other, SlotValue::I64(0));
            }
        }
    }

    #[test]
    fn reduce_start_first_item_in_input_tail_in_output() {
        // Given a frame with a 2-item list
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let plan = minimal_workflow_with_constant(ConstValue::I64(0));
        let input = SlotIdx::new(0);
        let output = SlotIdx::new(2);
        list_in_slot(
            &mut run,
            &mut store,
            input,
            vec![SlotValue::I64(10), SlotValue::I64(20)],
        );
        // When calling reduce_start
        let result = reduce_start(
            &plan,
            &mut run,
            &mut store,
            input,
            SlotIdx::new(1),
            ConstIdx::new(0),
            StepIdx::new(1),
            StepIdx::new(2),
            Some(output),
        );
        // Then input slot has the first item and output slot has the tail list
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(
            *run.read_slot(input)
                .ok()
                .unwrap_or_else(|| panic!("must read")),
            SlotValue::I64(10)
        );
        match *run
            .read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("must read"))
        {
            SlotValue::List(id) => {
                let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
                assert_eq!(items.len(), 1);
                assert_eq!(items.get(0), Some(&SlotValue::I64(20)));
            }
            other => {
                assert_eq!(other, SlotValue::I64(0));
            }
        }
    }

    #[test]
    fn reduce_start_null_initial_constant_succeeds() {
        // Given a frame with a Null constant and a list
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let plan = minimal_workflow_with_constant(ConstValue::Null);
        let accumulator = SlotIdx::new(1);
        list_in_slot(
            &mut run,
            &mut store,
            SlotIdx::new(0),
            vec![SlotValue::I64(1)],
        );
        // When calling reduce_start with Null initial
        let result = reduce_start(
            &plan,
            &mut run,
            &mut store,
            SlotIdx::new(0),
            accumulator,
            ConstIdx::new(0),
            StepIdx::new(1),
            StepIdx::new(2),
            Some(SlotIdx::new(2)),
        );
        // Then accumulator has Null
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(
            *run.read_slot(accumulator)
                .ok()
                .unwrap_or_else(|| panic!("must read")),
            SlotValue::Null
        );
    }

    #[test]
    fn reduce_start_bool_initial_constant_succeeds() {
        // Given a frame with a Bool constant
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let plan = minimal_workflow_with_constant(ConstValue::Bool(true));
        let accumulator = SlotIdx::new(1);
        list_in_slot(
            &mut run,
            &mut store,
            SlotIdx::new(0),
            vec![SlotValue::I64(1)],
        );
        // When calling reduce_start
        let result = reduce_start(
            &plan,
            &mut run,
            &mut store,
            SlotIdx::new(0),
            accumulator,
            ConstIdx::new(0),
            StepIdx::new(1),
            StepIdx::new(2),
            Some(SlotIdx::new(2)),
        );
        // Then accumulator has Bool(true) -- no type checking on initial vs items
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(
            *run.read_slot(accumulator)
                .ok()
                .unwrap_or_else(|| panic!("must read")),
            SlotValue::Bool(true)
        );
    }

    #[test]
    fn reduce_next_single_remaining_item_produces_empty_tail() {
        // Given a frame with 1 remaining item
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let iterator_slot = SlotIdx::new(0);
        let output = SlotIdx::new(2);
        list_in_slot(
            &mut run,
            &mut store,
            iterator_slot,
            vec![SlotValue::I64(42)],
        );
        // When calling reduce_next
        let result = reduce_next(
            &mut run,
            &mut store,
            iterator_slot,
            SlotIdx::new(1),
            StepIdx::new(1),
            StepIdx::new(2),
            Some(output),
        );
        // Then output has the item and iterator has empty tail
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(
            *run.read_slot(output)
                .ok()
                .unwrap_or_else(|| panic!("must read")),
            SlotValue::I64(42)
        );
        match *run
            .read_slot(iterator_slot)
            .ok()
            .unwrap_or_else(|| panic!("must read"))
        {
            SlotValue::List(id) => {
                assert_eq!(
                    store
                        .list(id)
                        .ok()
                        .unwrap_or_else(|| panic!("must read"))
                        .len(),
                    0
                );
            }
            other => {
                assert_eq!(other, SlotValue::I64(0));
            }
        }
    }

    #[test]
    fn reduce_finish_copies_accumulator_to_output_verbatim() {
        // Given a frame with a non-I64 accumulator (Bool)
        let mut run = fresh_frame();
        let accumulator = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        let next_step = StepIdx::new(3);
        run.write_slot(accumulator, SlotValue::Bool(false))
            .ok()
            .unwrap_or_else(|| panic!("write"));
        // When calling reduce_finish
        let result = reduce_finish(
            &mut run,
            accumulator,
            Some(output),
            Some(next_step),
            StepIdx::ZERO,
        );
        // Then output has the exact accumulator value (no type coercion)
        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(
            *run.read_slot(output)
                .ok()
                .unwrap_or_else(|| panic!("must read")),
            SlotValue::Bool(false)
        );
    }
}
