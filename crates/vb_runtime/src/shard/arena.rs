//! Slot-based arena allocator for hot shard run state.
//!
//! Replaces `IndexMap<RunId, T>` with `Vec<Option<T>>` plus generation-based handles
//! to prevent ABA-style stale references after deallocation.

use super::types::{PendingTimer, RunState, RuntimeState};
use crate::frame_pool::FramePool;
use core::fmt;
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
        if self.is_terminal() {
            self
        } else {
            Generation(self.0.saturating_add(1))
        }
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
/// Combines SlotId with Generation so stale handles can be rejected after reuse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlotHandle {
    slot_id: SlotId,
    generation: Generation,
}

impl SlotHandle {
    /// Create a new SlotHandle.
    #[inline]
    #[must_use]
    pub fn new(slot_id: SlotId, generation: Generation) -> Self {
        Self {
            slot_id,
            generation,
        }
    }

    /// Returns the slot id portion of the handle.
    #[inline]
    #[must_use]
    pub fn slot_id(self) -> SlotId {
        self.slot_id
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
        write!(f, "Handle({}:{})", self.slot_id, self.generation)
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

/// Slot-based arena allocator using `Vec<Option<T>>` storage.
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
    /// Returns a generation-checked handle for the new allocation.
    ///
    /// # Errors
    /// Returns `ArenaError::ArenaExhausted` if the arena is at maximum capacity.
    pub fn allocate(&mut self, value: T) -> Result<SlotHandle, ArenaError> {
        let slot_id = self.next_slot_id()?;
        let idx = Self::slot_index(slot_id)?;
        self.write_slot(idx, value)?;
        self.live_count = self.live_count.saturating_add(1);
        let generation = self.generation_at(idx)?;
        Ok(SlotHandle::new(slot_id, generation))
    }

    fn next_slot_id(&mut self) -> Result<SlotId, ArenaError> {
        match self.free_list.pop() {
            Some(free_id) => Ok(free_id),
            None => self.push_new_slot(),
        }
    }

    fn push_new_slot(&mut self) -> Result<SlotId, ArenaError> {
        let id = self.slots.len();
        let max_slots = usize::try_from(MAX_ARENA_SLOTS).map_err(|_| ArenaError::ArenaExhausted)?;
        if id >= max_slots {
            return Err(ArenaError::ArenaExhausted);
        }
        self.slots.push(None);
        self.generations.push(Generation::INITIAL);
        let slot = u32::try_from(id).map_err(|_| ArenaError::ArenaExhausted)?;
        Ok(SlotId::new(slot))
    }

    fn slot_index(slot_id: SlotId) -> Result<usize, ArenaError> {
        if slot_id.is_invalid() {
            return Err(ArenaError::InvalidSlotId);
        }
        usize::try_from(slot_id.raw()).map_err(|_| ArenaError::InvalidSlotId)
    }

    fn write_slot(&mut self, idx: usize, value: T) -> Result<(), ArenaError> {
        let slot = self.slots.get_mut(idx).ok_or(ArenaError::InvalidSlotId)?;
        *slot = Some(value);
        Ok(())
    }

    fn generation_at(&self, idx: usize) -> Result<Generation, ArenaError> {
        self.generations
            .get(idx)
            .copied()
            .ok_or(ArenaError::InvalidSlotId)
    }

    /// Deallocate the slot referenced by the given handle.
    /// The handle generation must match the live slot generation.
    ///
    /// # Errors
    /// Returns `ArenaError::SlotNotAllocated` if the slot is not currently allocated.
    pub fn deallocate(&mut self, handle: SlotHandle) -> Result<(), ArenaError> {
        let idx = self.validated_handle_index(handle)?;

        if let Some(slot) = self.slots.get_mut(idx) {
            *slot = None;
        }
        self.live_count = self.live_count.saturating_sub(1);

        let generation = self
            .generations
            .get_mut(idx)
            .ok_or(ArenaError::InvalidSlotId)?;
        *generation = generation.successor();

        // Terminal generations are retired permanently rather than wrapped.
        if !generation.is_terminal() {
            self.free_list.push(handle.slot_id());
        }

        Ok(())
    }

    /// Get an immutable reference to the value at the given handle.
    ///
    /// # Errors
    /// Returns `ArenaError::SlotNotAllocated` if the slot is not currently allocated.
    pub fn get(&self, handle: SlotHandle) -> Result<&T, ArenaError> {
        let idx = self.validated_handle_index(handle)?;
        match self.slots.get(idx) {
            Some(Some(v)) => Ok(v),
            _ => Err(ArenaError::SlotNotAllocated),
        }
    }

    /// Get a mutable reference to the value at the given handle.
    ///
    /// # Errors
    /// Returns `ArenaError::SlotNotAllocated` if the slot is not currently allocated.
    pub fn get_mut(&mut self, handle: SlotHandle) -> Result<&mut T, ArenaError> {
        let idx = self.validated_handle_index(handle)?;
        match self.slots.get_mut(idx) {
            Some(Some(v)) => Ok(v),
            _ => Err(ArenaError::SlotNotAllocated),
        }
    }

    /// Returns true if the given handle references a currently allocated slot.
    #[inline]
    #[must_use]
    pub fn contains(&self, handle: SlotHandle) -> bool {
        self.validated_handle_index(handle).is_ok()
    }

    fn validated_handle_index(&self, handle: SlotHandle) -> Result<usize, ArenaError> {
        let idx = Self::slot_index(handle.slot_id())?;
        let generation = self.generation_at(idx)?;
        if generation != handle.generation() {
            return Err(ArenaError::GenerationMismatch);
        }
        if self.slots.get(idx).is_none_or(Option::is_none) {
            return Err(ArenaError::SlotNotAllocated);
        }
        Ok(idx)
    }

    /// Iterate over all live allocations as (SlotId, &T) pairs.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (SlotId, &T)> {
        self.slots.iter().enumerate().filter_map(|(i, slot)| {
            slot.as_ref()
                .map(|v| (SlotId::new(u32::try_from(i).unwrap_or(u32::MAX)), v))
        })
    }

    /// Iterate over all live allocations as (SlotId, &mut T) pairs.
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (SlotId, &mut T)> {
        self.slots.iter_mut().enumerate().filter_map(|(i, slot)| {
            slot.as_mut()
                .map(|v| (SlotId::new(u32::try_from(i).unwrap_or(u32::MAX)), v))
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
        Self {
            arena: Arena::new(),
        }
    }

    /// Insert a slot into the set.
    #[inline]
    pub fn insert(&mut self, handle: SlotHandle) -> Result<(), ArenaError> {
        let (idx, new_slot) = self.ensure_insert_slot(handle)?;
        self.insert_at(idx, handle, new_slot)
    }

    fn ensure_insert_slot(&mut self, handle: SlotHandle) -> Result<(usize, bool), ArenaError> {
        let idx = Arena::<()>::slot_index(handle.slot_id())?;
        if idx > self.arena.slots.len() {
            return Err(ArenaError::InvalidSlotId);
        }
        if idx == self.arena.slots.len() {
            self.arena.slots.push(None);
            self.arena.generations.push(handle.generation());
            Ok((idx, true))
        } else {
            Ok((idx, false))
        }
    }

    fn insert_at(
        &mut self,
        idx: usize,
        handle: SlotHandle,
        new_slot: bool,
    ) -> Result<(), ArenaError> {
        let generation = self
            .arena
            .generations
            .get_mut(idx)
            .ok_or(ArenaError::InvalidSlotId)?;
        if !new_slot && *generation != handle.generation() {
            return Err(ArenaError::GenerationMismatch);
        }
        let slot = self
            .arena
            .slots
            .get_mut(idx)
            .ok_or(ArenaError::InvalidSlotId)?;
        if slot.is_some() {
            return Ok(());
        }
        *slot = Some(());
        self.arena.live_count = self.arena.live_count.saturating_add(1);
        Ok(())
    }

    /// Remove a slot from the set.
    #[inline]
    pub fn remove(&mut self, handle: SlotHandle) -> Result<(), ArenaError> {
        self.arena.deallocate(handle)
    }

    /// Returns true if the set contains the given slot.
    #[inline]
    #[must_use]
    pub fn contains(&self, handle: SlotHandle) -> bool {
        self.arena.contains(handle)
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

    /// Deallocate all state associated with a given slot handle from all arenas.
    /// This is the synchronized deallocation operation — all 4 per-run arenas
    /// are freed together atomically.
    pub fn deallocate_all(&mut self, handle: SlotHandle) -> Result<(), ArenaError> {
        // Deallocate in dependency order (no deps first).
        // Errors are collected but we continue deallocating from remaining arenas.
        let r1 = self.frame_pools.deallocate(handle);
        let r2 = self.pending_timers.deallocate(handle);
        let r3 = self.journal_sequences.deallocate(handle);
        let r4 = self.runtime_states.deallocate(handle);
        let r5 = self.run_states.deallocate(handle);
        let r6 = self.terminal_runs.remove(handle);
        // Return the first error if any occurred, Ok(()) if all succeeded.
        r1.or(r2).or(r3).or(r4).or(r5).or(r6)
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
        assert_eq!(Generation::TERMINAL.successor(), Generation::TERMINAL);
        assert!(!g.is_terminal());
    }

    #[test]
    fn arena_allocate_deallocate() {
        let mut arena: Arena<String> = Arena::new();

        let handle1 = arena.allocate("test".to_string()).unwrap();
        assert_eq!(arena.get(handle1).unwrap(), "test");
        assert_eq!(handle1.generation(), Generation::INITIAL);

        arena.deallocate(handle1).unwrap();
        assert!(matches!(
            arena.get(handle1),
            Err(ArenaError::GenerationMismatch)
        ));

        // Reuse slot
        let handle2 = arena.allocate("test2".to_string()).unwrap();
        assert_eq!(handle2.slot_id(), handle1.slot_id());
        assert_eq!(handle2.generation(), handle1.generation().successor());
        assert_eq!(arena.get(handle2).unwrap(), "test2");
        assert!(matches!(
            arena.get(handle1),
            Err(ArenaError::GenerationMismatch)
        ));
    }

    #[test]
    fn arena_contains() {
        let mut arena: Arena<i32> = Arena::new();
        let handle = arena.allocate(42).unwrap();

        assert!(arena.contains(handle));
        assert!(!Arena::<i32>::new().contains(handle));

        arena.deallocate(handle).unwrap();
        assert!(!arena.contains(handle));
    }

    #[test]
    fn slot_set_basic() {
        let mut set = SlotSet::new();
        let handle = set.arena.allocate(()).unwrap();

        assert!(set.contains(handle));
        assert_eq!(set.len(), 1);

        set.insert(handle).unwrap();
        assert_eq!(set.len(), 1);

        set.remove(handle).unwrap();
        assert!(!set.contains(handle));
        assert!(set.is_empty());
    }

    #[test]
    fn slot_set_rejects_invalid_or_gapped_handle() {
        let mut set = SlotSet::new();
        let invalid = SlotHandle::new(SlotId::INVALID, Generation::INITIAL);
        let gapped = SlotHandle::new(SlotId::new(8), Generation::INITIAL);

        assert_eq!(set.insert(invalid), Err(ArenaError::InvalidSlotId));
        assert_eq!(set.insert(gapped), Err(ArenaError::InvalidSlotId));
        assert!(set.is_empty());
    }

    #[test]
    fn slot_set_rejects_stale_reinsert() {
        let mut set = SlotSet::new();
        let handle = SlotHandle::new(SlotId::new(0), Generation::INITIAL);
        let successor = SlotHandle::new(handle.slot_id(), handle.generation().successor());

        set.insert(handle).unwrap();
        set.remove(handle).unwrap();

        assert_eq!(set.insert(handle), Err(ArenaError::GenerationMismatch));
        set.insert(successor).unwrap();
        assert!(set.contains(successor));
    }

    #[test]
    #[ignore = "types lack Default impl — must be rewritten with proper construction"]
    fn arena_manager_deallocate_all() {
        todo!(
            "arena_manager_deallocate_all requires Default on RunState, RuntimeState, PendingTimer, FramePool"
        )
    }
}
