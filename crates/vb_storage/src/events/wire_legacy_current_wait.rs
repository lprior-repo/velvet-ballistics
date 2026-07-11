#![forbid(unsafe_code)]

use super::super::super::super::JournalEvent;
use crate::EventSeq;
use vb_core::{RunId, StepIdx};

use super::super::LegacyJournalEvent;

pub(super) fn from_legacy(event: LegacyJournalEvent) -> JournalEvent {
    match event {
        LegacyJournalEvent::WaitScheduledEvent {
            run,
            seq,
            step,
            attempt,
        } => scheduled(run, seq, step, attempt),
        LegacyJournalEvent::AskScheduledEvent {
            run,
            seq,
            step,
            attempt,
        } => ask_scheduled(run, seq, step, attempt),
        LegacyJournalEvent::AskAnsweredEvent {
            run,
            seq,
            step,
            attempt,
        } => ask_answered(run, seq, step, attempt),
        other => resolved_retry_or_timeout(other),
    }
}

fn resolved_retry_or_timeout(event: LegacyJournalEvent) -> JournalEvent {
    match event {
        LegacyJournalEvent::WaitResolvedEvent {
            run,
            seq,
            step,
            attempt,
        } => resolved(run, seq, step, attempt),
        LegacyJournalEvent::RetryScheduledEvent {
            run,
            seq,
            step,
            attempt,
        } => retry_scheduled(run, seq, step, attempt),
        LegacyJournalEvent::AskTimedOutEvent {
            run,
            seq,
            step,
            attempt,
        } => ask_timed_out(run, seq, step, attempt),
        other => super::into_current_by_category(other),
    }
}

pub(super) fn scheduled(run: RunId, seq: EventSeq, step: StepIdx, attempt: u16) -> JournalEvent {
    JournalEvent::WaitScheduledEvent {
        run,
        seq,
        step,
        attempt,
    }
}

pub(super) fn ask_scheduled(
    run: RunId,
    seq: EventSeq,
    step: StepIdx,
    attempt: u16,
) -> JournalEvent {
    JournalEvent::AskScheduledEvent {
        run,
        seq,
        step,
        attempt,
    }
}

pub(super) fn ask_answered(run: RunId, seq: EventSeq, step: StepIdx, attempt: u16) -> JournalEvent {
    JournalEvent::AskAnsweredEvent {
        run,
        seq,
        step,
        attempt,
    }
}

pub(super) fn resolved(run: RunId, seq: EventSeq, step: StepIdx, attempt: u16) -> JournalEvent {
    JournalEvent::WaitResolvedEvent {
        run,
        seq,
        step,
        attempt,
    }
}

pub(super) fn retry_scheduled(
    run: RunId,
    seq: EventSeq,
    step: StepIdx,
    attempt: u16,
) -> JournalEvent {
    JournalEvent::RetryScheduledEvent {
        run,
        seq,
        step,
        attempt,
    }
}

pub(super) fn ask_timed_out(
    run: RunId,
    seq: EventSeq,
    step: StepIdx,
    attempt: u16,
) -> JournalEvent {
    JournalEvent::AskTimedOutEvent {
        run,
        seq,
        step,
        attempt,
    }
}
