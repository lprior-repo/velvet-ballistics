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
