use std::num::NonZeroUsize;
use std::sync::Arc;

use vb_core::{RuntimePolicy, Taint};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::{DecodedSlotWrittenExtra, FjallJournal, JournalEvent};

use crate::support::{RUN, TestResult};

#[derive(Debug, Clone, Copy)]
pub(super) enum Corruption {
    Cursor,
    PageSize,
    ItemCount,
    Source,
    Taint,
    MissingFrameExtra,
}

pub(super) fn runtime(journal: Arc<FjallJournal>) -> Runtime {
    Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Strict,
            step_budget_per_tick: 1,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(journal),
    )
}

pub(super) fn expected_error(corruption: Corruption, error: &str) -> bool {
    match corruption {
        Corruption::Taint => error.ends_with("capture differs from artifact replay"),
        _ => error == "Collect frame extra differs from replay",
    }
}

pub(super) fn collect_extra_count(events: &[JournalEvent]) -> usize {
    events
        .iter()
        .filter(|event| match event {
            JournalEvent::SlotWrittenEvent {
                extra: Some(bytes), ..
            } => matches!(
                vb_storage::decode_slot_written_extra(bytes),
                Ok(DecodedSlotWrittenExtra::Envelope(envelope)) if envelope.frame_extra.is_some()
            ),
            _ => false,
        })
        .count()
}

pub(super) fn mutate_first_collect_capture(
    events: &mut [JournalEvent],
    corruption: Corruption,
) -> TestResult<bool> {
    for event in events {
        let JournalEvent::SlotWrittenEvent {
            extra: Some(bytes), ..
        } = event
        else {
            continue;
        };
        let DecodedSlotWrittenExtra::Envelope(mut envelope) =
            vb_storage::decode_slot_written_extra(bytes)
                .map_err(|error| format!("extra decode: {error:?}"))?
        else {
            continue;
        };
        let Some(frame_extra) = envelope.frame_extra.take() else {
            continue;
        };
        if matches!(corruption, Corruption::MissingFrameExtra) {
            *event_extra_mut(event)? = Some(
                vb_storage::encode_slot_written_extra(envelope.taint, None)
                    .map_err(|error| format!("extra encode: {error:?}"))?,
            );
            return Ok(true);
        }
        let mut state = postcard::from_bytes::<
            vb_runtime::primitives::collect::CollectPaginationState,
        >(&frame_extra)?;
        match corruption {
            Corruption::Cursor => state.cursor = 0,
            Corruption::PageSize => {
                state.page_size = state.page_size.checked_add(1).ok_or("page size overflow")?
            }
            Corruption::ItemCount => {
                state.item_count = state
                    .item_count
                    .checked_add(1)
                    .ok_or("item count overflow")?
            }
            Corruption::Source => {
                state.source = vb_core::ListId::new(if state.source.get() == 77 { 78 } else { 77 });
            }
            Corruption::Taint => envelope.taint = Taint::Clean,
            Corruption::MissingFrameExtra => return Err("unreachable missing-extra branch".into()),
        }
        let bytes = postcard::to_allocvec(&state)?;
        *event_extra_mut(event)? = Some(
            vb_storage::encode_slot_written_extra(envelope.taint, Some(bytes))
                .map_err(|error| format!("extra encode: {error:?}"))?,
        );
        return Ok(true);
    }
    Ok(false)
}

pub(super) fn mutate_empty_collect_capture(
    events: &mut [JournalEvent],
    workflow: &vb_core::CompiledWorkflow,
) -> TestResult<bool> {
    let mut active_step = None;
    for event in events {
        match event {
            JournalEvent::StepStarted { step, .. } => active_step = Some(*step),
            JournalEvent::StepSucceeded { .. } | JournalEvent::StepFailed { .. } => {
                active_step = None;
            }
            JournalEvent::SlotWrittenEvent { slot, extra, .. } => {
                let Some(step) = active_step else {
                    continue;
                };
                let Some(node) = workflow.node(step) else {
                    continue;
                };
                let vb_core::CompiledNodeKind::CollectStart { source, .. } = node.kind else {
                    continue;
                };
                if node.output.slot().unwrap_or(source) != *slot {
                    continue;
                }
                let bytes = extra
                    .as_deref()
                    .ok_or("empty Collect capture has no envelope")?;
                let DecodedSlotWrittenExtra::Envelope(mut envelope) =
                    vb_storage::decode_slot_written_extra(bytes)
                        .map_err(|error| format!("extra decode: {error:?}"))?
                else {
                    return Err("empty Collect capture uses legacy extra".into());
                };
                if envelope.frame_extra.is_some() {
                    return Err("empty Collect replay unexpectedly retained state".into());
                }
                let bogus = vb_runtime::primitives::collect::CollectPaginationState {
                    run_id: RUN,
                    collector_slot: *slot,
                    source: vb_core::ListId::new(77),
                    current_page: vb_core::ListId::new(78),
                    cursor: 1,
                    page_size: 1,
                    item_count: 1,
                    limit: 1,
                    time_limit_ms: None,
                    start_millis: 0,
                };
                envelope.frame_extra = Some(postcard::to_allocvec(&bogus)?);
                *extra = Some(
                    vb_storage::encode_slot_written_extra(envelope.taint, envelope.frame_extra)
                        .map_err(|error| format!("extra encode: {error:?}"))?,
                );
                return Ok(true);
            }
            _ => {}
        }
    }
    Ok(false)
}

pub(super) fn has_collect_start_capture(
    events: &[JournalEvent],
    workflow: &vb_core::CompiledWorkflow,
) -> bool {
    let mut active_step = None;
    events.iter().any(|event| match event {
        JournalEvent::StepStarted { step, .. } => {
            active_step = Some(*step);
            false
        }
        JournalEvent::SlotWrittenEvent { slot, .. } => active_step
            .and_then(|step| workflow.node(step).map(|node| (step, node)))
            .is_some_and(|(_, node)| match node.kind {
                vb_core::CompiledNodeKind::CollectStart { source, .. } => {
                    node.output.slot().unwrap_or(source) == *slot
                }
                _ => false,
            }),
        JournalEvent::StepSucceeded { .. } | JournalEvent::StepFailed { .. } => {
            active_step = None;
            false
        }
        _ => false,
    })
}

fn event_extra_mut(event: &mut JournalEvent) -> TestResult<&mut Option<Vec<u8>>> {
    match event {
        JournalEvent::SlotWrittenEvent { extra, .. } => Ok(extra),
        _ => Err("mutated event changed kind".into()),
    }
}
