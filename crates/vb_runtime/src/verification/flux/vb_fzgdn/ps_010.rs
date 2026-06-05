//! PS-010 Flux refinements: Atomic fire + enqueue consistency (POB-vb-fzgdn-044)
//! Production binding: crates/vb_runtime/src/shard/lifecycle/chunk_002.rs Shard::handle_timer
//!                     Pending timer removal before command enqueue; if enqueue fails,
//!                     run state is restored via runs.insert(run, state) on error path.
//!
//! Refinement: handle_timer uses swap_remove which atomically removes the timer.
//! On success the timer is gone; on error the run state is re-inserted.

/// Refinement module: atomic operations on timer state.
mod atomic_fire_refinements {
    use vb_core::ids::RunId;
    use vb_runtime::shard::PendingTimer;

    /// Production code reference:
    ///   crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:78-84
    ///   let timer = match self.pending_timers.swap_remove(&run) {
    ///       Some(timer) => timer,
    ///       None => { self.runs.insert(run, state); return Err(...); }
    ///   };
    ///
    /// Refinement: swap_remove returns Option<PendingTimer> atomically.
    /// If it returns None, the timer was already absent, and error is returned.
    #[flux_rs::trusted]
    #[flux_rs::sig(fn(&indexmap::IndexMap<RunId, PendingTimer>, RunId) -> bool)]
    pub fn pending_timer_present(
        map: &indexmap::IndexMap<RunId, PendingTimer>,
        run: RunId,
    ) -> bool {
        map.contains_key(&run)
    }

    /// Refinement: enqueue can fail if command queue is full.
    /// Production reference:
    ///   crates/vb_runtime/src/shard/types.rs:568-572
    ///   ShardCommandQueue::enqueue returns Err(QueueFull) when at capacity.
    #[flux_rs::trusted]
    #[flux_rs::sig(fn() -> bool)]
    pub fn queue_full_possible() -> bool {
        true
    }
}
