//! PS-010 proptest: Atomic fire + state preservation (POB-vb-fzgdn-045)
//! Production binding: crates/vb_runtime/src/shard/timer_wheel.rs TimerWheel
//!
//! Property: TimerWheel operations maintain internal consistency between
//! by_deadline and by_run indices.

use proptest::prelude::*;
use vb_runtime::shard::timer_wheel::TimerWheel;
use vb_runtime::shard::PendingTimerKind;

proptest! {
    #[test]
    fn ps_010_fire_clears_both_indices(
        n_runs in 1usize..10,
    ) {
        let mut wheel = TimerWheel::new();
        let deadline = std::time::Instant::now();

        for i in 0..n_runs {
            let run = vb_core::ids::RunId::new(i as u64 + 1);
            wheel.insert(run, deadline, PendingTimerKind::Wait).unwrap();
        }
        prop_assert_eq!(wheel.len(), n_runs);

        let _fired = wheel.fire_expired(deadline);
        prop_assert!(wheel.is_empty());
        prop_assert_eq!(wheel.len(), 0);

        // Verify no entries remain accessible
        for i in 0..n_runs {
            let run = vb_core::ids::RunId::new(i as u64 + 1);
            prop_assert!(wheel.get_entry(run).is_none());
        }
    }

    #[test]
    fn ps_010_replacement_consistency(
        run_id in 1u64..1000,
    ) {
        let mut wheel = TimerWheel::new();
        let run = vb_core::ids::RunId::new(run_id);
        let d1 = std::time::Instant::now();
        let d2 = d1 + std::time::Duration::from_secs(5);

        wheel.insert(run, d1, PendingTimerKind::Wait).unwrap();
        let g1 = wheel.get_entry(run).unwrap().generation;

        wheel.insert(run, d2, PendingTimerKind::Ask).unwrap();
        let g2 = wheel.get_entry(run).unwrap().generation;

        prop_assert_eq!(g2, g1 + 1);
        prop_assert_eq!(wheel.len(), 1);
    }

    #[test]
    fn ps_010_multiple_runs_same_deadline_all_fire(
        n_runs in 2usize..8,
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
}
