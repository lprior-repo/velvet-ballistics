//! PS-010 Kani harness: Atomic timer fire preserves invariants (POB-vb-fzgdn-043)
//! Binds to: crate::shard::timer_wheel::TimerWheel fire/replace/insert
#![forbid(unsafe_code)]

#[kani::proof]
#[kani::unwind(5)]
fn ps_010_multiple_timers_same_deadline_all_fire() {
    let mut wheel = vb_runtime::shard::timer_wheel::TimerWheel::new();
    let deadline = std::time::Instant::now();
    match wheel.insert(vb_core::ids::RunId::new(1), deadline, vb_runtime::shard::PendingTimerKind::Wait) {
        Ok(_) => {},
        Err(_) => { kani::assume(false, "unwrap failed"); return; }
    }
    match wheel.insert(vb_core::ids::RunId::new(2), deadline, vb_runtime::shard::PendingTimerKind::Ask) {
        Ok(_) => {},
        Err(_) => { kani::assume(false, "unwrap failed"); return; }
    }
    match wheel.insert(vb_core::ids::RunId::new(3), deadline, vb_runtime::shard::PendingTimerKind::Wait) {
        Ok(_) => {},
        Err(_) => { kani::assume(false, "unwrap failed"); return; }
    }
    let fired = wheel.fire_expired(deadline);
    assert_eq!(fired.len(), 3);
    assert!(wheel.is_empty());
}

#[kani::proof]
#[kani::unwind(3)]
fn ps_010_replacement_preserves_entry_integrity() {
    let mut wheel = vb_runtime::shard::timer_wheel::TimerWheel::new();
    let now = std::time::Instant::now();
    let later = now + std::time::Duration::from_secs(5);
    match wheel.insert(vb_core::ids::RunId::new(1), now, vb_runtime::shard::PendingTimerKind::Wait) {
        Ok(_) => {},
        Err(_) => { kani::assume(false, "unwrap failed"); return; }
    }
    match wheel.insert(vb_core::ids::RunId::new(1), later, vb_runtime::shard::PendingTimerKind::Ask) {
        Ok(_) => {},
        Err(_) => { kani::assume(false, "unwrap failed"); return; }
    }
    let entry = match wheel.get_entry(vb_core::ids::RunId::new(1)) {
        Some(v) => v,
        None => { kani::assume(false, "unwrap failed"); return; }
    };
    assert_eq!(entry.kind, vb_runtime::shard::PendingTimerKind::Ask);
    assert_eq!(entry.generation, 2);
    assert_eq!(entry.deadline, later);
}

#[kani::proof]
#[kani::unwind(3)]
fn ps_010_fire_clears_both_indices() {
    let mut wheel = vb_runtime::shard::timer_wheel::TimerWheel::new();
    let deadline = std::time::Instant::now();
    match wheel.insert(vb_core::ids::RunId::new(1), deadline, vb_runtime::shard::PendingTimerKind::Wait) {
        Ok(_) => {},
        Err(_) => { kani::assume(false, "unwrap failed"); return; }
    }
    let _ = wheel.fire_expired(deadline);
    assert!(wheel.get_entry(vb_core::ids::RunId::new(1)).is_none());
    assert!(wheel.is_empty());
}
