use super::{RUN, TestResult};
use vb_core::{SlotIdx, SlotValue, StepIdx, Taint};
use vb_storage::JournalEvent;

pub(super) fn lane_activity(
    events: &[JournalEvent],
    step: StepIdx,
) -> TestResult<Vec<JournalEvent>> {
    let mut active = false;
    let mut activity = Vec::new();
    for event in events {
        match event {
            JournalEvent::StepStarted { step: actual, .. } if *actual == step => {
                if active {
                    return Err(format!("overlapping journal activation for lane {step:?}"));
                }
                active = true;
                activity.push(event.clone());
            }
            JournalEvent::StepSucceeded { step: actual, .. } if *actual == step => {
                if !active {
                    return Err(format!("journal lane {step:?} succeeded without a start"));
                }
                activity.push(event.clone());
                active = false;
            }
            JournalEvent::StepFailed { step: actual, .. } if *actual == step => {
                return Err(format!("journal lane {step:?} failed"));
            }
            JournalEvent::SlotWrittenEvent { .. } if active => activity.push(event.clone()),
            JournalEvent::StepStarted { .. }
            | JournalEvent::StepSucceeded { .. }
            | JournalEvent::StepFailed { .. }
                if active =>
            {
                return Err(format!("another journal step interleaved lane {step:?}"));
            }
            _ => {}
        }
    }
    if active {
        return Err(format!("journal lane {step:?} has no matching success"));
    }
    Ok(activity)
}

pub(super) fn assert_lane_activity(
    activity: &[JournalEvent],
    step: StepIdx,
    output: SlotIdx,
    expected: &[(SlotValue, Taint)],
) -> TestResult {
    let mut events = activity.iter();
    for (ordinal, (value, taint)) in expected.iter().enumerate() {
        match events.next() {
            Some(JournalEvent::StepStarted {
                run,
                step: actual,
                attempt,
                ..
            }) if *run == RUN && *actual == step && *attempt == 1 => {}
            other => {
                return Err(format!(
                    "lane {step:?} item {ordinal} expected StepStarted, got {other:?}"
                ));
            }
        }
        let write = events
            .next()
            .ok_or_else(|| format!("lane {step:?} item {ordinal} missing SlotWritten"))?;
        match write {
            JournalEvent::SlotWrittenEvent {
                run,
                slot,
                extra: Some(extra),
                attempt,
                ..
            } if *run == RUN && *slot == output && *attempt == 1 => {
                let actual_value = write
                    .slot_value()
                    .map_err(|error| format!("lane {step:?} item {ordinal} value: {error}"))?;
                if actual_value != Some(*value) {
                    return Err(format!(
                        "lane {step:?} item {ordinal} value mismatch: {actual_value:?}"
                    ));
                }
                match vb_storage::decode_slot_written_extra(extra)
                    .map_err(|error| format!("lane {step:?} item {ordinal} taint: {error:?}"))?
                {
                    vb_storage::DecodedSlotWrittenExtra::Envelope(envelope)
                        if envelope.taint == *taint && envelope.frame_extra.is_none() => {}
                    other => {
                        return Err(format!(
                            "lane {step:?} item {ordinal} taint mismatch: {other:?}"
                        ));
                    }
                }
            }
            other => {
                return Err(format!(
                    "lane {step:?} item {ordinal} expected SlotWritten({output:?}), got {other:?}"
                ));
            }
        }
        match events.next() {
            Some(JournalEvent::StepSucceeded {
                run,
                step: actual,
                output: actual_output,
                ..
            }) if *run == RUN && *actual == step && *actual_output == output => {}
            other => {
                return Err(format!(
                    "lane {step:?} item {ordinal} expected StepSucceeded, got {other:?}"
                ));
            }
        }
    }
    if let Some(extra) = events.next() {
        return Err(format!("unexpected extra lane event: {extra:?}"));
    }
    Ok(())
}
