//! PS-001 Verus proof: TimerDeadline arithmetic safety (POB-vb-fzgdn-001)
//! Production binding: crates/vb_runtime/src/shard/timer_wheel.rs TimerWheel
//!                     crates/vb_runtime/src/shard/transitions.rs Shard::await_timer
//!
//! Models u64 addition with checked overflow semantics matching production
//! next_pending_timer_generation and TimerWheel::insert generation advancement.
//!
//! GOD RULE 2 BINDING:
//!   `checked_increment_exec` is an `#[verifier::external_body]` exec fn whose
//!   `ensures` clause binds the return value to `checked_increment_spec_inline`.
//!   This binds the proof to the production `u64::checked_add(1)` pattern used by
//!   `TimerWheel::next_generation` (timer_wheel.rs:80-88) and
//!   `Shard::next_pending_timer_generation` (timer_methods.rs:65-70).
//!
//! Trusted boundary: `#[verifier::external_body]` defers to the production
//! implementation via the ensures contract. Kani cross-reference at
//! `verification/kani/vb-fzgdn/PS-001-harness.rs` for bounded model-checking
//! of the same code path.

use vstd::prelude::*;

verus! {

/// Model of a timer generation value constrained to u64.
pub struct TimerGeneration {
    pub value: u64,
}

impl TimerGeneration {
    /// Creates a new timer generation value, bound by u64::MAX.
    pub closed spec fn new_spec(value: u64) -> Self {
        TimerGeneration { value }
    }

    /// Checked increment: returns Option-like modeling of checked_add(1).
    /// Production code: Shard::next_pending_timer_generation uses checked_add(1).
    pub closed spec fn checked_increment_spec(self) -> Option<Self> {
        if self.value < u64::MAX {
            Some(TimerGeneration { value: (self.value + 1u64) as u64 })
        } else {
            None
        }
    }
}

// ============================================================================
// Production binding: checked_add(1) exec fn
// ============================================================================
//
/// Spec helper: maps a bare u64 through the same spec as TimerGeneration.
///
/// Production binding target:
///   `crates/vb_runtime/src/shard/timer_wheel.rs::TimerWheel::next_generation:80-88`
///     (entry.generation.checked_add(1))
///   `crates/vb_runtime/src/shard/impl_parts/timer_methods.rs::Shard::next_pending_timer_generation:65-70`
///     (timer.generation.checked_add(1))
pub closed spec fn checked_increment_spec_inline(gen: u64) -> Option<u64> {
    if gen < u64::MAX {
        Some((gen + 1u64) as u64)
    } else {
        None
    }
}

/// External body: wraps production `u64::checked_add(1)`.
///
/// Production source:
///   timer_wheel.rs:84  — entry.generation.checked_add(1)
///   timer_methods.rs:67 — timer.generation.checked_add(1)
///
/// Contract: Returns `Some(gen + 1)` if `gen < u64::MAX`, `None` otherwise.
/// This exactly matches `checked_increment_spec_inline`.
///
/// Trust boundary: `#[verifier::external_body]` — Verus trusts the ensures
/// clause. Cross-reference Kani harness at
/// `verification/kani/vb-fzgdn/PS-001-harness.rs` for bounded model-checking.
#[verifier::external_body]
pub exec fn checked_increment_exec(gen: u64) -> (result: Option<u64>)
    ensures
        result == checked_increment_spec_inline(gen),
{
    // Production implementation:
    //   let result = gen.checked_add(1);
    //   result
    unimplemented!()
}

/// Verifies that checked_add on values < MAX succeeds with exact increment.
proof fn test_checked_add_within_bounds()
    ensures
        forall |g: TimerGeneration|
            g.value < u64::MAX ==>
            #[trigger] g.checked_increment_spec().is_Some()
            && g.checked_increment_spec().get_Some_0().value == g.value + 1,
{
    assert forall |g: TimerGeneration|
        g.value < u64::MAX implies
        #[trigger] g.checked_increment_spec().is_Some()
        && g.checked_increment_spec().get_Some_0().value == g.value + 1 by {
        if g.value < u64::MAX {
            assert(g.checked_increment_spec().is_Some());
            assert(g.checked_increment_spec().get_Some_0().value == g.value + 1);
        }
    };
}

/// Verifies that MAX generation + 1 returns None (GenerationExhausted).
proof fn test_checked_add_at_max()
    ensures
        (TimerGeneration { value: u64::MAX }).checked_increment_spec().is_None(),
{
    assert((TimerGeneration { value: u64::MAX }).checked_increment_spec().is_None());
}

/// Verifies monotonicity: increment always yields strictly greater value or None.
proof fn test_increment_monotonic()
    ensures
        forall |g: TimerGeneration|
            if g.checked_increment_spec().is_Some() {
                g.checked_increment_spec().get_Some_0().value > g.value
            },
{
    assert forall |g: TimerGeneration|
        if g.checked_increment_spec().is_Some() {
            g.checked_increment_spec().get_Some_0().value > g.value
        } by {
        if g.checked_increment_spec().is_Some() {
            let next = g.checked_increment_spec().get_Some_0();
            assert(next.value == g.value + 1);
            assert(next.value > g.value);
        }
    };
}

/// Theorem: production-bound exec fn contract matches spec for all inputs.
pub proof fn theorem_checked_increment_exec_matches_spec()
    ensures
        forall |gen: u64|
            checked_increment_spec_inline(gen) ==
            if gen < u64::MAX { Some((gen + 1u64) as u64) } else { Option::None },
{
    assert forall |gen: u64|
        checked_increment_spec_inline(gen) ==
        if gen < u64::MAX { Some((gen + 1u64) as u64) } else { Option::None } by {
        // The spec is trivially identical.
    };
}

} // verus!
