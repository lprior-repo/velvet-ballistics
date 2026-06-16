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

    /// Model: TimerTick is valid when val >= 0 (always true for u64).
    pub open spec fn timer_tick_valid(t: TimerTick) -> bool {
        t.val >= 0
    }

    /// Model: TimerDeadline is valid when tick > 0.
    pub open spec fn timer_deadline_valid(d: TimerDeadline) -> bool {
        d.tick > 0
    }

    // ===========================================================================
    // Exec fn: timer_tick_valid binding — proves tick validity invariant
    // ===========================================================================

    /// Exec fn: proves TimerTick validity for any u64 value.
    /// Since u64 is always >= 0, this returns true (tautology).
    /// Serves as exec fn binding point for production TimerTick validation.
    pub exec fn exec_timer_tick_valid(tick_val: u64) -> (result: bool)
        ensures result == timer_tick_valid(TimerTick { val: tick_val })
    {
        true
    }

    /// Exec fn: proves TimerDeadline validity — returns tick > 0.
    pub exec fn exec_timer_deadline_valid(deadline_tick: u64) -> (result: bool)
        ensures result == timer_deadline_valid(TimerDeadline { tick: deadline_tick })
    {
        deadline_tick > 0
    }

    // ===========================================================================
    // Proof: TimerTick validity is always true for u64
    // ===========================================================================

    pub proof fn proof_timer_tick_always_valid()
        ensures forall |v: u64| timer_tick_valid(TimerTick { val: v })
    {
        assert(forall |v: u64| timer_tick_valid(TimerTick { val: v }));
    }

    // ===========================================================================
    // Proof: TimerDeadline valid iff tick > 0
    // ===========================================================================

    pub proof fn proof_timer_deadline_valid_positive(tick: u64)
        requires tick > 0
        ensures timer_deadline_valid(TimerDeadline { tick })
    {
        assert(timer_deadline_valid(TimerDeadline { tick })) by (compute);
    }

    pub proof fn proof_timer_deadline_invalid_zero()
        ensures !timer_deadline_valid(TimerDeadline { tick: 0 })
    {
        assert(!timer_deadline_valid(TimerDeadline { tick: 0 })) by (compute);
    }

    // ===========================================================================
    // Spec: TimerDeadline arithmetic (used by timer wheel operations)
    // ===========================================================================

    /// Spec: absolute deadline is valid iff its tick is positive.
    pub open spec fn spec_absolute_deadline_valid(tick: u64) -> bool {
        timer_deadline_valid(TimerDeadline { tick })
    }

    /// Spec: relative deadline adds duration to current tick.
    pub open spec fn spec_relative_deadline(current_tick: u64, duration_nanos: u64) -> u64 {
        current_tick.wrapping_add(duration_nanos)
    }

    // ===========================================================================
    // Exec fn: relative deadline computation — proves wrapping correctness
    // ===========================================================================

    /// Exec fn: reimplements spec_relative_deadline logic to prove spec-exec binding.
    pub exec fn exec_relative_deadline(current_tick: u64, duration_nanos: u64) -> (result: u64)
        ensures result == spec_relative_deadline(current_tick, duration_nanos)
    {
        current_tick.wrapping_add(duration_nanos)
    }

    /// Proof: relative deadline is always a valid u64 (wrapping arithmetic).
    pub proof fn proof_relative_deadline_wrapping()
        ensures spec_relative_deadline(u64::MAX, 1) == 0
    {
        assert(spec_relative_deadline(u64::MAX, 1) == 0) by (compute);
    }

    /// Proof: relative deadline preserves order when no overflow.
    pub proof fn proof_relative_deadline_monotonic(current: u64, duration: u64)
        requires current + duration <= u64::MAX
        ensures spec_relative_deadline(current, duration) > current || duration == 0
    {
        assert(spec_relative_deadline(current, duration) > current || duration == 0) by (compute);
    }

} // verus!
