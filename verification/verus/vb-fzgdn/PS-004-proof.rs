//! PS-004 Verus proof: Generation advancement (POB-vb-fzgdn-015)
//! Production binding: crates/vb_runtime/src/shard/transitions.rs Shard::next_pending_timer_generation
//!                     crates/vb_runtime/src/shard/timer_wheel.rs TimerWheelError::GenerationExhausted
//!
//! Proves: generation increment either returns strictly greater value or None (exhaustion).

use vstd::prelude::*;

verus! {

/// Spec of checked_add(1) used by next_pending_timer_generation.
pub closed spec fn generation_advance(gen: u64) -> Option<u64> {
    if gen < u64::MAX {
        Some(gen.wrapping_add(1) as int)
    } else {
        None
    }
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
            if generation_advance(g).is_Some() {
                generation_advance(g).get_Some_0() > g
            },
{
    assert forall |g: u64|
        if generation_advance(g).is_Some() {
            generation_advance(g).get_Some_0() > g
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
    ensures generation_advance(0) == Some(1),
{
    assert(generation_advance(0) == Some(1)) by (compute);
}

} // verus!
