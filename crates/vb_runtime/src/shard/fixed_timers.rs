use super::timer_wheel::TimerEntry;
use super::types::PendingTimer;
use crate::{RuntimeError, RuntimeResult};
use std::time::Instant;
use vb_core::ids::{RunId, StepIdx};

pub(crate) struct AdmittedTimers {
    entries: Box<[Option<TimerEntry>]>,
    steps: Box<[Option<(RunId, StepIdx)>]>,
    len: usize,
}

struct TimerPositions {
    old: usize,
    new: usize,
    step: usize,
    len: usize,
}

fn empty_slots<T: Clone>(capacity: usize) -> RuntimeResult<Box<[Option<T>]>> {
    let mut slots = Vec::new();
    slots
        .try_reserve_exact(capacity)
        .map_err(|_| RuntimeError::ActiveRunCapacityExceeded { capacity })?;
    slots.resize(capacity, None);
    Ok(slots.into_boxed_slice())
}

impl AdmittedTimers {
    pub(crate) fn try_new(capacity: usize) -> RuntimeResult<Self> {
        Ok(Self {
            entries: empty_slots(capacity)?,
            steps: empty_slots(capacity)?,
            len: 0,
        })
    }

    #[cfg(kani)]
    pub(crate) fn empty() -> Self {
        Self {
            entries: Box::new([]),
            steps: Box::new([]),
            len: 0,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }

    pub(crate) fn get(&self, run: RunId) -> Option<PendingTimer> {
        let entry = self
            .entries
            .iter()
            .flatten()
            .find(|entry| entry.run == run)?;
        let (_, step) = self
            .steps
            .iter()
            .flatten()
            .find(|(owner, _)| *owner == run)?;
        Some(PendingTimer {
            step: *step,
            kind: entry.kind,
            generation: entry.generation,
            deadline: entry.deadline,
        })
    }

    pub(crate) fn reserve(&self, run: RunId, capacity: usize) -> RuntimeResult<()> {
        if self.get(run).is_some() {
            return Ok(());
        }
        if self.len >= capacity || self.len >= self.entries.len() || self.len >= self.steps.len() {
            return Err(RuntimeError::ActiveRunCapacityExceeded { capacity });
        }
        Ok(())
    }

    fn positions(&self, entry: TimerEntry, capacity: usize) -> RuntimeResult<TimerPositions> {
        self.reserve(entry.run, capacity)?;
        let (old, step, len) = self.owned_positions(entry.run)?;
        let new = self
            .entries
            .iter()
            .flatten()
            .filter(|other| {
                other.run != entry.run && (other.deadline, other.run) < (entry.deadline, entry.run)
            })
            .count();
        Ok(TimerPositions {
            old,
            new,
            step,
            len,
        })
    }

    fn owned_positions(&self, run: RunId) -> RuntimeResult<(usize, usize, usize)> {
        let old = self
            .entries
            .iter()
            .position(|slot| slot.is_some_and(|old| old.run == run));
        let step = self
            .steps
            .iter()
            .position(|slot| slot.is_some_and(|(owner, _)| owner == run));
        match (old, step) {
            (Some(old), Some(step)) => Ok((old, step, self.len)),
            (None, None) => Ok((
                self.len,
                self.len,
                self.len
                    .checked_add(1)
                    .ok_or(RuntimeError::InvalidTimerFire)?,
            )),
            _ => Err(RuntimeError::InvalidTimerFire),
        }
    }

    pub(crate) fn prepare(
        &self,
        run: RunId,
        timer: PendingTimer,
        capacity: usize,
    ) -> RuntimeResult<()> {
        self.positions(Self::entry(run, timer), capacity)
            .map(|_| ())
    }

    fn entry(run: RunId, timer: PendingTimer) -> TimerEntry {
        TimerEntry {
            run,
            generation: timer.generation,
            deadline: timer.deadline,
            kind: timer.kind,
        }
    }

    pub(crate) fn insert(
        &mut self,
        run: RunId,
        timer: PendingTimer,
        capacity: usize,
    ) -> RuntimeResult<Option<PendingTimer>> {
        let entry = Self::entry(run, timer);
        let positions = self.positions(entry, capacity)?;
        let previous = self.get(run);
        self.commit(entry, timer.step, positions)?;
        Ok(previous)
    }

    fn commit(
        &mut self,
        entry: TimerEntry,
        step: StepIdx,
        positions: TimerPositions,
    ) -> RuntimeResult<()> {
        let start = positions.old.min(positions.new);
        let end = positions
            .old
            .max(positions.new)
            .checked_add(1)
            .ok_or(RuntimeError::InvalidTimerFire)?;
        let entries = self
            .entries
            .get_mut(start..end)
            .ok_or(RuntimeError::InvalidTimerFire)?;
        let metadata = self
            .steps
            .get_mut(positions.step)
            .ok_or(RuntimeError::InvalidTimerFire)?;
        Self::rotate_replace(entries, entry, positions.old < positions.new);
        *metadata = Some((entry.run, step));
        self.len = positions.len;
        Ok(())
    }

    fn rotate_replace(entries: &mut [Option<TimerEntry>], entry: TimerEntry, forward: bool) {
        if forward {
            entries.rotate_left(1);
            if let Some(slot) = entries.last_mut() {
                *slot = Some(entry);
            }
        } else {
            entries.rotate_right(1);
            if let Some(slot) = entries.first_mut() {
                *slot = Some(entry);
            }
        }
    }

    pub(crate) fn remove(&mut self, run: RunId) -> Option<PendingTimer> {
        let previous = self.get(run)?;
        let entry = self
            .entries
            .iter()
            .position(|slot| slot.is_some_and(|entry| entry.run == run))?;
        let step = self
            .steps
            .iter()
            .position(|slot| slot.is_some_and(|(owner, _)| owner == run))?;
        let len = self.len.checked_sub(1)?;
        let entries = self.entries.get_mut(entry..self.len)?;
        let steps = self.steps.get_mut(step..self.len)?;
        entries.rotate_left(1);
        if let Some(last) = entries.last_mut() {
            *last = None;
        }
        if let Some((last, rest)) = steps.split_last_mut() {
            if let Some(first) = rest.first_mut() {
                *first = *last;
            }
            *last = None;
        }
        self.len = len;
        Some(previous)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (RunId, PendingTimer)> + '_ {
        self.steps
            .iter()
            .flatten()
            .filter_map(|(run, _)| self.get(*run).map(|timer| (*run, timer)))
    }

    pub(crate) fn clear(&mut self) {
        self.entries.fill(None);
        self.steps.fill(None);
        self.len = 0;
    }

    pub(crate) fn due_peek(&self, now: Instant) -> Option<TimerEntry> {
        self.entries
            .first()
            .copied()
            .flatten()
            .filter(|entry| entry.deadline <= now)
    }
}

#[cfg(test)]
#[path = "fixed_timers_tests.rs"]
mod tests;
