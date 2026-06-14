//! Standalone model for PendingTimer and PendingTimerKind.
//!
//! Production binding targets:
//! - `crates/vb_runtime/src/shard/types.rs:29-54` - PendingTimerKind and PendingTimer

use vstd::prelude::*;

verus! {

    // Standalone model types

    /// Model of PendingTimerKind
    pub enum PendingTimerKind {
        WaitUntil,
        WaitEvent,
        Ask,
    }

    /// Model of PendingTimer
    pub struct PendingTimer {
        pub step: u64,
        pub kind: PendingTimerKind,
    }

    /// Model: PendingTimer is valid when step > 0
    pub open spec fn pending_timer_valid(t: PendingTimer) -> bool {
        t.step > 0
    }

} // verus!
