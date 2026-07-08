#![forbid(unsafe_code)]

use crate::RecordKind;

pub(super) const TAG_RUN_ACCEPTED: u16 = RecordKind::RunAccepted.id();
pub(super) const TAG_RUN_ADMISSION: u16 = RecordKind::RunAdmission.id();
pub(super) const TAG_STEP_STARTED: u16 = RecordKind::StepStarted.id();
pub(super) const TAG_STEP_SUCCEEDED: u16 = RecordKind::StepSucceeded.id();
pub(super) const TAG_STEP_FAILED: u16 = RecordKind::StepFailed.id();
pub(super) const TAG_ACTION_SCHEDULED: u16 = RecordKind::ActionScheduled.id();
pub(super) const TAG_ACTION_COMPLETED: u16 = RecordKind::ActionCompleted.id();
pub(super) const TAG_ACTION_SCHEDULED_TICKET: u16 = RecordKind::ActionScheduledTicket.id();
pub(super) const TAG_ACTION_COMPLETED_ENVELOPE: u16 = RecordKind::ActionCompletedEnvelope.id();
pub(super) const TAG_ACTION_FAILED: u16 = RecordKind::ActionFailed.id();
pub(super) const TAG_ACTION_ABANDONED: u16 = RecordKind::ActionAbandoned.id();
pub(super) const TAG_SLOT_WRITTEN: u16 = RecordKind::SlotWritten.id();
pub(super) const TAG_WAIT_SCHEDULED: u16 = RecordKind::WaitScheduled.id();
pub(super) const TAG_ASK_SCHEDULED: u16 = RecordKind::AskScheduled.id();
pub(super) const TAG_ASK_ANSWERED: u16 = RecordKind::AskAnswered.id();
pub(super) const TAG_WAIT_RESOLVED: u16 = RecordKind::WaitResolved.id();
pub(super) const TAG_RETRY_SCHEDULED: u16 = RecordKind::RetryScheduled.id();
pub(super) const TAG_RUN_CANCELLED: u16 = RecordKind::RunCancelled.id();
pub(super) const TAG_RUN_KILLED: u16 = RecordKind::RunKilled.id();
pub(super) const TAG_RUN_FINISHED: u16 = RecordKind::RunFinished.id();
pub(super) const TAG_RUN_FAILED: u16 = RecordKind::RunFailed.id();
pub(super) const TAG_RUN_RESUMED: u16 = RecordKind::RunResumed.id();
pub(super) const TAG_RUN_RETRIED: u16 = RecordKind::RunRetried.id();
pub(super) const TAG_RUN_ANSWERED: u16 = RecordKind::RunAnswered.id();
pub(super) const TAG_ASK_TIMED_OUT: u16 = RecordKind::AskTimedOut.id();

pub(super) const JOURNAL_EVENT_VARIANTS: &[&str] = &[
    "RunAccepted",
    "RunAdmission",
    "StepStarted",
    "StepSucceeded",
    "StepFailed",
    "ActionScheduled",
    "ActionCompletedEvent",
    "ActionScheduledTicket",
    "ActionCompletedEnvelope",
    "ActionFailedEvent",
    "ActionAbandoned",
    "SlotWrittenEvent",
    "WaitScheduledEvent",
    "AskScheduledEvent",
    "AskAnsweredEvent",
    "WaitResolvedEvent",
    "RetryScheduledEvent",
    "RunCancelled",
    "RunKilled",
    "RunFinished",
    "RunFailedEvent",
    "RunResumed",
    "RunRetried",
    "RunAnswered",
    "AskTimedOutEvent",
];
