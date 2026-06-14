//! Standalone model for TimerWheel and related types.
//!
//! Production binding target: `crates/vb_runtime/src/shard/timer_wheel.rs`

use vstd::prelude::*;

verus! {

    // Standalone model types

    /// Model of PendingTimerKind
    pub enum PendingTimerKind {
        WaitUntil,
        WaitEvent,
        Ask,
    }

    /// Model of TimerEntry
    pub struct TimerEntry {
        pub run: u64,
        pub generation: u64,
        pub deadline: u64,
        pub kind: PendingTimerKind,
    }

    /// Model of TimerWheelError
    pub enum TimerWheelError {
        GenerationExhausted,
    }

    /// Model of TimerWheel
    pub struct TimerWheel {
        pub entries: Map<(u64, u64), TimerEntry>,
        pub next_generation: u64,
    }

    /// Model: TimerWheel is valid when next_generation > 0
    pub open spec fn timer_wheel_valid(tw: TimerWheel) -> bool {
        tw.next_generation > 0
    }

} // verus!
