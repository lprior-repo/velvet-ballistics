//! PS-004 proptest: Generation advancement (POB-vb-fzgdn-018)
//! Production binding: checked_add arithmetic pattern used in Shard::next_pending_timer_generation
//!
//! Property: checked_add(1) on values < u64::MAX always succeeds an yields gen+1.

use proptest::prelude::*;

proptest! {
    #[test]
    fn ps_004_checked_add_within_bounds(
        gen in 0u64..(u64::MAX - 1),
    ) {
        let next = gen.checked_add(1);
        prop_assert!(next.is_some());
        prop_assert_eq!(next.unwrap(), gen + 1);
    }

    #[test]
    fn ps_004_checked_add_at_max_returns_none() {
        prop_assert!(u64::MAX.checked_add(1).is_none());
    }

    #[test]
    fn ps_004_increment_is_strictly_monotonic(
        gen in 0u64..(u64::MAX - 1),
    ) {
        let next = gen.checked_add(1).unwrap();
        prop_assert!(next > gen);
    }

    #[test]
    fn ps_004_zero_to_one() {
        prop_assert_eq!(0u64.checked_add(1), Some(1));
    }
}
