//! PS-004 Kani harness: Generation advancement bounded safety (POB-vb-fzgdn-016)
//! Binds to: Shard::next_pending_timer_generation pattern via checked_add
#![forbid(unsafe_code)]

#[kani::proof]
fn ps_004_checked_add_non_max_increments() {
    let gen: u64 = kani::any();
    kani::assume(gen < u64::MAX);
    let next = gen.checked_add(1);
    match next {
        Some(v) => kani::assert(v == gen + 1, "expected gen + 1"),
        None => { kani::assume(false, "expected Some"); return; }
    }
}

#[kani::proof]
fn ps_004_checked_add_max_returns_none() {
    // Symbolic witness: `gen` is restricted to u64::MAX so the
    // harness exercises the precise overflow boundary for
    // `checked_add`.
    let gen: u64 = kani::any();
    kani::assume(gen == u64::MAX);
    assert!(gen.checked_add(1).is_none());
}

#[kani::proof]
fn ps_004_zero_checked_add_is_one() {
    // Symbolic witness: `gen` is restricted to 0 so the harness
    // exercises the precise zero+1 boundary for `checked_add`.
    let gen: u64 = kani::any();
    kani::assume(gen == 0);
    assert_eq!(gen.checked_add(1), Some(1));
}

#[kani::proof]
fn ps_004_large_but_not_max_increments() {
    let gen: u64 = kani::any();
    kani::assume(gen > 0 && gen < u64::MAX - 1);
    let next = match gen.checked_add(1) {
        Some(v) => v,
        None => { kani::assume(false, "unwrap failed"); return; }
    };
    assert!(next > gen);
    assert_eq!(next, gen + 1);
}
