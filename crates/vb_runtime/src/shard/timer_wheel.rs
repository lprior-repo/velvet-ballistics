//! Timer wheel for wait/ask deadline tracking.
//!
//! Uses a BTreeMap<Instant, Vec<TimerEntry>> as the primary time-index
//! and a HashMap<RunId, (Instant, PendingTimerKind)> as the run-index.
//! This gives O(log n) insert/cancel and O(k) fire where k is expired timers.

use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

use vb_core::ids::RunId;

use super::types::PendingTimerKind;

/// A single timer entry keyed by its deadline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimerEntry {
    /// The run this timer belongs to.
    pub run: RunId,
    /// The kind of timer (Wait or Ask).
    pub kind: PendingTimerKind,
}

/// Dual-index timer data structure for O(log n) operations.
#[derive(Debug)]
pub struct TimerWheel {
    /// Time-indexed entries for efficient fire_expired.
    by_deadline: BTreeMap<Instant, Vec<TimerEntry>>,
    /// Run-indexed entries for O(1) cancel/lookup.
    by_run: HashMap<RunId, (Instant, PendingTimerKind)>,
}

impl TimerWheel {
    /// Creates an empty timer wheel.
    #[must_use]
    pub fn new() -> Self {
        Self {
            by_deadline: BTreeMap::new(),
            by_run: HashMap::new(),
        }
    }

    /// Inserts a timer for the given run with the specified deadline.
    ///
    /// If a timer already exists for this run, it is replaced.
    pub fn insert(&mut self, run: RunId, deadline: Instant, kind: PendingTimerKind) {
        self.cancel(run);
        let entry = TimerEntry { run, kind };
        self.by_deadline.entry(deadline).or_default().push(entry);
        self.by_run.insert(run, (deadline, kind));
    }

    /// Cancels the timer for the given run, if one exists.
    ///
    /// Returns true if a timer was removed.
    pub fn cancel(&mut self, run: RunId) -> bool {
        let Some((deadline, _kind)) = self.by_run.remove(&run) else {
            return false;
        };
        if let Some(entries) = self.by_deadline.get_mut(&deadline) {
            entries.retain(|e| e.run != run);
            if entries.is_empty() {
                self.by_deadline.remove(&deadline);
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
            .keys()
            .copied()
            .take_while(|&deadline| deadline <= now)
            .collect();

        for key in expired_keys {
            if let Some(entries) = self.by_deadline.remove(&key) {
                for entry in &entries {
                    self.by_run.remove(&entry.run);
                }
                fired.extend(entries);
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
        self.by_run.get(&run).map(|(_, kind)| *kind)
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
        wheel.insert(run(1), now, PendingTimerKind::Wait);
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

        wheel.insert(run(1), past, PendingTimerKind::Wait);
        wheel.insert(run(2), future, PendingTimerKind::Ask);

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

        wheel.insert(run(1), d1, PendingTimerKind::Wait);
        wheel.insert(run(2), d2, PendingTimerKind::Ask);

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

        wheel.insert(run(1), late, PendingTimerKind::Wait);
        wheel.insert(run(2), early, PendingTimerKind::Ask);

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

        wheel.insert(run(1), d1, PendingTimerKind::Wait);
        wheel.insert(run(1), d2, PendingTimerKind::Ask);

        assert_eq!(wheel.len(), 1);
        assert_eq!(wheel.get_kind(run(1)), Some(PendingTimerKind::Ask));
        assert_eq!(wheel.next_deadline(), Some(d2));
    }

    #[test]
    fn multiple_runs_at_same_deadline() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();
        let deadline = now + std::time::Duration::from_millis(50);

        wheel.insert(run(1), deadline, PendingTimerKind::Wait);
        wheel.insert(run(2), deadline, PendingTimerKind::Ask);
        wheel.insert(run(3), deadline, PendingTimerKind::Wait);

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

        wheel.insert(run(1), now, PendingTimerKind::Wait);
        assert_eq!(wheel.len(), 1);

        wheel.insert(run(2), now, PendingTimerKind::Ask);
        assert_eq!(wheel.len(), 2);

        wheel.cancel(run(1));
        assert_eq!(wheel.len(), 1);
    }

    #[test]
    fn get_kind_returns_correct_kind() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();

        wheel.insert(run(1), now, PendingTimerKind::Ask);
        assert_eq!(wheel.get_kind(run(1)), Some(PendingTimerKind::Ask));
        assert_eq!(wheel.get_kind(run(2)), None);
    }

    #[test]
    fn fire_expired_at_exact_deadline_fires() {
        let mut wheel = TimerWheel::new();
        let deadline = Instant::now();

        wheel.insert(run(1), deadline, PendingTimerKind::Wait);
        let fired = wheel.fire_expired(deadline);
        assert_eq!(fired.len(), 1);
    }

    #[test]
    fn default_is_empty() {
        let wheel = TimerWheel::default();
        assert!(wheel.is_empty());
    }
}
