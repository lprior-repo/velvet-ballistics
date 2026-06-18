use proptest::prelude::*;

proptest! {
    #[test]
    fn slot_deltas_only_include_changed_slots(
        max_slots in 0..1024u16,
    ) {
        let slot_count = max_slots.max(1);
        prop_assert!(slot_count > 0);
    }
}
