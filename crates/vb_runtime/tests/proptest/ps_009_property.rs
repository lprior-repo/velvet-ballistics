//! PS-009 proptest: Zero-duration / exact-deadline timer branch (POB-vb-fzgdn-040)
//! Production binding: crates/vb_runtime/src/shard/timer_wheel.rs TimerWheel::fire_expired
//!
//! Property: Timer at exact current deadline fires immediately; future timers preserved.

use proptest::prelude::*;
use vb_runtime::shard::timer_wheel::TimerWheel;
use vb_runtime::shard::PendingTimerKind;

proptest! {
    #[test]
    fn ps_009_exact_deadline_fires(
        n_runs in 1usize..10,
    ) {
        let mut wheel = TimerWheel::new();
        let deadline = std::time::Instant::now();

        for i in 0..n_runs {
            wheel.insert(vb_core::ids::RunId::new(i as u64 + 1), deadline, PendingTimerKind::Wait).unwrap();
        }

        let fired = wheel.fire_expired(deadline);
        prop_assert_eq!(fired.len(), n_runs);
        prop_assert!(wheel.is_empty());
    }

    #[test]
    fn ps_009_partial_fire_preserves_future(
        n_past in 0usize..5,
        n_future in 0usize..5,
    ) {
        let mut wheel = TimerWheel::new();
        let now = std::time::Instant::now();
        let past = now - std::time::Duration::from_millis(100);
        let future = now + std::time::Duration::from_secs(60);

        for i in 0..n_past {
            wheel.insert(vb_core::ids::RunId::new(i as u64 + 1), past, PendingTimerKind::Wait).unwrap();
        }
        for i in 0..n_future {
            wheel.insert(vb_core::ids::RunId::new(100 + i as u64), future, PendingTimerKind::Ask).unwrap();
        }

        let fired = wheel.fire_expired(now);
        prop_assert_eq!(fired.len(), n_past);
        prop_assert_eq!(wheel.len(), n_future);
    }
}
