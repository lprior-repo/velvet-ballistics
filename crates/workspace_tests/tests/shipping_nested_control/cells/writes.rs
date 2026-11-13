use vb_core::{SlotIdx, SlotValue, StepIdx, Taint};
use vb_storage::{DecodedSlotWrittenExtra, JournalEvent, decode_slot_written_extra};

use crate::support::TestResult;

pub(super) fn for_slot(
    events: &[JournalEvent],
    slot: SlotIdx,
) -> TestResult<Vec<(SlotValue, Taint)>> {
    project_writes(events, slot, None)
}

pub(super) fn for_step(
    events: &[JournalEvent],
    slot: SlotIdx,
    step: StepIdx,
) -> TestResult<Vec<(SlotValue, Taint)>> {
    project_writes(events, slot, Some(step))
}

fn project_writes(
    events: &[JournalEvent],
    slot: SlotIdx,
    step: Option<StepIdx>,
) -> TestResult<Vec<(SlotValue, Taint)>> {
    let mut active_attempt = None;
    let mut writes = Vec::new();
    for event in events {
        track_step(event, step, &mut active_attempt)?;
        if let Some(write) = matching_write(event, slot, step, active_attempt) {
            writes.push(write?);
        }
    }
    if active_attempt.is_some() {
        return Err("leaf step did not complete".to_owned());
    }
    Ok(writes)
}

fn track_step(
    event: &JournalEvent,
    step: Option<StepIdx>,
    active_attempt: &mut Option<u16>,
) -> TestResult {
    match event {
        JournalEvent::StepStarted {
            step: started,
            attempt,
            ..
        } if Some(*started) == step => {
            if active_attempt.replace(*attempt).is_some() {
                return Err(format!("step {started:?} started before prior completion"));
            }
        }
        JournalEvent::StepSucceeded {
            step: completed, ..
        } if Some(*completed) == step => {
            if active_attempt.take().is_none() {
                return Err(format!("step {completed:?} completed without start"));
            }
        }
        JournalEvent::StepFailed { step: failed, .. } if Some(*failed) == step => {
            *active_attempt = None;
        }
        _ => {}
    }
    Ok(())
}

fn matching_write(
    event: &JournalEvent,
    slot: SlotIdx,
    step: Option<StepIdx>,
    active_attempt: Option<u16>,
) -> Option<TestResult<(SlotValue, Taint)>> {
    match event {
        JournalEvent::SlotWrittenEvent {
            slot: written,
            attempt,
            value: Some(bytes),
            extra: Some(extra),
            ..
        } if *written == slot && step.map_or(true, |_| active_attempt == Some(*attempt)) => {
            Some(decode_write(slot, bytes, extra))
        }
        _ => None,
    }
}

fn decode_write(slot: SlotIdx, bytes: &[u8], extra: &[u8]) -> TestResult<(SlotValue, Taint)> {
    let value = postcard::from_bytes::<SlotValue>(bytes)
        .map_err(|error| format!("slot {slot:?} decode: {error}"))?;
    let taint = match decode_slot_written_extra(extra) {
        Ok(DecodedSlotWrittenExtra::Envelope(envelope)) => envelope.taint,
        Ok(DecodedSlotWrittenExtra::LegacyFrameExtra(_)) => {
            return Err(format!("slot {slot:?} legacy taint"));
        }
        Err(error) => return Err(format!("slot {slot:?} extra: {error:?}")),
    };
    Ok((value, taint))
}
