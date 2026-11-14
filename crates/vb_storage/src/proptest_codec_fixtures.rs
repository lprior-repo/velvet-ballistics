// Codec fixtures: deterministic event construction for property and matrix tests.
// Matches each RecordKind to its actual JournalEvent variant with exact declared
// fields using source-backed recipes. Uses pinned UNIX_EPOCH timestamps and
// ConstValue::Null for RunAnswered. No wall clock or new dependencies.

use crate::{EventSeq, JournalEvent, RecordKind};
use chrono::DateTime;
use chrono::Utc;
use proptest::test_runner::TestCaseError;
use vb_core::{ConstValue, RunId};
use vb_core::{SlotIdx, StepIdx};

/// Construct a JournalEvent from a RecordKind, generated run/seq/attempt coordinates,
/// and source-backed recipes. Returns TestCaseError::fail for unsupported kinds.
/// The 16 attempt-bearing variants use the actual generated attempt; IDs
/// 10,24,25,26,27 have no attempt.
pub(super) fn journal_event(
    kind: RecordKind,
    run: RunId,
    seq: EventSeq,
    attempt: u16,
) -> Result<JournalEvent, TestCaseError> {
    let step = StepIdx::new(0);
    let slot = SlotIdx::new(0);
    let result = SlotIdx::new(0);
    let action = vb_core::ActionId::new(1);
    let timestamp = DateTime::<Utc>::UNIX_EPOCH;

    match kind {
        RecordKind::RunAccepted => {
            let workflow = vb_core::WorkflowDigest::from_bytes([0; 32]);
            Ok(JournalEvent::RunAccepted { run, seq, workflow })
        }
        RecordKind::StepStarted => Ok(JournalEvent::StepStarted {
            run,
            seq,
            step,
            attempt,
        }),
        RecordKind::SlotWritten => Ok(JournalEvent::SlotWrittenEvent {
            run,
            seq,
            slot,
            value: None,
            extra: None,
            attempt,
        }),
        RecordKind::ActionScheduled => Ok(JournalEvent::ActionScheduled {
            run,
            seq,
            step,
            action,
            attempt,
        }),
        RecordKind::ActionCompleted => Ok(JournalEvent::ActionCompletedEvent {
            run,
            seq,
            step,
            action,
            attempt,
        }),
        RecordKind::ActionFailed => Ok(JournalEvent::ActionFailedEvent {
            run,
            seq,
            step,
            action,
            attempt,
        }),
        RecordKind::WaitScheduled => Ok(JournalEvent::WaitScheduledEvent {
            run,
            seq,
            step,
            attempt,
        }),
        RecordKind::AskScheduled => Ok(JournalEvent::AskScheduledEvent {
            run,
            seq,
            step,
            attempt,
        }),
        RecordKind::AskAnswered => Ok(JournalEvent::AskAnsweredEvent {
            run,
            seq,
            step,
            attempt,
        }),
        RecordKind::RetryScheduled => Ok(JournalEvent::RetryScheduledEvent {
            run,
            seq,
            step,
            attempt,
        }),
        RecordKind::StepFailed => Ok(JournalEvent::StepFailed {
            run,
            seq,
            step,
            attempt,
        }),
        RecordKind::RunCancelled => Ok(JournalEvent::RunCancelled {
            run,
            seq,
            attempt,
            reason: None,
        }),
        RecordKind::RunFinished => Ok(JournalEvent::RunFinished {
            run,
            seq,
            result,
            attempt,
        }),
        RecordKind::RunFailed => Ok(JournalEvent::RunFailedEvent { run, seq, attempt }),
        RecordKind::RunAdmission => {
            let digest = vb_core::WorkflowDigest::from_bytes([0; 32]);
            let capabilities = vb_core::CapabilitySet::empty();
            Ok(JournalEvent::RunAdmission {
                run,
                seq,
                input_digest: crate::RunInputBinding::LegacyUnbound,
                policy_pin: crate::RunPolicyPin::Unverified,
                artifact_digest: digest,
                granted_capabilities: capabilities,
                policy: vb_core::RuntimePolicy::Strict,
            })
        }
        RecordKind::RunResumed => Ok(JournalEvent::RunResumed {
            run,
            seq,
            timestamp,
        }),
        RecordKind::RunRetried => Ok(JournalEvent::RunRetried {
            run,
            seq,
            timestamp,
        }),
        RecordKind::RunAnswered => Ok(JournalEvent::RunAnswered {
            run,
            seq,
            slot_idx: slot,
            answer: ConstValue::Null,
            timestamp,
        }),
        RecordKind::RunKilled => Ok(JournalEvent::RunKilled { run, seq, attempt }),
        RecordKind::AskTimedOut => Ok(JournalEvent::AskTimedOutEvent {
            run,
            seq,
            step,
            attempt,
        }),
        RecordKind::WaitResolved => Ok(JournalEvent::WaitResolvedEvent {
            run,
            seq,
            step,
            attempt,
        }),
        _ => Err(TestCaseError::fail(format!("unsupported kind {:?}", kind))),
    }
}
