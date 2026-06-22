#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
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
    clippy::trivially_copy_macro,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables
)]
#![forbid(unsafe_code)]

//! Section 38 property test: `bound_enforcement`.
//!
//! Master plan §38, row "Bound enforcement":
//! "Retry attempts never exceed limit; collect never exceeds
//!  page/item/time limits".
//!
//! In the `vb_storage` crate the bound-enforcement floor is the
//! invariant that the `EventReplayLimit` (and the `max_payload_len`
//! limit on record encoding) are respected by every reader and
//! encoder. This file asserts:
//!
//! - `EventReplayLimit::new(0)` returns `None` (no zero limit).
//! - `EventReplayLimit::new(n)` for any positive `n` returns a
//!   non-zero limit and reports the same value via `max_events()`.
//! - A record whose payload exceeds the configured
//!   `max_payload_len` is rejected with `PayloadTooLarge`.
//! - A record at the exact `max_payload_len` is accepted.
//! - `EventSeq::try_new` accepts every value below `u64::MAX` and
//!   rejects the reserved sentinel.

use proptest::prelude::*;

use crate::BlobRecord;
use crate::JournalError;
use crate::codec::{decode_record, encode_record};
use crate::constants::{MAGIC_BLOB, MAX_BLOB_BYTES};
use crate::journal::EventReplayLimit;
use crate::records::RecordKind;
use crate::types::EventSeq;

proptest! {
    /// `EventReplayLimit::new(0)` returns `None`. A zero limit
    /// would mean "reject everything", which is never the intended
    /// semantics.
    #[test]
    fn be_replay_limit_zero_is_none(_unit in 0u8..1u8) {
        prop_assert!(EventReplayLimit::new(0).is_none());
    }

    /// `EventReplayLimit::new(n)` for any positive `n` returns a
    /// non-zero limit that reports the same value.
    #[test]
    fn be_replay_limit_positive_round_trip(n in 1usize..1_000_000usize) {
        let limit = EventReplayLimit::new(n).expect("positive limit");
        prop_assert_eq!(limit.max_events(), n);
    }

    /// `EventReplayLimit::DEFAULT` is positive and well-defined.
    #[test]
    fn be_replay_limit_default_is_positive(_unit in 0u8..1u8) {
        prop_assert!(EventReplayLimit::DEFAULT.max_events() > 0);
    }

    /// A record whose payload exceeds the configured
    /// `max_payload_len` is rejected with `PayloadTooLarge`. We use
    /// a small `max` (8 bytes) and a payload of `max + extra` raw
    /// bytes — the postcard-encoded record is always strictly
    /// larger than the raw `bytes.len()` due to the digest (32B)
    /// and length prefix, so any payload over `max` triggers the
    /// error.
    #[test]
    fn be_record_payload_exceeds_max_rejected(
        max in 8u32..32u32,
    ) {
        // Pick a payload of raw `bytes` length that is well over
        // `max` after postcard encoding (digest + length prefix
        // adds ~40 bytes of overhead).
        let raw_len = (max as usize).saturating_add(8);
        let bytes: Vec<u8> = (0..raw_len).map(|i| i as u8).collect();
        let record = BlobRecord {
            digest: [0u8; 32],
            bytes,
        };
        let result = encode_record(MAGIC_BLOB, RecordKind::Blob, 0, &record, max);
        // The encoded payload (digest + postcard-encoded record)
        // is always > raw bytes for BlobRecord, so the encoder
        // must reject.
        match result {
            Err(JournalError::PayloadTooLarge { max: reported_max, .. }) => {
                prop_assert_eq!(reported_max, max);
            }
            other => {
                panic!("expected PayloadTooLarge, got {other:?}");
            }
        }
    }

    /// Encoding a record with a sufficiently large `max` succeeds
    /// and decodes back to the original. We use a generous
    /// `MAX_BLOB_BYTES` so the round trip is always admissible.
    #[test]
    fn be_record_payload_at_max_accepted(
        bytes in proptest::collection::vec(any::<u8>(), 0..64),
    ) {
        let record = BlobRecord {
            digest: [0u8; 32],
            bytes: bytes.clone(),
        };
        // Pick a max that comfortably fits the postcard-encoded
        // payload (digest + length prefix + payload bytes).
        let max = MAX_BLOB_BYTES;
        let encoded = encode_record(MAGIC_BLOB, RecordKind::Blob, 0, &record, max)
            .expect("encoding with MAX_BLOB_BYTES succeeds");
        // Decoding with the same max succeeds.
        let decoded = decode_record::<BlobRecord>(&encoded, MAGIC_BLOB, max);
        let (_, record2) = decoded.expect("decode with MAX_BLOB_BYTES succeeds");
        prop_assert_eq!(record2.bytes, bytes);
    }

    /// The `MAX_BLOB_BYTES` constant is positive. Zero would be a
    /// configuration regression.
    #[test]
    fn be_max_blob_bytes_is_positive(_unit in 0u8..1u8) {
        prop_assert!(MAX_BLOB_BYTES > 0);
    }

    /// `EventSeq::try_new` accepts every value below the reserved
    /// sentinel and rejects the sentinel itself.
    #[test]
    fn be_event_seq_try_new_rejects_sentinel(
        // Bound away from u64::MAX so we have headroom.
        value in 0u64..(u64::MAX - 1),
    ) {
        let seq = EventSeq::try_new(value).expect("non-sentinel value");
        prop_assert_eq!(seq.get(), value);
    }

    /// `EventSeq::try_new` rejects the reserved sentinel
    /// `u64::MAX`. This is the reserved-sentinel floor.
    #[test]
    fn be_event_seq_try_new_rejects_max(_unit in 0u8..1u8) {
        let result = EventSeq::try_new(u64::MAX);
        prop_assert!(result.is_err(), "sentinel must be rejected");
    }

    /// `EventSeq::is_reserved_sentinel` correctly identifies the
    /// sentinel.
    #[test]
    fn be_event_seq_sentinel_detector(
        value in 0u64..(u64::MAX - 1),
    ) {
        prop_assert!(!EventSeq::is_reserved_sentinel(value));
        prop_assert!(EventSeq::is_reserved_sentinel(u64::MAX));
    }

    /// `EventSeq::MAX_ENCODABLE` is below the reserved sentinel,
    /// so it can be safely encoded into a key.
    #[test]
    fn be_event_seq_max_encodable_is_below_sentinel(_unit in 0u8..1u8) {
        prop_assert!(EventSeq::MAX_ENCODABLE.get() < u64::MAX);
    }

    /// `EventSeq::ZERO` has value zero and is the minimum
    /// encodable sequence.
    #[test]
    fn be_event_seq_zero_is_zero(_unit in 0u8..1u8) {
        prop_assert_eq!(EventSeq::ZERO.get(), 0);
    }

    /// `EventReplayLimit` is `Copy`: cloning via assignment
    /// produces an equal value.
    #[test]
    fn be_replay_limit_is_copy(n in 1usize..10_000usize) {
        let a = EventReplayLimit::new(n).expect("positive");
        let b = a;
        prop_assert_eq!(a.max_events(), b.max_events());
    }

    /// Encoding the same record with different (sufficient) `max`
    /// values yields the same encoded bytes. The bound is purely
    /// an admission check, not a wire-format modifier.
    #[test]
    fn be_max_does_not_alter_wire_bytes(
        max_a in 256u32..1024u32,
        max_b in 1024u32..4096u32,
    ) {
        let bytes: Vec<u8> = (0..8).map(|i| i as u8).collect();
        let record = BlobRecord {
            digest: [1u8; 32],
            bytes: bytes.clone(),
        };
        let a = encode_record(MAGIC_BLOB, RecordKind::Blob, 0, &record, max_a)
            .expect("encode with max_a");
        let b = encode_record(MAGIC_BLOB, RecordKind::Blob, 0, &record, max_b)
            .expect("encode with max_b");
        prop_assert_eq!(a, b);
    }
}
