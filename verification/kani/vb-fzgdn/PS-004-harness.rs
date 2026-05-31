//! PS-004 Kani harness: Generation advancement bounded safety (POB-vb-fzgdn-016)
//! Binds to: Shard::next_pending_timer_generation pattern via checked_add
#![forbid(unsafe_code)]

#[kani::proof]
fn ps_004_checked_add_non_max_increments() {
    let gen: u64 = kani::any();
    kani::assume(gen < u64::MAX);
    let next = gen.checked_add(1);
    assert!(next.is_some());
    assert_eq!(next.unwrap(), gen + 1);
}

#[kani::proof]
fn ps_004_checked_add_max_returns_none() {
    assert!(u64::MAX.checked_add(1).is_none());
}

#[kani::proof]
fn ps_004_zero_checked_add_is_one() {
    assert_eq!(0u64.checked_add(1), Some(1));
}

#[kani::proof]
fn ps_004_large_but_not_max_increments() {
    let gen: u64 = kani::any();
    kani::assume(gen > 0 && gen < u64::MAX - 1);
    let next = gen.checked_add(1).unwrap();
    assert!(next > gen);
    assert_eq!(next, gen + 1);
}
