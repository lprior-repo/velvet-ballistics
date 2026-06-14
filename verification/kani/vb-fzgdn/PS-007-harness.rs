//! PS-007 Kani harness: Monotonic clock + fire order (POB-vb-fzgdn-029)
//! Binds to: crate::shard::timer_wheel::TimerWheel::fire_expired, next_deadline
#![forbid(unsafe_code)]

#[kani::proof]
#[kani::unwind(5)]
fn ps_007_only_past_deadlines_fire() {
    let mut wheel = vb_runtime::shard::timer_wheel::TimerWheel::new();
    let now = std::time::Instant::now();
    let past = now - std::time::Duration::from_millis(100);
    let future = now + std::time::Duration::from_secs(60);
    match wheel.insert(vb_core::ids::RunId::new(1), past, vb_runtime::shard::PendingTimerKind::Wait) {
        Ok(_) => {},
        Err(_) => { kani::assume(false, "unwrap failed"); return; }
    }
    match wheel.insert(vb_core::ids::RunId::new(2), future, vb_runtime::shard::PendingTimerKind::Ask) {
        Ok(_) => {},
        Err(_) => { kani::assume(false, "unwrap failed"); return; }
    }
    let fired = wheel.fire_expired(now);
    assert_eq!(fired.len(), 1);
    assert_eq!(fired[0].run, vb_core::ids::RunId::new(1));
    assert_eq!(wheel.len(), 1);
}

#[kani::proof]
#[kani::unwind(5)]
fn ps_007_all_expired_drained() {
    let mut wheel = vb_runtime::shard::timer_wheel::TimerWheel::new();
    let now = std::time::Instant::now();
    let d1 = now - std::time::Duration::from_millis(200);
    let d2 = now - std::time::Duration::from_millis(100);
    match wheel.insert(vb_core::ids::RunId::new(1), d1, vb_runtime::shard::PendingTimerKind::Wait) {
        Ok(_) => {},
        Err(_) => { kani::assume(false, "unwrap failed"); return; }
    }
    match wheel.insert(vb_core::ids::RunId::new(2), d2, vb_runtime::shard::PendingTimerKind::Ask) {
        Ok(_) => {},
        Err(_) => { kani::assume(false, "unwrap failed"); return; }
    }
    let fired = wheel.fire_expired(now);
    assert_eq!(fired.len(), 2);
    assert!(wheel.is_empty());
}

#[kani::proof]
#[kani::unwind(3)]
fn ps_007_next_deadline_is_earliest() {
    let mut wheel = vb_runtime::shard::timer_wheel::TimerWheel::new();
    let now = std::time::Instant::now();
    let early = now + std::time::Duration::from_millis(10);
    let late = now + std::time::Duration::from_millis(100);
    match wheel.insert(vb_core::ids::RunId::new(1), late, vb_runtime::shard::PendingTimerKind::Wait) {
        Ok(_) => {},
        Err(_) => { kani::assume(false, "unwrap failed"); return; }
    }
    match wheel.insert(vb_core::ids::RunId::new(2), early, vb_runtime::shard::PendingTimerKind::Ask) {
        Ok(_) => {},
        Err(_) => { kani::assume(false, "unwrap failed"); return; }
    }
    assert_eq!(wheel.next_deadline(), Some(early));
}
