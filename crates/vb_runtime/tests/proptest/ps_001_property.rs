//! PS-001 proptest: TimerDeadline arithmetic (POB-vb-fzgdn-004)
//! Production binding: crates/vb_runtime/src/shard/timer_wheel.rs TimerWheel
//!
//! Property: For any RunId and Instant deadline, TimerWheel::insert succeeds,
//! get_entry returns the entry with generation=1 on first insert.

use proptest::prelude::*;
use std::time::Instant;
use vb_core::ids::RunId;
use vb_runtime::shard::timer_wheel::TimerWheel;
use vb_runtime::shard::PendingTimerKind;

proptest! {
    #[test]
    fn ps_001_insert_sets_generation_to_one(
        run_id in 1u64..u64::MAX,
        wait in proptest::bool::ANY,
    ) {
        let mut wheel = TimerWheel::new();
        let run = RunId::new(run_id);
        let kind = if wait { PendingTimerKind::Wait } else { PendingTimerKind::Ask };
        let now = Instant::now();

        let result = wheel.insert(run, now, kind);
        prop_assert!(result.is_ok());

        let entry = wheel.get_entry(run);
        prop_assert!(entry.is_some());
        prop_assert_eq!(entry.unwrap().generation, 1);
    }

    #[test]
    fn ps_001_replacement_increments_generation(
        run_id in 1u64..1000,
    ) {
        let mut wheel = TimerWheel::new();
        let run = RunId::new(run_id);
        let now = Instant::now();
        let later = now + std::time::Duration::from_secs(1);

        wheel.insert(run, now, PendingTimerKind::Wait).unwrap();
        prop_assert_eq!(wheel.get_entry(run).unwrap().generation, 1);

        wheel.insert(run, later, PendingTimerKind::Ask).unwrap();
        prop_assert_eq!(wheel.get_entry(run).unwrap().generation, 2);
    }

    #[test]
    fn ps_001_generation_never_zero(
        run_id in 1u64..u64::MAX,
    ) {
        let mut wheel = TimerWheel::new();
        let run = RunId::new(run_id);

        for i in 0..5u64 {
            let deadline = Instant::now();
            wheel.insert(run, deadline, PendingTimerKind::Wait).unwrap();
            let entry = wheel.get_entry(run).unwrap();
            prop_assert!(entry.generation > 0, "generation={} must be > 0 after insert {}", entry.generation, i);
        }
    }
}
