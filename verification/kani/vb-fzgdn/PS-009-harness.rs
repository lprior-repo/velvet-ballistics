//! PS-009 Kani harness: Zero-duration / exact-deadline timer branch (POB-vb-fzgdn-038)
//! Binds to: crate::shard::timer_wheel::TimerWheel::fire_expired
#![forbid(unsafe_code)]

#[kani::proof]
#[kani::unwind(3)]
fn ps_009_exact_deadline_fires() {
    let mut wheel = vb_runtime::shard::timer_wheel::TimerWheel::new();
    let deadline = std::time::Instant::now();
    match wheel.insert(vb_core::ids::RunId::new(1), deadline, vb_runtime::shard::PendingTimerKind::Wait) {
        Ok(_) => {},
        Err(_) => { kani::assume(false, "unwrap failed"); return; }
    }
    let fired = wheel.fire_expired(deadline);
    assert_eq!(fired.len(), 1);
    assert!(wheel.is_empty());
}

#[kani::proof]
#[kani::unwind(3)]
fn ps_009_future_deadline_not_fired() {
    let mut wheel = vb_runtime::shard::timer_wheel::TimerWheel::new();
    let now = std::time::Instant::now();
    let future = now + std::time::Duration::from_millis(1);
    match wheel.insert(vb_core::ids::RunId::new(1), future, vb_runtime::shard::PendingTimerKind::Wait) {
        Ok(_) => {},
        Err(_) => { kani::assume(false, "unwrap failed"); return; }
    }
    let fired = wheel.fire_expired(now);
    assert_eq!(fired.len(), 0);
    assert_eq!(wheel.len(), 1);
}

#[kani::proof]
#[kani::unwind(3)]
fn ps_009_fire_at_exact_preserves_untouched_future() {
    let mut wheel = vb_runtime::shard::timer_wheel::TimerWheel::new();
    let now = std::time::Instant::now();
    let future = now + std::time::Duration::from_secs(10);
    match wheel.insert(vb_core::ids::RunId::new(1), now, vb_runtime::shard::PendingTimerKind::Wait) {
        Ok(_) => {},
        Err(_) => { kani::assume(false, "unwrap failed"); return; }
    }
    match wheel.insert(vb_core::ids::RunId::new(2), future, vb_runtime::shard::PendingTimerKind::Ask) {
        Ok(_) => {},
        Err(_) => { kani::assume(false, "unwrap failed"); return; }
    }
    let fired = wheel.fire_expired(now);
    assert_eq!(fired.len(), 1);
    assert_eq!(wheel.len(), 1);
    assert!(wheel.get_entry(vb_core::ids::RunId::new(2)).is_some());
}
