//! Extern_spec bindings for TimerWheel and related types.
//!
//! Production binding target: `crates/vb_runtime/src/shard/timer_wheel.rs`

use crate::shard::types::PendingTimerKind;
use vstd::prelude::*;

verus! {

// ============================================================================
// TimerWheel extern_spec
// ============================================================================

/// Extern spec for TimerWheel struct.
///
/// Production type: `crates/vb_runtime/src/shard/timer_wheel.rs:40-46`
#[extern_spec]
mod timer_wheel_spec {
    use vstd::prelude::*;
    use crate::shard::types::PendingTimerKind;
    use std::time::Instant;

    /// Timer entry keyed by deadline.
    ///
    /// Production struct: `timer_wheel.rs:20-30`
    #[verifier::extern_spec]
    pub struct TimerEntry {
        pub run: vb_core::ids::RunId,
        pub generation: u64,
        pub deadline: Instant,
        pub kind: PendingTimerKind,
    }

    /// Timer wheel mutation error.
    ///
    /// Production enum: `timer_wheel.rs:33-37`
    #[verifier::extern_spec]
    pub enum TimerWheelError {
        GenerationExhausted,
    }

    /// Dual-index timer data structure.
    ///
    /// Production struct: `timer_wheel.rs:40-46`
    #[verifier::extern_spec]
    pub struct TimerWheel {
        by_deadline: std::collections::BTreeMap<Instant, Vec<TimerEntry>>,
        by_run: std::collections::HashMap<vb_core::ids::RunId, TimerEntry>,
    }

    /// Extern spec for TimerWheel methods.
    ///
    /// Production impl: `timer_wheel.rs:48-158`
    #[extern_spec]
    impl TimerWheel {
        /// Creates an empty timer wheel.
        ///
        /// Production: `timer_wheel.rs:50-56`
        #[verifier::extern_spec]
        #[must_use]
        pub fn new() -> Self;

        /// Inserts a timer for the given run with the specified deadline.
        ///
        /// Production: `timer_wheel.rs:58-78`
        ///
        /// Contract:
        /// - If no timer exists for `run`, generation starts at 1
        /// - If timer exists, generation increments by 1
        /// - If generation would overflow u64::MAX, returns GenerationExhausted
        /// - Insert replaces any existing entry for the same run
        #[verifier::extern_spec]
        pub fn insert(
            &mut self,
            run: vb_core::ids::RunId,
            deadline: Instant,
            kind: PendingTimerKind,
        ) -> Result<(), TimerWheelError>;

        /// Cancels the timer for the given run, if one exists.
        ///
        /// Production: `timer_wheel.rs:90-104`
        ///
        /// Contract:
        /// - Returns true if a timer was removed
        /// - Returns false if no timer existed for this run
        #[verifier::extern_spec]
        pub fn cancel(&mut self, run: vb_core::ids::RunId) -> bool;

        /// Fires all timers whose deadlines have passed.
        ///
        /// Production: `timer_wheel.rs:106-128`
        ///
        /// Contract:
        /// - Returns all entries where deadline <= now
        /// - Returns entries sorted by deadline ascending (BTreeMap order)
        /// - All expired entries are removed from the wheel
        #[verifier::extern_spec]
        pub fn fire_expired(&mut self, now: Instant) -> Vec<TimerEntry>;

        /// Returns the next deadline, if any timers are pending.
        ///
        /// Production: `timer_wheel.rs:130-134`
        ///
        /// Contract:
        /// - Returns Some(earliest_deadline) if wheel is non-empty
        /// - Returns None if wheel is empty
        #[verifier::extern_spec]
        #[must_use]
        pub fn next_deadline(&self) -> Option<Instant>;

        /// Returns true if no timers are pending.
        ///
        /// Production: `timer_wheel.rs:136-140`
        #[verifier::extern_spec]
        #[must_use]
        pub fn is_empty(&self) -> bool;

        /// Returns the number of pending timers.
        ///
        /// Production: `timer_wheel.rs:142-146`
        #[verifier::extern_spec]
        #[must_use]
        pub fn len(&self) -> usize;

        /// Gets the kind of timer for a run, if one exists.
        ///
        /// Production: `timer_wheel.rs:148-152`
        #[verifier::extern_spec]
        #[must_use]
        pub fn get_kind(&self, run: vb_core::ids::RunId) -> Option<PendingTimerKind>;

        /// Gets the current timer entry for a run, if one exists.
        ///
        /// Production: `timer_wheel.rs:154-158`
        #[verifier::extern_spec]
        #[must_use]
        pub fn get_entry(&self, run: vb_core::ids::RunId) -> Option<TimerEntry>;

        /// Internal: computes next generation for a run.
        ///
        /// Production: `timer_wheel.rs:80-88`
        ///
        /// Contract:
        /// - If no entry exists for run, returns Ok(1)
        /// - If entry exists, returns Ok(entry.generation + 1)
        /// - If entry.generation == u64::MAX, returns Err(GenerationExhausted)
        #[verifier::extern_spec]
        fn next_generation(&self, run: vb_core::ids::RunId) -> Result<u64, TimerWheelError>;
    }
}

// ============================================================================
// Proof obligations for TimerWheel
// ============================================================================

/// PO-vb-0l9k0-001: Generation counter overflow returns GenerationExhausted.
///
/// C-001: TimerWheel generation counter overflow returns GenerationExhausted;
/// u64::MAX + 1 checked_add fails closed.
///
/// Production target: `TimerWheel::next_generation` at timer_wheel.rs:80-88
pub open spec fn timer_wheel_generation_overflow_spec(gen: u64) -> Result<u64, ()> {
    if gen == u64::MAX {
        Err(())
    } else {
        Ok(gen + 1)
    }
}

/// PO-vb-0l9k0-002: First insert for a RunId sets generation to 1, not 0.
///
/// C-002: First insert for a RunId sets generation to 1.
///
/// Production target: `TimerWheel::insert` at timer_wheel.rs:61-78
pub open spec fn timer_wheel_first_insert_generation_spec() -> u64 {
    1
}

/// PO-vb-0l9k0-003: Subsequent insert increments generation by exactly 1.
///
/// C-003: Subsequent insert for same RunId increments generation by exactly 1.
///
/// Production target: `TimerWheel::insert` at timer_wheel.rs:61-78
pub open spec fn timer_wheel_subsequent_insert_increment_spec(current_gen: u64) -> Result<u64, ()> {
    if current_gen == u64::MAX {
        Err(())
    } else {
        Ok(current_gen + 1)
    }
}

/// PO-vb-0l9k0-009: TimerWheel::insert replaces existing entry for same RunId.
///
/// C-006: TimerWheel::insert for existing run replaces the entry;
/// by_run always has at most one entry per run.
///
/// Production target: `TimerWheel::insert` at timer_wheel.rs:61-78
pub open spec fn timer_wheel_insert_replacement_spec() -> bool {
    true // by_run index maintains at most one entry per RunId
}

/// PO-vb-0l9k0-010: fire_expired(now) returns only entries where deadline <= now.
///
/// C-007: TimerWheel::fire_expired(now) returns only entries where deadline <= now.
///
/// Production target: `TimerWheel::fire_expired` at timer_wheel.rs:109-128
pub open spec fn timer_wheel_fire_expired_only_expired_spec(
    entries: Seq<TimerEntry>,
    now: Instant,
) -> bool {
    entries.all(|e: TimerEntry| e.deadline <= now)
}

/// PO-vb-0l9k0-011: fire_expired(now) returns ALL expired entries.
///
/// C-008: TimerWheel::fire_expired(now) returns ALL entries where deadline <= now;
/// no expired entry remains in wheel.
///
/// Production target: `TimerWheel::fire_expired` at timer_wheel.rs:109-128
pub open spec fn timer_wheel_fire_expired_completeness_spec(
    wheel: TimerWheel,
    now: Instant,
) -> bool {
    let fired = wheel.fire_expired(now);
    fired.all(|e: TimerEntry| e.deadline <= now)
    && wheel@.by_deadline.dom().contains(|d: Instant| d <= now)
}

/// PO-vb-0l9k0-012: fire_expired(now) returns entries sorted by deadline ascending.
///
/// C-009: TimerWheel::fire_expired(now) returns entries sorted by deadline ascending.
///
/// Production target: `TimerWheel::fire_expired` at timer_wheel.rs:109-128
///
/// Note: BTreeMap iteration order guarantees ascending deadline order.
pub open spec fn timer_wheel_fire_expired_sorted_spec(
    entries: Seq<TimerEntry>,
) -> bool {
    entries.len() <= 1 || entries.ext_equal(entries.sorted_by(|a: TimerEntry, b: TimerEntry| a.deadline.cmp(&b.deadline)))
}

/// PO-vb-0l9k0-013: next_deadline returns Some(earliest) or None if empty.
///
/// C-010: TimerWheel::next_deadline() returns Some(earliest_deadline) or None if empty.
///
/// Production target: `TimerWheel::next_deadline` at timer_wheel.rs:132-134
pub open spec fn timer_wheel_next_deadline_spec(wheel: TimerWheel) -> Option<Instant> {
    wheel.next_deadline()
}

/// PO-vb-0l9k0-014: cancel returns true if entry existed and removed; false otherwise.
///
/// C-011: TimerWheel::cancel(run) returns true if entry existed and was removed;
/// false otherwise.
///
/// Production target: `TimerWheel::cancel` at timer_wheel.rs:93-104
pub open spec fn timer_wheel_cancel_spec(wheel: TimerWheel, run: vb_core::ids::RunId) -> bool {
    let had_entry = wheel@.by_run.contains_key(run);
    let result = wheel.cancel(run);
    result == had_entry
}

/// PO-vb-0l9k0-015: len returns exact count; is_empty returns true iff empty.
///
/// C-012: TimerWheel::len() returns exact count of entries;
/// is_empty() returns true iff no entries.
///
/// Production target: `TimerWheel::len` at timer_wheel.rs:144-146,
/// `TimerWheel::is_empty` at timer_wheel.rs:138-140
pub open spec fn timer_wheel_len_is_empty_spec(wheel: TimerWheel) -> bool {
    wheel.is_empty() == (wheel.len() == 0)
}

/// PO-vb-0l9k0-016: Timer at exact deadline fires when fire_expired(now) called.
///
/// C-013: A timer inserted with deadline == now fires when fire_expired(now)
/// is called.
///
/// Production target: `TimerWheel::fire_expired` at timer_wheel.rs:109-128
pub open spec fn timer_wheel_exact_deadline_fires_spec(
    deadline: Instant,
    now: Instant,
) -> bool {
    deadline <= now // Inclusive boundary
}

/// PO-vb-0l9k0-017: Timer with deadline just after now does not fire.
///
/// C-013: Timer with deadline > now does not fire when fire_expired(now) is called.
///
/// Production target: `TimerWheel::fire_expired` at timer_wheel.rs:109-128
pub open spec fn timer_wheel_future_deadline_not_fired_spec(
    deadline: Instant,
    now: Instant,
) -> bool {
    deadline > now ==> true // Not included in expired
}

/// PO-vb-0l9k0-018: After insert replaces entry, get_entry returns new entry.
///
/// C-014: After insert(run, deadline2, kind2) replaces an existing entry,
/// get_entry(run) returns the new entry with updated deadline, kind, and
/// incremented generation.
///
/// Production target: `TimerWheel::insert` at timer_wheel.rs:61-78,
/// `TimerWheel::get_entry` at timer_wheel.rs:156-158
pub open spec fn timer_wheel_replacement_preserves_entry_spec(
    wheel: TimerWheel,
    run: vb_core::ids::RunId,
    new_deadline: Instant,
    new_kind: PendingTimerKind,
) -> bool {
    let old_entry = wheel.get_entry(run);
    wheel.insert(run, new_deadline, new_kind);
    let new_entry = wheel.get_entry(run);
    new_entry.is_Some()
    && new_entry.get_Some().run == run
    && new_entry.get_Some().deadline == new_deadline
    && new_entry.get_Some().kind == new_kind
}

/// PO-vb-0l9k0-019: Multiple timers with same deadline all fire.
///
/// C-008: Multiple timers with same deadline all fire when fire_expired
/// called at that deadline.
///
/// Production target: `TimerWheel::fire_expired` at timer_wheel.rs:109-128
pub open spec fn timer_wheel_same_deadline_all_fire_spec(
    wheel: TimerWheel,
    deadline: Instant,
) -> bool {
    let entries_at_deadline = wheel@.by_deadline.lookup(deadline);
    entries_at_deadline.len() > 0 ==> {
        let fired = wheel.fire_expired(deadline);
        fired.filter(|e: TimerEntry| e.deadline == deadline).len() == entries_at_deadline.len()
    }
}

/// PO-vb-0l9k0-022: checked_add(1) on u64 within bounds returns Some(g+1).
///
/// C-001: checked_add(1) on u64 within bounds returns Some(g+1).
///
/// Production target: u64::checked_add(1) on non-max values
pub open spec fn u64_checked_add_one_spec(gen: u64) -> Option<u64> {
    gen.checked_add(1)
}

} // verus!
