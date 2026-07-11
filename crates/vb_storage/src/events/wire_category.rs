#![forbid(unsafe_code)]

use crate::RecordKind;

#[derive(Clone, Copy)]
pub(super) enum JournalRecordCategory {
    Run,
    Step,
    Action,
    WaitAsk,
}

pub(super) fn journal_record_category(kind: RecordKind) -> Option<JournalRecordCategory> {
    if is_run_record(kind) {
        Some(JournalRecordCategory::Run)
    } else if is_step_record(kind) {
        Some(JournalRecordCategory::Step)
    } else if is_action_record(kind) {
        Some(JournalRecordCategory::Action)
    } else if is_wait_or_ask_record(kind) {
        Some(JournalRecordCategory::WaitAsk)
    } else {
        None
    }
}

fn is_run_record(kind: RecordKind) -> bool {
    matches!(
        kind,
        RecordKind::RunAccepted
            | RecordKind::RunAdmission
            | RecordKind::RunCancelled
            | RecordKind::RunKilled
            | RecordKind::RunFinished
            | RecordKind::RunFailed
            | RecordKind::RunResumed
            | RecordKind::RunRetried
            | RecordKind::RunAnswered
    )
}

fn is_step_record(kind: RecordKind) -> bool {
    matches!(
        kind,
        RecordKind::StepStarted
            | RecordKind::StepSucceeded
            | RecordKind::StepFailed
            | RecordKind::SlotWritten
    )
}

fn is_action_record(kind: RecordKind) -> bool {
    matches!(
        kind,
        RecordKind::ActionScheduled
            | RecordKind::ActionScheduledTicket
            | RecordKind::ActionCompleted
            | RecordKind::ActionCompletedEnvelope
            | RecordKind::ActionFailed
            | RecordKind::ActionAbandoned
    )
}

fn is_wait_or_ask_record(kind: RecordKind) -> bool {
    matches!(
        kind,
        RecordKind::WaitScheduled
            | RecordKind::AskScheduled
            | RecordKind::AskAnswered
            | RecordKind::WaitResolved
            | RecordKind::RetryScheduled
            | RecordKind::AskTimedOut
    )
}
