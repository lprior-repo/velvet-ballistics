use crate::constants::MAX_BATCH_COUNT;
use crate::error::JournalError;
use crate::events::JournalEvent;

/// Ergonomic builder for batching journal events.
///
/// `cap` is the maximum number of events the builder will accept before
/// `push` returns [`JournalError::QueueFull`]. The default `new()`
/// constructor uses [`crate::constants::MAX_BATCH_COUNT`] (10_000) which
/// matches the bound enforced by [`crate::JournalWriteBatch`]. Callers
/// that need a smaller cap can use [`BatchBuilder::with_capacity`].
#[derive(Debug)]
pub struct BatchBuilder {
    events: Vec<JournalEvent>,
    cap: usize,
}

impl Default for BatchBuilder {
    /// Default builder uses the storage-wide [`MAX_BATCH_COUNT`] cap.
    fn default() -> Self {
        Self {
            events: Vec::new(),
            cap: MAX_BATCH_COUNT,
        }
    }
}

impl BatchBuilder {
    /// Creates an empty batch builder with the default cap
    /// ([`MAX_BATCH_COUNT`]).
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty batch builder with a custom event cap.
    ///
    /// `cap` must be > 0; the builder will reject further `push`es with
    /// [`JournalError::QueueFull`] once `len()` reaches `cap`.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            events: Vec::with_capacity(cap),
            cap,
        }
    }

    /// Adds an event to the batch.
    ///
    /// Returns [`JournalError::QueueFull`] if the builder has already
    /// accepted `cap` events; the batch state is left unchanged on
    /// rejection so the caller can decide whether to flush, drop, or
    /// retry.
    pub fn push(&mut self, event: JournalEvent) -> Result<(), JournalError> {
        if self.events.len() >= self.cap {
            return Err(JournalError::QueueFull);
        }
        self.events.push(event);
        Ok(())
    }

    /// Returns the configured event cap.
    #[must_use]
    pub fn cap(&self) -> usize {
        self.cap
    }

    /// Returns the number of events in the batch.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Returns true if the batch contains no events.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Returns the built event slice.
    #[must_use]
    pub fn as_slice(&self) -> &[JournalEvent] {
        &self.events
    }
}
