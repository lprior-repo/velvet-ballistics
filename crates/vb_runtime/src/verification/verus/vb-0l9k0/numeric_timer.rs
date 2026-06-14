//! Standalone model for numeric timer seam types.
//!
//! Production binding targets:
//! - `crates/vb_runtime/src/shard/types.rs:865-974`
//!   - TimerTick (865-899)
//!   - TimerDuration (901-929)
//!   - TimerDeadline (931-961)
//!   - TimerKind (963-974)

use vstd::prelude::*;

verus! {

    // Standalone model types

    /// Model of TimerTick - monotonically increasing timer value
    pub struct TimerTick {
        pub val: u64,
    }

    /// Model of TimerDuration
    pub struct TimerDuration {
        pub nanos: u64,
    }

    /// Model of TimerDeadline
    pub struct TimerDeadline {
        pub tick: u64,
    }

    /// Model of TimerKind
    pub enum TimerKind {
        Absolute,
        Relative,
    }

    /// Model: TimerTick is valid when val >= 0
    pub open spec fn timer_tick_valid(t: TimerTick) -> bool {
        t.val >= 0
    }

    /// Model: TimerDeadline is valid when tick > 0
    pub open spec fn timer_deadline_valid(d: TimerDeadline) -> bool {
        d.tick > 0
    }

} // verus!
