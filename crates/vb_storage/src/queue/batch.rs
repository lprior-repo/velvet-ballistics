use crate::error::JournalError;
use crate::events::JournalEvent;

/// Ergonomic builder for batching journal events.
///
/// # Bound contract
///
/// Constructed via [`BatchBuilder::with_capacity`] so that growth is
/// bounded and fallible. Use [`BatchBuilder::try_push`] to add events
/// while observing the configured capacity; pushing past the bound
/// returns [`JournalError::QueueFull`] instead of growing unbounded.
#[derive(Debug)]
pub struct BatchBuilder {
    events: Vec<JournalEvent>,
    capacity: usize,
}

impl BatchBuilder {
    /// Creates a bounded batch builder with the given capacity.
    ///
    /// Returns [`JournalError::QueueCapacity`] if `capacity == 0` so the
    /// builder can never enter an unsized state. Mirrors the
    /// `JournalWriterQueue::new` contract.
    pub fn with_capacity(capacity: usize) -> Result<Self, JournalError> {
        if capacity == 0 {
            return Err(JournalError::QueueCapacity);
        }
        Ok(Self {
            events: Vec::with_capacity(capacity),
            capacity,
        })
    }

    /// Fallible bounded push: rejects with [`JournalError::QueueFull`]
    /// when the configured capacity would be exceeded.
    pub fn try_push(&mut self, event: JournalEvent) -> Result<(), JournalError> {
        if self.events.len() >= self.capacity {
            return Err(JournalError::QueueFull);
        }
        self.events.push(event);
        Ok(())
    }

    /// Returns the configured maximum capacity.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the number of events currently in the batch.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Returns true if the batch contains no events.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Returns true if the batch is at its configured capacity.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.events.len() >= self.capacity
    }

    /// Returns the built event slice.
    #[must_use]
    pub fn as_slice(&self) -> &[JournalEvent] {
        &self.events
    }
}
