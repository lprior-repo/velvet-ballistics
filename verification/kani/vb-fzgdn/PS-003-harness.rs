//! PS-003 Kani harness: Invalid authority cannot mutate state (POB-vb-fzgdn-012)
//! Binds to: crate::shard::PendingTimer::matches_authority in types.rs
#![forbid(unsafe_code)]

#[kani::proof]
fn ps_003_rejects_wrong_generation() {
    let timer = vb_runtime::shard::PendingTimer {
        step: vb_core::ids::StepIdx::ZERO,
        kind: vb_runtime::shard::PendingTimerKind::Wait,
        generation: 42,
        deadline: std::time::Instant::now(),
    };
    let gen: u64 = kani::any();
    kani::assume(gen != 42);
    assert!(!timer.matches_authority(gen, timer.deadline, vb_runtime::shard::PendingTimerKind::Wait));
}

#[kani::proof]
fn ps_003_rejects_wrong_kind() {
    let timer = vb_runtime::shard::PendingTimer {
        step: vb_core::ids::StepIdx::ZERO,
        kind: vb_runtime::shard::PendingTimerKind::Wait,
        generation: 1,
        deadline: std::time::Instant::now(),
    };
    assert!(!timer.matches_authority(1, timer.deadline, vb_runtime::shard::PendingTimerKind::Ask));
}

#[kani::proof]
fn ps_003_rejects_wrong_deadline() {
    let timer = vb_runtime::shard::PendingTimer {
        step: vb_core::ids::StepIdx::ZERO,
        kind: vb_runtime::shard::PendingTimerKind::Wait,
        generation: 1,
        deadline: std::time::Instant::now(),
    };
    let different = timer.deadline + std::time::Duration::from_nanos(1);
    assert!(!timer.matches_authority(1, different, vb_runtime::shard::PendingTimerKind::Wait));
}

#[kani::proof]
fn ps_003_exact_match_succeeds() {
    let timer = vb_runtime::shard::PendingTimer {
        step: vb_core::ids::StepIdx::new(3),
        kind: vb_runtime::shard::PendingTimerKind::Ask,
        generation: 7,
        deadline: std::time::Instant::now(),
    };
    assert!(timer.matches_authority(7, timer.deadline, vb_runtime::shard::PendingTimerKind::Ask));
}
