#![forbid(unsafe_code)]

#[path = "wire_legacy_current_action.rs"]
mod action;
#[path = "wire_legacy_current_run.rs"]
mod run;
#[path = "wire_legacy_current_step.rs"]
mod step;
#[path = "wire_legacy_current_wait.rs"]
mod wait;

use super::super::super::JournalEvent;
use super::LegacyJournalEvent;

#[derive(Clone, Copy)]
enum LegacyEventCategory {
    Run,
    Step,
    Action,
    Wait,
}

impl LegacyJournalEvent {
    pub(super) fn into_current(self) -> JournalEvent {
        into_current_by_category(self)
    }

    fn category(&self) -> LegacyEventCategory {
        if self.is_run_event() {
            LegacyEventCategory::Run
        } else if self.is_step_event() {
            LegacyEventCategory::Step
        } else if self.is_action_event() {
            LegacyEventCategory::Action
        } else {
            LegacyEventCategory::Wait
        }
    }

    fn is_run_event(&self) -> bool {
        matches!(
            self,
            Self::RunAccepted { .. }
                | Self::RunAdmission { .. }
                | Self::RunCancelled { .. }
                | Self::RunKilled { .. }
                | Self::RunFinished { .. }
                | Self::RunFailedEvent { .. }
                | Self::RunResumed { .. }
                | Self::RunRetried { .. }
                | Self::RunAnswered { .. }
        )
    }

    fn is_step_event(&self) -> bool {
        matches!(
            self,
            Self::StepStarted { .. } | Self::StepSucceeded { .. } | Self::SlotWrittenEvent { .. }
        )
    }

    fn is_action_event(&self) -> bool {
        matches!(
            self,
            Self::ActionScheduled { .. }
                | Self::ActionCompletedEvent { .. }
                | Self::ActionScheduledTicket { .. }
                | Self::ActionCompletedEnvelope { .. }
                | Self::ActionFailedEvent { .. }
                | Self::ActionAbandoned { .. }
        )
    }
}

fn into_current_by_category(event: LegacyJournalEvent) -> JournalEvent {
    match event.category() {
        LegacyEventCategory::Run => run::from_legacy(event),
        LegacyEventCategory::Step => step::from_legacy(event),
        LegacyEventCategory::Action => action::from_legacy(event),
        LegacyEventCategory::Wait => wait::from_legacy(event),
    }
}
