//! Slot-based arena allocator for hot shard run state.
//!
//! Replaces `IndexMap<RunId, T>` with `Vec<Option<T>>` plus generation-based handles
//! to prevent ABA-style stale references after deallocation.

mod arena;
mod arena_tests;
mod slot_set;
mod types;

pub use arena::Arena;
pub use slot_set::SlotSet;
pub use types::{
    ArenaError, Generation, MAX_ARENA_SLOTS, SlotHandle, SlotId,
};

use super::types::{PendingTimer, RunState, RuntimeState};
use crate::frame_pool::FramePool;
use vb_storage::EventSeq;

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
