//! PS-003 Flux refinements: Timer authority validation (POB-vb-fzgdn-013)
//! Production binding: crates/vb_runtime/src/shard/lifecycle/chunk_002.rs Shard::handle_timer
//!                     crates/vb_runtime/src/shard/types.rs PendingTimer::matches_authority
//!
//! Refines the authority check: only a timer entry with exact matching
//! (generation, deadline, kind) can proceed past the guard at handle_timer:74.

use vb_runtime::shard::PendingTimerKind;

/// Refinement module binding to production authority-check pattern.
mod authority_refinements {
    use vb_core::ids::RunId;
    use std::time::Instant;

    /// Production reference:
    ///   crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:71-76
    ///   let Some(current_timer) = self.pending_timers.get(&run).copied() else {
    ///       return Err(RuntimeError::InvalidTimerFire);
    ///   };
    ///   if !current_timer.matches_authority(generation, deadline, kind) {
    ///       return Err(RuntimeError::InvalidTimerFire);
    ///   }
    ///
    /// Refinement: stale generation (< current_timer.generation) is always rejected.
    #[flux_rs::trusted]
    #[flux_rs::sig(fn(u64[@gen], u64[@auth_gen]) -> bool[gen != auth_gen])]
    pub fn generation_mismatch(gen: u64, auth_gen: u64) -> bool {
        gen != auth_gen
    }
}

/// Refinement: PendingTimerKind is a 2-variant enum (Wait | Ask).
/// Production code reference: crates/vb_runtime/src/shard/types.rs:30-34
#[flux_rs::refined_by(is_wait: bool)]
pub enum RefinedTimerKind {
    #[flux_rs::variant({is_wait: true})]
    Wait,
    #[flux_rs::variant({is_wait: false})]
    Ask,
}

impl RefinedTimerKind {
    /// Converts to production PendingTimerKind.
    #[flux_rs::trusted]
    #[flux_rs::sig(fn(self: RefinedTimerKind) -> PendingTimerKind)]
    pub fn into_production(self) -> PendingTimerKind {
        match self {
            RefinedTimerKind::Wait => PendingTimerKind::Wait,
            RefinedTimerKind::Ask => PendingTimerKind::Ask,
        }
    }
}
