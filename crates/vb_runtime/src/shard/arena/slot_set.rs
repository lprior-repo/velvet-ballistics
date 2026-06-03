//! Membership-only set for terminal runs.
//!
//! Does not store values — just tracks which slots are "in the set".

use super::types::{ArenaError, SlotHandle};

use super::arena_core::Arena;

/// Membership-only set for terminal runs.
/// Does not store values — just tracks which slots are "in the set".
#[derive(Debug, Clone)]
pub struct SlotSet {
    pub(crate) arena: Arena<()>,
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
