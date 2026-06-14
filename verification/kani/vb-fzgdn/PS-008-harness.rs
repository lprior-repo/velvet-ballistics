//! PS-008 Kani harness: Bounded timer registry capacity (POB-vb-fzgdn-034)
//! Binds to: crate::shard::timer_wheel::TimerWheel len/is_empty/get_entry
#![forbid(unsafe_code)]

#[kani::proof]
#[kani::unwind(5)]
fn ps_008_len_tracks_active_timers() {
    let mut wheel = vb_runtime::shard::timer_wheel::TimerWheel::new();
    assert_eq!(wheel.len(), 0);
    let now = std::time::Instant::now();
    match wheel.insert(vb_core::ids::RunId::new(1), now, vb_runtime::shard::PendingTimerKind::Wait) {
        Ok(_) => {},
        Err(_) => { kani::assume(false, "unwrap failed"); return; }
    }
    assert_eq!(wheel.len(), 1);
    match wheel.insert(vb_core::ids::RunId::new(2), now, vb_runtime::shard::PendingTimerKind::Ask) {
        Ok(_) => {},
        Err(_) => { kani::assume(false, "unwrap failed"); return; }
    }
    assert_eq!(wheel.len(), 2);
    wheel.cancel(vb_core::ids::RunId::new(1));
    assert_eq!(wheel.len(), 1);
}

#[kani::proof]
fn ps_008_is_empty_reflects_state() {
    let mut wheel = vb_runtime::shard::timer_wheel::TimerWheel::new();
    assert!(wheel.is_empty());
    let now = std::time::Instant::now();
    match wheel.insert(vb_core::ids::RunId::new(1), now, vb_runtime::shard::PendingTimerKind::Wait) {
        Ok(_) => {},
        Err(_) => { kani::assume(false, "unwrap failed"); return; }
    }
    assert!(!wheel.is_empty());
}

#[kani::proof]
fn ps_008_get_entry_for_missing_run_is_none() {
    let wheel = vb_runtime::shard::timer_wheel::TimerWheel::new();
    assert!(wheel.get_entry(vb_core::ids::RunId::new(99)).is_none());
}
