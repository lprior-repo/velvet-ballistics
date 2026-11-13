#![forbid(unsafe_code)]
//! Summary and pending-action facts for owned-arena replay.

use std::collections::BTreeMap;

use crate::recovery::types::{
    RecoveredPendingAction, RecoveryRuntimeSummary, RecoveryTerminalState,
};
use crate::{EventSeq, JournalEvent};
use vb_core::{ActionId, StepIdx};

pub(super) fn summary(events: &[JournalEvent]) -> RecoveryRuntimeSummary {
    let run = events
        .first()
        .map_or_else(|| vb_core::RunId::new(0), JournalEvent::run_id);
    let first_seq = events.first().map_or(EventSeq::ZERO, JournalEvent::seq);
    let last_seq = events.last().map_or(EventSeq::ZERO, JournalEvent::seq);
    let workflow = events.iter().find_map(|event| match event {
        JournalEvent::RunAccepted { workflow, .. } => Some(*workflow),
        _ => None,
    });
    let mut steps_started = 0_u64;
    let mut steps_succeeded = 0_u64;
    let mut actions_scheduled = 0_u64;
    let mut actions_resolved = 0_u64;
    let mut suspensions = 0_u64;
    let mut slots_written = 0_u64;
    let mut terminal = None;
    for event in events {
        match event {
            JournalEvent::StepStarted { .. } => steps_started = steps_started.saturating_add(1),
            JournalEvent::StepSucceeded { .. } => {
                steps_succeeded = steps_succeeded.saturating_add(1);
            }
            JournalEvent::ActionScheduled { .. } | JournalEvent::ActionScheduledTicket { .. } => {
                actions_scheduled = actions_scheduled.saturating_add(1);
            }
            JournalEvent::ActionCompletedEvent { .. }
            | JournalEvent::ActionFailedEvent { .. }
            | JournalEvent::ActionAbandoned { .. } => {
                actions_resolved = actions_resolved.saturating_add(1);
            }
            JournalEvent::ActionCompletedEnvelope { .. } => {
                actions_resolved = actions_resolved.saturating_add(1);
                steps_succeeded = steps_succeeded.saturating_add(1);
                slots_written = slots_written.saturating_add(1);
            }
            JournalEvent::WaitScheduledEvent { .. }
            | JournalEvent::AskScheduledEvent { .. }
            | JournalEvent::RetryTimerScheduled { .. }
            | JournalEvent::RetryScheduledEvent { .. } => {
                suspensions = suspensions.saturating_add(1);
            }
            JournalEvent::AskTimedOutEvent { .. } => {
                steps_succeeded = steps_succeeded.saturating_add(1);
            }
            JournalEvent::SlotWrittenEvent { .. } => {
                slots_written = slots_written.saturating_add(1);
            }
            JournalEvent::RunFinished { result, .. } => {
                terminal = Some(RecoveryTerminalState::Finished { result: *result });
            }
            JournalEvent::RunFailedEvent { .. } => terminal = Some(RecoveryTerminalState::Failed),
            JournalEvent::RunCancelled { .. } => terminal = Some(RecoveryTerminalState::Cancelled),
            JournalEvent::RunKilled { .. } => terminal = Some(RecoveryTerminalState::Killed),
            _ => {}
        }
    }
    RecoveryRuntimeSummary {
        run,
        first_seq,
        last_seq,
        workflow,
        steps_started,
        steps_succeeded,
        actions_scheduled,
        actions_resolved,
        suspensions,
        slots_written,
        terminal,
    }
}

pub(super) fn pending_actions(events: &[JournalEvent]) -> Vec<RecoveredPendingAction> {
    let mut pending: BTreeMap<(StepIdx, ActionId), ()> = BTreeMap::new();
    for event in events {
        match event {
            JournalEvent::ActionScheduled { step, action, .. }
            | JournalEvent::ActionScheduledTicket {
                ticket: vb_core::action::ActionTicket { step, action, .. },
                ..
            } => {
                pending.insert((*step, *action), ());
            }
            JournalEvent::ActionCompletedEvent { step, action, .. }
            | JournalEvent::ActionFailedEvent { step, action, .. }
            | JournalEvent::ActionCompletedEnvelope {
                ticket: vb_core::action::ActionTicket { step, action, .. },
                ..
            }
            | JournalEvent::ActionAbandoned {
                ticket: vb_core::action::ActionTicket { step, action, .. },
                ..
            } => {
                pending.remove(&(*step, *action));
            }
            _ => {}
        }
    }
    pending
        .into_keys()
        .map(|(step, action)| RecoveredPendingAction { step, action })
        .collect()
}

/// Legal terminal/retry events outside this helper's deterministic replay
/// contract. Returning `None` lets the generic recovery path retain its exact
/// typed terminal or cannot-resume classification instead of rejecting.
pub(super) fn unsupported_replay_event(event: &JournalEvent) -> bool {
    matches!(
        event,
        JournalEvent::ActionScheduled { .. }
            | JournalEvent::AskTimedOutEvent { .. }
            | JournalEvent::ActionCompletedEvent { .. }
            | JournalEvent::ActionAbandoned { .. }
            | JournalEvent::RetryTimerScheduled { .. }
            | JournalEvent::RetryScheduledEvent { .. }
            | JournalEvent::RunCancelled { .. }
            | JournalEvent::RunKilled { .. }
    )
}
