#![forbid(unsafe_code)]

use super::super::super::super::JournalEvent;
use crate::EventSeq;
use vb_core::{RunId, SlotIdx, StepIdx};

use super::super::LegacyJournalEvent;

pub(super) fn from_legacy(event: LegacyJournalEvent) -> JournalEvent {
    match event {
        LegacyJournalEvent::StepStarted {
            run,
            seq,
            step,
            attempt,
        } => started(run, seq, step, attempt),
        LegacyJournalEvent::StepSucceeded {
            run,
            seq,
            step,
            output,
        } => succeeded(run, seq, step, output),
        other => slot_or_dispatch(other),
    }
}

fn slot_or_dispatch(event: LegacyJournalEvent) -> JournalEvent {
    match event {
        LegacyJournalEvent::SlotWrittenEvent {
            run,
            seq,
            slot,
            value,
            extra,
            attempt,
        } => slot_written(run, seq, slot, value, extra, attempt),
        other => super::into_current_by_category(other),
    }
}

pub(super) fn started(run: RunId, seq: EventSeq, step: StepIdx, attempt: u16) -> JournalEvent {
    JournalEvent::StepStarted {
        run,
        seq,
        step,
        attempt,
    }
}

pub(super) fn succeeded(run: RunId, seq: EventSeq, step: StepIdx, output: SlotIdx) -> JournalEvent {
    JournalEvent::StepSucceeded {
        run,
        seq,
        step,
        output,
    }
}

pub(super) fn slot_written(
    run: RunId,
    seq: EventSeq,
    slot: SlotIdx,
    value: Option<Vec<u8>>,
    extra: Option<Vec<u8>>,
    attempt: u16,
) -> JournalEvent {
    JournalEvent::SlotWrittenEvent {
        run,
        seq,
        slot,
        value,
        extra,
        attempt,
    }
}
