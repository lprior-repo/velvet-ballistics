//! PS-005 Kani harness: Duplicate delayed-action key handling (POB-vb-fzgdn-020)
//! Binds to: crate::shard::timer_wheel::TimerWheel insert/cancel/get_kind
#![forbid(unsafe_code)]

#[kani::proof]
#[kani::unwind(3)]
fn ps_005_insert_replace_maintains_one_entry() {
    let mut wheel = vb_runtime::shard::timer_wheel::TimerWheel::new();
    let run = vb_core::ids::RunId::new(1);
    let now = std::time::Instant::now();
    let later = now + std::time::Duration::from_secs(10);
    assert!(wheel.insert(run, now, vb_runtime::shard::PendingTimerKind::Wait).is_ok());
    assert_eq!(wheel.len(), 1);
    assert_eq!(wheel.get_kind(run), Some(vb_runtime::shard::PendingTimerKind::Wait));
    assert!(wheel.insert(run, later, vb_runtime::shard::PendingTimerKind::Ask).is_ok());
    assert_eq!(wheel.len(), 1);
    assert_eq!(wheel.get_kind(run), Some(vb_runtime::shard::PendingTimerKind::Ask));
}

#[kani::proof]
#[kani::unwind(3)]
fn ps_005_cancel_removes_and_returns_true() {
    let mut wheel = vb_runtime::shard::timer_wheel::TimerWheel::new();
    let run = vb_core::ids::RunId::new(1);
    assert!(wheel.insert(run, std::time::Instant::now(), vb_runtime::shard::PendingTimerKind::Wait).is_ok());
    assert!(wheel.cancel(run));
    assert!(wheel.is_empty());
}

#[kani::proof]
fn ps_005_cancel_nonexistent_returns_false() {
    let mut wheel = vb_runtime::shard::timer_wheel::TimerWheel::new();
    assert!(!wheel.cancel(vb_core::ids::RunId::new(99)));
}
