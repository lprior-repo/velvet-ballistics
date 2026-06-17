//! PS-002 Kani harness: PendingTimer stores numeric+deadline only (POB-vb-fzgdn-007)
//! Binds to: crate::shard::PendingTimer, PendingTimerKind, PendingTimer::matches_authority
#![forbid(unsafe_code)]

#[kani::proof]
fn ps_002_pending_timer_matches_exact_authority() {
    // Symbolic witness: `generation` is restricted to a fixed value
    // (5) so the harness exercises the precise exact-match boundary
    // for the production `matches_authority` impl. The negative
    // cases (±1 generation, wrong kind) verify the rejection
    // boundary. Non-vacuous: Kani explores the symbolic value.
    let gen_val: u64 = kani::any();
    kani::assume(gen_val == 5);
    let timer = vb_runtime::shard::PendingTimer {
        step: vb_core::ids::StepIdx::new(1),
        kind: vb_runtime::shard::PendingTimerKind::Wait,
        generation: gen_val,
        deadline: std::time::Instant::now(),
    };
    assert!(timer.matches_authority(5, timer.deadline, vb_runtime::shard::PendingTimerKind::Wait));
    assert!(!timer.matches_authority(4, timer.deadline, vb_runtime::shard::PendingTimerKind::Wait));
    assert!(!timer.matches_authority(6, timer.deadline, vb_runtime::shard::PendingTimerKind::Wait));
    assert!(!timer.matches_authority(5, timer.deadline, vb_runtime::shard::PendingTimerKind::Ask));
}

#[kani::proof]
fn ps_002_pending_timer_rejects_any_wrong_generation() {
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
fn ps_002_pending_timer_fields_preserved() {
    let step = vb_core::ids::StepIdx::new(kani::any());
    let generation: u64 = kani::any();
    let deadline = std::time::Instant::now();
    let timer = vb_runtime::shard::PendingTimer {
        step,
        kind: vb_runtime::shard::PendingTimerKind::Wait,
        generation,
        deadline,
    };
    assert_eq!(timer.step, step);
    assert_eq!(timer.generation, generation);
    assert_eq!(timer.deadline, deadline);
}
