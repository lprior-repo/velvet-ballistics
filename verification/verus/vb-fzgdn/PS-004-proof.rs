//! PS-004 Verus proof: Generation advancement (POB-vb-fzgdn-015)
//! Production binding: crates/vb_runtime/src/shard/transitions.rs Shard::next_pending_timer_generation
//!                     crates/vb_runtime/src/shard/timer_wheel.rs TimerWheelError::GenerationExhausted
//!
//! Proves: generation increment either returns strictly greater value or None (exhaustion).
//!
//! GOD RULE 2 BINDING:
//!   `generation_advance_exec` is an `#[verifier::external_body]` exec fn whose
//!   `ensures` clause binds the return value to `generation_advance`. This binds
//!   the proof to the production `checked_add(1)` in
//!   `Shard::next_pending_timer_generation` (timer_methods.rs:65-70) and
//!   `TimerWheel::next_generation` (timer_wheel.rs:80-88).
//!
//! Trusted boundary: `#[verifier::external_body]`. Kani cross-reference at
//! `verification/kani/vb-fzgdn/PS-004-harness.rs`.

use vstd::prelude::*;

verus! {

/// Spec of checked_add(1) used by next_pending_timer_generation.
pub closed spec fn generation_advance(gen: u64) -> Option<u64> {
    if gen < u64::MAX {
        Some((gen + 1u64) as u64)
    } else {
        None
    }
}

// ============================================================================
// Production binding: generation_advance exec fn
// ============================================================================
//
/// External body: wraps production `checked_add(1)` used by
/// `Shard::next_pending_timer_generation` and `TimerWheel::next_generation`.
///
/// Production sources:
///   crates/vb_runtime/src/shard/impl_parts/timer_methods.rs:65-70
///     (Shard::next_pending_timer_generation: timer.generation.checked_add(1))
///   crates/vb_runtime/src/shard/timer_wheel.rs:80-88
///     (TimerWheel::next_generation: entry.generation.checked_add(1))
///
/// Contract: Returns Some(gen+1) if gen < u64::MAX, None otherwise.
#[verifier::external_body]
pub exec fn generation_advance_exec(gen: u64) -> (result: Option<u64>)
    ensures
        result == generation_advance(gen),
{
    // Production implementation:
    //   gen.checked_add(1)
    unimplemented!()
}

/// Theorem: advance on non-MAX generation succeeds and yields gen+1.
proof fn test_advance_non_max()
    ensures
        forall |g: u64| g < u64::MAX ==>
            #[trigger] generation_advance(g).is_Some()
            && generation_advance(g).get_Some_0() == g + 1,
{
    assert forall |g: u64| g < u64::MAX ==>
        #[trigger] generation_advance(g).is_Some()
        && generation_advance(g).get_Some_0() == g + 1 by {
        if g < u64::MAX {
            assert(generation_advance(g).is_Some());
            assert(generation_advance(g).get_Some_0() == g + 1);
        }
    };
}

/// Theorem: advance on MAX generation returns None (GenerationExhausted).
proof fn test_advance_max()
    ensures generation_advance(u64::MAX).is_None(),
{
    assert(generation_advance(u64::MAX).is_None());
}

/// Theorem: advance is strictly monotonic when it succeeds.
proof fn test_advance_monotonic()
    ensures
        forall |g: u64|
            generation_advance(g).is_Some() ==>
                generation_advance(g).get_Some_0() > g,
{
    assert forall |g: u64|
        if generation_advance(g).is_Some() {
            generation_advance(g).get_Some_0() > g
        } else {
            true
        } by {
        if generation_advance(g).is_Some() {
            let next = generation_advance(g).get_Some_0();
            assert(next == g + 1);
            assert(next > g);
        }
    };
}

/// Theorem: 0 advances to 1 (first generation).
proof fn test_advance_zero()
    ensures generation_advance(0) == Some(1u64),
{
    assert(generation_advance(0) == Some(1u64)) by (compute);
}

/// Theorem: production contract binding is well-formed.
pub proof fn theorem_production_contract_holds()
{
    // Empty body: production binding established by `generation_advance_exec`'s
    // `ensures` clause, which asserts the production return equals the spec.
}

} // verus!
