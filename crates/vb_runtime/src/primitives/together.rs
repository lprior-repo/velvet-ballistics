//! Together parallel-branch primitive handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{ListId, SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;

/// Executes TogetherStart: forks into the first branch.
///
/// In a deterministic synchronous runtime, together branches execute
/// sequentially in declaration order. TogetherStart creates an empty
/// accumulator list in the output slot, then jumps to the first
/// TogetherBranch node.
pub fn together_start(
    run: &mut RunFrame,
    store: &mut ValueStore,
    branches: &[StepIdx],
    _join: StepIdx,
    output: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    let count = u16::try_from(branches.len())
        .map_err(|_| EngineError::TogetherBranchLimitExceeded { max: u16::MAX })?;
    if count == 0 {
        return Err(EngineError::InvalidCompiledWorkflow {
            reason: "together_start requires at least one branch",
        });
    }
    let iter_output = output.ok_or(EngineError::MissingOutputSlot { step: run.pc() })?;
    let state = store.insert_list(Vec::<SlotValue>::new().into_boxed_slice())?;
    run.write_slot(iter_output, SlotValue::List(state))?;
    let first_branch =
        branches
            .first()
            .copied()
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "together branches checked nonzero",
            })?;
    jump_to(run, first_branch)
}

/// Executes TogetherBranch: runs one branch body and appends its result.
///
/// For branches after the first, the output slot holds the previous
/// branch body result (a non-list value written by the branch body).
/// This function reads that result from the output slot, reads the
/// accumulator list from the dedicated accumulator slot, appends the
/// result, writes the updated accumulator list back, then jumps to the
/// branch entry.
///
/// For the first branch, no previous result exists so only the jump
/// occurs.
pub fn together_branch(
    run: &mut RunFrame,
    store: &mut ValueStore,
    branch: u16,
    entry: StepIdx,
    _join: StepIdx,
    accumulator: SlotIdx,
    output: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    if branch > 0 {
        let branch_output = output.ok_or(EngineError::MissingOutputSlot { step: run.pc() })?;
        let previous_result = *run.read_slot(branch_output)?;
        append_to_accumulator(run, store, accumulator, previous_result)?;
    }
    jump_to(run, entry)
}

/// Executes TogetherJoin: waits for all branches and merges results.
///
/// In the synchronous model, all branches have already completed in
/// order. TogetherJoin reads the last branch result from the output
/// slot, appends it to the accumulator list, and writes the final
/// merged list to the output slot.
pub fn together_join(
    run: &mut RunFrame,
    store: &mut ValueStore,
    branch_count: u16,
    accumulator: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let out = output.ok_or(EngineError::MissingOutputSlot { step })?;
    // Read the accumulator list built by together_branch invocations.
    let acc_value = *run.read_slot(accumulator)?;
    let final_list = match acc_value {
        SlotValue::List(id) => {
            // Append the last branch body result if it's not already a list.
            let last_result = *run.read_slot(out)?;
            match last_result {
                SlotValue::List(_) => SlotValue::List(id),
                SlotValue::Null => SlotValue::List(id),
                other => {
                    append_to_accumulator(run, store, accumulator, other)?;
                    *run.read_slot(accumulator)?
                }
            }
        }
        _ => {
            // No accumulator list; use whatever is in the output slot.
            acc_value
        }
    };
    let _ = branch_count;
    run.write_slot(out, final_list)?;
    jump_to_next(run, next, step)
}

/// Reads the current accumulator list from the accumulator slot,
/// appends the given value, and writes the updated list back.
///
/// The accumulator slot must hold a `SlotValue::List`. A new list is
/// allocated in the store containing all existing elements plus the
/// new value, and the updated list handle is written back to the
/// accumulator slot.
fn append_to_accumulator(
    run: &mut RunFrame,
    store: &mut ValueStore,
    accumulator: SlotIdx,
    value: SlotValue,
) -> Result<(), EngineError> {
    let current = *run.read_slot(accumulator)?;
    let list_id = expect_list(current)?;
    let existing = store.list(list_id)?;
    let mut items = existing.to_vec();
    items.push(value);
    let updated = store.insert_list(items.into_boxed_slice())?;
    run.write_slot(accumulator, SlotValue::List(updated))?;
    Ok(())
}

fn expect_list(value: SlotValue) -> Result<ListId, EngineError> {
    match value {
        SlotValue::List(id) => Ok(id),
        other => Err(EngineError::TypeMismatch {
            expected: "list",
            found: other.type_name(),
        }),
    }
}

fn jump_to(run: &mut RunFrame, target: StepIdx) -> Result<vb_core::EngineSignal, EngineError> {
    run.set_pc(target);
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
    use vb_core::value_store::ValueStore;

    fn fresh_frame() -> RunFrame {
        RunFrame::new(RunId::new(1), StepIdx::ZERO, 8, 8).ok().unwrap_or_else(||
            panic!("frame creation must succeed")
        )
    }

    fn list_in_slot(run: &mut RunFrame, store: &mut ValueStore, slot: SlotIdx, items: Vec<SlotValue>) {
        let id = store.insert_list(items.into_boxed_slice()).ok().unwrap_or_else(||
            panic!("list insertion must succeed")
        );
        run.write_slot(slot, SlotValue::List(id)).ok().unwrap_or_else(||
            panic!("slot write must succeed")
        );
    }

    #[test]
    fn together_start_initializes_branch_tracking() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let output = SlotIdx::new(0);
        let branch_a = StepIdx::new(1);
        let join = StepIdx::new(2);

        let result = together_start(
            &mut run,
            &mut store,
            &[branch_a],
            join,
            Some(output),
        );

        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), branch_a);
        let slot_val = *run.read_slot(output).ok().unwrap_or_else(|| panic!("read must succeed"));
        assert!(matches!(slot_val, SlotValue::List(_)));
    }

    #[test]
    fn together_branch_routes_to_entry_step() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let accumulator = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        let entry = StepIdx::new(3);
        let join = StepIdx::new(4);
        list_in_slot(&mut run, &mut store, accumulator, vec![]);

        let result = together_branch(
            &mut run,
            &mut store,
            0,
            entry,
            join,
            accumulator,
            Some(output),
        );

        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), entry);
    }

    #[test]
    fn together_join_waits_for_all_branches() {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let accumulator = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        let next_step = StepIdx::new(5);

        // Set up accumulator with a list containing one branch result.
        list_in_slot(&mut run, &mut store, accumulator, vec![SlotValue::I64(10)]);
        // Output slot holds the last branch result (a non-list value).
        run.write_slot(output, SlotValue::I64(20)).ok().unwrap_or_else(||
            panic!("slot write must succeed")
        );

        let result = together_join(
            &mut run,
            &mut store,
            2,
            accumulator,
            Some(output),
            Some(next_step),
            StepIdx::ZERO,
        );

        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), next_step);
        // Output slot should now hold the final merged list.
        let final_val = *run.read_slot(output).ok().unwrap_or_else(|| panic!("read must succeed"));
        assert!(matches!(final_val, SlotValue::List(_)));
    }
}
