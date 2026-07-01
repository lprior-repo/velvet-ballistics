use std::num::NonZeroUsize;

use crate::{
    error::JournalError,
    events::JournalEvent,
    types::{DurabilityProfile, JournalWriterQueueProfileCounts},
};

use super::{JournalWriterQueue, JournalWriterQueueState, QueuedJournalEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum QueuedJournalGroup {
    Single,
    AtomicBatchStart { event_count: NonZeroUsize },
    AtomicBatchMember,
}

impl JournalWriterQueue {
    pub(super) fn enqueue_batch(
        &self,
        events: Vec<JournalEvent>,
        profile: DurabilityProfile,
    ) -> Result<(), JournalError> {
        let Some(event_count) = NonZeroUsize::new(events.len()) else {
            return Ok(());
        };
        let mut state = self
            .state
            .lock()
            .map_err(|_| JournalError::WriteLockPoisoned)?;
        if state.shutdown {
            return Err(JournalError::QueueShutdown);
        }
        let available = self
            .capacity
            .checked_sub(state.pending.len())
            .ok_or(JournalError::QueueCapacity)?;
        if event_count.get() > available {
            return Err(JournalError::QueueFull);
        }
        for (position, event) in events.into_iter().enumerate() {
            state.pending.push_back(QueuedJournalEvent {
                event,
                profile,
                group: group_for_batch_position(position, event_count),
            });
        }
        Ok(())
    }
}

pub(super) fn selected_flush_len(
    state: &JournalWriterQueueState,
    batch_size: usize,
) -> Result<usize, JournalError> {
    let mut selected = 0usize;
    while selected < batch_size {
        let Some(item) = state.pending.get(selected) else {
            break;
        };
        let span = queued_event_span(item)?;
        let candidate = selected
            .checked_add(span)
            .ok_or(JournalError::QueueCapacity)?;
        if candidate > state.pending.len() {
            return Err(JournalError::WriteLockPoisoned);
        }
        if selected > 0 && candidate > batch_size {
            break;
        }
        selected = candidate;
    }
    Ok(selected)
}

pub(super) fn count_profile(
    counts: &mut JournalWriterQueueProfileCounts,
    profile: DurabilityProfile,
) {
    match profile {
        DurabilityProfile::Journaled => {
            counts.journaled = counts.journaled.saturating_add(1);
        }
        DurabilityProfile::Strict => {
            counts.strict = counts.strict.saturating_add(1);
        }
        DurabilityProfile::Volatile => {}
    }
}

fn group_for_batch_position(position: usize, event_count: NonZeroUsize) -> QueuedJournalGroup {
    if event_count.get() == 1 {
        return QueuedJournalGroup::Single;
    }
    if position == 0 {
        return QueuedJournalGroup::AtomicBatchStart { event_count };
    }
    QueuedJournalGroup::AtomicBatchMember
}

fn queued_event_span(item: &QueuedJournalEvent) -> Result<usize, JournalError> {
    match item.group {
        QueuedJournalGroup::Single => Ok(1),
        QueuedJournalGroup::AtomicBatchStart { event_count } => Ok(event_count.get()),
        QueuedJournalGroup::AtomicBatchMember => Err(JournalError::WriteLockPoisoned),
    }
}
