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
        self.enqueue_batch_with_inputs(events, None, profile)
    }

    pub(super) fn enqueue_admission_batch(
        &self,
        events: Vec<JournalEvent>,
        inputs: crate::inputs::RunInputsRecord,
        profile: DurabilityProfile,
    ) -> Result<(), JournalError> {
        self.enqueue_batch_with_inputs(events, Some(inputs), profile)
    }

    fn enqueue_batch_with_inputs(
        &self,
        events: Vec<JournalEvent>,
        retained_inputs: Option<crate::inputs::RunInputsRecord>,
        profile: DurabilityProfile,
    ) -> Result<(), JournalError> {
        let Some(event_count) = validated_batch_count(&events, retained_inputs.as_ref())? else {
            return Ok(());
        };
        let mut state = self
            .state
            .lock()
            .map_err(|_| JournalError::WriteLockPoisoned)?;
        if state.shutdown {
            return Err(JournalError::QueueShutdown);
        }
        let reservation =
            self.reserve_batch(&state, &events, retained_inputs.as_ref(), event_count)?;
        stage_batch(&mut state, events, retained_inputs, profile, reservation);
        Ok(())
    }

    fn reserve_batch(
        &self,
        state: &JournalWriterQueueState,
        events: &[JournalEvent],
        inputs: Option<&crate::inputs::RunInputsRecord>,
        event_count: NonZeroUsize,
    ) -> Result<BatchReservation, JournalError> {
        let input_bytes = inputs
            .map(retained_input_record_bytes)
            .transpose()?
            .unwrap_or(0);
        let record_bytes = self.checked_batch_bytes(state, events, input_bytes, event_count)?;
        let record_sizes = self.admission_record_sizes(events, input_bytes)?;
        let pending_bytes = super::reserve_pending_bytes(
            state.pending_bytes,
            record_bytes,
            self.max_pending_bytes,
        )?;
        Ok(BatchReservation {
            event_count,
            record_sizes,
            pending_bytes,
        })
    }

    fn admission_record_sizes(
        &self,
        events: &[JournalEvent],
        input_bytes: u64,
    ) -> Result<Vec<u64>, JournalError> {
        let mut sizes = batch_record_sizes(events, self.max_event_payload_bytes)?;
        if let Some(first) = sizes.first_mut() {
            *first = first
                .checked_add(input_bytes)
                .ok_or(JournalError::QueueCapacity)?;
        }
        Ok(sizes)
    }

    fn checked_batch_bytes(
        &self,
        state: &JournalWriterQueueState,
        events: &[JournalEvent],
        input_bytes: u64,
        event_count: NonZeroUsize,
    ) -> Result<u64, JournalError> {
        // Validate every payload before capacity; no queue mutation precedes either check.
        let record_bytes = total_batch_record_bytes(events, self.max_event_payload_bytes)?
            .and_then(|bytes| bytes.checked_add(input_bytes));
        let available = self
            .capacity
            .checked_sub(state.pending.len())
            .ok_or(JournalError::QueueCapacity)?;
        if event_count.get() > available {
            return Err(JournalError::QueueFull);
        }
        record_bytes.ok_or(JournalError::JournalBatchBytesExceeded {
            attempted: u64::MAX,
            limit: self.max_pending_bytes,
        })
    }
}

struct BatchReservation {
    event_count: NonZeroUsize,
    record_sizes: Vec<u64>,
    pending_bytes: u64,
}

fn validated_batch_count(
    events: &[JournalEvent],
    inputs: Option<&crate::inputs::RunInputsRecord>,
) -> Result<Option<NonZeroUsize>, JournalError> {
    if events.is_empty() {
        return if inputs.is_some() {
            Err(JournalError::InvalidEvent)
        } else {
            Ok(None)
        };
    }
    let event_count = NonZeroUsize::new(events.len()).ok_or(JournalError::QueueCapacity)?;
    if let Some(inputs) = inputs {
        crate::inputs::validate_admission_inputs(events, inputs)?;
    }
    Ok(Some(event_count))
}

fn stage_batch(
    state: &mut JournalWriterQueueState,
    events: Vec<JournalEvent>,
    mut retained_inputs: Option<crate::inputs::RunInputsRecord>,
    profile: DurabilityProfile,
    reservation: BatchReservation,
) {
    for ((position, event), record_bytes) in
        events.into_iter().enumerate().zip(reservation.record_sizes)
    {
        state.pending.push_back(QueuedJournalEvent {
            event,
            retained_inputs: if position == 0 {
                retained_inputs.take()
            } else {
                None
            },
            record_bytes,
            profile,
            group: group_for_batch_position(position, reservation.event_count),
        });
    }
    state.pending_bytes = reservation.pending_bytes;
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

fn batch_record_sizes(
    events: &[JournalEvent],
    max_payload_bytes: u32,
) -> Result<Vec<u64>, JournalError> {
    events
        .iter()
        .map(|event| super::check_event_payload_size(event, max_payload_bytes))
        .collect()
}

fn total_batch_record_bytes(
    events: &[JournalEvent],
    max_payload_bytes: u32,
) -> Result<Option<u64>, JournalError> {
    events.iter().try_fold(Some(0_u64), |total, event| {
        let record_bytes = super::check_event_payload_size(event, max_payload_bytes)?;
        Ok(total.and_then(|current| current.checked_add(record_bytes)))
    })
}

fn retained_input_record_bytes(
    inputs: &crate::inputs::RunInputsRecord,
) -> Result<u64, JournalError> {
    let payload_bytes = postcard::experimental::serialized_size(inputs)
        .map_err(JournalError::PostcardEncodeFailed)?;
    let payload_len = u32::try_from(payload_bytes).map_err(|_| JournalError::PayloadTooLarge {
        len: u32::MAX,
        max: crate::constants::MAX_RUN_INPUT_BYTES,
    })?;
    if payload_len > crate::constants::MAX_RUN_INPUT_BYTES {
        return Err(JournalError::PayloadTooLarge {
            len: payload_len,
            max: crate::constants::MAX_RUN_INPUT_BYTES,
        });
    }
    u64::from(payload_len)
        .checked_add(u64::from(crate::constants::RECORD_HEADER_LEN))
        .ok_or(JournalError::QueueCapacity)
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
