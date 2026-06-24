use crate::events::JournalEvent;

/// Ergonomic builder for batching journal events.
#[derive(Debug, Default)]
pub struct BatchBuilder {
    events: Vec<JournalEvent>,
}

impl BatchBuilder {
    /// Creates an empty batch builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a batch builder with pre-allocated capacity.
    ///
    /// CC-003 / SA-006 fix: producers that know the predicted batch
    /// size in advance (e.g. caller-side chunking that knows it will
    /// emit exactly `n` events before flushing) reuse this constructor
    /// to avoid the `Vec::new()` doubling-reallocations on growth.
    /// `n` is the initial capacity; pushing more than `n` events is
    /// still permitted and falls back to `Vec`'s standard growth.
    #[must_use]
    pub fn with_capacity(n: usize) -> Self {
        Self {
            events: Vec::with_capacity(n),
        }
    }

    /// Adds an event to the batch.
    pub fn push(&mut self, event: JournalEvent) {
        self.events.push(event);
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