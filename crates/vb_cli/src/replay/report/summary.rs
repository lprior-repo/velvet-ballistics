#![forbid(unsafe_code)]

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ReplayEventSummary {
    pub(super) event_count: u64,
    pub(super) first_sequence: Option<u64>,
    pub(super) last_sequence: Option<u64>,
    pub(super) terminal_event: Option<&'static str>,
    pub(super) terminal_status: &'static str,
    pub(super) step_event_count: u64,
    pub(super) action_event_count: u64,
    pub(super) slot_event_count: u64,
    pub(super) action_scheduled_count: u64,
    pub(super) action_resolved_count: u64,
    pub(super) pending_unresolved_action_count: u64,
}

impl ReplayEventSummary {
    pub(super) fn from_events(events: &[vb_storage::JournalEvent]) -> Self {
        let terminal = vb_storage::recovery::extract_terminal(events);
        let mut summary = Self {
            event_count: 0,
            first_sequence: events.first().map(|event| event.seq().get()),
            last_sequence: events.last().map(|event| event.seq().get()),
            terminal_event: terminal.map(crate::commands_diff::event_name),
            terminal_status: terminal.map_or("none", terminal_status_for_event),
            step_event_count: 0,
            action_event_count: 0,
            slot_event_count: 0,
            action_scheduled_count: 0,
            action_resolved_count: 0,
            pending_unresolved_action_count: 0,
        };
        for event in events {
            summary.record(event);
        }
        summary.pending_unresolved_action_count = summary
            .action_scheduled_count
            .saturating_sub(summary.action_resolved_count);
        summary
    }

    fn record(&mut self, event: &vb_storage::JournalEvent) {
        increment_counter(&mut self.event_count);
        match event {
            vb_storage::JournalEvent::StepStarted { .. }
            | vb_storage::JournalEvent::StepSucceeded { .. } => {
                increment_counter(&mut self.step_event_count);
            }
            vb_storage::JournalEvent::ActionScheduled { .. }
            | vb_storage::JournalEvent::ActionScheduledTicket { .. } => {
                increment_counter(&mut self.action_event_count);
                increment_counter(&mut self.action_scheduled_count);
            }
            vb_storage::JournalEvent::ActionCompletedEvent { .. }
            | vb_storage::JournalEvent::ActionCompletedEnvelope { .. }
            | vb_storage::JournalEvent::ActionFailedEvent { .. } => {
                increment_counter(&mut self.action_event_count);
                increment_counter(&mut self.action_resolved_count);
            }
            vb_storage::JournalEvent::SlotWrittenEvent { .. } => {
                increment_counter(&mut self.slot_event_count);
            }
            _ => {}
        }
    }
}

fn increment_counter(counter: &mut u64) {
    *counter = (*counter).saturating_add(1);
}

fn terminal_status_for_event(event: &vb_storage::JournalEvent) -> &'static str {
    match event {
        vb_storage::JournalEvent::RunFinished { .. } => "finished",
        vb_storage::JournalEvent::RunCancelled { .. } => "cancelled",
        vb_storage::JournalEvent::RunFailedEvent { .. } => "failed",
        _ => "unknown",
    }
}

pub(super) fn optional_static_label(value: Option<&'static str>) -> &'static str {
    match value {
        Some(label) => label,
        None => "none",
    }
}

pub(super) fn optional_sequence_label(value: Option<u64>) -> String {
    match value {
        Some(sequence) => sequence.to_string(),
        None => String::from("none"),
    }
}
