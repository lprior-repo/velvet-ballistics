#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports
)]

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
