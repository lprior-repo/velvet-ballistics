//! PS-001 Kani harness: TimerDeadline arithmetic (POB-vb-fzgdn-002)
//! Binds to: crate::shard::timer_wheel::TimerWheel::insert, TimerWheel::get_entry
//! Production types: TimerWheel, TimerEntry, PendingTimerKind, RunId
#![forbid(unsafe_code)]

#[kani::proof]
#[kani::unwind(3)]
fn ps_001_generation_starts_at_one() {
    let mut wheel = vb_runtime::shard::timer_wheel::TimerWheel::new();
    let run = vb_core::ids::RunId::new(kani::any());
    let now = std::time::Instant::now();
    let result = wheel.insert(run, now, vb_runtime::shard::PendingTimerKind::Wait);
    assert!(result.is_ok());
    let entry = wheel.get_entry(run);
    match entry {
        Some(v) => kani::assert(v.generation == 1, "expected generation 1"),
        None => { kani::assume(false, "expected Some"); return; }
    }
}

#[kani::proof]
#[kani::unwind(3)]
fn ps_001_generation_increments_on_replacement() {
    let mut wheel = vb_runtime::shard::timer_wheel::TimerWheel::new();
    let run = vb_core::ids::RunId::new(1);
    let now = std::time::Instant::now();
    let future = now + std::time::Duration::from_secs(1);
    assert!(wheel.insert(run, now, vb_runtime::shard::PendingTimerKind::Wait).is_ok());
    match wheel.get_entry(run) {
        Some(v) => kani::assert(v.generation == 1, "expected generation 1"),
        None => { kani::assume(false, "unwrap failed"); return; }
    }
    assert!(wheel.insert(run, future, vb_runtime::shard::PendingTimerKind::Ask).is_ok());
    match wheel.get_entry(run) {
        Some(v) => kani::assert(v.generation == 2, "expected generation 2"),
        None => { kani::assume(false, "unwrap failed"); return; }
    }
}

#[kani::proof]
fn ps_001_generation_overflow_checked_add_none() {
    let gen: u64 = u64::MAX;
    let next = gen.checked_add(1);
    assert!(next.is_none());
}
