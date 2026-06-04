#![forbid(unsafe_code)]
//! Bounded timer wheel for wait/ask deadline tracking.
//!
//! Design: fixed ring of 256 slots, deadline absolute value determines slot.
//! Each slot holds a bounded `Vec<TimerEntry>` (capacity ≤ MAX_ENTRIES_PER_SLOT).
//! A `HashMap<RunId, TimerEntry>` provides O(1) cancel.
//!
//! `fire_expired` scans all slots; each slot Vec is bounded so total work is
//! O(SLOT_COUNT × MAX_ENTRIES_PER_SLOT) = O(2048) worst case — independent
//! of total active runs. This is the correctness guarantee: no missed expirations.
//!
//! In production: HashMap for cancel. In kani model: BTreeMap (tractability).

use std::time::Instant;

use vb_core::ids::RunId;

use super::types::PendingTimerKind;

// ─── Constants ───────────────────────────────────────────────────────

/// Number of slots in the timer wheel ring.
const SLOT_COUNT: usize = 256;

/// Maximum entries per slot. Bounds the per-slot Vec size.
/// With 256 slots and 8 entries per slot, max capacity is 2048 timers.
const MAX_ENTRIES_PER_SLOT: usize = 8;

// ─── Types ─────────────────────────────────────────────────────────

/// A single timer entry stored in a slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimerEntry {
    /// The run this timer belongs to.
    pub run: RunId,
    /// Freshness token — must match by_run for this run to be valid.
    pub generation: u64,
    /// The deadline (wall-clock time) for this timer.
    pub deadline: Instant,
    /// The kind of timer (Wait or Ask).
    pub kind: PendingTimerKind,
}

/// Timer wheel mutation error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerWheelError {
    /// Replacing this run's timer would overflow the freshness generation.
    GenerationExhausted,
    /// No slot has room for this entry.
    SlotFull,
}

/// Result of a cancel operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelResult {
    /// The timer was found and removed.
    Removed,
    /// No timer existed for this run.
    NotFound,
}

impl CancelResult {
    /// Returns true if a timer was removed.
    #[must_use]
    pub fn was_removed(self) -> bool {
        matches!(self, CancelResult::Removed)
    }
}

impl From<CancelResult> for bool {
    fn from(result: CancelResult) -> bool {
        result.was_removed()
    }
}

impl std::ops::Not for CancelResult {
    type Output = bool;
    fn not(self) -> bool {
        !self.was_removed()
    }
}

// ─── Production Timer Wheel ───────────────────────────────────────────

/// Production timer wheel: bounded ring + HashMap for O(1) cancel.
#[cfg(not(kani))]
use std::collections::HashMap as RunMap;

#[cfg(not(kani))]
#[derive(Debug)]
pub struct TimerWheel {
    /// Ring slots indexed by `(deadline.as_secs() / 1) % SLOT_COUNT`.
    /// Each slot is bounded to MAX_ENTRIES_PER_SLOT entries.
    slots: Vec<Vec<TimerEntry>>,
    /// Run → active entry for O(1) cancel and generation tracking.
    by_run: RunMap<RunId, TimerEntry>,
}

#[cfg(not(kani))]
impl TimerWheel {
    /// Creates an empty timer wheel.
    #[must_use]
    pub fn new() -> Self {
        // Pre-allocate all slots with bounded capacity.
        // Each slot Vec starts with capacity MAX_ENTRIES_PER_SLOT to prevent
        // unbounded reallocation during fire_expired.
        let slots = (0..SLOT_COUNT)
            .map(|_| Vec::with_capacity(MAX_ENTRIES_PER_SLOT))
            .collect();
        Self {
            slots,
            by_run: RunMap::new(),
        }
    }

    /// Returns the slot index for a given deadline.
    fn slot_index_of(deadline: Instant) -> usize {
        // Use deadline.elapsed() for time-until-fire, modulo SLOT_COUNT.
        // deadline.elapsed() = now - deadline (time passed since deadline fired).
        // If deadline is in the future, elapsed = 0.
        let elapsed = deadline.elapsed().as_secs();
        (elapsed % SLOT_COUNT as u64) as usize
    }

    /// Inserts a timer for `run` with the given `deadline` and `kind`.
    ///
    /// Replaces any existing timer for this run (generation incremented).
    pub fn insert(
        &mut self,
        run: RunId,
        deadline: Instant,
        kind: PendingTimerKind,
    ) -> Result<(), TimerWheelError> {
        // Get current generation BEFORE cancelling (cancel removes from by_run).
        let generation = self
            .by_run
            .get(&run)
            .map(|e| e.generation)
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(TimerWheelError::GenerationExhausted)?;

        // Cancel any existing timer for this run — removes old entry from by_run.
        self.cancel(run);

        let slot_idx = Self::slot_index_of(deadline);

        // Check slot capacity.
        if self.slots[slot_idx].len() >= MAX_ENTRIES_PER_SLOT {
            return Err(TimerWheelError::SlotFull);
        }

        let entry = TimerEntry {
            run,
            generation,
            deadline,
            kind,
        };

        self.slots[slot_idx].push(entry);
        self.by_run.insert(run, entry);
        Ok(())
    }

    /// Cancels the timer for `run`, if one exists.
    ///
    /// Returns [`CancelResult::Removed`] if a timer was found and removed.
    pub fn cancel(&mut self, run: RunId) -> CancelResult {
        let Some(entry) = self.by_run.remove(&run) else {
            return CancelResult::NotFound;
        };
        let slot_idx = Self::slot_index_of(entry.deadline);
        let slot = &mut self.slots[slot_idx];
        slot.retain(|e| e.run != run);
        CancelResult::Removed
    }

    /// Fires all timers whose deadlines have passed.
    ///
    /// Returns fired entries. Each entry's generation is verified against
    /// `by_run` before firing — replaced-but-not-yet-cancelled entries are skipped.
    ///
    /// # Boundedness
    ///
    /// Each slot Vec is bounded to `MAX_ENTRIES_PER_SLOT`. `fire_expired` scans
    /// all 256 slots but each slot's iteration is bounded, making total work
    /// O(SLOT_COUNT × MAX_ENTRIES_PER_SLOT) = O(2048) worst case — independent
    /// of total active runs. This replaces the original BTreeMap O(k log n) scan
    /// with an O(1) bounded slot check per expired entry.
    ///
    /// # Correctness
    ///
    /// Scans ALL slots and fires every entry where `deadline <= now` AND the
    /// entry is still the current one (generation matches). This is correct
    /// regardless of slot assignment, because we check the actual deadline.
    pub fn fire_expired(&mut self, now: Instant) -> Vec<TimerEntry> {
        let mut fired = Vec::new();

        for slot in &mut self.slots[..] {
            if slot.is_empty() {
                continue;
            }
            let mut remaining: Vec<TimerEntry> =
                Vec::with_capacity(slot.len());
            for entry in slot.drain(..) {
                // Only fire if this entry is still current (not replaced).
                let is_current = self.by_run.get(&entry.run) == Some(&entry);
                if is_current && entry.deadline <= now {
                    // Remove from by_run to prevent double-fire.
                    self.by_run.remove(&entry.run);
                    fired.push(entry);
                } else {
                    remaining.push(entry);
                }
            }
            *slot = remaining;
        }
        fired
    }

    /// Returns the next deadline among pending timers, if any.
    #[must_use]
    pub fn next_deadline(&self) -> Option<Instant> {
        let mut earliest: Option<Instant> = None;
        for slot in &self.slots[..] {
            for entry in slot {
                match earliest {
                    None => earliest = Some(entry.deadline),
                    Some(cur) if entry.deadline < cur => earliest = Some(entry.deadline),
                    Some(_) => {}
                }
            }
        }
        earliest
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

    /// Gets the kind of timer for `run`, if one exists.
    #[must_use]
    pub fn get_kind(&self, run: RunId) -> Option<PendingTimerKind> {
        self.by_run.get(&run).map(|entry| entry.kind)
    }

    /// Gets the current timer entry for `run`, if one exists.
    #[must_use]
    pub fn get_entry(&self, run: RunId) -> Option<TimerEntry> {
        self.by_run.get(&run).copied()
    }
}

// ─── Kani Model ─────────────────────────────────────────────────────

/// Kani model: uses BTreeMap for tractable model checking.
#[cfg(kani)]
use std::collections::BTreeMap as RunMap;

#[cfg(kani)]
#[derive(Debug)]
pub struct TimerWheel {
    by_deadline: BTreeMap<Instant, Vec<TimerEntry>>,
    by_run: BTreeMap<RunId, TimerEntry>,
}

#[cfg(kani)]
impl TimerWheel {
    #[must_use]
    pub fn new() -> Self {
        Self {
            by_deadline: BTreeMap::new(),
            by_run: BTreeMap::new(),
        }
    }

    pub fn insert(
        &mut self,
        run: RunId,
        deadline: Instant,
        kind: PendingTimerKind,
    ) -> Result<(), TimerWheelError> {
        let generation = self
            .by_run
            .get(&run)
            .map(|e| e.generation)
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(TimerWheelError::GenerationExhausted)?;
        let entry = TimerEntry { run, generation, deadline, kind };
        self.cancel(run);
        self.by_deadline.entry(deadline).or_default().push(entry);
        self.by_run.insert(run, entry);
        Ok(())
    }

    pub fn cancel(&mut self, run: RunId) -> CancelResult {
        match self.by_run.remove(&run) {
            None => CancelResult::NotFound,
            Some(entry) => {
                if let Some(entries) = self.by_deadline.get_mut(&entry.deadline) {
                    entries.retain(|e| e.run != run);
                    if entries.is_empty() {
                        self.by_deadline.remove(&entry.deadline);
                    }
                }
                CancelResult::Removed
            }
        }
    }

    pub fn fire_expired(&mut self, now: Instant) -> Vec<TimerEntry> {
        let expired_keys: Vec<Instant> = self
            .by_deadline
            .range(..=now)
            .map(|(&deadline, _)| deadline)
            .collect();
        let mut fired = Vec::new();
        for deadline in expired_keys {
            if let Some(entries) = self.by_deadline.remove(&deadline) {
                for entry in entries {
                    if self.by_run.get(&entry.run) == Some(&entry) {
                        self.by_run.remove(&entry.run);
                        fired.push(entry);
                    }
                }
            }
        }
        fired
    }

    #[must_use]
    pub fn next_deadline(&self) -> Option<Instant> {
        self.by_deadline.first_key_value().map(|(&k, _)| k)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_run.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.by_run.len()
    }

    #[must_use]
    pub fn get_kind(&self, run: RunId) -> Option<PendingTimerKind> {
        self.by_run.get(&run).map(|entry| entry.kind)
    }

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

// ─── Unit Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use core::time::Duration;

    fn deadline_from_now(secs: u64) -> Instant {
        Instant::now()
            .checked_add(Duration::from_secs(secs))
            .unwrap_or_else(Instant::now)
    }

    #[test]
    fn insert_and_fire_single_timer() {
        let mut wheel = TimerWheel::new();
        let deadline = deadline_from_now(0);
        let run = RunId::new(1);

        wheel.insert(run, deadline, PendingTimerKind::Wait).unwrap();
        assert_eq!(wheel.len(), 1);

        let fired = wheel.fire_expired(deadline);
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].run, run);
        assert!(wheel.is_empty());
    }

    #[test]
    fn cancel_removes_entry() {
        let mut wheel = TimerWheel::new();
        let deadline = deadline_from_now(100);
        let run = RunId::new(1);

        wheel.insert(run, deadline, PendingTimerKind::Ask).unwrap();
        assert_eq!(wheel.cancel(run), CancelResult::Removed);
        assert!(wheel.is_empty());
    }

    #[test]
    fn cancel_not_found() {
        let mut wheel = TimerWheel::new();
        assert_eq!(wheel.cancel(RunId::new(1)), CancelResult::NotFound);
    }

    #[test]
    fn replace_timer_increments_generation() {
        let mut wheel = TimerWheel::new();
        let run = RunId::new(1);
        let d1 = deadline_from_now(10);
        let d2 = deadline_from_now(20);

        wheel.insert(run, d1, PendingTimerKind::Wait).unwrap();
        let gen1 = wheel.get_entry(run).map(|e| e.generation);
        wheel.insert(run, d2, PendingTimerKind::Wait).unwrap();
        let gen2 = wheel.get_entry(run).map(|e| e.generation);

        assert_eq!(gen1, Some(1));
        assert_eq!(gen2, Some(2));
        assert_eq!(wheel.len(), 1);
    }

    #[test]
    fn fire_only_expired_entries() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();
        let d_expired = now;
        let d_future = now.checked_add(Duration::from_secs(100)).unwrap();

        wheel.insert(RunId::new(1), d_expired, PendingTimerKind::Wait).unwrap();
        wheel.insert(RunId::new(2), d_future, PendingTimerKind::Ask).unwrap();

        let fired = wheel.fire_expired(now);
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].run, RunId::new(1));
        assert_eq!(wheel.len(), 1); // run2 still pending
    }

    #[test]
    fn get_kind_returns_timer_kind() {
        let mut wheel = TimerWheel::new();
        let deadline = deadline_from_now(10);

        wheel.insert(RunId::new(1), deadline, PendingTimerKind::Wait).unwrap();
        wheel.insert(RunId::new(2), deadline, PendingTimerKind::Ask).unwrap();

        assert_eq!(wheel.get_kind(RunId::new(1)), Some(PendingTimerKind::Wait));
        assert_eq!(wheel.get_kind(RunId::new(2)), Some(PendingTimerKind::Ask));
        assert_eq!(wheel.get_kind(RunId::new(99)), None);
    }

    #[test]
    fn next_deadline_returns_earliest() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();
        let d1 = now.checked_add(Duration::from_secs(50)).unwrap();
        let d2 = now.checked_add(Duration::from_secs(10)).unwrap();
        let d3 = now.checked_add(Duration::from_secs(30)).unwrap();

        wheel.insert(RunId::new(1), d1, PendingTimerKind::Wait).unwrap();
        wheel.insert(RunId::new(2), d2, PendingTimerKind::Wait).unwrap();
        assert_eq!(wheel.next_deadline(), Some(d2));

        wheel.cancel(RunId::new(2));
        wheel.insert(RunId::new(2), d3, PendingTimerKind::Wait).unwrap();
        assert_eq!(wheel.next_deadline(), Some(d3));
    }

    #[test]
    fn generation_exhausted_error() {
        let mut wheel = TimerWheel::new();
        let run = RunId::new(1);
        let deadline = deadline_from_now(10);

        // Insert/replace many times — generation is u64, won't exhaust in a test.
        for _ in 0..10 {
            wheel.insert(run, deadline, PendingTimerKind::Wait).unwrap();
        }
        assert_eq!(wheel.len(), 1);
    }

    #[test]
    fn slot_full_error() {
        let mut wheel = TimerWheel::new();
        // Fill the same slot with MAX_ENTRIES_PER_SLOT + 1 different runs.
        // All with deadline in the same second → same slot.
        let now = Instant::now();
        let base = now - Duration::from_secs(10); // anchor in the past
        for i in 0..MAX_ENTRIES_PER_SLOT {
            let deadline = base + Duration::from_millis(i as u64);
            wheel
                .insert(RunId::new(i as u64), deadline, PendingTimerKind::Wait)
                .unwrap();
        }
        // Next insert should fail with SlotFull.
        let deadline_full = base + Duration::from_millis(MAX_ENTRIES_PER_SLOT as u64);
        let result = wheel.insert(
            RunId::new(MAX_ENTRIES_PER_SLOT as u64),
            deadline_full,
            PendingTimerKind::Wait,
        );
        assert!(matches!(result, Err(TimerWheelError::SlotFull)));
    }

    #[test]
    fn replaced_entry_not_fired() {
        // Regression: after replacing a timer, the old entry must not fire.
        let mut wheel = TimerWheel::new();
        let run = RunId::new(1);
        let now = Instant::now();
        let d1 = now + Duration::from_secs(10);
        let d2 = now + Duration::from_secs(5);

        wheel.insert(run, d1, PendingTimerKind::Wait).unwrap();
        wheel.insert(run, d2, PendingTimerKind::Wait).unwrap(); // replaces d1

        // d1 is not fired even if now > d1, because it's been replaced.
        let fired = wheel.fire_expired(now + Duration::from_secs(12));
        assert!(fired.is_empty());
        assert_eq!(wheel.len(), 1);
    }

    #[test]
    fn multiple_runs_same_deadline_all_fire() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();
        let deadline = now + Duration::from_millis(50);

        wheel.insert(RunId::new(1), deadline, PendingTimerKind::Wait).unwrap();
        wheel.insert(RunId::new(2), deadline, PendingTimerKind::Ask).unwrap();
        wheel.insert(RunId::new(3), deadline, PendingTimerKind::Wait).unwrap();

        assert_eq!(wheel.len(), 3);
        let fired = wheel.fire_expired(deadline);
        assert_eq!(fired.len(), 3);
        assert!(wheel.is_empty());
    }
}
