//! Standalone model for TimerWheel and related types.
//!
//! Production binding target: `crates/vb_runtime/src/shard/timer_wheel.rs`

use vstd::prelude::*;

verus! {

    // Simplified model: use u64 directly for IDs (no newtype wrapping)

    /// Model: TimerWheel tracks timers by (generation, deadline)
    pub struct TimerWheel {
        pub entries: Map<(u64, u64), (u64, u64)>,
        pub next_generation: u64,
    }

    impl TimerWheel {
        pub closed spec fn entries(&self) -> Map<(u64, u64), (u64, u64)> { self.entries }
        pub closed spec fn next_generation(&self) -> u64 { self.next_generation }
    }

    /// Model: TimerWheel has valid state when next_generation > 0
    pub open spec fn timer_wheel_valid(tw: TimerWheel) -> bool {
        tw.next_generation > 0
    }
}

