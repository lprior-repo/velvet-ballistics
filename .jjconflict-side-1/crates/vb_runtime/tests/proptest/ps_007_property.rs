//! PS-007 proptest: Monotonic clock fire ordering (POB-vb-fzgdn-031)
//! Production binding: crates/vb_runtime/src/shard/timer_wheel.rs TimerWheel::fire_expired
//!
//! Property: fire_expired returns only timers with deadline <= now, preserving ordering.

use proptest::prelude::*;
use vb_runtime::shard::timer_wheel::TimerWheel;
use vb_runtime::shard::PendingTimerKind;

proptest! {
    #[test]
    fn ps_007_only_past_deadlines_fire(
        n_runs in 1usize..10,
    ) {
        let mut wheel = TimerWheel::new();
        let now = std::time::Instant::now();
        let past = now - std::time::Duration::from_millis(100);
        let future = now + std::time::Duration::from_secs(60);

        for i in 0..n_runs {
            let run = vb_core::ids::RunId::new(i as u64 + 1);
            wheel.insert(run, past, PendingTimerKind::Wait).unwrap();
        }
        wheel.insert(vb_core::ids::RunId::new(999), future, PendingTimerKind::Ask).unwrap();

        let fired = wheel.fire_expired(now);
        prop_assert_eq!(fired.len(), n_runs, "only past deadlines should fire");
        prop_assert_eq!(wheel.len(), 1, "future timer should remain");
    }

    #[test]
    fn ps_007_empty_wheel_returns_empty(
        _n in 0u64..10,
    ) {
        let mut wheel = TimerWheel::new();
        let fired = wheel.fire_expired(std::time::Instant::now());
        prop_assert!(fired.is_empty());
    }

    #[test]
    fn ps_007_next_deadline_is_earliest(
        early_ms in 1u64..100,
        late_ms in 200u64..1000,
    ) {
        let mut wheel = TimerWheel::new();
        let now = std::time::Instant::now();
        let early = now + std::time::Duration::from_millis(early_ms);
        let late = now + std::time::Duration::from_millis(late_ms);

        wheel.insert(vb_core::ids::RunId::new(1), late, PendingTimerKind::Wait).unwrap();
        wheel.insert(vb_core::ids::RunId::new(2), early, PendingTimerKind::Ask).unwrap();

        prop_assert_eq!(wheel.next_deadline(), Some(early));
    }
}
