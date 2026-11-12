#![forbid(unsafe_code)]

//! Bounded cursor iterator for a validated retry policy.

use super::{RetryCursor, RetryPolicy};

/// Bounded iterator over the cursors a validated [`RetryPolicy`] admits.
///
/// Yields the initial cursor (`attempt == 1`, `delay_ms == 0`) first; every
/// later cursor carries the delay that precedes its attempt. A caller can
/// therefore write `for cursor in policy.cursors(max)?.take(..)` and sleep
/// `cursor.delay_ms` before running attempt `cursor.attempt`.
#[derive(Debug, Clone)]
pub struct RetryCursorIter {
    pub(super) policy: RetryPolicy,
    pub(super) max_interval_ms: u64,
    pub(super) cursor: Option<RetryCursor>,
}

impl Iterator for RetryCursorIter {
    type Item = RetryCursor;

    fn next(&mut self) -> Option<RetryCursor> {
        let current = self.cursor?;
        if current.exhausted {
            self.cursor = None;
            return None;
        }
        self.cursor = self.policy.next_cursor(self.max_interval_ms, current).ok();
        Some(current)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.cursor {
            Some(cursor) if !cursor.exhausted => {
                let remaining = usize::from(cursor.remaining);
                (remaining, Some(remaining))
            }
            _ => (0, Some(0)),
        }
    }
}

impl ExactSizeIterator for RetryCursorIter {}
