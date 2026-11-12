//! PS-005 proptest: Duplicate key handling idempotency (POB-vb-fzgdn-022)
//! Production binding: crates/vb_runtime/src/shard/timer_wheel.rs TimerWheel::insert
//!
//! Property: Duplicate run inserts always maintain exactly 1 entry and update kind.

use proptest::prelude::*;
use vb_runtime::shard::timer_wheel::TimerWheel;
use vb_runtime::shard::PendingTimerKind;

proptest! {
    #[test]
    fn ps_005_duplicate_insert_preserves_count_of_one(
        run_id in 1u64..1000,
    ) {
        let mut wheel = TimerWheel::new();
        let run = vb_core::ids::RunId::new(run_id);
        let now = std::time::Instant::now();

        for _ in 0..10 {
            wheel.insert(run, now, PendingTimerKind::Wait).unwrap();
            prop_assert_eq!(wheel.len(), 1);
        }
    }

    #[test]
    fn ps_005_cancel_then_insert_works(
        run_id in 1u64..1000,
    ) {
        let mut wheel = TimerWheel::new();
        let run = vb_core::ids::RunId::new(run_id);
        let now = std::time::Instant::now();

        wheel.insert(run, now, PendingTimerKind::Wait).unwrap();
        prop_assert!(wheel.cancel(run));
        prop_assert!(wheel.is_empty());

        wheel.insert(run, now, PendingTimerKind::Ask).unwrap();
        prop_assert_eq!(wheel.len(), 1);
        prop_assert_eq!(wheel.get_kind(run), Some(PendingTimerKind::Ask));
    }

    #[test]
    fn ps_005_cancel_nonexistent_safe(
        run_id in 1u64..1000,
    ) {
        let mut wheel = TimerWheel::new();
        prop_assert!(!wheel.cancel(vb_core::ids::RunId::new(run_id)));
    }
}
