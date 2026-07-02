//! PS-008 proptest: Bounded capacity admission (POB-vb-fzgdn-036)
//! Production binding: crates/vb_runtime/src/shard/timer_wheel.rs TimerWheel len/is_empty
//!
//! Property: TimerWheel len always equals number of distinct run entries.

use proptest::prelude::*;
use vb_runtime::shard::timer_wheel::TimerWheel;
use vb_runtime::shard::PendingTimerKind;

proptest! {
    #[test]
    fn ps_008_len_equals_distinct_runs(
        n_runs in 0usize..20,
    ) {
        let mut wheel = TimerWheel::new();
        let now = std::time::Instant::now();

        for i in 0..n_runs {
            let run = vb_core::ids::RunId::new(i as u64 + 1);
            wheel.insert(run, now, PendingTimerKind::Wait).unwrap();
        }
        prop_assert_eq!(wheel.len(), n_runs);
        prop_assert_eq!(wheel.is_empty(), n_runs == 0);
    }

    #[test]
    fn ps_008_cancel_decrements_len(
        n_runs in 1usize..10,
    ) {
        let mut wheel = TimerWheel::new();
        let now = std::time::Instant::now();

        for i in 0..n_runs {
            wheel.insert(vb_core::ids::RunId::new(i as u64 + 1), now, PendingTimerKind::Wait).unwrap();
        }
        let initial = wheel.len();
        wheel.cancel(vb_core::ids::RunId::new(1));
        prop_assert_eq!(wheel.len(), initial.saturating_sub(1));
    }

    #[test]
    fn ps_008_new_wheel_is_empty() {
        let wheel = TimerWheel::new();
        prop_assert!(wheel.is_empty());
        prop_assert_eq!(wheel.len(), 0);
        prop_assert!(wheel.next_deadline().is_none());
    }
}
