//! Slot-based arena allocator for hot shard run state.
//!
//! Replaces `IndexMap<RunId, T>` with `Vec<Option<T>>` plus generation-based handles
//! to prevent ABA-style stale references after deallocation.

use super::types::{ArenaError, Generation, SlotHandle, SlotId, MAX_ARENA_SLOTS};
use core::fmt;

/// Slot-based arena allocator using `Vec<Option<T>>` storage.
///
/// # Type Parameters
/// - `T`: The type stored in each slot.
/// - `MAX`: Maximum number of slots (defaults to MAX_ARENA_SLOTS).
#[derive(Debug, Clone)]
pub struct Arena<T> {
    /// Slot storage — None = free, Some(T) = allocated.
    pub(crate) slots: Vec<Option<T>>,
    /// Generation counter for each slot — incremented on deallocation.
    pub(crate) generations: Vec<Generation>,
    /// Free list of deallocated slot ids for O(1) reuse.
    pub(crate) free_list: Vec<SlotId>,
    /// Current count of live allocations.
    pub(crate) live_count: usize,
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

    pub(crate) fn slot_index(slot_id: SlotId) -> Result<usize, ArenaError> {
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
