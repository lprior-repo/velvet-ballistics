use proptest::prelude::*;
use vb_core::ids::SlotIdx;

proptest! {
    #[test]
    fn matching_slot_pair_accepted(match_val in 0..=255u16) {
        let asked = SlotIdx::new(match_val);
        let resume = SlotIdx::new(match_val);
        assert_eq!(asked, resume, "matching slots must be equal");
    }

    #[test]
    fn mismatching_slot_pair_rejected(a in 0..=254u16, b in 1..=255u16) {
        let asked = SlotIdx::new(a);
        let resume = SlotIdx::new(b);
        prop_assume!(a != b, "skip matching pairs");
        assert_ne!(asked, resume, "mismatched slots must be unequal");
    }

    #[test]
    fn zero_slot_matches_zero() {
        let asked = SlotIdx::ZERO;
        let resume = SlotIdx::ZERO;
        assert_eq!(asked, resume, "zero slots must match");
    }

    #[test]
    fn all_equal_slots_match(count in 0..=255u16) {
        let asked = SlotIdx::new(count);
        let resume = SlotIdx::new(count);
        assert_eq!(asked, resume, "same-value slots must match");
    }

    #[test]
    fn all_unequal_slots_rejected(a in 0..=254u16, b in 1..=255u16) {
        let asked = SlotIdx::new(a);
        let resume = SlotIdx::new(b);
        prop_assume!(a != b, "skip matching pairs");
        assert_ne!(asked, resume, "different-value slots must not match");
    }
}
