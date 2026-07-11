#![forbid(unsafe_code)]

use super::super::super::super::JournalEvent;
use crate::EventSeq;
use chrono::{DateTime, Utc};
use vb_core::{CapabilitySet, ConstValue, RunId, RuntimePolicy, SlotIdx, WorkflowDigest};

use super::super::LegacyJournalEvent;

pub(super) fn from_legacy(event: LegacyJournalEvent) -> JournalEvent {
    match event {
        LegacyJournalEvent::RunAccepted { run, seq, workflow } => accepted(run, seq, workflow),
        LegacyJournalEvent::RunAdmission {
            run,
            seq,
            artifact_digest,
            granted_capabilities,
            policy,
        } => admission(run, seq, artifact_digest, granted_capabilities, policy),
        other => terminal_or_lifecycle(other),
    }
}

fn terminal_or_lifecycle(event: LegacyJournalEvent) -> JournalEvent {
    match event {
        LegacyJournalEvent::RunCancelled {
            run,
            seq,
            attempt,
            reason,
        } => cancelled(run, seq, attempt, reason),
        LegacyJournalEvent::RunKilled { run, seq, attempt } => killed(run, seq, attempt),
        LegacyJournalEvent::RunFinished {
            run,
            seq,
            result,
            attempt,
        } => finished(run, seq, result, attempt),
        LegacyJournalEvent::RunFailedEvent { run, seq, attempt } => failed(run, seq, attempt),
        other => lifecycle(other),
    }
}

fn lifecycle(event: LegacyJournalEvent) -> JournalEvent {
    match event {
        LegacyJournalEvent::RunResumed {
            run,
            seq,
            timestamp,
        } => resumed(run, seq, timestamp),
        LegacyJournalEvent::RunRetried {
            run,
            seq,
            timestamp,
        } => retried(run, seq, timestamp),
        LegacyJournalEvent::RunAnswered {
            run,
            seq,
            slot_idx,
            answer,
            timestamp,
        } => answered(run, seq, slot_idx, answer, timestamp),
        other => super::into_current_by_category(other),
    }
}

pub(super) fn accepted(run: RunId, seq: EventSeq, workflow: WorkflowDigest) -> JournalEvent {
    JournalEvent::RunAccepted { run, seq, workflow }
}

pub(super) fn admission(
    run: RunId,
    seq: EventSeq,
    artifact_digest: WorkflowDigest,
    granted_capabilities: CapabilitySet,
    policy: RuntimePolicy,
) -> JournalEvent {
    JournalEvent::RunAdmission {
        run,
        seq,
        artifact_digest,
        granted_capabilities,
        policy,
    }
}

pub(super) fn cancelled(
    run: RunId,
    seq: EventSeq,
    attempt: u16,
    reason: Option<String>,
) -> JournalEvent {
    JournalEvent::RunCancelled {
        run,
        seq,
        attempt,
        reason,
    }
}

pub(super) fn killed(run: RunId, seq: EventSeq, attempt: u16) -> JournalEvent {
    JournalEvent::RunKilled { run, seq, attempt }
}

pub(super) fn finished(run: RunId, seq: EventSeq, result: SlotIdx, attempt: u16) -> JournalEvent {
    JournalEvent::RunFinished {
        run,
        seq,
        result,
        attempt,
    }
}

pub(super) fn failed(run: RunId, seq: EventSeq, attempt: u16) -> JournalEvent {
    JournalEvent::RunFailedEvent { run, seq, attempt }
}

pub(super) fn resumed(run: RunId, seq: EventSeq, timestamp: DateTime<Utc>) -> JournalEvent {
    JournalEvent::RunResumed {
        run,
        seq,
        timestamp,
    }
}

pub(super) fn retried(run: RunId, seq: EventSeq, timestamp: DateTime<Utc>) -> JournalEvent {
    JournalEvent::RunRetried {
        run,
        seq,
        timestamp,
    }
}

pub(super) fn answered(
    run: RunId,
    seq: EventSeq,
    slot_idx: SlotIdx,
    answer: ConstValue,
    timestamp: DateTime<Utc>,
) -> JournalEvent {
    JournalEvent::RunAnswered {
        run,
        seq,
        slot_idx,
        answer,
        timestamp,
    }
}
