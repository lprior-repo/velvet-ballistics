//! Lifecycle-state derivation from journal events.
//!
//! Maps journal events to [`LifecycleState`](vb_core::workflow::LifecycleState)
//! values and produces human-readable status strings for CLI output.

use vb_core::workflow::LifecycleState;

/// Maps a lifecycle state to a human-readable status string for the inspect command.
///
/// Terminal states map to their name; Active/WaitingAnswer map to "running".
#[must_use]
pub fn lifecycle_state_to_inspect_status(state: LifecycleState) -> &'static str {
    match state {
        LifecycleState::Cancelled => "cancelled",
        LifecycleState::Completed => "finished",
        LifecycleState::Failed => "failed",
        LifecycleState::Pending | LifecycleState::Active | LifecycleState::WaitingAnswer => {
            "running"
        }
        _ => "running",
    }
}

/// Derives the final lifecycle state from a sequence of journal events.
///
/// The last event in the sequence determines the final state:
/// - `RunCancelled` → Cancelled
/// - `RunResumed` → Active
/// - `RunRetried` → Active
/// - `RunAnswered` → Completed
/// - `RunFinished` → Completed
/// - `RunFailedEvent` → Failed
///
/// If no events exist, defaults to Pending.
#[must_use]
#[allow(unreachable_patterns)]
pub fn derive_lifecycle_state_from_events(
    events: &[crate::events::JournalEvent],
) -> LifecycleState {
    events
        .last()
        .map(|e| match e {
            crate::events::JournalEvent::RunCancelled { .. } => LifecycleState::Cancelled,
            crate::events::JournalEvent::RunResumed { .. } => LifecycleState::Active,
            crate::events::JournalEvent::RunRetried { .. } => LifecycleState::Active,
            crate::events::JournalEvent::RunAnswered { .. } => LifecycleState::Completed,
            crate::events::JournalEvent::RunFinished { .. } => LifecycleState::Completed,
            crate::events::JournalEvent::RunFailedEvent { .. } => LifecycleState::Failed,
            crate::events::JournalEvent::RunAccepted { .. } => LifecycleState::Active,
            crate::events::JournalEvent::RunAdmission { .. } => LifecycleState::Active,
            crate::events::JournalEvent::StepStarted { .. } => LifecycleState::Active,
            crate::events::JournalEvent::StepSucceeded { .. } => LifecycleState::Active,
            crate::events::JournalEvent::ActionScheduled { .. } => LifecycleState::Active,
            crate::events::JournalEvent::SlotWrittenEvent { .. } => LifecycleState::Active,
            crate::events::JournalEvent::ActionCompletedEvent { .. } => LifecycleState::Active,
            crate::events::JournalEvent::ActionFailedEvent { .. } => LifecycleState::Failed,
            crate::events::JournalEvent::WaitScheduledEvent { .. } => LifecycleState::WaitingAnswer,
            crate::events::JournalEvent::AskScheduledEvent { .. } => LifecycleState::WaitingAnswer,
            crate::events::JournalEvent::AskAnsweredEvent { .. } => LifecycleState::WaitingAnswer,
            crate::events::JournalEvent::RetryScheduledEvent { .. } => LifecycleState::Active,
            _ => LifecycleState::Active,
        })
        .unwrap_or(LifecycleState::Pending)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EventSeq, JournalEvent};
    use vb_core::{ActionId, ConstValue, RunId, SlotIdx, StepIdx};

    /// Helper: create a minimal RunFinished event.
    fn run_finished() -> JournalEvent {
        JournalEvent::RunFinished {
            run: RunId::new(1),
            seq: EventSeq::new(10),
            result: SlotIdx::new(1),
            attempt: 1,
        }
    }

    /// Helper: create a minimal RunFailedEvent.
    fn run_failed() -> JournalEvent {
        JournalEvent::RunFailedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(10),
            attempt: 1,
        }
    }

    /// Helper: create a minimal RunCancelled event.
    fn run_cancelled() -> JournalEvent {
        JournalEvent::RunCancelled {
            run: RunId::new(1),
            seq: EventSeq::new(10),
            attempt: 1,
            reason: None,
        }
    }

    // ---- T-101: Empty events → Pending ----
    #[test]
    fn t_101_empty_events() {
        assert_eq!(
            derive_lifecycle_state_from_events(&[]),
            LifecycleState::Pending
        );
    }

    // ---- T-102: RunFinished → Completed ----
    #[test]
    fn t_102_run_finished() {
        assert_eq!(
            derive_lifecycle_state_from_events(&[run_finished()]),
            LifecycleState::Completed
        );
    }

    // ---- T-103: RunFailedEvent → Failed ----
    #[test]
    fn t_103_run_failed() {
        assert_eq!(
            derive_lifecycle_state_from_events(&[run_failed()]),
            LifecycleState::Failed
        );
    }

    // ---- T-104: RunCancelled → Cancelled ----
    #[test]
    fn t_104_run_cancelled() {
        assert_eq!(
            derive_lifecycle_state_from_events(&[run_cancelled()]),
            LifecycleState::Cancelled
        );
    }

    // ---- T-105: RunResumed → Active ----
    #[test]
    fn t_105_run_resumed() {
        let events = vec![JournalEvent::RunResumed {
            run: RunId::new(1),
            seq: EventSeq::new(10),
            timestamp: chrono::Utc::now(),
        }];
        assert_eq!(
            derive_lifecycle_state_from_events(&events),
            LifecycleState::Active
        );
    }

    // ---- T-106: RunRetried → Active ----
    #[test]
    fn t_106_run_retried() {
        let events = vec![JournalEvent::RunRetried {
            run: RunId::new(1),
            seq: EventSeq::new(10),
            timestamp: chrono::Utc::now(),
        }];
        assert_eq!(
            derive_lifecycle_state_from_events(&events),
            LifecycleState::Active
        );
    }

    // ---- T-107: RunAnswered → Completed ----
    #[test]
    fn t_107_run_answered() {
        let events = vec![JournalEvent::RunAnswered {
            run: RunId::new(1),
            seq: EventSeq::new(10),
            slot_idx: SlotIdx::new(1),
            answer: ConstValue::Bool(true),
            timestamp: chrono::Utc::now(),
        }];
        assert_eq!(
            derive_lifecycle_state_from_events(&events),
            LifecycleState::Completed
        );
    }

    // ---- T-108: Last event wins (active → failed) ----
    #[test]
    fn t_108_last_event_wins() {
        let events = vec![
            JournalEvent::StepStarted {
                run: RunId::new(1),
                seq: EventSeq::new(1),
                step: StepIdx::new(1),
                attempt: 1,
            },
            JournalEvent::ActionCompletedEvent {
                run: RunId::new(1),
                seq: EventSeq::new(2),
                step: StepIdx::new(1),
                action: ActionId::new(1),
                attempt: 1,
            },
            run_failed(),
        ];
        assert_eq!(
            derive_lifecycle_state_from_events(&events),
            LifecycleState::Failed
        );
    }

    // ---- T-109: lifecycle_state_to_inspect_status (terminal states) ----
    #[test]
    fn t_109_inspect_status_terminal() {
        assert_eq!(
            lifecycle_state_to_inspect_status(LifecycleState::Cancelled),
            "cancelled"
        );
        assert_eq!(
            lifecycle_state_to_inspect_status(LifecycleState::Completed),
            "finished"
        );
        assert_eq!(
            lifecycle_state_to_inspect_status(LifecycleState::Failed),
            "failed"
        );
    }

    // ---- T-110: lifecycle_state_to_inspect_status (active states) ----
    #[test]
    fn t_110_inspect_status_active() {
        assert_eq!(
            lifecycle_state_to_inspect_status(LifecycleState::Pending),
            "running"
        );
        assert_eq!(
            lifecycle_state_to_inspect_status(LifecycleState::Active),
            "running"
        );
        assert_eq!(
            lifecycle_state_to_inspect_status(LifecycleState::WaitingAnswer),
            "running"
        );
    }
}
