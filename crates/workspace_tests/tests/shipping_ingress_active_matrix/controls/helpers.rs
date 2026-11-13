use vb_core::frame::{CheckpointStepState, RunFrameCheckpointV1};
use vb_core::value_store::ValueStoreCheckpointV1;
use vb_core::{RunId, RuntimePolicy, SlotIdx, SlotValue, StepIdx, StepState, Taint};
use vb_runtime::runtime::Runtime;
use vb_storage::JournalEvent;
use vb_storage::slot_extra::{DecodedSlotWrittenExtra, decode_slot_written_extra};

use super::super::support::{self, TestResult};

pub(super) fn config() -> vb_runtime::shard::ShardConfig {
    vb_runtime::shard::ShardConfig {
        command_queue_capacity: 32,
        trace_capacity: 64,
        step_budget_per_tick: 1,
        max_active_runs: 8,
        policy: RuntimePolicy::Strict,
    }
}

pub(super) fn cut_inside_control(
    runtime: &mut Runtime,
    journal: &vb_storage::FjallJournal,
    run: RunId,
    outer: StepIdx,
    body: StepIdx,
    finish: StepIdx,
) -> TestResult<(
    RunFrameCheckpointV1,
    ValueStoreCheckpointV1,
    Vec<JournalEvent>,
)> {
    let mut observations = Vec::new();
    for tick in 0..512 {
        runtime.tick_all()?;
        let state = runtime.inspect_run_state(run).map_err(|error| {
            format!(
                "active cut lost run {run:?} at tick {tick}; target steps outer={outer:?} body={body:?} finish={finish:?}; frame states: {observations:?}; error: {error:?}"
            )
        })?;
        observations.push(state.frame.checkpoint_v1().states);
        if state.frame.step_state(body)? == StepState::Succeeded
            && state.frame.step_state(finish)? == StepState::Pending
        {
            return Ok((
                state.frame.checkpoint_v1(),
                state.store.checkpoint_v1(),
                support::events(journal, run)?,
            ));
        }
    }
    Err(format!("run {run:?} did not reach an active control cut").into())
}

pub(super) fn assert_remaining_control_work(
    workflow: &vb_core::CompiledWorkflow,
    outer: StepIdx,
    frame: &RunFrameCheckpointV1,
    store: &ValueStoreCheckpointV1,
) -> TestResult {
    let node = workflow
        .node(outer)
        .ok_or("missing compiled control owner")?;
    match &node.kind {
        vb_core::CompiledNodeKind::ForEachStart { .. }
        | vb_core::CompiledNodeKind::ReduceStart { .. } => {
            let cursor_slot = node.output.slot().ok_or("loop owner has no cursor slot")?;
            let cursor = frame
                .slots
                .get(usize::from(cursor_slot.get()))
                .copied()
                .flatten()
                .ok_or("loop cursor slot is uninitialized")?;
            let SlotValue::List(cursor_id) = cursor else {
                return Err(format!("loop cursor is not a list: {cursor:?}").into());
            };
            let cursor_items = checkpoint_list(store, cursor_id)?;
            let source_token = match cursor_items.first() {
                Some(SlotValue::I64(value)) => *value,
                other => {
                    return Err(format!("invalid source identity in cursor: {other:?}").into());
                }
            };
            let source_index = usize::try_from(source_token)?;
            let source_items = store
                .lists
                .get(source_index)
                .ok_or("loop source list handle is invalid")?;
            let cursor_position = match cursor_items.get(1) {
                Some(SlotValue::I64(value)) => usize::try_from(*value)?,
                other => return Err(format!("invalid cursor position: {other:?}").into()),
            };
            assert_eq!(
                source_items.as_ref(),
                [SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)]
            );
            if !has_remaining_items(cursor_position, source_items.len()) {
                return Err("control cursor has no remaining source items".into());
            }
        }
        vb_core::CompiledNodeKind::RepeatStart { max_attempts, .. } => {
            let state_slot = node.output.slot().ok_or("Repeat owner has no state slot")?;
            let state = frame
                .slots
                .get(usize::from(state_slot.get()))
                .copied()
                .flatten()
                .ok_or("Repeat state slot is uninitialized")?;
            let SlotValue::I64(packed) = state else {
                return Err(format!("Repeat state is not packed I64: {state:?}").into());
            };
            let packed = u64::try_from(packed)?;
            let encoded_max = u32::try_from(packed >> 32)?;
            let current_attempt = u16::try_from(packed & u64::from(u16::MAX))?;
            assert_eq!(encoded_max, u32::from(max_attempts.get()));
            if current_attempt >= max_attempts.get() {
                return Err("Repeat owner exhausted its attempts".into());
            }
        }
        vb_core::CompiledNodeKind::CollectStart { source, .. } => {
            let source = frame
                .slots
                .get(usize::from(source.get()))
                .copied()
                .flatten()
                .ok_or("Collect source slot is uninitialized")?;
            let SlotValue::List(source_id) = source else {
                return Err(format!("Collect source is not a list: {source:?}").into());
            };
            let collector_slot = node
                .output
                .slot()
                .ok_or("Collect owner has no collector slot")?;
            let current = frame
                .slots
                .get(usize::from(collector_slot.get()))
                .copied()
                .flatten()
                .ok_or("Collect page slot is uninitialized")?;
            let SlotValue::List(page_id) = current else {
                return Err(format!("Collect page is not a list: {current:?}").into());
            };
            let source_items = checkpoint_list(store, source_id)?;
            let page = checkpoint_list(store, page_id)?;
            if source_items.len() <= page.len() {
                return Err("Collect owner exhausted its source".into());
            }
            assert_eq!(page, [SlotValue::I64(1), SlotValue::I64(2)]);
        }
        vb_core::CompiledNodeKind::TogetherStart { branches, join } => {
            assert!(branches.len() > 1, "Together needs remaining branches");
            assert_eq!(
                frame.states.get(usize::from(join.get())),
                Some(&CheckpointStepState::Pending),
                "Together join completed before its branches"
            );
            let remaining = branches.iter().skip(1).try_fold(false, |found, branch| {
                let entry = workflow
                    .node(*branch)
                    .and_then(|branch_node| match branch_node.kind {
                        vb_core::CompiledNodeKind::TogetherBranch { entry, .. } => Some(entry),
                        _ => None,
                    })
                    .ok_or("Together branch lacks a compiled entry")?;
                Ok::<_, Box<dyn std::error::Error>>(
                    found
                        || frame.states.get(usize::from(entry.get()))
                            == Some(&CheckpointStepState::Pending),
                )
            })?;
            if !remaining {
                return Err("Together owner has no unexecuted branches".into());
            }
        }
        other => return Err(format!("unsupported active control: {other:?}").into()),
    }
    assert_eq!(
        frame.states.get(usize::from(outer.get())),
        Some(&CheckpointStepState::Succeeded)
    );
    Ok(())
}

pub(super) fn checkpoint_list(
    store: &ValueStoreCheckpointV1,
    id: vb_core::ListId,
) -> TestResult<&[SlotValue]> {
    let index = usize::try_from(id.get())?;
    store
        .lists
        .get(index)
        .map(AsRef::as_ref)
        .ok_or_else(|| format!("missing checkpoint list {id:?}").into())
}

fn has_remaining_items(cursor: usize, item_count: usize) -> bool {
    cursor < item_count
}

pub(super) fn written_values(
    events: &[JournalEvent],
    run: RunId,
    slot: SlotIdx,
) -> TestResult<Vec<(vb_storage::EventSeq, SlotValue, Taint, u16)>> {
    events
        .iter()
        .filter_map(|event| match event {
            JournalEvent::SlotWrittenEvent {
                run: event_run,
                seq,
                slot: event_slot,
                attempt,
                value: Some(value),
                extra: Some(extra),
            } if *event_run == run && *event_slot == slot => Some((*seq, *attempt, value, extra)),
            _ => None,
        })
        .map(|(seq, attempt, value, extra)| {
            let value = postcard::from_bytes::<SlotValue>(value)?;
            let taint = match decode_slot_written_extra(extra)
                .map_err(|error| format!("body taint envelope: {error:?}"))?
            {
                DecodedSlotWrittenExtra::Envelope(envelope) => envelope.taint,
                DecodedSlotWrittenExtra::LegacyFrameExtra(_) => {
                    return Err("body slot write used legacy taint extra".into());
                }
            };
            Ok((seq, value, taint, attempt))
        })
        .collect()
}
