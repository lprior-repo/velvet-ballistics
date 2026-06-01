
// =========================================================================
// Numeric Timer Seam — Shard-Level Unit Tests
// =========================================================================
// Tests for advance_clock_to, current_tick, next_pending_timer_generation
// on Shard using pub(crate) access to pending_timers.

fn run_numeric(id: u64) -> RunId {
    RunId::new(id)
}

// =========================================================================
// advance_clock_to tests
// =========================================================================

#[test]
fn advance_clock_to_accepts_forward_tick() {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.advance_clock_to(TimerTick::new(10)), Ok(()));
    assert_eq!(shard.current_tick(), TimerTick::new(10));
}

#[test]
fn advance_clock_to_accepts_equal_tick() {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.advance_clock_to(TimerTick::new(5)), Ok(()));
    // Advancing to the same tick is a no-op success
    assert_eq!(shard.advance_clock_to(TimerTick::new(5)), Ok(()));
    assert_eq!(shard.current_tick(), TimerTick::new(5));
}

#[test]
fn advance_clock_to_rejects_backward_tick() {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.advance_clock_to(TimerTick::new(10)), Ok(()));
    let result = shard.advance_clock_to(TimerTick::new(5));
    assert_eq!(result, Err(RuntimeError::InvalidTimerFire));
    // Current tick must be preserved after rejection
    assert_eq!(shard.current_tick(), TimerTick::new(10));
}

#[test]
fn advance_clock_to_rejects_backward_from_zero() {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.current_tick(), TimerTick::new(0));
    // Advancing to 0 from 0 should be a no-op
    assert_eq!(shard.advance_clock_to(TimerTick::new(0)), Ok(()));
}

#[test]
fn advance_clock_to_multiple_forward_steps() {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.advance_clock_to(TimerTick::new(100)), Ok(()));
    assert_eq!(shard.advance_clock_to(TimerTick::new(200)), Ok(()));
    assert_eq!(shard.advance_clock_to(TimerTick::new(500)), Ok(()));
    assert_eq!(shard.current_tick(), TimerTick::new(500));
}

#[test]
fn advance_clock_to_accepts_max_u64_tick() {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.advance_clock_to(TimerTick::new(u64::MAX)), Ok(()));
    assert_eq!(shard.current_tick(), TimerTick::new(u64::MAX));
}

#[test]
fn advance_clock_to_accepts_zero_tick_when_current_is_zero() {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.current_tick(), TimerTick::new(0));
    assert_eq!(shard.advance_clock_to(TimerTick::new(0)), Ok(()));
}

#[test]
fn advance_clock_to_rejects_slightly_backward() {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.advance_clock_to(TimerTick::new(1000)), Ok(()));
    // 999 < 1000 — should reject
    let result = shard.advance_clock_to(TimerTick::new(999));
    assert_eq!(result, Err(RuntimeError::InvalidTimerFire));
    assert_eq!(shard.current_tick(), TimerTick::new(1000));
}

// =========================================================================
// current_tick tests
// =========================================================================

#[test]
fn current_tick_starts_at_zero() {
    let shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.current_tick(), TimerTick::new(0));
}

#[test]
fn current_tick_starts_at_zero_for_custom_config() {
    let config = ShardConfig {
        command_queue_capacity: 64,
        trace_capacity: 128,
        step_budget_per_tick: 100,
        max_active_runs: 16,
        policy: vb_core::policy::RuntimePolicy::Strict,
    };
    let shard = Shard::new(config);
    assert_eq!(shard.current_tick(), TimerTick::new(0));
}

#[test]
fn current_tick_reflects_last_advance() {
    let mut shard = Shard::new(ShardConfig::default());
    for tick in &[1u64, 10, 100, 1000, 10_000] {
        assert_eq!(shard.advance_clock_to(TimerTick::new(*tick)), Ok(()));
        assert_eq!(shard.current_tick(), TimerTick::new(*tick));
    }
}

#[test]
fn current_tick_is_read_only_and_idempotent() {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.advance_clock_to(TimerTick::new(42)), Ok(()));
    // Reading current_tick multiple times gives the same value
    for _ in 0..5 {
        assert_eq!(shard.current_tick(), TimerTick::new(42));
    }
}

// =========================================================================
// next_pending_timer_generation tests
// =========================================================================

#[test]
fn next_pending_timer_generation_returns_one_for_no_timer() {
    let shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.next_pending_timer_generation(run_numeric(1)), Some(1));
}

#[test]
fn next_pending_timer_generation_returns_one_for_multiple_unknown_runs() {
    let shard = Shard::new(ShardConfig::default());
    // No timers exist, all runs get generation 1
    assert_eq!(shard.next_pending_timer_generation(run_numeric(42)), Some(1));
    assert_eq!(shard.next_pending_timer_generation(run_numeric(99)), Some(1));
}

#[test]
fn next_pending_timer_generation_increments_for_existing_timer() {
    let mut shard = Shard::new(ShardConfig::default());
    let r = run_numeric(1);
    shard.pending_timer_insert(
        r,
        PendingTimer {
            step: vb_core::ids::StepIdx::ZERO,
            kind: PendingTimerKind::Wait,
            generation: 5,
            deadline: std::time::Instant::now(),
        },
    );
    assert_eq!(shard.next_pending_timer_generation(r), Some(6));
}

#[test]
fn next_pending_timer_generation_increments_from_one_to_two() {
    let mut shard = Shard::new(ShardConfig::default());
    let r = run_numeric(1);
    shard.pending_timer_insert(
        r,
        PendingTimer {
            step: vb_core::ids::StepIdx::ZERO,
            kind: PendingTimerKind::Ask,
            generation: 1,
            deadline: std::time::Instant::now(),
        },
    );
    assert_eq!(shard.next_pending_timer_generation(r), Some(2));
}

#[test]
fn next_pending_timer_generation_returns_none_at_max_u64() {
    let mut shard = Shard::new(ShardConfig::default());
    let r = run_numeric(1);
    shard.pending_timer_insert(
        r,
        PendingTimer {
            step: vb_core::ids::StepIdx::ZERO,
            kind: PendingTimerKind::Wait,
            generation: u64::MAX,
            deadline: std::time::Instant::now(),
        },
    );
    assert_eq!(shard.next_pending_timer_generation(r), None);
}

#[test]
fn next_pending_timer_generation_does_not_mutate_on_overflow_check() {
    let mut shard = Shard::new(ShardConfig::default());
    let r = run_numeric(1);
    shard.pending_timer_insert(
        r,
        PendingTimer {
            step: vb_core::ids::StepIdx::ZERO,
            kind: PendingTimerKind::Wait,
            generation: u64::MAX,
            deadline: std::time::Instant::now(),
        },
    );
    assert_eq!(shard.next_pending_timer_generation(r), None);
    assert_eq!(shard.next_pending_timer_generation(r), None);
    // Timer is still present with original generation
    let timer = shard.pending_timer_get(r);
    assert_eq!(timer.map(|t| t.generation), Some(u64::MAX));
}

#[test]
fn next_pending_timer_generation_is_independent_per_run() {
    let mut shard = Shard::new(ShardConfig::default());
    let r1 = run_numeric(1);
    let r2 = run_numeric(2);
    shard.pending_timer_insert(
        r1,
        PendingTimer {
            step: vb_core::ids::StepIdx::ZERO,
            kind: PendingTimerKind::Wait,
            generation: 3,
            deadline: std::time::Instant::now(),
        },
    );
    shard.pending_timer_insert(
        r2,
        PendingTimer {
            step: vb_core::ids::StepIdx::new(1),
            kind: PendingTimerKind::Ask,
            generation: 7,
            deadline: std::time::Instant::now(),
        },
    );
    assert_eq!(shard.next_pending_timer_generation(r1), Some(4));
    assert_eq!(shard.next_pending_timer_generation(r2), Some(8));
    // r1 and r2 timers unchanged in registry
    assert_eq!(shard.pending_timer_get(r1).map(|t| t.generation), Some(3));
    assert_eq!(shard.pending_timer_get(r2).map(|t| t.generation), Some(7));
}

#[test]
fn next_pending_timer_generation_at_max_minus_one_returns_max() {
    let mut shard = Shard::new(ShardConfig::default());
    let r = run_numeric(1);
    shard.pending_timer_insert(
        r,
        PendingTimer {
            step: vb_core::ids::StepIdx::ZERO,
            kind: PendingTimerKind::Wait,
            generation: u64::MAX - 1,
            deadline: std::time::Instant::now(),
        },
    );
    assert_eq!(shard.next_pending_timer_generation(r), Some(u64::MAX));
}

// =========================================================================
// pending_timer_count consistency with numeric timer seam
// =========================================================================

#[test]
fn pending_timer_count_starts_at_zero_numeric_seam() {
    let shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.pending_timer_count(), 0);
}

#[test]
fn pending_timer_count_reflects_insertions_numeric_seam() {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.pending_timer_count(), 0);
    shard.pending_timer_insert(
        run_numeric(1),
        PendingTimer {
            step: vb_core::ids::StepIdx::ZERO,
            kind: PendingTimerKind::Wait,
            generation: 1,
            deadline: std::time::Instant::now(),
        },
    );
    assert_eq!(shard.pending_timer_count(), 1);
    shard.pending_timer_insert(
        run_numeric(2),
        PendingTimer {
            step: vb_core::ids::StepIdx::new(1),
            kind: PendingTimerKind::Ask,
            generation: 1,
            deadline: std::time::Instant::now(),
        },
    );
    assert_eq!(shard.pending_timer_count(), 2);
}

// =========================================================================
// Shard advance_clock_to + pending timer integration
// =========================================================================

#[test]
fn advance_clock_to_does_not_affect_pending_timers() {
    let mut shard = Shard::new(ShardConfig::default());
    let r = run_numeric(1);
    shard.pending_timer_insert(
        r,
        PendingTimer {
            step: vb_core::ids::StepIdx::ZERO,
            kind: PendingTimerKind::Wait,
            generation: 1,
            deadline: std::time::Instant::now(),
        },
    );
    let count_before = shard.pending_timer_count();
    assert_eq!(shard.advance_clock_to(TimerTick::new(100)), Ok(()));
    // Pending timers unchanged by clock advance (firing not yet wired)
    assert_eq!(shard.pending_timer_count(), count_before);
    assert!(shard.pending_timer_contains(r));
}
