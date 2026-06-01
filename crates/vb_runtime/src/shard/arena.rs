//! Slot-based arena allocator for hot shard run state.
//!
//! Replaces `IndexMap<RunId, T>` with `Vec<Option<T>>` plus generation-based handles
//! to prevent ABA-style stale references after deallocation.

use super::types::{PendingTimer, RuntimeState, RunState};
use core::fmt;
use crate::frame_pool::FramePool;
use vb_core::RunId;
use vb_storage::EventSeq;

/// Maximum number of slots per arena. u32::MAX reserved as INVALID sentinel.
pub const MAX_ARENA_SLOTS: u32 = u32::MAX - 1;

/// Slot identifier — index into the arena's slot vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SlotId(u32);

impl SlotId {
    /// Sentinel value indicating an invalid/unallocated slot.
    pub const INVALID: SlotId = SlotId(u32::MAX);

    /// Create a new SlotId from raw u32 value.
    #[inline]
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Returns true if this is the invalid sentinel.
    #[inline]
    #[must_use]
    pub fn is_invalid(self) -> bool {
        self.0 == u32::MAX
    }

    /// Returns the raw u32 value.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

impl fmt::Display for SlotId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SlotId({})", self.0)
    }
}

/// Generation token for ABA prevention.
/// Incremented on each deallocation; handles with stale generation are rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Generation(u64);

impl Generation {
    /// Initial generation for a freshly allocated slot.
    pub const INITIAL: Generation = Generation(0);

    /// Terminal generation — slot is permanently deallocated.
    pub const TERMINAL: Generation = Generation(u64::MAX);

    /// Returns the next generation after this one.
    #[inline]
    #[must_use]
    pub fn successor(self) -> Generation {
        Generation(self.0.wrapping_add(1))
    }

    /// Returns true if this generation is terminal (slot permanently dead).
    #[inline]
    #[must_use]
    pub fn is_terminal(self) -> bool {
        self.0 == u64::MAX
    }
}

impl fmt::Display for Generation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Gen({})", self.0)
    }
}

/// Stable handle to a slot in an arena.
/// Combines RunId (for external identity) with Generation (for lifetime safety).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlotHandle {
    run_id: RunId,
    generation: Generation,
}

impl SlotHandle {
    /// Create a new SlotHandle.
    #[inline]
    #[must_use]
    pub fn new(run_id: RunId, generation: Generation) -> Self {
        Self { run_id, generation }
    }

    /// Returns the run id portion of the handle.
    #[inline]
    #[must_use]
    pub fn run_id(self) -> RunId {
        self.run_id
    }

    /// Returns the generation portion of the handle.
    #[inline]
    #[must_use]
    pub fn generation(self) -> Generation {
        self.generation
    }
}

impl fmt::Display for SlotHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // RunId only implements Debug (via numeric_id! macro), not Display.
        // Use Debug format (:?) for run_id to satisfy Display contract.
        write!(f, "Handle({:?}:{})", self.run_id, self.generation)
    }
}

/// Errors that can occur during arena operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArenaError {
    /// Slot is currently allocated and cannot be modified.
    SlotAllocated,
    /// Slot is not currently allocated (dead or never allocated).
    SlotNotAllocated,
    /// Generation mismatch — handle is stale.
    GenerationMismatch,
    /// Arena has reached maximum capacity.
    ArenaExhausted,
    /// Invalid slot id.
    InvalidSlotId,
}

impl fmt::Display for ArenaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArenaError::SlotAllocated => write!(f, "slot is currently allocated"),
            ArenaError::SlotNotAllocated => write!(f, "slot is not allocated"),
            ArenaError::GenerationMismatch => write!(f, "generation mismatch — handle is stale"),
            ArenaError::ArenaExhausted => write!(f, "arena has reached maximum capacity"),
            ArenaError::InvalidSlotId => write!(f, "invalid slot id"),
        }
    }
}

/// Slot-based arena allocator using Vec<Option<T>> storage.
///
/// # Type Parameters
/// - `T`: The type stored in each slot.
/// - `MAX`: Maximum number of slots (defaults to MAX_ARENA_SLOTS).
#[derive(Debug, Clone)]
pub struct Arena<T> {
    /// Slot storage — None = free, Some(T) = allocated.
    slots: Vec<Option<T>>,
    /// Generation counter for each slot — incremented on deallocation.
    generations: Vec<Generation>,
    /// Free list of deallocated slot ids for O(1) reuse.
    free_list: Vec<SlotId>,
    /// Current count of live allocations.
    live_count: usize,
}

impl<T> Arena<T> {
    /// Create a new empty arena.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            generations: Vec::new(),
            free_list: Vec::new(),
            live_count: 0,
        }
    }

    /// Create a new arena with pre-allocated capacity.
    #[inline]
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            slots: Vec::with_capacity(capacity),
            generations: Vec::with_capacity(capacity),
            free_list: Vec::new(),
            live_count: 0,
        }
    }

    /// Returns the number of currently live allocations.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.live_count
    }

    /// Returns true if the arena has no live allocations.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.live_count == 0
    }

    /// Returns the total capacity (allocated slots + free list).
    #[inline]
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    /// Allocate a new slot with the given value.
    /// Returns the SlotId and Generation for the new allocation.
    ///
    /// # Errors
    /// Returns `ArenaError::ArenaExhausted` if the arena is at maximum capacity.
    pub fn allocate(&mut self, value: T) -> Result<(SlotId, Generation), ArenaError> {
        let slot_id = if let Some(free_id) = self.free_list.pop() {
            free_id
        } else {
            let id = self.slots.len();
            if id >= MAX_ARENA_SLOTS as usize {
                return Err(ArenaError::ArenaExhausted);
            }
            self.slots.push(None);
            self.generations.push(Generation::INITIAL);
            SlotId::new(id as u32)
        };

        if let Some(g) = self.generations.get_mut(slot_id.0 as usize) {
            *g = g.successor();
        }

        self.slots[slot_id.0 as usize] = Some(value);
        self.live_count += 1;

        Ok((slot_id, self.generations[slot_id.0 as usize]))
    }

    /// Deallocate the slot at the given SlotId.
    /// The slot must be currently allocated.
    ///
    /// # Errors
    /// Returns `ArenaError::SlotNotAllocated` if the slot is not currently allocated.
    pub fn deallocate(&mut self, slot_id: SlotId) -> Result<(), ArenaError> {
        if slot_id.0 as usize >= self.slots.len() {
            return Err(ArenaError::InvalidSlotId);
        }
        if self.slots[slot_id.0 as usize].is_none() {
            return Err(ArenaError::SlotNotAllocated);
        }

        self.slots[slot_id.0 as usize] = None;
        self.live_count -= 1;

        // Add to free list for O(1) reuse
        self.free_list.push(slot_id);

        Ok(())
    }

    /// Get an immutable reference to the value at the given SlotId.
    ///
    /// # Errors
    /// Returns `ArenaError::SlotNotAllocated` if the slot is not currently allocated.
    pub fn get(&self, slot_id: SlotId) -> Result<&T, ArenaError> {
        if slot_id.0 as usize >= self.slots.len() {
            return Err(ArenaError::InvalidSlotId);
        }
        match &self.slots[slot_id.0 as usize] {
            Some(v) => Ok(v),
            None => Err(ArenaError::SlotNotAllocated),
        }
    }

    /// Get a mutable reference to the value at the given SlotId.
    ///
    /// # Errors
    /// Returns `ArenaError::SlotNotAllocated` if the slot is not currently allocated.
    pub fn get_mut(&mut self, slot_id: SlotId) -> Result<&mut T, ArenaError> {
        if slot_id.0 as usize >= self.slots.len() {
            return Err(ArenaError::InvalidSlotId);
        }
        match &mut self.slots[slot_id.0 as usize] {
            Some(v) => Ok(v),
            None => Err(ArenaError::SlotNotAllocated),
        }
    }

    /// Returns true if the slot at the given SlotId is currently allocated.
    #[inline]
    #[must_use]
    pub fn contains(&self, slot_id: SlotId) -> bool {
        if slot_id.0 as usize >= self.slots.len() {
            return false;
        }
        self.slots[slot_id.0 as usize].is_some()
    }

    /// Iterate over all live allocations as (SlotId, &T) pairs.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (SlotId, &T)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| {
                slot.as_ref().map(|v| (SlotId::new(i as u32), v))
            })
    }

    /// Iterate over all live allocations as (SlotId, &mut T) pairs.
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (SlotId, &mut T)> {
        self.slots
            .iter_mut()
            .enumerate()
            .filter_map(|(i, slot)| {
                slot.as_mut().map(|v| (SlotId::new(i as u32), v))
            })
    }

    /// Clear all allocations, resetting the arena to empty state.
    /// Does not deallocate backing storage.
    pub fn clear(&mut self) {
        for slot in self.slots.iter_mut() {
            *slot = None;
        }
        self.free_list.clear();
        self.live_count = 0;
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Membership-only set for terminal runs.
/// Does not store values — just tracks which slots are "in the set".
#[derive(Debug, Clone)]
pub struct SlotSet {
    arena: Arena<()>,
}

impl SlotSet {
    /// Create a new empty SlotSet.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self { arena: Arena::new() }
    }

    /// Insert a slot into the set.
    #[inline]
    pub fn insert(&mut self, slot_id: SlotId, generation: Generation) -> Result<(), ArenaError> {
        self.arena.slots[slot_id.0 as usize] = Some(());
        self.arena.generations[slot_id.0 as usize] = generation;
        self.arena.live_count += 1;
        Ok(())
    }

    /// Remove a slot from the set.
    #[inline]
    pub fn remove(&mut self, slot_id: SlotId) -> Result<(), ArenaError> {
        self.arena.deallocate(slot_id)
    }

    /// Returns true if the set contains the given slot.
    #[inline]
    #[must_use]
    pub fn contains(&self, slot_id: SlotId) -> bool {
        self.arena.contains(slot_id)
    }

    /// Returns the number of slots in the set.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.arena.len()
    }

    /// Returns true if the set is empty.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.arena.is_empty()
    }
}

impl Default for SlotSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Manager for all 6 per-run arenas in the shard.
#[derive(Debug, Clone)]
pub struct ArenaManager {
    /// Run state arena.
    pub run_states: Arena<RunState>,
    /// Runtime state arena.
    pub runtime_states: Arena<RuntimeState>,
    /// Journal sequence arena.
    pub journal_sequences: Arena<EventSeq>,
    /// Pending timer arena.
    pub pending_timers: Arena<PendingTimer>,
    /// Terminal runs set.
    pub terminal_runs: SlotSet,
    /// Frame pool arena.
    pub frame_pools: Arena<FramePool>,
}

impl ArenaManager {
    /// Create a new empty ArenaManager.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            run_states: Arena::new(),
            runtime_states: Arena::new(),
            journal_sequences: Arena::new(),
            pending_timers: Arena::new(),
            terminal_runs: SlotSet::new(),
            frame_pools: Arena::new(),
        }
    }

    /// Deallocate all state associated with a given SlotId from all arenas.
    /// This is the synchronized deallocation operation — all 4 per-run arenas
    /// are freed together atomically.
    pub fn deallocate_all(&mut self, slot_id: SlotId) -> Result<(), ArenaError> {
        // Deallocate in dependency order (no deps first)
        let _ = self.frame_pools.deallocate(slot_id);
        let _ = self.pending_timers.deallocate(slot_id);
        let _ = self.journal_sequences.deallocate(slot_id);
        let _ = self.runtime_states.deallocate(slot_id);
        let _ = self.run_states.deallocate(slot_id);
        let _ = self.terminal_runs.remove(slot_id);
        Ok(())
    }
}

impl Default for ArenaManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_id_constants() {
        assert!(SlotId::INVALID.is_invalid());
        assert!(!SlotId::new(0).is_invalid());
    }

    #[test]
    fn generation_successor() {
        let g = Generation::INITIAL;
        assert_eq!(g.successor(), Generation(1));
        assert!(!g.is_terminal());
    }

    #[test]
    fn arena_allocate_deallocate() {
        let mut arena: Arena<String> = Arena::new();

        let (id1, gen1) = arena.allocate("test".to_string()).unwrap();
        assert_eq!(arena.get(id1).unwrap(), "test");
        assert_eq!(gen1, Generation::INITIAL);

        arena.deallocate(id1).unwrap();
        assert!(arena.get(id1).is_err());

        // Reuse slot
        let (id2, gen2) = arena.allocate("test2".to_string()).unwrap();
        assert_eq!(id2, id1); // Same slot
        assert_eq!(gen2.raw(), 1); // Generation incremented
    }

    #[test]
    fn arena_contains() {
        let mut arena: Arena<i32> = Arena::new();
        let (id, _) = arena.allocate(42).unwrap();

        assert!(arena.contains(id));
        assert!(!Arena::<i32>::new().contains(id));

        arena.deallocate(id).unwrap();
        assert!(!arena.contains(id));
    }

    #[test]
    fn slot_set_basic() {
        let mut set = SlotSet::new();
        let (id, _gen) = set.arena.allocate(()).unwrap();

        assert!(set.contains(id));
        assert_eq!(set.len(), 1);

        set.remove(id).unwrap();
        assert!(!set.contains(id));
        assert!(set.is_empty());
    }

    #[test]
    fn arena_manager_deallocate_all() {
        let mut mgr = ArenaManager::new();

        // Allocate across all arenas
        let run_slot = mgr.run_states.allocate(vb_core::RunState::default()).unwrap().0;
        let rt_slot = mgr.runtime_states.allocate(vb_core::RuntimeState::default()).unwrap().0;
        let _ = mgr.journal_sequences.allocate(vb_core::EventSeq::default()).unwrap();
        let _ = mgr.pending_timers.allocate(vb_core::PendingTimer::default()).unwrap();
        let _ = mgr.frame_pools.allocate(vb_core::FramePool::default()).unwrap();

        // Deallocate all from run_slot
        mgr.deallocate_all(run_slot).unwrap();

        // Verify all deallocated
        assert!(!mgr.run_states.contains(run_slot));
        assert!(!mgr.runtime_states.contains(rt_slot));
    }
}
