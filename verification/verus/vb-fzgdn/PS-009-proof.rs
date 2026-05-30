//! PS-009 Verus proof: Zero-duration timer branch determinism (POB-vb-fzgdn-037)
//! Production binding: crates/vb_runtime/src/shard timer API zero-duration path
//!
//! Models: when deadline equals current tick, the timer fires immediately
//! (deadline <= now). Never mapped through wall-clock. Deterministic result.

use vstd::prelude::*;

verus! {

/// Clock model with tick value.
pub struct ClockModel {
    pub tick: u64,
}

impl ClockModel {
    /// Check if a deadline has expired.
    pub closed spec fn is_deadline_expired_spec(self, deadline: u64) -> bool {
        deadline <= self.tick
    }
}

/// Theorem: deadline == current tick means expired (zero-duration fires immediately).
proof fn test_zero_duration_fires()
    ensures
        forall |tick: u64|
            ClockModel { tick }.is_deadline_expired_spec(tick),
{
    assert forall |tick: u64|
        ClockModel { tick }.is_deadline_expired_spec(tick) by {
        assert(ClockModel { tick }.is_deadline_expired_spec(tick));
    };
}

/// Theorem: deadline > current tick means not expired.
proof fn test_future_deadline_not_expired()
    ensures
        forall |tick: u64, d: u64| d > tick ==>
            !ClockModel { tick }.is_deadline_expired_spec(d),
{
    assert forall |tick: u64, d: u64| d > tick ==>
        !ClockModel { tick }.is_deadline_expired_spec(d) by {
        // From spec: deadline <= tick is false when d > tick.
    };
}

/// Theorem: deadline < current tick means expired (past deadline).
proof fn test_past_deadline_expired()
    ensures
        forall |tick: u64, d: u64| d < tick ==>
            ClockModel { tick }.is_deadline_expired_spec(d),
{
    assert forall |tick: u64, d: u64| d < tick ==>
        ClockModel { tick }.is_deadline_expired_spec(d) by {
        // d < tick implies d <= tick.
    };
}

/// Theorem: expired check is deterministic for all tick/deadline combos.
proof fn test_expired_deterministic()
    ensures
        forall |tick: u64, d: u64|
            ClockModel { tick }.is_deadline_expired_spec(d) == (d <= tick),
{
    assert forall |tick: u64, d: u64|
        ClockModel { tick }.is_deadline_expired_spec(d) == (d <= tick) by {
        // By spec definition.
    };
}

} // verus!
