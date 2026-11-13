use vb_core::frame::{CheckpointStepState, RunFrameCheckpointV1};
use vb_core::value_store::ValueStoreCheckpointV1;
use vb_core::{CompiledNodeKind, CompiledWorkflow, SlotValue, StepIdx};

use super::super::support::TestResult;
use super::helpers::{assert_remaining_control_work, checkpoint_list};

pub(super) fn assert_completed_control_rejected(
    workflow: &CompiledWorkflow,
    outer: StepIdx,
    frame: &RunFrameCheckpointV1,
    store: &ValueStoreCheckpointV1,
) -> TestResult {
    let node = workflow.node(outer).ok_or("missing control owner")?;
    let mut completed = frame.clone();
    let mut completed_store = store.clone();
    let expected_reason = match &node.kind {
        CompiledNodeKind::ForEachStart { .. } | CompiledNodeKind::ReduceStart { .. } => {
            let cursor = node.output.slot().ok_or("cursor slot absent")?;
            let SlotValue::List(id) = frame_slot(frame, cursor)? else {
                return Err("cursor is not a list".into());
            };
            let parts = checkpoint_list(store, id)?;
            let source_index = match parts.first() {
                Some(SlotValue::I64(index)) => usize::try_from(*index)?,
                _ => return Err("cursor source index absent".into()),
            };
            let source = store
                .lists
                .get(source_index)
                .ok_or("cursor source absent")?;
            let position = i64::try_from(source.len())?;
            let mut exhausted = parts.to_vec();
            *exhausted.get_mut(1).ok_or("cursor position absent")? = SlotValue::I64(position);
            *completed_store
                .lists
                .get_mut(usize::try_from(id.get())?)
                .ok_or("cursor list absent")? = exhausted.into_boxed_slice();
            "control cursor has no remaining source items"
        }
        CompiledNodeKind::RepeatStart { max_attempts, .. } => {
            let slot = node.output.slot().ok_or("Repeat state slot absent")?;
            let SlotValue::I64(packed) = frame_slot(frame, slot)? else {
                return Err("Repeat state is not an integer".into());
            };
            let packed = u64::try_from(packed)?;
            let exhausted = (packed & !u64::from(u16::MAX)) | u64::from(max_attempts.get());
            *completed
                .slots
                .get_mut(usize::from(slot.get()))
                .ok_or("Repeat frame slot absent")? =
                Some(SlotValue::I64(i64::try_from(exhausted)?));
            "Repeat owner exhausted its attempts"
        }
        CompiledNodeKind::CollectStart { source, .. } => {
            let SlotValue::List(source_id) = frame_slot(frame, *source)? else {
                return Err("Collect source is not a list".into());
            };
            let collector = node.output.slot().ok_or("Collect page slot absent")?;
            let SlotValue::List(page_id) = frame_slot(frame, collector)? else {
                return Err("Collect page is not a list".into());
            };
            let source_index = usize::try_from(source_id.get())?;
            let page_index = usize::try_from(page_id.get())?;
            *completed_store
                .lists
                .get_mut(page_index)
                .ok_or("page absent")? = store
                .lists
                .get(source_index)
                .ok_or("source absent")?
                .clone();
            *completed_store
                .list_taints
                .get_mut(page_index)
                .ok_or("page taint absent")? = store
                .list_taints
                .get(source_index)
                .ok_or("source taint absent")?
                .clone();
            "Collect owner exhausted its source"
        }
        CompiledNodeKind::TogetherStart { branches, .. } => {
            for branch in branches.iter().skip(1) {
                let entry = workflow
                    .node(*branch)
                    .and_then(|node| match node.kind {
                        CompiledNodeKind::TogetherBranch { entry, .. } => Some(entry),
                        _ => None,
                    })
                    .ok_or("Together branch entry absent")?;
                *completed
                    .states
                    .get_mut(usize::from(entry.get()))
                    .ok_or("branch state absent")? = CheckpointStepState::Succeeded;
            }
            "Together owner has no unexecuted branches"
        }
        _ => return Err("unsupported control counterfactual".into()),
    };
    let rejection = assert_remaining_control_work(workflow, outer, &completed, &completed_store);
    assert_eq!(
        rejection.err().map(|error| error.to_string()).as_deref(),
        Some(expected_reason),
        "completed control must be rejected for the expected reason: {outer:?}"
    );
    Ok(())
}

fn frame_slot(frame: &RunFrameCheckpointV1, slot: vb_core::SlotIdx) -> TestResult<SlotValue> {
    frame
        .slots
        .get(usize::from(slot.get()))
        .copied()
        .flatten()
        .ok_or_else(|| format!("uninitialized control slot {slot:?}").into())
}
