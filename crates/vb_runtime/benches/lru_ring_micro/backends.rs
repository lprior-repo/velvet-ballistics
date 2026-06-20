use std::collections::{BTreeSet, VecDeque};

use indexmap::IndexSet;

use super::types::{LruFullError, LruRingCounters, RunId, TimerTick};

/// `IndexSet` backend — mirrors the pre-scan production representation.
pub(super) struct LruRingIndex {
    capacity: usize,
    ttl_ticks: u64,
    order: VecDeque<(RunId, TimerTick)>,
    members: IndexSet<RunId>,
    counters: LruRingCounters,
}

impl LruRingIndex {
    pub(super) fn new(capacity: usize, ttl_ticks: u64) -> Self {
        let bounded_capacity = if capacity == 0 { 1 } else { capacity };
        Self {
            capacity: bounded_capacity,
            ttl_ticks,
            order: VecDeque::with_capacity(bounded_capacity),
            members: IndexSet::with_capacity(bounded_capacity),
            counters: LruRingCounters::default(),
        }
    }

    pub(super) fn contains(&self, item: &RunId) -> bool {
        self.members.contains(item)
    }

    pub(super) fn insert(&mut self, item: RunId, now: TimerTick) -> Result<(), LruFullError> {
        if self.members.contains(&item) {
            return Ok(());
        }
        self.sweep_expired(now);
        if self.members.len() >= self.capacity {
            self.counters.capacity_overflows = self.counters.capacity_overflows.saturating_add(1);
            return Err(LruFullError {
                capacity: self.capacity,
            });
        }
        self.order.push_back((item, now));
        let _ = self.members.insert(item);
        Ok(())
    }

    pub(super) fn force_insert(&mut self, item: RunId, now: TimerTick) {
        if self.members.contains(&item) {
            return;
        }
        self.sweep_expired(now);
        let before = self.members.len();
        self.order.push_back((item, now));
        let _ = self.members.insert(item);
        if self.members.len() > before && self.members.len() > self.capacity {
            self.counters.capacity_overflows = self.counters.capacity_overflows.saturating_add(1);
        }
    }

    pub(super) fn sweep_expired(&mut self, now: TimerTick) {
        if self.ttl_ticks == 0 {
            return;
        }
        let cutoff: i128 = match now.get().checked_sub(self.ttl_ticks) {
            Some(value) => i128::from(value),
            None => -1,
        };
        while let Some(&(value, ts)) = self.order.front() {
            if i128::from(ts.get()) <= cutoff {
                let _ = self.order.pop_front();
                self.members.swap_remove(&value);
                self.counters.expired_evictions = self.counters.expired_evictions.saturating_add(1);
            } else {
                break;
            }
        }
    }

    pub(super) fn remove(&mut self, item: &RunId) {
        if self.members.swap_remove(item) {
            if let Some(position) = self.order.iter().position(|(value, _)| value == item) {
                let _ = self.order.remove(position);
            }
        }
    }

    pub(super) fn push_existing(&mut self, item: RunId, inserted_at: TimerTick) {
        self.order.push_back((item, inserted_at));
        let _ = self.members.insert(item);
    }

    pub(super) fn counters(&self) -> &LruRingCounters {
        &self.counters
    }
}

/// `BTreeSet` backend — deterministic replacement candidate.
pub(super) struct LruRingBTree {
    capacity: usize,
    ttl_ticks: u64,
    order: VecDeque<(RunId, TimerTick)>,
    members: BTreeSet<RunId>,
    counters: LruRingCounters,
}

impl LruRingBTree {
    pub(super) fn new(capacity: usize, ttl_ticks: u64) -> Self {
        let bounded_capacity = if capacity == 0 { 1 } else { capacity };
        Self {
            capacity: bounded_capacity,
            ttl_ticks,
            order: VecDeque::with_capacity(bounded_capacity),
            members: BTreeSet::new(),
            counters: LruRingCounters::default(),
        }
    }

    pub(super) fn contains(&self, item: &RunId) -> bool {
        self.members.contains(item)
    }

    pub(super) fn insert(&mut self, item: RunId, now: TimerTick) -> Result<(), LruFullError> {
        if self.members.contains(&item) {
            return Ok(());
        }
        self.sweep_expired(now);
        if self.members.len() >= self.capacity {
            self.counters.capacity_overflows = self.counters.capacity_overflows.saturating_add(1);
            return Err(LruFullError {
                capacity: self.capacity,
            });
        }
        self.order.push_back((item, now));
        let _ = self.members.insert(item);
        Ok(())
    }

    pub(super) fn force_insert(&mut self, item: RunId, now: TimerTick) {
        if self.members.contains(&item) {
            return;
        }
        self.sweep_expired(now);
        let before = self.members.len();
        self.order.push_back((item, now));
        let _ = self.members.insert(item);
        if self.members.len() > before && self.members.len() > self.capacity {
            self.counters.capacity_overflows = self.counters.capacity_overflows.saturating_add(1);
        }
    }

    pub(super) fn sweep_expired(&mut self, now: TimerTick) {
        if self.ttl_ticks == 0 {
            return;
        }
        let cutoff: i128 = match now.get().checked_sub(self.ttl_ticks) {
            Some(value) => i128::from(value),
            None => -1,
        };
        while let Some(&(value, ts)) = self.order.front() {
            if i128::from(ts.get()) <= cutoff {
                let _ = self.order.pop_front();
                self.members.remove(&value);
                self.counters.expired_evictions = self.counters.expired_evictions.saturating_add(1);
            } else {
                break;
            }
        }
    }

    pub(super) fn remove(&mut self, item: &RunId) {
        if self.members.remove(item) {
            if let Some(position) = self.order.iter().position(|(value, _)| value == item) {
                let _ = self.order.remove(position);
            }
        }
    }

    pub(super) fn push_existing(&mut self, item: RunId, inserted_at: TimerTick) {
        self.order.push_back((item, inserted_at));
        let _ = self.members.insert(item);
    }

    pub(super) fn counters(&self) -> &LruRingCounters {
        &self.counters
    }
}
