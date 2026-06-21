//! Runtime surface: command-surface admission mapping.
//!
//! Maps public queue-backed admission failures to exact queue-full transition
//! summaries. This is the imperative-shell boundary that surfaces queue
//! state to production callers.

use crate::transitions::helper_runtime_queue_full_maps;

/// Public runtime queue-backed surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeQueueSurface {
    /// Submit-family command admission.
    Submit,
    /// Cancel command admission.
    Cancel,
    /// Resume command admission.
    Resume,
    /// Inspect command admission.
    Inspect,
}

/// Public runtime queue-full transition summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeQueueFullTransition {
    /// Surface that reached queue admission.
    pub surface: RuntimeQueueSurface,
    /// Queue capacity at rejection.
    pub capacity: usize,
    /// Queue depth at rejection.
    pub depth: usize,
    /// True only when the rejected command must not be admitted.
    pub rejected_without_admission: bool,
}

/// Maps a public queue-backed admission failure to an exact queue-full transition.
#[must_use]
pub const fn runtime_queue_full_error_transition(
    depth: usize,
    capacity: usize,
    surface: RuntimeQueueSurface,
) -> Option<RuntimeQueueFullTransition> {
    if helper_runtime_queue_full_maps(depth, capacity) {
        return Some(RuntimeQueueFullTransition {
            surface,
            capacity,
            depth,
            rejected_without_admission: true,
        });
    }
    None
}

#[cfg(test)]
mod tests;
