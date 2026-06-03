//! Slot-based arena allocator types.
//!
//! Types for generation-based handle validation to prevent ABA-style stale references.

use core::fmt;

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
pub struct Generation(pub(crate) u64);

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
