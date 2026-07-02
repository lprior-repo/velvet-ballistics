#![forbid(unsafe_code)]
//! Timer wheel for wait/ask deadline tracking.
//!
//! Uses `BTreeMap<Instant, Vec<TimerEntry>>` as the primary time-index
//! and `HashMap<RunId, TimerEntry>` as the run-index.
//! This gives O(log n) insert/cancel and O(k) fire where k is expired timers.

#[cfg(kani)]
use std::collections::BTreeMap as Map;
use std::collections::BTreeMap;
#[cfg(not(kani))]
use std::collections::HashMap as Map;
use std::time::Instant;

use vb_core::ids::RunId;

use super::types::{LogicalDeadline, PendingTimerKind};

/// A single timer entry keyed by its deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimerEntry {
    /// The run this timer belongs to.
    pub run: RunId,
    /// Freshness token incremented on replacement.
    pub generation: u64,
    /// The deadline that keyed this entry.
    pub deadline: Instant,
    /// Logical deadline captured when the timer was emitted.
    pub logical_deadline: Option<LogicalDeadline>,
    /// The kind of timer (Wait or Ask).
    pub kind: PendingTimerKind,
}

impl Default for TimerEntry {
    fn default() -> Self {
        Self {
            run: RunId::ZERO,
            generation: 0,
            deadline: Instant::now(),
            logical_deadline: None,
            kind: PendingTimerKind::Wait,
        }
    }
}

/// Timer wheel mutation error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerWheelError {
    /// Replacing this run's timer would overflow the freshness generation.
    GenerationExhausted,
}

/// Dual-index timer data structure for O(log n) operations.
#[derive(Debug)]
pub struct TimerWheel {
    /// Time-indexed entries for efficient fire_expired.
    by_deadline: BTreeMap<Instant, Vec<TimerEntry>>,
    /// Run-indexed entries for O(1) cancel/lookup.
    by_run: Map<RunId, TimerEntry>,
}

impl TimerWheel {
    /// Creates an empty timer wheel.
    #[must_use]
    pub fn new() -> Self {
        Self {
            by_deadline: BTreeMap::new(),
            by_run: Map::new(),
        }
    }

    /// Inserts a timer for the given run with the specified deadline.
    ///
    /// If a timer already exists for this run, it is replaced.
    pub fn insert(
        &mut self,
        run: RunId,
        deadline: Instant,
        kind: PendingTimerKind,
    ) -> Result<(), TimerWheelError> {
        self.insert_with_logical(run, deadline, None, kind)
    }

    /// Inserts a timer with an optional logical deadline.
    pub fn insert_with_logical(
        &mut self,
        run: RunId,
        deadline: Instant,
        logical_deadline: Option<LogicalDeadline>,
        kind: PendingTimerKind,
    ) -> Result<(), TimerWheelError> {
        let generation = self.next_generation(run)?;
        self.cancel(run);
        let entry = TimerEntry {
            run,
            generation,
            deadline,
            logical_deadline,
            kind,
        };
        self.by_deadline.entry(deadline).or_default().push(entry);
        self.by_run.insert(run, entry);
        Ok(())
    }

    fn next_generation(&self, run: RunId) -> Result<u64, TimerWheelError> {
        match self.by_run.get(&run).copied() {
            Some(entry) => entry
                .generation
                .checked_add(1)
                .ok_or(TimerWheelError::GenerationExhausted),
            None => Ok(1),
        }
    }

    /// Cancels the timer for the given run, if one exists.
    ///
    /// Returns true if a timer was removed.
    pub fn cancel(&mut self, run: RunId) -> bool {
        let Some(entry) = self.by_run.remove(&run) else {
            return false;
        };
        if let Some(entries) = self.by_deadline.get_mut(&entry.deadline) {
            entries.retain(|e| e.run != run);
            if entries.is_empty() {
                self.by_deadline.remove(&entry.deadline);
            }
        }
        true
    }

    /// Fires all timers whose deadlines have passed.
    ///
    /// Returns the fired entries in deadline order.
    pub fn fire_expired(&mut self, now: Instant) -> Vec<TimerEntry> {
        let mut fired = Vec::new();
        let expired_keys: Vec<Instant> = self
            .by_deadline
            .range(..=now)
            .map(|(&deadline, _)| deadline)
            .collect();

        for deadline in expired_keys {
            if let Some(entries) = self.by_deadline.remove(&deadline) {
                for entry in entries {
                    if self.by_run.get(&entry.run).copied() == Some(entry) {
                        self.by_run.remove(&entry.run);
                    }
                    fired.push(entry);
                }
            }
        }
        fired
    }

    /// Returns the next deadline, if any timers are pending.
    #[must_use]
    pub fn next_deadline(&self) -> Option<Instant> {
        self.by_deadline.first_key_value().map(|(k, _)| *k)
    }

    /// Returns true if no timers are pending.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_run.is_empty()
    }

    /// Returns the number of pending timers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_run.len()
    }

    /// Gets the kind of timer for a run, if one exists.
    #[must_use]
    pub fn get_kind(&self, run: RunId) -> Option<PendingTimerKind> {
        self.by_run.get(&run).map(|entry| entry.kind)
    }

    /// Gets the current timer entry for a run, if one exists.
    #[must_use]
    pub fn get_entry(&self, run: RunId) -> Option<TimerEntry> {
        self.by_run.get(&run).copied()
    }
}

impl Default for TimerWheel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::RunId;

    fn run(id: u64) -> RunId {
        RunId::new(id)
    }

    #[test]
    fn insert_and_cancel() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();
        assert_eq!(wheel.insert(run(1), now, PendingTimerKind::Wait), Ok(()));
        assert!(!wheel.is_empty());
        assert!(wheel.cancel(run(1)));
        assert!(wheel.is_empty());
    }

    #[test]
    fn cancel_nonexistent_returns_false() {
        let mut wheel = TimerWheel::new();
        assert!(!wheel.cancel(run(99)));
    }

    #[test]
    fn fire_expired_returns_only_past_deadlines() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();
        let past = now - std::time::Duration::from_millis(100);
        let future = now + std::time::Duration::from_secs(60);

        assert_eq!(wheel.insert(run(1), past, PendingTimerKind::Wait), Ok(()));
        assert_eq!(wheel.insert(run(2), future, PendingTimerKind::Ask), Ok(()));

        let fired = wheel.fire_expired(now);
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].run, run(1));
        assert!(!wheel.is_empty());
        assert_eq!(wheel.len(), 1);
    }

    #[test]
    fn fire_expired_drains_all_expired() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();
        let d1 = now - std::time::Duration::from_millis(200);
        let d2 = now - std::time::Duration::from_millis(100);

        assert_eq!(wheel.insert(run(1), d1, PendingTimerKind::Wait), Ok(()));
        assert_eq!(wheel.insert(run(2), d2, PendingTimerKind::Ask), Ok(()));

        let fired = wheel.fire_expired(now);
        assert_eq!(fired.len(), 2);
        assert!(wheel.is_empty());
    }

    #[test]
    fn next_deadline_returns_earliest() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();
        let early = now + std::time::Duration::from_millis(10);
        let late = now + std::time::Duration::from_millis(100);

        assert_eq!(wheel.insert(run(1), late, PendingTimerKind::Wait), Ok(()));
        assert_eq!(wheel.insert(run(2), early, PendingTimerKind::Ask), Ok(()));

        assert_eq!(wheel.next_deadline(), Some(early));
    }

    #[test]
    fn next_deadline_none_when_empty() {
        let wheel = TimerWheel::new();
        assert!(wheel.next_deadline().is_none());
    }

    #[test]
    fn replace_existing_timer() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();
        let d1 = now + std::time::Duration::from_millis(10);
        let d2 = now + std::time::Duration::from_millis(20);

        assert_eq!(wheel.insert(run(1), d1, PendingTimerKind::Wait), Ok(()));
        assert_eq!(wheel.insert(run(1), d2, PendingTimerKind::Ask), Ok(()));

        assert_eq!(wheel.len(), 1);
        assert_eq!(wheel.get_kind(run(1)), Some(PendingTimerKind::Ask));
        assert_eq!(wheel.next_deadline(), Some(d2));
    }

    #[test]
    fn multiple_runs_at_same_deadline() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();
        let deadline = now + std::time::Duration::from_millis(50);

        assert_eq!(
            wheel.insert(run(1), deadline, PendingTimerKind::Wait),
            Ok(())
        );
        assert_eq!(
            wheel.insert(run(2), deadline, PendingTimerKind::Ask),
            Ok(())
        );
        assert_eq!(
            wheel.insert(run(3), deadline, PendingTimerKind::Wait),
            Ok(())
        );

        assert_eq!(wheel.len(), 3);
        let fired = wheel.fire_expired(deadline);
        assert_eq!(fired.len(), 3);
        assert!(wheel.is_empty());
    }

    #[test]
    fn len_tracks_active_timers() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();
        assert_eq!(wheel.len(), 0);

        assert_eq!(wheel.insert(run(1), now, PendingTimerKind::Wait), Ok(()));
        assert_eq!(wheel.len(), 1);

        assert_eq!(wheel.insert(run(2), now, PendingTimerKind::Ask), Ok(()));
        assert_eq!(wheel.len(), 2);

        wheel.cancel(run(1));
        assert_eq!(wheel.len(), 1);
    }

    #[test]
    fn get_kind_returns_correct_kind() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();

        assert_eq!(wheel.insert(run(1), now, PendingTimerKind::Ask), Ok(()));
        assert_eq!(wheel.get_kind(run(1)), Some(PendingTimerKind::Ask));
        assert_eq!(wheel.get_kind(run(2)), None);
    }

    #[test]
    fn fire_expired_at_exact_deadline_fires() {
        let mut wheel = TimerWheel::new();
        let deadline = Instant::now();

        assert_eq!(
            wheel.insert(run(1), deadline, PendingTimerKind::Wait),
            Ok(())
        );
        let fired = wheel.fire_expired(deadline);
        assert_eq!(fired.len(), 1);
    }

    #[test]
    fn replacement_generation_overflow_fails_closed() {
        let mut wheel = TimerWheel::new();
        let deadline = Instant::now();
        let entry = TimerEntry {
            run: run(1),
            generation: u64::MAX,
            deadline,
            logical_deadline: None,
            kind: PendingTimerKind::Wait,
        };
        wheel.by_deadline.entry(deadline).or_default().push(entry);
        wheel.by_run.insert(run(1), entry);

        let replacement = deadline + std::time::Duration::from_secs(1);
        assert_eq!(
            wheel.insert(run(1), replacement, PendingTimerKind::Ask),
            Err(TimerWheelError::GenerationExhausted)
        );
        assert_eq!(wheel.get_entry(run(1)), Some(entry));
    }

    #[test]
    fn default_is_empty() {
        let wheel = TimerWheel::default();
        assert!(wheel.is_empty());
    }
}
