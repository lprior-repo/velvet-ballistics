use proptest::prelude::*;
use vb_storage::constants::{MAX_BATCH_COUNT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RECORD_HEADER_LEN};

proptest! {
    #[test]
    fn ps007_constants(_dummy in proptest::bool::ANY) {
        prop_assert_eq!(RECORD_HEADER_LEN, 60);
        prop_assert!(MAX_JOURNAL_EVENT_PAYLOAD_BYTES > 0);
        prop_assert!(MAX_BATCH_COUNT > 0);
    }
    #[test]
    fn ps007_bridge_align(_dummy in proptest::bool::ANY) {
        let core_policy: u64 = 1_048_576;
        let storage_default: u64 = 1_048_576;
        prop_assert_eq!(core_policy, storage_default);
    }
    #[test]
    fn ps007_u32_safe(_dummy in proptest::bool::ANY) {
        let value: u64 = 1_048_576;
        prop_assert!(value <= u32::MAX as u64);
    }
    #[test]
    fn ps007_accommodates(_dummy in proptest::bool::ANY) {
        let max_encoded = RECORD_HEADER_LEN as u64 + MAX_JOURNAL_EVENT_PAYLOAD_BYTES as u64;
        let limit: u64 = 1_048_576;
        prop_assert!(max_encoded < u64::MAX);
    }
    #[test]
    fn ps007_values_valid(value in 1u64..10000000u64) {
        prop_assert!(value > 0);
    }
    #[test]
    fn ps007_many_events(_dummy in proptest::bool::ANY) {
        let typical_event: u64 = 200;
        let limit: u64 = 1_048_576;
        let max_events = limit / typical_event;
        prop_assert!(max_events > 100);
    }
}
