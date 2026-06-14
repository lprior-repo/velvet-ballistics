//! Standalone model for numeric timer seam types.
//!
//! Production binding targets:
//! - `crates/vb_runtime/src/shard/types.rs:865-974` - TimerTick

use vstd::prelude::*;

verus! {

    // Standalone model types
    struct Instant(u64);

    impl Instant {
        pub closed spec fn val(&self) -> u64 { self.0 }
    }

    struct TimerTick {
        pub tick: u64,
        pub deadline: Instant,
    }

    impl TimerTick {
        pub closed spec fn tick(&self) -> u64 { self.tick }
        pub closed spec fn deadline(&self) -> Instant { self.deadline }
    }

    /// Model: TimerTick deadline is always >= tick timestamp
    pub closed spec fn timer_tick_valid(tt: TimerTick) -> bool {
        tt.deadline.val() >= tt.tick
    }
}

