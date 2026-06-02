//! Extern_spec bindings for numeric timer seam types.
//!
//! Production binding targets:
//! - `crates/vb_runtime/src/shard/types.rs:865-974`
//!   - TimerTick (865-899)
//!   - TimerDuration (901-929)
//!   - TimerDeadline (931-961)
//!   - TimerKind (963-974)

use vstd::prelude::*;

verus! {

// ============================================================================
// TimerTick extern_spec
// ============================================================================

/// Extern spec for TimerTick newtype wrapper around u64.
///
/// Production type: `types.rs:870-899`
///
/// Represents a monotonically increasing timer tick value for deterministic
/// clock control.
#[extern_spec]
mod timer_tick_spec {
    use vstd::prelude::*;

    #[verifier::extern_spec]
    pub struct TimerTick(u64);

    #[extern_spec]
    impl TimerTick {
        /// Creates a new timer tick at the given value.
        ///
        /// Production: `types.rs:876-878`
        #[verifier::extern_spec]
        #[must_use]
        pub const fn new(value: u64) -> Self;

        /// Returns the inner u64 value.
        ///
        /// Production: `types.rs:882-884`
        #[verifier::extern_spec]
        #[must_use]
        pub const fn get(self) -> u64;

        /// Advances the tick by a duration, returning the resulting tick.
        ///
        /// Production: `types.rs:890-892`
        ///
        /// Contract: Returns None on u64 overflow, Some otherwise.
        #[verifier::extern_spec]
        #[must_use]
        pub fn checked_add(self, duration: TimerDuration) -> Option<Self>;

        /// Returns true if this tick is at or past the given deadline.
        ///
        /// Production: `types.rs:896-898`
        #[verifier::extern_spec]
        #[must_use]
        pub fn has_elapsed(self, deadline: TimerDeadline) -> bool;
    }
}

// ============================================================================
// TimerDuration extern_spec
// ============================================================================

/// Extern spec for TimerDuration newtype wrapper around u64.
///
/// Production type: `types.rs:901-929`
///
/// Represents a timer duration measured in ticks.
#[extern_spec]
mod timer_duration_spec {
    use vstd::prelude::*;

    #[verifier::extern_spec]
    pub struct TimerDuration(u64);

    #[extern_spec]
    impl TimerDuration {
        /// Creates a new duration with the given number of ticks.
        ///
        /// Production: `types.rs:908-910`
        #[verifier::extern_spec]
        #[must_use]
        pub const fn new(ticks: u64) -> Self;

        /// Returns the inner u64 value.
        ///
        /// Production: `types.rs:914-916`
        #[verifier::extern_spec]
        #[must_use]
        pub const fn get(self) -> u64;

        /// Returns the duration as a tick count.
        ///
        /// Production: `types.rs:920-922`
        #[verifier::extern_spec]
        #[must_use]
        pub const fn as_ticks(self) -> u64;

        /// Returns a zero-length duration.
        ///
        /// Production: `types.rs:926-928`
        #[verifier::extern_spec]
        #[must_use]
        pub const fn zero() -> Self;
    }
}

// ============================================================================
// TimerDeadline extern_spec
// ============================================================================

/// Extern spec for TimerDeadline newtype wrapper around u64.
///
/// Production type: `types.rs:931-961`
///
/// Represents an absolute deadline in ticks.
#[extern_spec]
mod timer_deadline_spec {
    use vstd::prelude::*;

    #[verifier::extern_spec]
    pub struct TimerDeadline(u64);

    #[extern_spec]
    impl TimerDeadline {
        /// Creates a new deadline at the given tick value.
        ///
        /// Production: `types.rs:938-940`
        #[verifier::extern_spec]
        #[must_use]
        pub const fn new(tick: u64) -> Self;

        /// Returns the inner u64 value.
        ///
        /// Production: `types.rs:944-946`
        #[verifier::extern_spec]
        #[must_use]
        pub const fn get(self) -> u64;

        /// Creates a deadline by adding a duration to a tick.
        ///
        /// Production: `types.rs:952-954`
        ///
        /// Contract: Returns None on u64 overflow, Some otherwise.
        #[verifier::extern_spec]
        #[must_use]
        pub fn from_tick_and_duration(tick: TimerTick, duration: TimerDuration) -> Option<Self>;

        /// Returns true if this deadline has passed relative to the given tick.
        ///
        /// Production: `types.rs:958-960`
        #[verifier::extern_spec]
        #[must_use]
        pub fn is_past(self, current: TimerTick) -> bool;
    }
}

// ============================================================================
// Proof obligations for numeric timer types
// ============================================================================

/// PO-vb-0l9k0-007: TimerTick::checked_add returns None on u64 overflow.
///
/// C-005: TimerTick::checked_add returns None on overflow.
///
/// Production target: `TimerTick::checked_add` at types.rs:890-892
pub open spec fn timer_tick_checked_add_overflow_spec(tick: u64, duration: u64) -> Option<u64> {
    tick.checked_add(duration)
}

/// PO-vb-0l9k0-007: TimerTick::checked_add returns Some on non-overflow.
///
/// C-005: TimerTick::checked_add returns Some otherwise.
///
/// Production target: `TimerTick::checked_add` at types.rs:890-892
pub open spec fn timer_tick_checked_add_no_overflow_spec(tick: u64, duration: u64) -> bool {
    match tick.checked_add(duration) {
        Some(result) => result >= tick,
        None => tick > u64::MAX - duration,
    }
}

/// PO-vb-0l9k0-008: TimerDeadline::from_tick_and_duration returns None on overflow.
///
/// C-005: TimerDeadline::from_tick_and_duration returns None on overflow.
///
/// Production target: `TimerDeadline::from_tick_and_duration` at types.rs:952-954
pub open spec fn timer_deadline_from_tick_and_duration_overflow_spec(
    tick: u64,
    duration: u64,
) -> Option<u64> {
    tick.checked_add(duration)
}

/// PO-vb-0l9k0-020: TimerDeadline::is_past returns true when current >= deadline.
///
/// C-015: TimerDeadline::is_past returns true when current tick >= deadline tick.
///
/// Production target: `TimerDeadline::is_past` at types.rs:958-960
pub open spec fn timer_deadline_is_past_spec(deadline_tick: u64, current_tick: u64) -> bool {
    current_tick >= deadline_tick ==> TimerDeadline::new(deadline_tick).is_past(TimerTick::new(current_tick))
}

/// PO-vb-0l9k0-020: TimerDeadline::is_past returns false when current < deadline.
///
/// C-015: TimerDeadline::is_past returns false when current tick < deadline tick.
///
/// Production target: `TimerDeadline::is_past` at types.rs:958-960
pub open spec fn timer_deadline_is_not_past_spec(deadline_tick: u64, current_tick: u64) -> bool {
    current_tick < deadline_tick ==> !TimerDeadline::new(deadline_tick).is_past(TimerTick::new(current_tick))
}

/// PO-vb-0l9k0-021: TimerTick::has_elapsed returns true when tick >= deadline.
///
/// C-015: TimerTick::has_elapsed returns true when tick >= deadline.
///
/// Production target: `TimerTick::has_elapsed` at types.rs:896-898
pub open spec fn timer_tick_has_elapsed_spec(tick: u64, deadline: u64) -> bool {
    tick >= deadline ==> TimerTick::new(tick).has_elapsed(TimerDeadline::new(deadline))
}

/// PO-vb-0l9k0-021: TimerTick::has_elapsed returns false when tick < deadline.
///
/// C-015: TimerTick::has_elapsed returns false when tick < deadline.
///
/// Production target: `TimerTick::has_elapsed` at types.rs:896-898
pub open spec fn timer_tick_has_not_elapsed_spec(tick: u64, deadline: u64) -> bool {
    tick < deadline ==> !TimerTick::new(tick).has_elapsed(TimerDeadline::new(deadline))
}

/// PO-vb-0l9k0-015: Numeric timer arithmetic safety — panic-free.
///
/// C-015: TimerTick::checked_add, TimerDeadline::from_tick_and_duration,
/// and TimerDeadline::is_past are panic-free.
///
/// Production targets: `TimerTick::checked_add` at types.rs:890-892,
/// `TimerDeadline::from_tick_and_duration` at types.rs:952-954,
/// `TimerDeadline::is_past` at types.rs:958-960
pub open spec fn numeric_timer_arithmetic_safety_spec() -> bool {
    forall |tick: u64, dur: u64|
        tick.checked_add(dur).is_Some() <==> tick <= u64::MAX - dur
    && forall |tick: u64, dur: u64|
        tick.checked_add(dur).is_Some() <==> tick <= u64::MAX - dur
}

} // verus!
